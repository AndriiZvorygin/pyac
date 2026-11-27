//! The gate on the source renderer: `.pyac` -> `.en.pya` -> `.pyac` must be the
//! identity, byte for byte.
//!
//! `dictionary_roundtrip.rs` proves the model survives decode/encode. This proves
//! the *source text* survives it, which is stricter: it exercises the source
//! writer, the parser's inverse, and every record kind against the whole published
//! vocabulary, not a fixture. A renderer that drops a field or escapes text
//! differently would still decode/encode cleanly and fail only here.
//!
//! Skips when no volume is present; see `tests/common/mod.rs`.

mod common;

use std::fs::File;

use pyac::dictionary::{decode_dictionary, render_source};

#[test]
fn rendering_the_published_volume_and_recompiling_reproduces_it() {
    let Some(path) = common::reference_volume_or_skip() else {
        return;
    };

    let volume = decode_dictionary(File::open(&path).expect("open the published volume"))
        .expect("the published volume decodes");

    let rendered = render_source(&volume).expect("the volume renders to source text");

    // Re-parse the rendered source and re-encode it through the same path
    // `compile` takes. This is the whole point: text out, text back in, same bytes.
    let reparsed = pyac::dictionary::parse_source(&rendered).expect("the rendered source parses");
    let recompiled = compile_source(reparsed);

    let original = std::fs::read(&path).expect("the published volume is readable");
    if recompiled != original {
        let first = original
            .iter()
            .zip(recompiled.iter())
            .position(|(left, right)| left != right);
        panic!(
            "render is not the identity: published {} bytes, recompiled {} bytes, first \
             difference at {first:?}",
            original.len(),
            recompiled.len()
        );
    }
}

/// Mirror `compile`'s source -> volume -> bytes path.
fn compile_source(source: pyac::dictionary::DictionarySource) -> Vec<u8> {
    let volume = pyac::dictionary::DictionaryVolume {
        metadata: pyac::dictionary::DictionaryMetadata {
            kind: source.metadata.kind,
            title: source.metadata.title,
            revision: source.metadata.revision,
            languages: source.metadata.languages,
            normalization_policy: source.metadata.normalization_policy,
        },
        entries: source
            .entries
            .into_iter()
            .map(|entry| pyac::dictionary::DictionaryVolumeEntry {
                key: entry.key,
                lexical_forms: entry
                    .lexical_forms
                    .into_iter()
                    .map(|form| pyac::dictionary::LexicalForm {
                        language: form.language,
                        surface: form.surface,
                        normalized: form.normalized,
                    })
                    .collect(),
                source_links: entry
                    .source_links
                    .into_iter()
                    .map(|link| pyac::dictionary::SourceLink {
                        dataset: link.dataset,
                        release: link.release,
                        identifier: link.identifier,
                        relation: link.relation,
                    })
                    .collect(),
                source_glosses: entry
                    .source_glosses
                    .into_iter()
                    .map(|gloss| pyac::dictionary::SourceGloss {
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
                    .map(|signature| pyac::dictionary::OperationSignature {
                        verb: signature.verb,
                        mood: signature.mood,
                        arguments: signature
                            .arguments
                            .into_iter()
                            .map(|argument| pyac::dictionary::OperationParameter {
                                case_type: pyac::dictionary::ArgumentSlot {
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
                            .map(|result| pyac::dictionary::OperationResult {
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
    };
    pyac::dictionary::encode_dictionary(&volume).expect("the rendered source re-encodes")
}
