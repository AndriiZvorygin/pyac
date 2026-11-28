//! The document loop and the record dispatcher.

use super::errors::SourceError;
use super::keys::key_for_name;
use super::lexing::{ClauseValue, parse_line};
use super::records::{
    DictionarySource, EntryRecord, LexicalFormRecord, ResultRecord, SourceGlossRecord,
    SourceLinkRecord, VolumeMetadataRecord,
};
use super::signatures::{parse_signature, parse_slot};
use std::collections::HashMap;

/// Parse the whole source into a build request.
///
/// **parse**, Pyac key `flis` (`pyash dict from pyac flis`): *"analyze
/// syntactically by assigning a constituent structure to (a sentence)"*.
pub fn parse_source(source: &str) -> Result<DictionarySource, SourceError> {
    let mut kind: Option<String> = None;
    let mut title: Option<String> = None;
    let mut revision: Option<u32> = None;
    let mut languages: Vec<String> = Vec::new();
    let mut policy: Option<String> = None;

    let mut entries: Vec<EntryRecord> = Vec::new();
    let mut by_key_name: HashMap<String, usize> = HashMap::new();

    for (index, raw) in source.lines().enumerate() {
        let line_number = index + 1;
        let line = raw.trim();
        if line.is_empty() || line == "prah" {
            continue;
        }
        let clauses = parse_line(line, line_number)?;

        let subject = match clauses.get("su") {
            Some(ClauseValue::Text(name)) => name.clone(),
            _ => {
                return Err(SourceError::Unsupported {
                    line: line_number,
                    text: line.to_owned(),
                });
            }
        };

        if subject == "lyin kwon" {
            match (&clauses.get("as"), &clauses.get("ob")) {
                // Metadata: `ob <title> as num <revision> from <languages> to <NFC> with <version>`
                (Some(ClauseValue::Number(value)), Some(ClauseValue::Text(t))) => {
                    revision = Some(*value);
                    title = Some(t.clone());
                    if let Some(ClauseValue::Text(list)) = clauses.get("from") {
                        languages = list
                            .split(',')
                            .map(|item| item.trim().to_owned())
                            .filter(|item| !item.is_empty())
                            .collect();
                    }
                    if let Some(ClauseValue::Text(value)) = clauses.get("to") {
                        policy = Some(value.clone());
                    }
                }
                // Marker: `ob <kind>`
                (_, Some(ClauseValue::Text(t))) => kind = Some(t.clone()),
                _ => {
                    return Err(SourceError::Unsupported {
                        line: line_number,
                        text: line.to_owned(),
                    });
                }
            }
            continue;
        }

        if subject != "tlat kwon" {
            return Err(SourceError::Unsupported {
                line: line_number,
                text: line.to_owned(),
            });
        }

        let record_kind = match clauses.get("at") {
            Some(ClauseValue::Text(value)) => value.clone(),
            _ => return Err(SourceError::DefinitionSentence { line: line_number }),
        };
        let key_name = match clauses.get("ob") {
            Some(ClauseValue::Text(value)) if !value.trim().is_empty() => value.clone(),
            _ => {
                return Err(SourceError::EmptyText {
                    line: line_number,
                    field: "entry key",
                });
            }
        };

        if record_kind == "entry" {
            if by_key_name.contains_key(&key_name) {
                return Err(SourceError::Duplicate {
                    line: line_number,
                    key: key_name,
                });
            }
            let key = key_for_name(&key_name, line_number)?;
            entries.push(EntryRecord {
                key,
                ..EntryRecord::default()
            });
            by_key_name.insert(key_name, entries.len() - 1);
            continue;
        }

        let Some(&position) = by_key_name.get(&key_name) else {
            return Err(SourceError::Orphan {
                line: line_number,
                key: key_name,
            });
        };
        apply_record(&mut entries[position], &record_kind, &clauses, line_number)?;
    }

    let (Some(kind), Some(title), Some(revision)) = (kind, title, revision) else {
        return Err(SourceError::MissingMetadata);
    };
    if entries.is_empty() {
        return Err(SourceError::MissingMetadata);
    }
    Ok(DictionarySource {
        metadata: VolumeMetadataRecord {
            kind,
            title,
            revision,
            languages,
            normalization_policy: policy.unwrap_or_else(|| "NFC".to_owned()),
        },
        entries,
    })
}
fn apply_record(
    entry: &mut EntryRecord,
    kind: &str,
    clauses: &HashMap<&'static str, ClauseValue>,
    line: usize,
) -> Result<(), SourceError> {
    let text = |case: &'static str, field: &'static str| -> Result<String, SourceError> {
        match clauses.get(case) {
            Some(ClauseValue::Text(value)) if !value.trim().is_empty() => Ok(value.clone()),
            _ => Err(SourceError::EmptyText { line, field }),
        }
    };
    let vector = |case: &'static str| -> Option<Vec<u16>> {
        match clauses.get(case) {
            Some(ClauseValue::Vector(words)) => Some(words.clone()),
            _ => None,
        }
    };

    match kind {
        "lexical-form" => entry.lexical_forms.push(LexicalFormRecord {
            language: text("as", "lexical language")?,
            surface: text("from", "lexical surface")?,
            normalized: text("to", "normalized lexical surface")?,
        }),
        "source-link" => entry.source_links.push(SourceLinkRecord {
            dataset: text("as", "source dataset")?,
            release: text("from", "source release")?,
            identifier: text("to", "source identifier")?,
            relation: text("by", "source relation")?,
        }),
        "source-gloss" => entry.source_glosses.push(SourceGlossRecord {
            language: text("as", "source gloss language")?,
            dataset: text("from", "source gloss dataset")?,
            release: text("to", "source gloss release")?,
            identifier: text("by", "source gloss identifier")?,
            text: text("with", "source gloss text")?,
        }),
        "component" => {
            let name = text("from", "component key")?;
            entry.component_keys.push(key_for_name(&name, line)?);
        }
        "task" => {
            let name = text("with", "task key")?;
            entry.task_keys.push(key_for_name(&name, line)?);
        }
        "reference" => {
            let name = text("to", "reference key")?;
            entry.references.push(key_for_name(&name, line)?);
        }
        "operation-signature" => {
            let Some(words) = vector("from") else {
                return Err(SourceError::Unsupported {
                    line,
                    text: "operation-signature requires a from vector".to_owned(),
                });
            };
            entry
                .operation_signatures
                .push(parse_signature(&words, line)?);
        }
        "operation-argument" => {
            let Some(words) = vector("as") else {
                return Err(SourceError::Unsupported {
                    line,
                    text: "operation-argument requires an as vector".to_owned(),
                });
            };
            let Some(signature) = entry.operation_signatures.last_mut() else {
                return Err(SourceError::Orphan {
                    line,
                    key: "operation-argument".to_owned(),
                });
            };
            let (grammatical_case, noun_class) = parse_slot(&words, line)?;
            let label = text("from", "operation argument name")?;
            let description = text("to", "operation argument description")?;
            // Fill the argument the signature's dispatch key already declared.
            // Pushing would double the list and leave the declared one unlabelled
            // — the defect the volume decoder had.
            let Some(argument) = signature.arguments.iter_mut().find(|argument| {
                argument.grammatical_case == grammatical_case
                    && argument.noun_class == noun_class
                    && argument.tag_label.is_empty()
            }) else {
                return Err(SourceError::Unsupported {
                    line,
                    text: "operation-argument does not match an argument in its signature"
                        .to_owned(),
                });
            };
            argument.tag_label = label;
            argument.descriptive_specification = description;
        }
        "operation-result" => {
            let Some(words) = vector("as") else {
                return Err(SourceError::Unsupported {
                    line,
                    text: "operation-result requires an as vector".to_owned(),
                });
            };
            let Some(signature) = entry.operation_signatures.last_mut() else {
                return Err(SourceError::Orphan {
                    line,
                    key: "operation-result".to_owned(),
                });
            };
            signature.results.push(ResultRecord {
                noun_class: words,
                tag_label: text("from", "operation result name")?,
                descriptive_specification: text("to", "operation result description")?,
            });
        }
        "definition" => {
            // The body is pre-lowered codelets, written flat as `from ve <n> <words…>`
            // and chunked back into sixteen-word codelets here.
            let Some(words) = vector("from") else {
                return Err(SourceError::Unsupported {
                    line,
                    text: "definition requires a from vector".to_owned(),
                });
            };
            if words.is_empty() || words.len() % 16 != 0 {
                return Err(SourceError::Unsupported {
                    line,
                    text: "definition must be a whole number of sixteen-word codelets".to_owned(),
                });
            }
            if entry.definition.is_some() {
                return Err(SourceError::Unsupported {
                    line,
                    text: "an entry may carry only one definition".to_owned(),
                });
            }
            entry.definition = Some(words.chunks(16).map(<[u16]>::to_vec).collect::<Vec<_>>());
        }
        other => {
            return Err(SourceError::Unsupported {
                line,
                text: format!("unsupported dictionary record type: {other}"),
            });
        }
    }
    Ok(())
}
