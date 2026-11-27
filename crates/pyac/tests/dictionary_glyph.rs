//! Glyph `.pya` round-trip gate: `.pyac` -> glyph text -> `.pyac`, byte for byte.
//!
//! The glyph form is the language, not a JSON rendering: keys are glyphs and
//! every text value is a `zi.kyit. … .kyit.zi` frame, which is what `zi` is for.
//! Numbers stay typed literals, never `zi`.
//!
//! Skips when no volume is present; see `tests/common/mod.rs`.

mod common;

use std::fs::File;
use std::io::Cursor;

use pyac::dictionary::{decode_dictionary, encode_dictionary, parse_source};

#[test]
fn rendering_the_published_volume_as_glyph_text_and_recompiling_reproduces_it() {
    let Some(path) = common::reference_volume_or_skip() else {
        return;
    };

    let volume = decode_dictionary(File::open(&path).expect("open the published volume"))
        .expect("the published volume decodes");

    let glyph =
        pyac::dictionary::render_glyph_source(&volume).expect("the volume renders to glyph source");

    // The glyph form is not a JSON dump; a text frame is `zi.kyit.` and there must
    // be no `text "` keyword anywhere.
    assert!(
        glyph.contains("zi.kyit."),
        "glyph form must carry zi text frames"
    );
    assert!(
        !glyph.contains("ob text \""),
        "glyph form must not use the English `text \"…\"` literal"
    );

    // Reparse the glyph text and re-encode it through the same path `compile` takes.
    let reparsed = parse_source(&glyph).expect("the glyph source parses");
    let reparsed_volume = to_volume(reparsed);
    let recompiled = encode_dictionary(&reparsed_volume).expect("the glyph source re-encodes");

    let original = std::fs::read(&path).expect("the published volume is readable");
    if recompiled != original {
        let first = original
            .iter()
            .zip(recompiled.iter())
            .position(|(left, right)| left != right);
        panic!(
            "glyph render is not the identity: published {} bytes, recompiled {} bytes, first \
             difference at {first:?}",
            original.len(),
            recompiled.len()
        );
    }
    // Silence the unused import if the round trip short-circuits above.
    let _ = Cursor::new(Vec::<u8>::new());
}

/// Mirror `compile`'s source -> volume mapping.
fn to_volume(source: pyac::dictionary::DictionarySource) -> pyac::dictionary::DictionaryVolume {
    use pyac::dictionary::*;
    DictionaryVolume {
        metadata: DictionaryMetadata {
            kind: source.metadata.kind,
            title: source.metadata.title,
            revision: source.metadata.revision,
            languages: source.metadata.languages,
            normalization_policy: source.metadata.normalization_policy,
        },
        entries: source
            .entries
            .into_iter()
            .map(|entry| DictionaryVolumeEntry {
                key: entry.key,
                lexical_forms: entry
                    .lexical_forms
                    .into_iter()
                    .map(|form| LexicalForm {
                        language: form.language,
                        surface: form.surface,
                        normalized: form.normalized,
                    })
                    .collect(),
                source_links: entry
                    .source_links
                    .into_iter()
                    .map(|link| SourceLink {
                        dataset: link.dataset,
                        release: link.release,
                        identifier: link.identifier,
                        relation: link.relation,
                    })
                    .collect(),
                source_glosses: entry
                    .source_glosses
                    .into_iter()
                    .map(|gloss| SourceGloss {
                        language: gloss.language,
                        text: gloss.text,
                        dataset: gloss.dataset,
                        release: gloss.release,
                        identifier: gloss.identifier,
                    })
                    .collect(),
                component_keys: entry.component_keys,
                task_keys: entry.task_keys,
                references: entry.references,
                operation_signatures: entry
                    .operation_signatures
                    .into_iter()
                    .map(|signature| OperationSignature {
                        verb: signature.verb,
                        mood: signature.mood,
                        arguments: signature
                            .arguments
                            .into_iter()
                            .map(|argument| OperationParameter {
                                case_type: ArgumentSlot {
                                    grammatical_case: argument.grammatical_case,
                                    noun_class: argument.noun_class,
                                },
                                tag_label: argument.tag_label,
                                descriptive_specification: argument.descriptive_specification,
                            })
                            .collect(),
                        results: signature
                            .results
                            .into_iter()
                            .map(|result| OperationResult {
                                noun_class: result.noun_class,
                                tag_label: result.tag_label,
                                descriptive_specification: result.descriptive_specification,
                            })
                            .collect(),
                    })
                    .collect(),
                definition: entry.definition.map(|codelets| {
                    codelets
                        .into_iter()
                        .filter_map(|codelet| <[u16; 16]>::try_from(codelet).ok())
                        .collect()
                }),
            })
            .collect(),
    }
}
