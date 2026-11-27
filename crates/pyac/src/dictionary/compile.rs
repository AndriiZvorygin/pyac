//! Parsed source to the encoder's model: the first half of `compile`.
//!
//! `parse_source` produces `DictionarySource`, a faithful view of what the text
//! said. `encode_dictionary` takes `DictionaryVolume`, the model the wire format
//! is written from. The two differ only in naming and in dropping the records'
//! authored shape, so the conversion is mechanical — and because it is mechanical
//! it was copied into the CLI and into two test files before this.
//!
//! Three copies of a conversion is three chances to disagree about what `compile`
//! means. The gates exist to prove the CLI agrees with itself; they cannot do that
//! if the test's idea of compile is a separate implementation.

use super::model::{LexicalForm, SourceGloss, SourceLink};
use super::operation::{ArgumentSlot, OperationParameter, OperationResult, OperationSignature};
use super::source::DictionarySource;
use super::volume::{
    DictionaryMetadata, DictionaryVolume, DictionaryVolumeEntry, DictionaryVolumeError,
};

/// The codelet width the wire format writes.
const CODELET_WORDS: usize = 16;

/// Convert parsed source into the volume model.
///
/// A definition is already-lowered codelets, so it must arrive as whole
/// sixteen-word codelets. A short one would be silently truncated by a filter,
/// which is indistinguishable from the entry having no definition at all; it is
/// reported instead.
pub fn source_to_volume(
    source: DictionarySource,
) -> Result<DictionaryVolume, DictionaryVolumeError> {
    let entries = source
        .entries
        .into_iter()
        .map(|entry| {
            let definition = entry
                .definition
                .map(|codelets| {
                    codelets
                        .into_iter()
                        .map(|codelet| {
                            <[u16; CODELET_WORDS]>::try_from(codelet).map_err(|_| {
                                DictionaryVolumeError::InvalidEntry(
                                    "authored definition is not whole sixteen-word codelets",
                                )
                            })
                        })
                        .collect::<Result<Vec<_>, _>>()
                })
                .transpose()?;
            Ok(DictionaryVolumeEntry {
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
                definition,
            })
        })
        .collect::<Result<Vec<_>, DictionaryVolumeError>>()?;
    Ok(DictionaryVolume {
        metadata: DictionaryMetadata {
            kind: source.metadata.kind,
            title: source.metadata.title,
            revision: source.metadata.revision,
            languages: source.metadata.languages,
            normalization_policy: source.metadata.normalization_policy,
        },
        entries,
    })
}
