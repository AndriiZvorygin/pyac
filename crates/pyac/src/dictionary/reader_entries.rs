//! Entry reads, index lookups, and the legacy (v2/v3) index paths.

use super::reader::{DictionaryFileReader, DictionaryReadError};
use super::reader_codec::{RecordSubject, read_sentence, read_sentence_v4, take_record_subject};
use super::words::{COMITATIVE, DESTINATION, ESSIVE, INSTRUMENTAL, SOURCE};

use crate::WORDS_PER_CODELET;
use std::io::{Read, Seek};

impl<R: Read + Seek> DictionaryFileReader<R> {
    /// Every canonical key the volume indexes, in index order.
    ///
    /// A verification pass needs this to check that each advertised entry is
    /// actually readable; `entry_count` alone cannot tell a corrupt record from
    /// a missing one.
    pub fn canonical_keys(&mut self) -> Result<Vec<Vec<u16>>, DictionaryReadError> {
        let section = self.section("canonical-index")?;
        let mut keys = Vec::new();
        let mut ordinal = section.codelet_start;
        let end = section.codelet_start + section.codelet_count;
        while ordinal < end {
            let sentence = read_sentence_v4(&mut self.source, ordinal, end)?;
            let mut cursor = 0;
            if let Ok(RecordSubject::Name(name)) = take_record_subject(&sentence, &mut cursor) {
                keys.push(name);
            }
            ordinal = ordinal
                .checked_add(sentence.consumed)
                .ok_or(DictionaryReadError::InvalidRecord("index ordinal overflow"))?;
        }
        Ok(keys)
    }
    /// Every canonical key, and where its entry body starts, in index order.
    ///
    /// The offsets are strictly increasing and the canonical index sits before
    /// the entries section, so walking this list in order touches the volume
    /// forward and sequentially. That is the difference between a decode that
    /// streams and one that seeks: `location` per key costs four binary searches
    /// over a 2 MB index plus a re-read of the entry, sixty-five thousand times.
    pub fn canonical_keys_with_offsets(
        &mut self,
    ) -> Result<Vec<(Vec<u16>, u32)>, DictionaryReadError> {
        let section = self.section("canonical-index")?;
        let entries = self.section("entries")?;
        let entries_end = entries.codelet_start + entries.codelet_count;
        let mut rows = Vec::with_capacity(self.entry_count);
        let mut ordinal = section.codelet_start;
        let end = section.codelet_start + section.codelet_count;
        while ordinal < end {
            let sentence = read_sentence_v4(&mut self.source, ordinal, end)?;
            let mut cursor = 0;
            let subject = take_record_subject(&sentence, &mut cursor)?;
            if let RecordSubject::Name(name) = subject {
                let offset = super::reader_codec::take_uint32(&sentence.words, &mut cursor)?;
                if offset < entries.codelet_start || offset >= entries_end {
                    return Err(DictionaryReadError::InvalidRecord(
                        "canonical index points outside the entries section",
                    ));
                }
                rows.push((name, offset));
            }
            ordinal = ordinal
                .checked_add(sentence.consumed)
                .ok_or(DictionaryReadError::InvalidRecord("index ordinal overflow"))?;
        }
        Ok(rows)
    }
    /// The codelets of the entry body starting at `offset`, ending where the next
    /// entry begins.
    ///
    /// `next_start` comes from the canonical index the caller already walked, so
    /// no index search happens here. That is what lets a full decode stream.
    pub fn entry_codelets_at(
        &mut self,
        offset: u32,
        next_start: u32,
    ) -> Result<Vec<[u16; WORDS_PER_CODELET]>, DictionaryReadError> {
        let entries = self.section("entries")?;
        let entries_end = entries.codelet_start + entries.codelet_count;
        if offset < entries.codelet_start || offset >= entries_end {
            return Err(DictionaryReadError::InvalidRecord(
                "entry offset is outside the entries section",
            ));
        }
        if next_start <= offset || next_start > entries_end {
            return Err(DictionaryReadError::InvalidRecord(
                "entry offsets are not strictly increasing within entries",
            ));
        }
        self.read_range(offset..next_start)
    }
    /// Every canonical key with all of its English lexical forms.
    ///
    /// The reference volume is the authority; this is the projection a consumer
    /// needs to build its own lookup table from it. Atoms come back as a
    /// one-word key, compounds as a multi-word key.
    pub fn entries_with_english(
        &mut self,
    ) -> Result<Vec<(Vec<u16>, Vec<String>)>, DictionaryReadError> {
        let keys = self.canonical_keys()?;
        let mut out = Vec::with_capacity(keys.len());
        for key in keys {
            let forms = self.english_forms(&key)?;
            out.push((key, forms));
        }
        Ok(out)
    }
    /// Every `en` lexical form recorded for `key`, in volume order.
    ///
    /// A lexical-form sentence is
    /// `name <key> su, text "<language>" as, text "<surface>" from, ...`.
    /// Values precede their case words, so the language text comes before `ESSIVE`.
    fn english_forms(&mut self, key: &[u16]) -> Result<Vec<String>, DictionaryReadError> {
        use super::reader_codec::{take_record_subject, take_text};
        let Some(range) = self.location(key)? else {
            return Ok(Vec::new());
        };
        let mut forms = Vec::new();
        let mut seen = std::collections::HashSet::new();
        let mut ordinal = range.start;
        while ordinal < range.end {
            // Entry bodies keep the ordinary quote encoding; only the index
            // sections use the indexed raw-continuation layout.
            let sentence = read_sentence(&mut self.source, ordinal, range.end)?;
            let words = &sentence.words;
            let mut cursor = 0;
            let is_ours = matches!(
                take_record_subject(&sentence, &mut cursor),
                Ok(RecordSubject::Name(ref name)) if name.as_slice() == key
            );
            if is_ours {
                // Accept only a record that is exactly `as L from S to N`, i.e.
                // one whose case words are ESSIVE, SOURCE, DESTINATION in that order. A
                // source-gloss is `as L from D to R by I with T`; matching the
                // full case sequence is what distinguishes a surface from a
                // dataset name, and it is insensitive to multi-codelet text.
                // Scan only past the subject: a key word can itself equal a case
                // word (`ma` is 0x243e, the destination marker).
                let cases: Vec<u16> = words
                    .iter()
                    .skip(cursor)
                    .copied()
                    .filter(|word| {
                        [ESSIVE, SOURCE, DESTINATION, INSTRUMENTAL, COMITATIVE].contains(word)
                    })
                    .collect();
                if cases == [ESSIVE, SOURCE, DESTINATION] {
                    let mut probe = cursor;
                    if let Ok(language) = take_text(words, &mut probe) {
                        if language == "en" && words.get(probe) == Some(&ESSIVE) {
                            probe += 1;
                            if let Ok(surface) = take_text(words, &mut probe) {
                                if seen.insert(surface.clone()) {
                                    forms.push(surface);
                                }
                            }
                        }
                    }
                }
            }
            ordinal = ordinal
                .checked_add(sentence.consumed)
                .ok_or(DictionaryReadError::InvalidRecord("entry ordinal overflow"))?;
        }
        Ok(forms)
    }
}
