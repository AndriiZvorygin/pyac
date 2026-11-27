//! The dispatch split: signatures are separable from vocabulary, and one verb one file.
//!
//! A ceremony signature says what an operation accepts. It changes when the
//! dispatch API changes, not when the vocabulary does, so it is separated:
//! otherwise every vocabulary edit reflows the signature text and every signature
//! change reflows a vocabulary shard.

use std::collections::{BTreeMap, BTreeSet};

use pyac::dictionary::assemble_shards;

use super::fixtures;

#[test]
fn dispatch_is_its_own_shard_and_carries_no_vocabulary() {
    let Some(set) = fixtures::split() else {
        return;
    };
    let dispatch: Vec<&String> = set
        .shards
        .iter()
        .filter(|(name, _)| name.starts_with("dispatch/"))
        .map(|(_, body)| body)
        .collect();
    assert!(!dispatch.is_empty(), "dispatch is split per entry");
    for body in &dispatch {
        assert!(
            body.contains(" at text \"operation-signature\" be text ya"),
            "a dispatch shard carries signatures"
        );
        // No entry block may appear in a dispatch shard: it annotates entries, it
        // does not declare them.
        assert!(
            !body.contains(" at text \"entry\" be text ya"),
            "a dispatch shard must not declare entries"
        );
    }

    // And the vocabulary shards must carry no dispatch records.
    for (name, body) in &set.shards {
        if name.starts_with("dispatch/") {
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
    let (Some(source), Some(set)) = (fixtures::source(), fixtures::split()) else {
        return;
    };
    let contents: BTreeMap<String, String> = set.shards.iter().cloned().collect();
    let assembled = assemble_shards(&set.manifest, &contents).expect("the shards assemble");

    let count = |text: &str, needle: &str| text.matches(needle).count();
    let records = " at text \"operation-signature\" be text ya";
    assert_eq!(
        count(&assembled, records),
        count(source, records),
        "assembly lost or duplicated signature records"
    );
    let entries = " at text \"entry\" be text ya";
    assert_eq!(
        count(&assembled, entries),
        count(source, entries),
        "assembly lost or duplicated entry records"
    );
}

#[test]
fn each_operation_has_exactly_one_verb_and_one_dispatch_file() {
    // This is what makes `dispatch/<verb>.en.pya` the right naming. A signature's
    // verb is its entry's spelling: measured, 96 entries carry signatures and
    // there are 96 distinct verbs, with no entry holding two and no verb shared.
    let Some(set) = fixtures::split() else {
        return;
    };

    // (entry key, verb words) for every signature record.
    let mut pairs: Vec<(String, Vec<u16>)> = Vec::new();
    for (name, body) in &set.shards {
        if !name.starts_with("dispatch/") {
            continue;
        }
        for line in body.lines() {
            if !line.contains(" at text \"operation-signature\" be text ya") {
                continue;
            }
            let Some(key) = line
                .split("ob text \"")
                .nth(1)
                .and_then(|rest| rest.split('"').next())
            else {
                continue;
            };
            let Some(vector) = line.split(" from ve ").nth(1) else {
                continue;
            };
            // `ve <count> <word> <word> ...`: the count is the run length, and the
            // first word of the vector is the verb's own length.
            let mut numbers = vector
                .split_whitespace()
                .filter_map(|token| token.parse::<u16>().ok());
            let Some(_run_length) = numbers.next() else {
                continue;
            };
            let Some(verb_length) = numbers.next() else {
                continue;
            };
            let verb: Vec<u16> = numbers.take(verb_length as usize).collect();
            if verb.len() == verb_length as usize {
                pairs.push((key.to_owned(), verb));
            }
        }
    }
    assert!(!pairs.is_empty(), "found no signature records to check");

    let mut entries: Vec<&str> = pairs.iter().map(|(key, _)| key.as_str()).collect();
    entries.sort_unstable();
    entries.dedup();
    let mut verbs: Vec<Vec<u16>> = pairs.iter().map(|(_, verb)| verb.clone()).collect();
    verbs.sort_unstable();
    verbs.dedup();

    assert_eq!(
        entries.len(),
        verbs.len(),
        "{} entries carry signatures but there are {} distinct verbs; \
         if these differ, `dispatch/<verb>.en.pya` would collide or fragment",
        entries.len(),
        verbs.len()
    );

    // Each entry must own exactly one verb, so its file is unambiguous.
    let mut by_entry: BTreeMap<&str, BTreeSet<&Vec<u16>>> = BTreeMap::new();
    for (key, verb) in &pairs {
        by_entry.entry(key.as_str()).or_default().insert(verb);
    }
    for (key, set) in &by_entry {
        assert_eq!(
            set.len(),
            1,
            "entry {key:?} carries {} distinct verbs, so one file cannot name it",
            set.len()
        );
    }
}
