//! The Pyac renderer gate: `render_pyac_source` re-encodes to the published codelets.
//!
//! Each rendered line is a record (`gi <key> na <value><case> … hwus li`). This
//! encodes every line with the ported tablet parser and compares the flattened
//! codelets to the entry's published codelets, entry by entry.
//!
//! It is the gate that distinguishes the glyph form from English-with-`zi`-frames:
//! the glyph form starts with `gi`, never `su name`.

use super::fixtures;

use pyac_vliw_core::text_encode::{CODELET_WIDTH, encode_sentence};

#[test]
fn rendering_as_pyac_and_re_encoding_reproduces_the_published_codelets() {
    let (Some(model), Some(path)) = (fixtures::volume(), fixtures::reference_volume()) else {
        return;
    };
    let mut reader =
        pyac::dictionary::DictionaryFileReader::open(std::fs::File::open(path).expect("open"))
            .expect("read volume");
    // Read the entry bodies forward, in index order. `read_entry(key)` searches the
    // index for every key, and the walk is already ordered.
    let rows = reader
        .canonical_keys_with_offsets()
        .expect("canonical index with offsets");
    let entries_end = reader
        .section_range("entries")
        .expect("entries section")
        .end;

    let text = pyac::dictionary::render_pyac_source(model).expect("render pyac");

    // A glyph render must not *begin* a line with the English subject form. The
    // line start, not a substring: a surface value may contain "su ".
    for line in text.lines().take(1000) {
        assert!(
            !line.starts_with("su "),
            "pyac line starts with English subject: {line}"
        );
    }
    assert!(text.contains("zi.kyit."), "text values must be zi frames");

    // Every line belongs to an entry, in order; walk entries and their line counts.
    let mut lines = text.lines();
    let (mut checked, mut matched) = (0usize, 0usize);
    for (position, entry) in model.entries.iter().enumerate() {
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
