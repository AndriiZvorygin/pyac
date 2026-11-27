//! The bootstrap gate: `compile` must reproduce the published volume exactly.
//!
//! This is the test that makes this repository self-hosting. Everything else can
//! be argued about; if the source compiles to a different volume than the one
//! the project ships, then regenerating the dictionary depends on something
//! outside this tree and the extraction has not achieved its purpose.
//!
//! Byte-identity is deliberately strict. An approximate match would let a parser
//! defect through that corrupts only the entries it happens to touch, which is
//! exactly the failure mode the decoder had before
//! `dictionary_roundtrip.rs` caught it.
//!
//! Skips when either artifact is absent; see `tests/common/mod.rs`.

mod common;

use std::path::PathBuf;

use pyac::dictionary::{encode_dictionary, parse_source};

/// The authored source, from `$PYAC_REFERENCE_SOURCE` or the workspace.
fn reference_source() -> Option<PathBuf> {
    if let Some(explicit) = std::env::var_os("PYAC_REFERENCE_SOURCE") {
        let path = PathBuf::from(explicit);
        return path.is_file().then_some(path);
    }
    let root = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .and_then(|path| path.parent())
        .map(PathBuf::from)?;
    [
        root.join("dictionary/reference.pya"),
        root.join("../pyash/program/library/dictionary/reference.pya"),
    ]
    .into_iter()
    .find(|path| path.is_file())
}

/// Turn a parsed source into encoded bytes, mirroring the `compile` subcommand.
fn compile(path: &std::path::Path) -> Vec<u8> {
    let text = std::fs::read_to_string(path).expect("the reference source is readable");
    let parsed = parse_source(&text).expect("the reference source parses");
    let volume = pyac::dictionary::DictionaryVolume {
        metadata: pyac::dictionary::DictionaryMetadata {
            kind: parsed.metadata.kind,
            title: parsed.metadata.title,
            revision: parsed.metadata.revision,
            languages: parsed.metadata.languages,
            normalization_policy: parsed.metadata.normalization_policy,
        },
        entries: parsed
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
                                descriptive_specification: argument
                                    .descriptive_specification,
                            })
                            .collect(),
                        results: signature
                            .results
                            .into_iter()
                            .map(|result| pyac::dictionary::OperationResult {
                                noun_class: result.noun_class,
                                tag_label: result.tag_label,
                                descriptive_specification: result
                                    .descriptive_specification,
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
    encode_dictionary(&volume).expect("the parsed source encodes")
}

#[test]
fn compiling_the_source_reproduces_the_published_volume() {
    let (Some(source), Some(published)) = (reference_source(), common::reference_volume_or_skip())
    else {
        return;
    };

    let compiled = compile(&source);
    let original = std::fs::read(&published).expect("the published volume is readable");

    if compiled != original {
        let first = original
            .iter()
            .zip(compiled.iter())
            .position(|(left, right)| left != right);
        panic!(
            "compile is not the identity: published {} bytes, compiled {} bytes, \
             first difference at {first:?}",
            original.len(),
            compiled.len()
        );
    }
}
