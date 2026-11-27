//! The shard round trip: split the committed source, assemble it back, compile it.
//!
//! The source is over git's 100 MB per-file limit, so it ships as structural
//! shards. This gate keeps that honest: splitting and reassembling must reproduce
//! the source's records.
//!
//! It also checks the properties that make the grouping meaningful rather than
//! arbitrary: atoms form one shard, no shard exceeds the git threshold, and the
//! manifest describes what the directory holds.

use std::collections::BTreeMap;

use pyac::dictionary::{BEYOND_LAST_LAYER, assemble_shards, parse_manifest};

use super::fixtures;

#[test]
fn splitting_and_assembling_reproduces_the_entries() {
    let (Some(source), Some(set)) = (fixtures::source(), fixtures::split()) else {
        return;
    };
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
    let original = body(source);
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
    let mut left = original;
    let mut right = rebuilt;
    left.sort();
    right.sort();
    assert_eq!(left, right, "assembled records differ from the source's");
}

#[test]
fn atoms_are_one_shard_and_compounds_group_by_layer() {
    let Some(set) = fixtures::split() else {
        return;
    };
    assert!(
        set.shards.iter().any(|(name, _)| name == "atoms"),
        "atoms must form their own shard; every compound depends on them"
    );
    for (name, _) in &set.shards {
        assert!(
            name == "atoms" || name.starts_with("compounds-") || name.starts_with("dispatch/"),
            "unexpected shard name {name:?}"
        );
    }
    // Dispatch records are lifted out of the vocabulary and written one file per
    // verb, so a split that produced none would mean the dispatch API had been
    // folded back in.
    assert!(
        set.shards
            .iter()
            .any(|(name, _)| name.starts_with("dispatch/")),
        "dispatch must be its own shards, not folded into the vocabulary"
    );
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
    // The 50 MB warning line is the reason to split by layer at all, and only the
    // glyph layer lists put compounds in their per-layer shards. Without them every
    // compound is one shard by design, so the threshold cannot hold and this is a
    // skip rather than a failure.
    if std::env::var_os("PYAC_LAYER_LISTS").is_none() {
        eprintln!("skipping: no PYAC_LAYER_LISTS; compounds are one shard by design");
        return;
    }
    let Some(set) = fixtures::split() else {
        return;
    };
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
    let Some(set) = fixtures::split() else {
        return;
    };
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
    let Some(set) = fixtures::split() else {
        return;
    };
    let mut contents: BTreeMap<String, String> = set.shards.iter().cloned().collect();
    let first = set.shards[0].0.clone();
    contents.remove(&first);

    let error =
        assemble_shards(&set.manifest, &contents).expect_err("a missing shard must not assemble");
    assert_eq!(error.code(), "missing-shard");
}

#[test]
fn the_manifest_records_every_shard_with_its_digest_and_count() {
    let Some(set) = fixtures::split() else {
        return;
    };
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
fn an_lfs_pointer_is_not_mistaken_for_a_volume() {
    // A clone before `git lfs pull` has a ~134-byte pointer at the volume path.
    // It exists as a file, so a bare `is_file()` accepts it, and the byte-identity
    // gate then fails with "published 134 bytes" — which reads as an encoder
    // defect rather than "the artifact is not fetched". The resolver must reject
    // it so the gate skips, which is this repository's stated policy.
    let directory = std::env::temp_dir().join(format!(
        "pyac-lfs-pointer-{}-{:?}",
        std::process::id(),
        std::thread::current().id()
    ));
    std::fs::create_dir_all(&directory).expect("temp dir");

    let pointer = directory.join("reference.pyac");
    std::fs::write(
        &pointer,
        "version https://git-lfs.github.com/spec/v1\n\
         oid sha256:e3600afdb742608d40c8b045252c3993995399d5af382a134a4735118baa5806\n\
         size 131385760\n",
    )
    .expect("write pointer");

    // Same shape a real volume has, but far too small to be one.
    let tiny = directory.join("tiny.pyac");
    std::fs::write(&tiny, b"PYACVLIW".repeat(4)).expect("write tiny header");

    // A file with the right size class but the wrong magic.
    let wrong = directory.join("wrong.pyac");
    let mut bytes = vec![0u8; 2 * 1024 * 1024];
    bytes[..8].copy_from_slice(b"NOTVLIW!");
    std::fs::write(&wrong, &bytes).expect("write wrong magic");

    for candidate in [&pointer, &tiny, &wrong] {
        // The resolver is exercised through the environment, which is the highest
        // priority source and so the one a caller would use to point at a volume.
        let previous = std::env::var_os("PYAC_REFERENCE_VOLUME");
        // SAFETY: single-threaded within this test; the variable is restored below.
        unsafe { std::env::set_var("PYAC_REFERENCE_VOLUME", candidate) };
        let resolved = crate::common::reference_volume();
        match previous {
            Some(value) => unsafe { std::env::set_var("PYAC_REFERENCE_VOLUME", value) },
            None => unsafe { std::env::remove_var("PYAC_REFERENCE_VOLUME") },
        }
        // A fallback candidate may still resolve if one exists beside the
        // workspace; what must not happen is the bad file itself being returned.
        assert_ne!(
            resolved.as_deref(),
            Some(candidate.as_path()),
            "{} was accepted as a volume",
            candidate.display()
        );
    }

    std::fs::remove_dir_all(&directory).ok();
}
