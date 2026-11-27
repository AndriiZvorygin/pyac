//! Section validation for both volume generations.

use super::super::reader::{DictionaryReadError, DictionarySection};
use super::super::volume_records::{TOC_ROLES, TOC_ROLES_V4};

pub(crate) fn validate_sections(
    sections: &[DictionarySection],
    marker_count: u32,
    metadata_count: u32,
    toc_end: u32,
    pack_count: u32,
) -> Result<(), DictionaryReadError> {
    if sections.len() != TOC_ROLES.len()
        || sections[0].codelet_start != marker_count
        || sections[0].codelet_count != metadata_count
    {
        return Err(DictionaryReadError::InvalidRecord(
            "metadata TOC range does not match its physical range",
        ));
    }
    let mut expected = toc_end;
    for (section, role) in sections.iter().skip(1).zip(TOC_ROLES.iter().skip(1)) {
        if section.role != *role || section.codelet_start != expected {
            return Err(DictionaryReadError::InvalidRecord(
                "TOC section order or range is noncanonical",
            ));
        }
        expected = expected
            .checked_add(section.codelet_count)
            .ok_or(DictionaryReadError::InvalidRecord("TOC range overflow"))?;
    }
    if expected != pack_count {
        return Err(DictionaryReadError::InvalidRecord(
            "TOC does not cover the complete pack",
        ));
    }
    Ok(())
}
pub(crate) fn validate_sections_v4(
    sections: &[DictionarySection],
    toc_end: u32,
    pack_count: u32,
    entry_count: u32,
) -> Result<(), DictionaryReadError> {
    if sections.len() != TOC_ROLES_V4.len() || entry_count == 0 {
        return Err(DictionaryReadError::InvalidRecord(
            "v4 section count or dictionary entry count is invalid",
        ));
    }
    let mut expected = toc_end;
    for (section, role) in sections.iter().zip(TOC_ROLES_V4) {
        if section.role != role || section.codelet_start != expected {
            return Err(DictionaryReadError::InvalidRecord(
                "v4 TOC section order or range is noncanonical",
            ));
        }
        expected = expected
            .checked_add(section.codelet_count)
            .ok_or(DictionaryReadError::InvalidRecord("v4 TOC range overflow"))?;
    }
    if expected != pack_count || sections[0].codelet_count == 0 || sections[4].codelet_count == 0 {
        return Err(DictionaryReadError::InvalidRecord(
            "v4 TOC does not cover the pack or required sections are empty",
        ));
    }
    Ok(())
}
