//! Entry reads, index lookups, and the legacy (v2/v3) index paths.

use std::ops::Range;

use super::reader::{DictionaryFileReader, DictionaryReadError};
use super::reader_codec::{Sentence, parse_location, read_sentence};
use super::volume_records::{PROFILE_VERSION_V2, PROFILE_VERSION_V3};
use std::io::{Read, Seek};

impl<R: Read + Seek> DictionaryFileReader<R> {
    pub(super) fn offsets_for_legacy_keys(
        &mut self,
        keys: &[Vec<u16>],
    ) -> Result<Vec<u32>, DictionaryReadError> {
        let mut offsets = Vec::with_capacity(keys.len());
        for key in keys {
            let Some((entry, _)) = self.find_legacy_location(key)? else {
                return Err(DictionaryReadError::MissingEntry(key.clone()));
            };
            offsets.push(entry.start);
        }
        offsets.sort_unstable();
        offsets.dedup();
        Ok(offsets)
    }
    pub(super) fn find_legacy_location(
        &mut self,
        key: &[u16],
    ) -> Result<Option<(Range<u32>, Range<u32>)>, DictionaryReadError> {
        let section = self.section("entry-location-index")?;
        let end = section.codelet_start + section.codelet_count;
        let mut ordinal = section.codelet_start;
        while ordinal < end {
            let Sentence {
                words, consumed, ..
            } = read_sentence(&mut self.source, ordinal, end)?;
            let (candidate, entry, definition) = parse_location(&words)?;
            match candidate.as_slice().cmp(key) {
                std::cmp::Ordering::Less => ordinal += consumed,
                std::cmp::Ordering::Equal => return Ok(Some((entry, definition))),
                std::cmp::Ordering::Greater => return Ok(None),
            }
        }
        Ok(None)
    }
    pub(super) fn find_legacy_range_by_offset(
        &mut self,
        offset: u32,
    ) -> Result<Option<Range<u32>>, DictionaryReadError> {
        let section = self.section("entry-location-index")?;
        let end = section.codelet_start + section.codelet_count;
        let mut ordinal = section.codelet_start;
        while ordinal < end {
            let Sentence {
                words, consumed, ..
            } = read_sentence(&mut self.source, ordinal, end)?;
            let (_, entry, _) = parse_location(&words)?;
            if entry.start == offset {
                return Ok(Some(entry));
            }
            ordinal += consumed;
        }
        Ok(None)
    }
    pub(super) fn validate_legacy_entries(&mut self) -> Result<usize, DictionaryReadError> {
        let locations = self.section("entry-location-index")?;
        let entries = self.section("entries")?;
        let entries_end = entries.codelet_start + entries.codelet_count;
        let locations_end = locations.codelet_start + locations.codelet_count;
        let mut ordinal = locations.codelet_start;
        let mut expected_entry_start = entries.codelet_start;
        let mut previous_key: Option<Vec<u16>> = None;
        let mut count = 0usize;
        while ordinal < locations_end {
            let Sentence {
                words, consumed, ..
            } = read_sentence(&mut self.source, ordinal, locations_end)?;
            let (key, entry, definition) = parse_location(&words)?;
            if previous_key
                .as_ref()
                .is_some_and(|previous| previous >= &key)
            {
                return Err(DictionaryReadError::InvalidRecord(
                    "entry-location index is not strictly sorted",
                ));
            }
            if entry.start != expected_entry_start
                || entry.is_empty()
                || entry.end > entries_end
                || definition.start < entry.start
                || definition.end > entry.end
                || (definition.is_empty()
                    && (self.profile_version != PROFILE_VERSION_V3
                        || definition.start != entry.end))
                || (!definition.is_empty()
                    && self.profile_version == PROFILE_VERSION_V2
                    && definition.end != entry.end)
            {
                return Err(DictionaryReadError::InvalidRecord(
                    "legacy entry or definition range is invalid",
                ));
            }
            previous_key = Some(key);
            expected_entry_start = entry.end;
            ordinal += consumed;
            count += 1;
        }
        if ordinal != locations_end || expected_entry_start != entries_end || count == 0 {
            return Err(DictionaryReadError::InvalidRecord(
                "legacy entry-location index does not cover entries",
            ));
        }
        Ok(count)
    }
}
