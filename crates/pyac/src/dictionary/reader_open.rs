//! Opening a volume: header validation, profile records, and the table of contents.
//!
//! `open` reads only the header and the TOC; the entry bodies are pulled as they
//! are asked for, through the reader's `Window`.

use super::reader::{DictionaryFileReader, DictionaryReadError, DictionarySection, ProfileRecords};
use super::reader_codec::{
    Sentence, parse_marker, parse_metadata, parse_toc, parse_toc_v4, read_sentence,
    validate_header, validate_sections, validate_sections_v4,
};
use super::volume_records::{PROFILE_VERSION, TOC_ROLES, TOC_ROLES_V4};
use super::window::Window;
use crate::{HEADER_BYTES, PyacError};
use std::io::{Read, Seek, SeekFrom};

impl<R: Read + Seek> DictionaryFileReader<R> {
    /// Open a volume, streaming it through a bounded window.
    ///
    /// Nothing is read up front beyond the header and the table of contents; the
    /// entry bodies are pulled as they are asked for. `Window` makes the
    /// seek-per-codelet access pattern cost one read per window rather than two
    /// syscalls per codelet.
    pub fn open(source: R) -> Result<Self, DictionaryReadError> {
        let mut source = Window::new(source);
        let mut header = [0u8; HEADER_BYTES];
        source.seek(SeekFrom::Start(0))?;
        source.read_exact(&mut header)?;
        let count = validate_header(&header)?;
        let declared_bytes = u32::from_le_bytes(header[24..28].try_into().expect("header field"));
        let actual_bytes = source.seek(SeekFrom::End(0))? as usize;
        if actual_bytes != declared_bytes as usize {
            return Err(PyacError::LengthMismatch {
                expected: declared_bytes as usize,
                declared: declared_bytes,
                actual: actual_bytes,
            }
            .into());
        }
        let ProfileRecords {
            kind,
            marker_count,
            metadata_count,
            mut metadata,
            toc_count,
            profile_version,
            metadata_entry_count,
            version_four,
        } = read_profile_records(&mut source, count)?;
        let expected_toc_count = if version_four {
            TOC_ROLES_V4.len()
        } else {
            TOC_ROLES.len()
        };
        if kind.trim().is_empty()
            || metadata.title.trim().is_empty()
            || metadata.normalization_policy != "NFC"
            || metadata.languages.is_empty()
            || metadata.languages.iter().any(String::is_empty)
            || metadata.languages.windows(2).any(|pair| pair[0] >= pair[1])
            || toc_count != expected_toc_count
            || (version_four && metadata_entry_count.is_none())
            || (!version_four && metadata_entry_count.is_some())
        {
            return Err(DictionaryReadError::InvalidRecord(
                "profile kind, metadata, or TOC count is invalid",
            ));
        }
        metadata.kind = kind;
        let toc_start = marker_count + metadata_count;
        let mut ordinal = toc_start;
        let mut sections = Vec::with_capacity(toc_count + usize::from(version_four));
        if version_four {
            sections.push(DictionarySection {
                role: "metadata".into(),
                codelet_start: marker_count,
                codelet_count: metadata_count,
            });
            let mut toc_sections = Vec::with_capacity(toc_count);
            for (index, role) in TOC_ROLES_V4.iter().enumerate() {
                let Sentence {
                    words, consumed, ..
                } = read_sentence(&mut source, ordinal, count)?;
                let section = parse_toc_v4(&words, index as u16, role).map_err(|error| {
                    DictionaryReadError::Context(format!("TOC record {index}"), Box::new(error))
                })?;
                toc_sections.push(section.clone());
                sections.push(section);
                ordinal = ordinal
                    .checked_add(consumed)
                    .ok_or(DictionaryReadError::InvalidRecord("TOC ordinal overflow"))?;
            }
            validate_sections_v4(
                &toc_sections,
                ordinal,
                count,
                metadata_entry_count.expect("checked above"),
            )?;
        } else {
            for role in TOC_ROLES {
                let Sentence {
                    words, consumed, ..
                } = read_sentence(&mut source, ordinal, count)?;
                let section = parse_toc(&words).map_err(|error| {
                    DictionaryReadError::Context(format!("TOC record for {role}"), Box::new(error))
                })?;
                if section.role != role {
                    return Err(DictionaryReadError::InvalidRecord(
                        "TOC records are not in canonical role order",
                    ));
                }
                sections.push(section);
                ordinal = ordinal
                    .checked_add(consumed)
                    .ok_or(DictionaryReadError::InvalidRecord("TOC ordinal overflow"))?;
            }
            validate_sections(&sections, marker_count, metadata_count, ordinal, count)?;
        }
        let mut reader = Self {
            source,
            codelet_count: count,
            profile_version,
            metadata,
            sections,
            entry_count: metadata_entry_count.unwrap_or(0) as usize,
            version_four,
        };
        if !reader.version_four {
            reader.entry_count = reader.validate_legacy_entries().map_err(|error| {
                DictionaryReadError::Context("entry-location index".into(), Box::new(error))
            })?;
        }
        Ok(reader)
    }
}

fn read_profile_records<R: Read + Seek>(
    source: &mut R,
    limit: u32,
) -> Result<ProfileRecords, DictionaryReadError> {
    let Sentence {
        words: marker,
        consumed: marker_count,
        ..
    } = read_sentence(source, 0, limit)?;
    let kind = parse_marker(&marker)
        .map_err(|error| DictionaryReadError::Context("profile marker".into(), Box::new(error)))?;
    let Sentence {
        words: metadata_words,
        consumed: metadata_count,
        ..
    } = read_sentence(source, marker_count, limit)?;
    let (metadata, toc_count, profile_version, metadata_entry_count) =
        parse_metadata(&metadata_words).map_err(|error| {
            DictionaryReadError::Context("profile metadata".into(), Box::new(error))
        })?;
    let version_four = profile_version == PROFILE_VERSION;
    Ok(ProfileRecords {
        kind,
        marker_count,
        metadata_count,
        metadata,
        toc_count,
        profile_version,
        metadata_entry_count,
        version_four,
    })
}
