//! Entry reads, index lookups, and the legacy (v2/v3) index paths.

use std::ops::Range;

use super::operation::OperationSignatureKey;
use super::reader::{DictionaryFileReader, DictionaryReadError};
use super::reader_index::{
    DirectIndexLookup, lookup_legacy_operation, lookup_legacy_source, lookup_legacy_surface,
    lookup_v4,
};
use std::io::{Read, Seek};
use unicode_normalization::UnicodeNormalization;

impl<R: Read + Seek> DictionaryFileReader<R> {
    pub fn location(&mut self, key: &[u16]) -> Result<Option<Range<u32>>, DictionaryReadError> {
        if !self.version_four {
            return Ok(self.find_legacy_location(key)?.map(|(entry, _)| entry));
        }
        let section = self.section("canonical-index")?;
        let entries = self.section("entries")?;
        let entries_end = entries.codelet_start + entries.codelet_count;
        // One search, then one sequential index row. The entry's end is the next
        // row's offset; searching the whole index for it again is what made a full
        // read-back cost 10 s.
        let Some((offset, next)) =
            super::reader_index::canonical_row(&mut self.source, &section, key, entries_end)?
        else {
            return Ok(None);
        };
        if offset < entries.codelet_start || offset >= entries_end {
            return Err(DictionaryReadError::InvalidRecord(
                "canonical index points outside the entries section",
            ));
        }
        let next = next.min(entries_end);
        if next <= offset || next > entries_end {
            return Err(DictionaryReadError::InvalidRecord(
                "canonical index offsets are not strictly increasing within entries",
            ));
        }
        Ok(Some(offset..next))
    }
    pub fn lookup_surface(
        &mut self,
        language: &str,
        surface: &str,
    ) -> Result<Vec<u32>, DictionaryReadError> {
        if self.version_four {
            let section = self.section("surface-index")?;
            let normalized: String = surface.nfc().collect();
            return lookup_v4(
                &mut self.source,
                &section,
                DirectIndexLookup::Surface(language, &normalized),
            );
        }
        let section = self.section("surface-index")?;
        let keys = lookup_legacy_surface(
            &mut self.source,
            &section,
            self.codelet_count,
            language,
            surface,
        )?;
        self.offsets_for_legacy_keys(&keys)
    }
    pub fn lookup_source(
        &mut self,
        dataset: &str,
        release: &str,
        identifier: &str,
    ) -> Result<Vec<u32>, DictionaryReadError> {
        if self.version_four {
            let section = self.section("source-index")?;
            return lookup_v4(
                &mut self.source,
                &section,
                DirectIndexLookup::Source(dataset, release, identifier),
            );
        }
        let section = self.section("source-index")?;
        let keys = lookup_legacy_source(
            &mut self.source,
            &section,
            self.codelet_count,
            dataset,
            release,
            identifier,
        )?;
        self.offsets_for_legacy_keys(&keys)
    }
    pub fn lookup_operation_signature(
        &mut self,
        key: &OperationSignatureKey,
    ) -> Result<Vec<u32>, DictionaryReadError> {
        if self.version_four {
            let section = self.section("operation-signature-index")?;
            return lookup_v4(
                &mut self.source,
                &section,
                DirectIndexLookup::Operation(key),
            );
        }
        let section = self.section("operation-signature-index")?;
        let keys = lookup_legacy_operation(&mut self.source, &section, self.codelet_count, key)?;
        self.offsets_for_legacy_keys(&keys)
    }
}
