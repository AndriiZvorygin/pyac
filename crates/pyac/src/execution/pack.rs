//! Pack execution: a codelet stream in, a list of values out.
//!
//! A pack is either flat or split into programlet sections. Both shapes go through
//! the same per-codelet dispatch; only the framing differs.

use crate::semantic::SemanticError;
use crate::{
    PackExecutionError, PyacPack, PyacValue, VectorElements, execute_date_codelet,
    execute_pack_sections_with_storage, execute_shared_programlet, execute_vector_codelet,
    prepare_storage_sections,
};

pub fn execute_pack(pack: &PyacPack) -> Result<Vec<PyacValue>, PackExecutionError> {
    use crate::semantic::{
        NumericCodeletValue, execute_conditional_number_codelets, execute_numeric_codelet,
    };

    let prepared_storage =
        prepare_storage_sections(&pack.codelets).map_err(|_| PackExecutionError {
            index: 0,
            source: SemanticError::MalformedStorageSection,
        })?;
    let codelet_count = prepared_storage.executable_codelet_count;
    let executable_codelets = &pack.codelets[..codelet_count];
    let words = executable_codelets.as_flattened();
    let storage = pyac_vliw_core::StorageView::new(
        &prepared_storage.descriptors,
        &prepared_storage.names,
        &prepared_storage.payload,
    );
    if pyac_vliw_core::has_programlet_section_header(&words, codelet_count) {
        let mut sections = execute_pack_sections_with_storage(&words, codelet_count, storage)?;
        // This compatibility API drops section identity, so retain its
        // historical source-order result contract. Callers that need actual
        // completion order should use execute_pack_sections instead.
        sections.sort_by_key(|section| section.section_index);
        return Ok(sections.into_iter().map(|section| section.value).collect());
    }
    if pyac_vliw_core::is_numeric_programlet(&words, codelet_count) {
        return execute_shared_programlet(&words, 0, codelet_count, 0, &storage)
            .map(|value| vec![value]);
    }

    let mut values = Vec::new();
    let mut index = 0;
    while index < codelet_count {
        let shared =
            pyac_vliw_core::interpret_group_with_storage(&words, index, codelet_count, &storage);
        if shared.error == pyac_vliw_core::ERROR_DIVISION_BY_ZERO {
            return Err(PackExecutionError {
                index,
                source: SemanticError::DivisionByZero,
            });
        }
        if shared.error == pyac_vliw_core::ERROR_FLOAT_DOMAIN {
            return Err(PackExecutionError {
                index,
                source: SemanticError::NonFiniteFloat,
            });
        }
        if shared.error == pyac_vliw_core::ERROR_FIXED16_DOMAIN {
            return Err(PackExecutionError {
                index,
                source: SemanticError::Fixed16Domain,
            });
        }
        if shared.error == 0 && shared.value_kind == 0 && shared.consumed_codelets > 0 {
            index += shared.consumed_codelets as usize;
            continue;
        }
        if shared.error == 0 && shared.value_kind != 0 {
            let value = match shared.value_kind {
                pyac_vliw_core::VALUE_UNSIGNED => PyacValue::Unsigned(
                    u64::from(shared.values[0]) | (u64::from(shared.values[1]) << 32),
                ),
                pyac_vliw_core::VALUE_SIGNED => {
                    PyacValue::Signed(i64::from(shared.values[0] as i32))
                }
                pyac_vliw_core::VALUE_FLOAT32 => PyacValue::Float32(shared.values[0]),
                pyac_vliw_core::VALUE_FLOAT64 => PyacValue::Float64(
                    u64::from(shared.values[0]) | (u64::from(shared.values[1]) << 32),
                ),
                pyac_vliw_core::VALUE_FIXED16 => PyacValue::Fixed16(shared.values[0] as u16),
                pyac_vliw_core::VALUE_BOOLEAN => PyacValue::Boolean(shared.values[0] != 0),
                pyac_vliw_core::VALUE_DATE => PyacValue::DateEpochMs(
                    u64::from(shared.values[0]) | (u64::from(shared.values[1]) << 32),
                ),
                pyac_vliw_core::VALUE_VECTOR_U16 => PyacValue::Vector(VectorElements::Numbers(
                    shared.values[..shared.length as usize]
                        .iter()
                        .map(|value| *value as u16)
                        .collect(),
                )),
                pyac_vliw_core::VALUE_VECTOR_U32 => PyacValue::Vector(VectorElements::Numbers32(
                    shared.values[..shared.length as usize].to_vec(),
                )),
                pyac_vliw_core::VALUE_VECTOR_BOOL => PyacValue::Vector(VectorElements::Booleans(
                    shared.values[..shared.length as usize]
                        .iter()
                        .map(|value| *value != 0)
                        .collect(),
                )),
                pyac_vliw_core::VALUE_VECTOR_FLOAT32 => PyacValue::Vector(VectorElements::Float32(
                    shared.values[..shared.length as usize].to_vec(),
                )),
                pyac_vliw_core::VALUE_VECTOR_FLOAT64 => {
                    let words = &shared.values[..shared.length as usize * 2];
                    PyacValue::Vector(VectorElements::Float64(
                        words
                            .chunks_exact(2)
                            .map(|pair| u64::from(pair[0]) | (u64::from(pair[1]) << 32))
                            .collect(),
                    ))
                }
                _ => unreachable!("shared VLIW core returned an unknown value kind"),
            };
            values.push(value);
            index += shared.consumed_codelets as usize;
            continue;
        }

        let start = index;
        while index < codelet_count && executable_codelets[index][0] & 1 == 0 {
            index += 1;
        }
        if index == codelet_count {
            return Err(PackExecutionError {
                index: start,
                source: SemanticError::UnsupportedCodelet,
            });
        }

        let group = &executable_codelets[start..=index];
        if group.len() > 1 {
            values.push(
                execute_conditional_number_codelets(group)
                    .map(|value| PyacValue::Unsigned(u64::from(value)))
                    .map_err(|source| PackExecutionError {
                        index: start,
                        source,
                    })?,
            );
        } else {
            let codelet = &group[0];
            if let Some(result) = execute_numeric_codelet(codelet) {
                values.push(
                    result
                        .map(|value| match value {
                            NumericCodeletValue::Unsigned(value) => PyacValue::Unsigned(value),
                            NumericCodeletValue::Signed(value) => PyacValue::Signed(value),
                            NumericCodeletValue::Vector(value) => PyacValue::Vector(value),
                        })
                        .map_err(|source| PackExecutionError {
                            index: start,
                            source,
                        })?,
                );
            } else if let Ok(value) = execute_date_codelet(codelet) {
                values.push(PyacValue::DateEpochMs(value));
            } else if let Ok(value) = execute_vector_codelet(codelet) {
                values.push(PyacValue::Vector(value));
            } else {
                return Err(PackExecutionError {
                    index: start,
                    source: SemanticError::UnsupportedCodelet,
                });
            }
        }
        index += 1;
    }
    Ok(values)
}
