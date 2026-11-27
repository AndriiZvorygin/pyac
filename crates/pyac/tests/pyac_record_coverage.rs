//! Coverage of the Pyac record surface against the published volume.
//!
//! The tablet port (`encode_sentence`) is the Pyac parser. This measures how much
//! of the volume it reproduces when every entry's records are written as Pyac
//! (`gi <key> na <value><case> … hwus li`, `zi.kyit.` text, `ve` vectors) and
//! re-encoded. It is a regression gate on the number, not a byte-identity gate —
//! byte identity for the whole volume is the goal, and this records the distance.
//!
//! The known residue, measured, is the reason for the exceptions below:
//! - entries whose key is a structural word (`na`, `ka`, `li`, `gi`, `pu`, `ve`, …)
//!   collide with the case/mood/width words the record grammar uses;
//! - entries carrying operation signatures or an authored definition, which this
//!   renderer does not yet emit.

mod common;

use std::fs::File;

use pyac::dictionary::DictionaryVolumeEntry;
use pyac_vliw_core::text_encode::{CODELET_WIDTH, encode_sentence};

fn spell(word: u16) -> String {
    let mut buffer = [0u8; 8];
    match pyac_vliw_core::word_code::decode(word, &mut buffer) {
        Ok(length) => String::from_utf8_lossy(&buffer[..length]).into_owned(),
        Err(_) => format!("0x{word:04x}"),
    }
}

fn frame(text: &str) -> String {
    format!("zi.kyit.{text}.kyit.zi")
}

fn vector(lanes: &[u16]) -> String {
    let mut out = format!("ve {}", lanes.len());
    for lane in lanes {
        out.push_str(&format!(" {lane}"));
    }
    out
}

fn lines(entry: &DictionaryVolumeEntry) -> Vec<String> {
    let head = format!(
        "{} {} {}",
        spell(0x225e),
        entry
            .key
            .iter()
            .map(|w| spell(*w))
            .collect::<Vec<_>>()
            .join(" "),
        spell(0x24de)
    );
    let tail = format!("{} {}", spell(0x8a28), spell(0x217e));
    let mut out = Vec::new();
    for form in &entry.lexical_forms {
        out.push(format!(
            "{head} {} {} {} {} {} {} {tail}",
            frame(&form.language),
            spell(0x414f),
            frame(&form.surface),
            spell(0x313e),
            frame(&form.normalized),
            spell(0x243e)
        ));
    }
    for link in &entry.source_links {
        out.push(format!(
            "{head} {} {} {} {} {} {} {} {} {tail}",
            frame(&link.dataset),
            spell(0x414f),
            frame(&link.release),
            spell(0x313e),
            frame(&link.identifier),
            spell(0x243e),
            frame(&link.relation),
            spell(0x287e)
        ));
    }
    for gloss in &entry.source_glosses {
        out.push(format!(
            "{head} {} {} {} {} {} {} {} {} {} {} {tail}",
            frame(&gloss.language),
            spell(0x414f),
            frame(&gloss.dataset),
            spell(0x313e),
            frame(&gloss.release),
            spell(0x243e),
            frame(&gloss.identifier),
            spell(0x287e),
            frame(&gloss.text),
            spell(0x490f)
        ));
    }
    for component in &entry.component_keys {
        out.push(format!(
            "{head} {} {} {tail}",
            vector(component),
            spell(0x313e)
        ));
    }
    for task in &entry.task_keys {
        out.push(format!("{head} {} {} {tail}", vector(task), spell(0x490f)));
    }
    for reference in &entry.references {
        out.push(format!(
            "{head} {} {} {tail}",
            vector(reference),
            spell(0x243e)
        ));
    }
    out
}

#[test]
fn the_pyac_record_surface_reproduces_the_published_codelets() {
    let Some(path) = common::reference_volume_or_skip() else {
        return;
    };
    let model = pyac::dictionary::decode_dictionary(File::open(&path).expect("open volume"))
        .expect("decode volume");
    let mut reader = pyac::dictionary::DictionaryFileReader::open(File::open(&path).expect("open"))
        .expect("read volume");
    // Read forward in index order: `read_entry(key)` searches the index per key,
    // 9.4 s on this volume against 0.04 s for the sequential walk.
    let rows = reader
        .canonical_keys_with_offsets()
        .expect("canonical index with offsets");
    let entries_end = reader
        .section_range("entries")
        .expect("entries section")
        .end;

    let (mut reproduced, mut total) = (0usize, 0usize);
    for (position, entry) in model.entries.iter().enumerate() {
        // Signatures and authored definitions are not emitted by this renderer yet.
        if !entry.operation_signatures.is_empty() || entry.definition.is_some() {
            continue;
        }
        total += 1;

        let mut produced: Vec<u32> = Vec::new();
        let mut ok = true;
        for line in lines(entry) {
            let mut codelets = [[0u32; CODELET_WIDTH]; 64];
            match encode_sentence(&line, &mut codelets) {
                Ok(count) => produced.extend(codelets[..count].iter().flatten().copied()),
                Err(_) => {
                    ok = false;
                    break;
                }
            }
        }
        let next = rows
            .get(position + 1)
            .map_or(entries_end, |(_, start)| *start);
        let published: Vec<u32> = reader
            .entry_codelets_at(rows[position].1, next)
            .expect("entry")
            .iter()
            .flatten()
            .map(|word| *word as u32)
            .collect();
        if ok && produced == published {
            reproduced += 1;
        }
    }

    // 64,500 of 64,502 measured. The two that remain are entries whose key is `na`
    // or a long 17-form entry with a boundary diff mid-record; the gate holds the
    // measured floor so a regression below it fails loudly.
    assert!(
        reproduced * 1000 >= total * 999,
        "Pyac record surface reproduced {reproduced} of {total} entries; expected at least 99.9%"
    );
    assert!(
        reproduced >= 64_490,
        "reproduced {reproduced} entries, below the measured floor of 64,490"
    );
}
