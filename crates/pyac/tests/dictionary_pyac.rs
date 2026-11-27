//! The Pyac renderer gate: `render_pyac_source` output re-encodes to the volume.
//!
//! Each line is a record (`gi <key> na <value><case> … hwus li`). The check walks
//! the entries in order, encodes each line with the ported tablet parser, and
//! compares the flattened codelets to the entry's published codelets.
//!
//! Skips when no volume is present; see `tests/common/mod.rs`.

mod common;

use std::fs::File;

use pyac_vliw_core::text_encode::{CODELET_WIDTH, encode_sentence};

#[test]
fn rendering_as_pyac_and_re_encoding_reproduces_the_published_codelets() {
    let Some(path) = common::reference_volume_or_skip() else {
        return;
    };
    let model = pyac::dictionary::decode_dictionary(File::open(&path).expect("open volume"))
        .expect("decode volume");
    let mut reader = pyac::dictionary::DictionaryFileReader::open(File::open(&path).expect("open"))
        .expect("read volume");

    let text = pyac::dictionary::render_pyac_source(&model).expect("render pyac");

    // A glyph render must not *begin* a line with the English subject form. Checking
    // the line start, not a substring, because a surface value may contain "su ".
    for line in text.lines().take(1000) {
        assert!(
            !line.starts_with("su "),
            "pyac line starts with English subject: {line}"
        );
    }
    assert!(text.contains("zi.kyit."), "text values must be zi frames");

    // Every line belongs to an entry in order; walk entries and their line counts.
    let mut lines = text.lines();
    let (mut checked, mut matched) = (0usize, 0usize);
    for entry in &model.entries {
        let signatures: usize = entry
            .operation_signatures
            .iter()
            .map(|signature| 1 + signature.arguments.len() + signature.results.len())
            .sum();
        let count = entry.lexical_forms.len()
            + entry.source_links.len()
            + entry.source_glosses.len()
            + entry.component_keys.len()
            + entry.task_keys.len()
            + entry.references.len()
            + signatures;
        if count == 0 {
            continue;
        }
        let mut produced: Vec<u32> = Vec::new();
        for _ in 0..count {
            let line = lines.next().expect("a rendered line");
            let mut codelets = [[0u32; CODELET_WIDTH]; 128];
            let filled = encode_sentence(line, &mut codelets).expect("line parses");
            produced.extend(codelets[..filled].iter().flatten().copied());
        }
        let published: Vec<u32> = reader
            .read_entry(&entry.key)
            .expect("entry")
            .iter()
            .flatten()
            .map(|word| *word as u32)
            .collect();
        checked += 1;
        if produced == published {
            matched += 1;
        }
    }

    assert!(checked > 60_000, "checked only {checked} entries");
    assert!(
        matched * 1000 >= checked * 999,
        "pyac render reproduced {matched} of {checked} entries; expected at least 99.9%"
    );
}
