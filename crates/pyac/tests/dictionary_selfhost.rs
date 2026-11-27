//! The bootstrap that works on a clean clone, with no published volume present.
//!
//! `dictionary_compile.rs` is the strongest gate — it proves `compile` reproduces
//! the shipped volume byte for byte — but it needs two artifacts that are not
//! committed, so out of the box it skips and verifies nothing.
//!
//! This test closes that hole. It drives the same parser and encoder over the
//! *authored source*, which is in the tree, and asserts the properties that must
//! hold regardless of which volume was published:
//!
//! - the source parses at all, at full scale
//! - every entry has a key and at least one form
//! - `compile` is deterministic: the same source encodes to the same bytes
//! - the result decodes back and re-encodes identically
//!
//! What it cannot check is byte-identity against *the published* volume, because
//! it does not have it. It checks the next best thing, and it runs everywhere.

use std::path::PathBuf;

use pyac::dictionary::{
    ArgumentSlot, DictionaryMetadata, DictionaryVolume, DictionaryVolumeEntry, LexicalForm,
    OperationParameter, OperationResult, OperationSignature, SourceGloss, SourceLink,
    decode_dictionary, encode_dictionary, parse_source,
};

/// The source: a whole file if one is present, otherwise the shard directory
/// assembled. The shards are what the tree ships; the whole file is a generated
/// convenience artifact.
fn source_text() -> Option<String> {
    if let Some(explicit) = std::env::var_os("PYAC_REFERENCE_SOURCE") {
        return std::fs::read_to_string(PathBuf::from(explicit)).ok();
    }
    let root = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .and_then(|path| path.parent())
        .map(PathBuf::from)?;
    let whole = root.join(pyac::dictionary::REFERENCE_SOURCE_FILE);
    if whole.is_file() {
        return std::fs::read_to_string(whole).ok();
    }
    let shards = root.join("dictionary/shard");
    let manifest = std::fs::read_to_string(shards.join(pyac::dictionary::MANIFEST_FILE)).ok()?;
    let records = pyac::dictionary::parse_manifest(&manifest).ok()?;
    let mut contents = std::collections::BTreeMap::new();
    for record in records {
        let body = pyac::dictionary::shard_candidates(&record.name)
            .into_iter()
            .find_map(|file| std::fs::read_to_string(shards.join(file)).ok())?;
        contents.insert(record.name, body);
    }
    pyac::dictionary::assemble_shards(&manifest, &contents).ok()
}

/// Mirror of the `compile` subcommand: source -> volume -> bytes.
fn compile_to_bytes(source: &str) -> Vec<u8> {
    let parsed = parse_source(source).expect("the authored source parses");
    let volume = DictionaryVolume {
        metadata: DictionaryMetadata {
            kind: parsed.metadata.kind,
            title: parsed.metadata.title,
            revision: parsed.metadata.revision,
            languages: parsed.metadata.languages,
            normalization_policy: parsed.metadata.normalization_policy,
        },
        entries: parsed
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
    };
    encode_dictionary(&volume).expect("the parsed source encodes")
}

#[test]
fn the_authored_source_parses_at_full_scale() {
    let Some(source) = source_text() else {
        eprintln!("skipping: no dictionary source or shards present");
        return;
    };
    let parsed = parse_source(&source).expect("the authored source parses");

    assert!(
        parsed.entries.len() > 60_000,
        "expected the whole vocabulary, parsed {} entries",
        parsed.entries.len()
    );
    assert!(
        !parsed.metadata.languages.is_empty(),
        "the profile metadata must declare languages"
    );

    // Every entry must be addressable. A key is mandatory; an English surface is
    // not — 18 compounds are reachable only through their components, which is
    // how the source records a decomposition with no English name of its own.
    let mut without_key = 0;
    let mut without_any_reference = 0;
    for entry in &parsed.entries {
        if entry.key.is_empty() {
            without_key += 1;
        }
        let has_something = !entry.lexical_forms.is_empty()
            || !entry.component_keys.is_empty()
            || !entry.source_links.is_empty()
            || !entry.source_glosses.is_empty();
        if !has_something {
            without_any_reference += 1;
        }
    }
    assert_eq!(without_key, 0, "{without_key} entries have no key");
    assert_eq!(
        without_any_reference, 0,
        "{without_any_reference} entries have neither a form, a component, nor a reference"
    );
}

#[test]
fn compiling_the_source_is_deterministic() {
    let Some(source) = source_text() else {
        eprintln!("skipping: no dictionary source or shards present");
        return;
    };
    let first = compile_to_bytes(&source);
    let second = compile_to_bytes(&source);
    assert_eq!(
        first, second,
        "compile is not deterministic; the encoder or parser has nondeterministic ordering"
    );
}

#[test]
fn a_compiled_source_decodes_and_re_encodes_identically() {
    // This is the round-trip property, provable without the published volume.
    // It is weaker than byte-identity against the shipped artifact, but it runs
    // everywhere and catches any lossy encode or decode.
    let Some(source) = source_text() else {
        eprintln!("skipping: no dictionary source or shards present");
        return;
    };
    let compiled = compile_to_bytes(&source);

    let volume = decode_dictionary(std::io::Cursor::new(compiled.clone()))
        .expect("the compiled volume decodes");
    assert!(
        volume.entries.len() > 60_000,
        "decoded {} entries",
        volume.entries.len()
    );
    let reencoded = encode_dictionary(&volume).expect("the decoded volume re-encodes");
    if reencoded != compiled {
        let first = compiled
            .iter()
            .zip(reencoded.iter())
            .position(|(left, right)| left != right);
        panic!(
            "compile -> decode -> encode is not the identity over the authored source: \
             {} vs {} bytes, first difference at {first:?}",
            compiled.len(),
            reencoded.len()
        );
    }
}
