//! Example corpus for the source round-trip, one case per record kind.
//!
//! `dictionary_render.rs` and `dictionary_glyph.rs` prove the *published* volume
//! round-trips, but that volume carries no definitions and exercises a narrow
//! slice of each record kind. A small hand-built volume per case fails with a name
//! that says which kind broke, instead of one 131 MB byte-difference.
//!
//! Each case goes both ways: English form (`.en.pya`, `text "…"`) and glyph form
//! (`zi.kyit. … .kyit.zi`). Both must re-encode to the identical bytes.

use pyac::dictionary::{
    ArgumentSlot, DictionaryMetadata, DictionaryVolume, DictionaryVolumeEntry, LexicalForm,
    OperationParameter, OperationResult, OperationSignature, SourceGloss, SourceLink,
    encode_dictionary, encode_text_sentence, parse_source, render_glyph_source, render_source,
};

const KEY: [u16; 2] = [0xa96a, 0x6c22]; // tlat kwon
const OTHER: [u16; 1] = [0x680b]; // lyin

fn empty_entry() -> DictionaryVolumeEntry {
    DictionaryVolumeEntry {
        key: KEY.to_vec(),
        lexical_forms: vec![],
        source_links: vec![],
        source_glosses: vec![],
        component_keys: vec![],
        task_keys: vec![],
        references: vec![],
        operation_signatures: vec![],
        definition: None,
    }
}

/// A second entry with one form, so a component or reference can resolve to it.
fn other_entry() -> DictionaryVolumeEntry {
    let mut entry = empty_entry();
    entry.key = OTHER.to_vec();
    entry.lexical_forms.push(LexicalForm {
        language: "en".into(),
        surface: "lyin".into(),
        normalized: "lyin".into(),
    });
    entry
}

fn volume(entry: DictionaryVolumeEntry) -> DictionaryVolume {
    volume_with(entry, vec!["en".into()])
}

fn volume_with(entry: DictionaryVolumeEntry, languages: Vec<String>) -> DictionaryVolume {
    let mut entries = vec![entry];
    // Every case gets the resolvable second entry; a case that does not reference
    // it simply carries it unused, which the encoder accepts.
    entries.push(other_entry());
    DictionaryVolume {
        metadata: DictionaryMetadata {
            kind: "corpus dictionary".into(),
            title: "corpus".into(),
            revision: 1,
            languages,
            normalization_policy: "NFC".into(),
        },
        entries,
    }
}

/// Round-trip one volume through both source forms, comparing bytes.
fn round_trips(name: &str, volume: DictionaryVolume) {
    let bytes = encode_dictionary(&volume).expect("the case encodes");

    // English form.
    let english = render_source(&volume).expect("renders English");
    let reparsed = parse_source(&english).unwrap_or_else(|error| {
        panic!("{name}: English source does not parse: {error}\n{english}")
    });
    let again = encode_dictionary(&to_volume(reparsed)).expect("English re-encodes");
    assert_eq!(again, bytes, "{name}: English round-trip differs");

    // Glyph form.
    let glyph = render_glyph_source(&volume).expect("renders glyph");
    let reparsed = parse_source(&glyph)
        .unwrap_or_else(|error| panic!("{name}: glyph source does not parse: {error}\n{glyph}"));
    let again = encode_dictionary(&to_volume(reparsed)).expect("glyph re-encodes");
    assert_eq!(again, bytes, "{name}: glyph round-trip differs");
}

#[test]
fn a_bare_entry_round_trips() {
    // An entry must carry at least one sentence, so the minimal case is one form.
    let mut entry = empty_entry();
    entry.lexical_forms.push(LexicalForm {
        language: "en".into(),
        surface: "minimal".into(),
        normalized: "minimal".into(),
    });
    round_trips("bare entry", volume(entry));
}

#[test]
fn a_lexical_form_round_trips() {
    let mut entry = empty_entry();
    entry.lexical_forms.push(LexicalForm {
        language: "en".into(),
        surface: "immune".into(),
        normalized: "immune".into(),
    });
    round_trips("lexical form", volume(entry));
}

#[test]
fn a_long_lexical_form_round_trips() {
    // Over fourteen units, so it uses raw quote continuation codelets.
    let mut entry = empty_entry();
    entry.lexical_forms.push(LexicalForm {
        language: "en".into(),
        surface: "a surface long enough to need continuation codelets".into(),
        normalized: "a surface long enough to need continuation codelets".into(),
    });
    round_trips("long lexical form", volume(entry));
}

#[test]
fn text_containing_the_frame_marker_round_trips() {
    // The payload must survive a literal closing marker and a backslash.
    let mut entry = empty_entry();
    entry.lexical_forms.push(LexicalForm {
        language: "en".into(),
        surface: "ends with .kyit.zi and a backslash \\".into(),
        normalized: "ends with .kyit.zi and a backslash \\".into(),
    });
    round_trips("frame marker in text", volume(entry));
}

#[test]
fn non_ascii_text_round_trips() {
    let mut entry = empty_entry();
    entry.lexical_forms.push(LexicalForm {
        language: "zh".into(),
        surface: "免疫化学".into(),
        normalized: "免疫化学".into(),
    });
    round_trips(
        "non-ascii text",
        volume_with(entry, vec!["en".into(), "zh".into()]),
    );
}

#[test]
fn a_source_link_round_trips() {
    let mut entry = empty_entry();
    entry.source_links.push(SourceLink {
        dataset: "wordnet".into(),
        release: "OEWN 2025".into(),
        identifier: "n:06311813-n".into(),
        relation: "exact-sense".into(),
    });
    round_trips("source link", volume(entry));
}

#[test]
fn a_source_gloss_round_trips() {
    let mut entry = empty_entry();
    // A gloss must cite a source link the entry already carries, so add one.
    entry.source_links.push(SourceLink {
        dataset: "wordnet".into(),
        release: "OEWN 2025".into(),
        identifier: "n:06311813-n".into(),
        relation: "exact-sense".into(),
    });
    entry.source_glosses.push(SourceGloss {
        language: "en".into(),
        text: "a sense-specific gloss".into(),
        dataset: "wordnet".into(),
        release: "OEWN 2025".into(),
        identifier: "n:06311813-n".into(),
    });
    round_trips("source gloss", volume(entry));
}

#[test]
fn a_component_round_trips() {
    let mut entry = empty_entry();
    entry.lexical_forms.push(LexicalForm {
        language: "en".into(),
        surface: "compound".into(),
        normalized: "compound".into(),
    });
    // A component must resolve to another entry in the volume.
    entry.component_keys.push(OTHER.to_vec());
    round_trips("component", volume(entry));
}

#[test]
fn a_task_round_trips() {
    let mut entry = empty_entry();
    entry.lexical_forms.push(LexicalForm {
        language: "en".into(),
        surface: "worker".into(),
        normalized: "worker".into(),
    });
    entry.task_keys.push(OTHER.to_vec());
    round_trips("task", volume(entry));
}

#[test]
fn a_reference_round_trips() {
    let mut entry = empty_entry();
    entry.lexical_forms.push(LexicalForm {
        language: "en".into(),
        surface: "referrer".into(),
        normalized: "referrer".into(),
    });
    // A reference must resolve to another entry in the volume.
    entry.references.push(OTHER.to_vec());
    round_trips("reference", volume(entry));
}

#[test]
fn an_operation_signature_with_arguments_and_results_round_trips() {
    let mut entry = empty_entry();
    entry.operation_signatures.push(OperationSignature {
        verb: vec![0x680b],
        mood: vec![0x217e],
        arguments: vec![OperationParameter {
            case_type: ArgumentSlot {
                grammatical_case: vec![0x245e],
                noun_class: vec![0x0e1d],
            },
            tag_label: "subject".into(),
            descriptive_specification: "the value being evaluated".into(),
        }],
        results: vec![OperationResult {
            noun_class: vec![0x0e1d],
            tag_label: "sum".into(),
            descriptive_specification: "the computed result".into(),
        }],
    });
    round_trips("signature", volume(entry));
}

#[test]
fn a_definition_body_round_trips() {
    // A body is pre-lowered codelets. The published volume has none, so this is the
    // case that proves the definition record has a source form at all.
    let mut entry = empty_entry();
    entry.definition = Some(
        encode_text_sentence(
            &KEY,
            &[pyac::dictionary::DictionaryField {
                grammatical_case: 0x245e,
                value: pyac::dictionary::DictionaryValue::Text("a body".into()),
            }],
        )
        .expect("lower the body"),
    );
    round_trips("definition", volume(entry));
}

/// Mirror `compile`'s source -> volume mapping.
fn to_volume(source: pyac::dictionary::DictionarySource) -> DictionaryVolume {
    volume_from_parts(source)
}

fn volume_from_parts(source: pyac::dictionary::DictionarySource) -> DictionaryVolume {
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
