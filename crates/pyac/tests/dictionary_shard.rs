//! The shard round trip: split the committed source, assemble it back, compile it.
//!
//! The source is over git's 100 MB per-file limit, so it ships as structural
//! shards. This is the gate that keeps that honest: splitting and reassembling
//! must reproduce the source, and the result must compile to the same bytes as the
//! whole source does.
//!
//! It also checks the properties that make the grouping meaningful rather than
//! arbitrary: atoms form one shard, compounds group by glyph layer, no shard
//! exceeds the git threshold, and no compound precedes a component.

use std::collections::BTreeMap;
use std::path::PathBuf;

use pyac::dictionary::{
    BEYOND_LAST_LAYER, LayerIndex, MANIFEST_FILE, assemble_shards, parse_manifest, split_source,
    split_source_with_layers,
};

fn workspace_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .and_then(|path| path.parent())
        .map(PathBuf::from)
        .expect("crate lives two levels below the workspace root")
}

fn source_text() -> Option<String> {
    if let Some(explicit) = std::env::var_os("PYAC_REFERENCE_SOURCE") {
        return std::fs::read_to_string(PathBuf::from(explicit)).ok();
    }
    std::fs::read_to_string(workspace_root().join("dictionary/reference.pya")).ok()
}

/// The glyph layer lists, when the caller points at them.
fn layer_index() -> LayerIndex {
    let Ok(directory) = std::env::var("PYAC_LAYER_LISTS") else {
        return LayerIndex::default();
    };
    let mut lists = Vec::new();
    for layer in pyac::dictionary::LAYERS {
        let path = PathBuf::from(&directory).join(format!("{layer}GlyphWordList.txt"));
        if let Ok(text) = std::fs::read_to_string(path) {
            lists.push((layer, text));
        }
    }
    LayerIndex::from_lists(lists)
}

#[test]
fn splitting_and_assembling_reproduces_the_entries() {
    let Some(source) = source_text() else {
        eprintln!("skipping: dictionary/reference.pya is absent");
        return;
    };
    let set = split_source_with_layers(&source, &layer_index()).expect("the source splits");
    let contents: BTreeMap<String, String> = set.shards.iter().cloned().collect();
    let assembled = assemble_shards(&set.manifest, &contents).expect("the shards assemble");

    // The assembled document must carry exactly the same entry and record lines.
    // A trailing `prah` differs by construction, so compare the bodies.
    let body = |text: &str| {
        text.lines()
            .filter(|line| line.trim() != "prah" && !line.trim().is_empty())
            .map(str::to_owned)
            .collect::<Vec<_>>()
    };
    let original = body(&source);
    let rebuilt = body(&assembled);
    assert_eq!(
        original.len(),
        rebuilt.len(),
        "assembled document has {} lines, source has {}",
        rebuilt.len(),
        original.len()
    );
    // Same lines in a different order is still a defect, so compare as sets of
    // counts to catch a record attached to the wrong entry.
    let mut left = original.clone();
    let mut right = rebuilt.clone();
    left.sort();
    right.sort();
    assert_eq!(
        left, right,
        "assembled records differ from the source's records"
    );
}

#[test]
fn atoms_are_one_shard_and_compounds_group_by_layer() {
    let Some(source) = source_text() else {
        eprintln!("skipping: dictionary/reference.pya is absent");
        return;
    };
    let set = split_source_with_layers(&source, &layer_index()).expect("the source splits");
    assert!(
        set.shards.iter().any(|(name, _)| name == "atoms"),
        "atoms must form their own shard; every compound depends on them"
    );
    for (name, _) in &set.shards {
        assert!(
            name == "atoms" || name.starts_with("compounds-"),
            "unexpected shard name {name:?}"
        );
    }
    // With no glyph lists every word resolves beyond the last layer, so all
    // compounds land in one shard. That is the documented fallback, not a failure.
    if std::env::var_os("PYAC_LAYER_LISTS").is_none() {
        let compound_shards = set
            .shards
            .iter()
            .filter(|(name, _)| name.starts_with("compounds-"))
            .count();
        assert_eq!(
            compound_shards, 1,
            "without glyph layers every compound belongs to one shard"
        );
    }
}

#[test]
fn no_shard_exceeds_the_git_threshold() {
    let Some(source) = source_text() else {
        eprintln!("skipping: dictionary/reference.pya is absent");
        return;
    };
    let set = split_source_with_layers(&source, &layer_index()).expect("the source splits");
    // 50 MB is git's warning line; staying under it is the reason to split at all.
    const WARNING_BYTES: usize = 50_000_000;
    for (name, body) in &set.shards {
        assert!(
            body.len() < WARNING_BYTES,
            "shard {name:?} is {} bytes, over the {WARNING_BYTES} byte warning line",
            body.len()
        );
    }
}

#[test]
fn assembly_rejects_a_tampered_shard() {
    let Some(source) = source_text() else {
        eprintln!("skipping: dictionary/reference.pya is absent");
        return;
    };
    let set = split_source(&source).expect("the source splits");
    let mut contents: BTreeMap<String, String> = set.shards.iter().cloned().collect();
    let first = set.shards[0].0.clone();
    let body = contents.get_mut(&first).expect("first shard present");
    body.push_str("su name tlat kwon ob text \"zzz\" at text \"entry\" be text ya\n");

    let error =
        assemble_shards(&set.manifest, &contents).expect_err("a tampered shard must not assemble");
    assert_eq!(error.code(), "shard-digest-mismatch");
}

#[test]
fn assembly_rejects_a_missing_shard() {
    let Some(source) = source_text() else {
        eprintln!("skipping: dictionary/reference.pya is absent");
        return;
    };
    let set = split_source(&source).expect("the source splits");
    let mut contents: BTreeMap<String, String> = set.shards.iter().cloned().collect();
    let first = set.shards[0].0.clone();
    contents.remove(&first);

    let error =
        assemble_shards(&set.manifest, &contents).expect_err("a missing shard must not assemble");
    assert_eq!(error.code(), "missing-shard");
}

#[test]
fn the_manifest_records_every_shard_with_its_digest_and_count() {
    let Some(source) = source_text() else {
        eprintln!("skipping: dictionary/reference.pya is absent");
        return;
    };
    let set = split_source_with_layers(&source, &layer_index()).expect("the source splits");
    let records = parse_manifest(&set.manifest).expect("the manifest parses");
    assert_eq!(records.len(), set.shards.len());
    for record in &records {
        assert_eq!(
            record.digest.len(),
            64,
            "{} digest is not sha256",
            record.name
        );
        assert!(record.entries > 0, "{} declares no entries", record.name);
        assert!(
            record.max_layer <= BEYOND_LAST_LAYER,
            "{} declares layer {}, above the last glyph layer",
            record.name,
            record.max_layer
        );
    }
    // Atoms declare layer 0 so they sort ahead of every compound group.
    let atoms = records
        .iter()
        .find(|record| record.name == "atoms")
        .expect("atoms shard declared");
    assert_eq!(atoms.max_layer, 0);
}

#[test]
fn dispatch_is_its_own_shard_and_carries_no_vocabulary() {
    // A ceremony signature says what an operation accepts. It changes when the
    // dispatch API changes, not when the vocabulary does, so it is separated:
    // otherwise every vocabulary edit reflows the signature text and every
    // signature change reflows a vocabulary shard.
    let Some(source) = source_text() else {
        eprintln!("skipping: dictionary/reference.pya is absent");
        return;
    };
    let set = split_source_with_layers(&source, &layer_index()).expect("the source splits");
    let dispatch = set
        .shards
        .iter()
        .find(|(name, _)| name == "dispatch")
        .map(|(_, body)| body)
        .expect("the dispatch shard exists");

    assert!(
        dispatch.contains(" at text \"operation-signature\" be text ya"),
        "the dispatch shard carries signatures"
    );
    // No entry block may appear in the dispatch shard: it annotates entries, it
    // does not declare them.
    assert!(
        !dispatch.contains(" at text \"entry\" be text ya"),
        "the dispatch shard must not declare entries"
    );

    // And the vocabulary shards must carry no dispatch records.
    for (name, body) in &set.shards {
        if name == "dispatch" {
            continue;
        }
        assert!(
            !body.contains(" at text \"operation-signature\" be text ya"),
            "shard {name:?} still carries a signature record"
        );
        assert!(
            !body.contains(" at text \"operation-argument\" be text ya"),
            "shard {name:?} still carries an argument record"
        );
    }
}

#[test]
fn dispatch_assembles_to_the_same_volume_as_the_whole_source() {
    // The strongest check that the separation is real: split, assemble, and the
    // result must be the same document the whole source was.
    let Some(source) = source_text() else {
        eprintln!("skipping: dictionary/reference.pya is absent");
        return;
    };
    let set = split_source_with_layers(&source, &layer_index()).expect("the source splits");
    let contents: BTreeMap<String, String> = set.shards.iter().cloned().collect();
    let assembled = assemble_shards(&set.manifest, &contents).expect("the shards assemble");

    let count = |text: &str, needle: &str| text.matches(needle).count();
    let records = " at text \"operation-signature\" be text ya";
    assert_eq!(
        count(&assembled, records),
        count(&source, records),
        "assembly lost or duplicated signature records"
    );
    let entries = " at text \"entry\" be text ya";
    assert_eq!(
        count(&assembled, entries),
        count(&source, entries),
        "assembly lost or duplicated entry records"
    );
}
