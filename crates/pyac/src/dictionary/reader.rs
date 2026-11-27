use super::reader_codec::{Sentence, read_codelet, read_sentence, read_sentence_v4};
use super::reader_index::next_canonical_offset_after;
use super::volume::{DictionaryMetadata, DictionaryVolumeError};
use super::window::Window;
use super::words::{COMITATIVE, DESTINATION, ESSIVE, INSTRUMENTAL, SOURCE};
use crate::{PyacError, WORDS_PER_CODELET};
use std::io::{Read, Seek};
use std::ops::Range;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DictionarySection {
    pub role: String,
    pub codelet_start: u32,
    pub codelet_count: u32,
}

impl DictionarySection {
    pub fn range(&self) -> Range<u32> {
        self.codelet_start..self.codelet_start + self.codelet_count
    }
}

#[derive(Debug)]
pub enum DictionaryReadError {
    Io(std::io::Error),
    Pack(PyacError),
    Volume(DictionaryVolumeError),
    InvalidRecord(&'static str),
    UnexpectedWord {
        expected: u16,
        actual: Option<u16>,
        offset: usize,
    },
    Context(String, Box<DictionaryReadError>),
    MissingSection(String),
    MissingEntry(Vec<u16>),
}

impl std::fmt::Display for DictionaryReadError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Io(error) => write!(f, "dictionary read failed: {error}"),
            Self::Pack(error) => write!(f, "invalid PyAC dictionary pack: {error}"),
            Self::Volume(error) => write!(f, "invalid PyAC dictionary volume: {error}"),
            Self::InvalidRecord(reason) => write!(f, "invalid dictionary record: {reason}"),
            Self::UnexpectedWord {
                expected,
                actual,
                offset,
            } => write!(
                f,
                "invalid dictionary record: expected case/verb 0x{expected:04x} at word {offset}, found {}",
                actual.map_or_else(|| "end of sentence".into(), |word| format!("0x{word:04x}"))
            ),
            Self::Context(context, error) => write!(f, "{context}: {error}"),
            Self::MissingSection(role) => write!(f, "dictionary has no {role} section"),
            Self::MissingEntry(key) => write!(f, "dictionary has no entry for Pyac key {key:?}"),
        }
    }
}

impl std::error::Error for DictionaryReadError {}
impl From<std::io::Error> for DictionaryReadError {
    fn from(error: std::io::Error) -> Self {
        Self::Io(error)
    }
}
impl From<PyacError> for DictionaryReadError {
    fn from(error: PyacError) -> Self {
        Self::Pack(error)
    }
}
impl From<DictionaryVolumeError> for DictionaryReadError {
    fn from(error: DictionaryVolumeError) -> Self {
        Self::Volume(error)
    }
}

pub(super) struct ProfileRecords {
    pub(super) kind: String,
    pub(super) marker_count: u32,
    pub(super) metadata_count: u32,
    pub(super) metadata: DictionaryMetadata,
    pub(super) toc_count: usize,
    pub(super) profile_version: String,
    pub(super) metadata_entry_count: Option<u32>,
    pub(super) version_four: bool,
}

pub struct DictionaryFileReader<R> {
    /// The volume, read through a bounded window.
    ///
    /// `read_codelet` seeks and reads 32 bytes at a time, and the volume holds
    /// four million codelets. Against a bare `File` that is two syscalls per
    /// codelet: 16.2 s for one decode, against 1.4 s from memory. `BufReader` is
    /// not the fix — a seek discards its buffer — so the reader gets a `Window`
    /// that keeps its buffer across forward seeks, which is the pattern here.
    /// Memory stays at one window, not one volume.
    pub(super) source: Window<R>,
    pub(super) codelet_count: u32,
    pub(super) profile_version: String,
    pub(super) metadata: DictionaryMetadata,
    pub(super) sections: Vec<DictionarySection>,
    pub(super) entry_count: usize,
    pub(super) version_four: bool,
}

impl<R: Read + Seek> DictionaryFileReader<R> {
    pub fn metadata(&self) -> &DictionaryMetadata {
        &self.metadata
    }
    pub fn profile_version(&self) -> &str {
        &self.profile_version
    }
    pub fn sections(&self) -> &[DictionarySection] {
        &self.sections
    }
    pub fn codelet_count(&self) -> u32 {
        self.codelet_count
    }

    pub fn entry_count(&self) -> usize {
        self.entry_count
    }

    pub fn read_section(
        &mut self,
        role: &str,
    ) -> Result<Vec<[u16; WORDS_PER_CODELET]>, DictionaryReadError> {
        let section = self
            .sections
            .iter()
            .find(|section| section.role == role)
            .ok_or_else(|| DictionaryReadError::MissingSection(role.into()))?;
        self.read_range(section.range())
    }

    /// Decode every sentence in an explicit codelet range.
    pub(crate) fn sentences_at(
        &mut self,
        range: Range<u32>,
    ) -> Result<Vec<super::reader_codec::Sentence>, DictionaryReadError> {
        self.sentences_in(range, false)
    }

    pub fn definition_location(
        &mut self,
        key: &[u16],
    ) -> Result<Option<Range<u32>>, DictionaryReadError> {
        if !self.version_four {
            return Ok(self
                .find_legacy_location(key)?
                .map(|(_, definition)| definition));
        }
        let Some(entry_range) = self.location(key)? else {
            return Ok(None);
        };
        let mut ordinal = entry_range.start;
        let mut definition_start = entry_range.end;
        while ordinal < entry_range.end {
            let sentence = read_sentence(&mut self.source, ordinal, entry_range.end)?;
            let consumed = sentence.consumed;
            if !is_v4_entry_metadata(&sentence, key)? {
                definition_start = ordinal;
                break;
            }
            ordinal += consumed;
        }
        Ok(Some(definition_start..entry_range.end))
    }

    pub fn has_definition(&mut self, key: &[u16]) -> Result<bool, DictionaryReadError> {
        Ok(self
            .definition_location(key)?
            .is_some_and(|range| !range.is_empty()))
    }

    pub fn read_entry(
        &mut self,
        key: &[u16],
    ) -> Result<Vec<[u16; WORDS_PER_CODELET]>, DictionaryReadError> {
        let range = self
            .location(key)?
            .ok_or_else(|| DictionaryReadError::MissingEntry(key.to_vec()))?;
        self.read_range(range)
    }

    pub fn read_entry_at(
        &mut self,
        offset: u32,
    ) -> Result<Vec<[u16; WORDS_PER_CODELET]>, DictionaryReadError> {
        if !self.version_four {
            let range = self.find_legacy_range_by_offset(offset)?.ok_or(
                DictionaryReadError::InvalidRecord("legacy entry offset is not indexed"),
            )?;
            return self.read_range(range);
        }
        let entries = self.section("entries")?;
        let entries_end = entries.codelet_start + entries.codelet_count;
        if offset < entries.codelet_start || offset >= entries_end {
            return Err(DictionaryReadError::InvalidRecord(
                "entry offset is outside the entries section",
            ));
        }
        let canonical = self.section("canonical-index")?;
        let next = next_canonical_offset_after(&mut self.source, &canonical, offset)?
            .unwrap_or(entries_end);
        if next <= offset || next > entries_end {
            return Err(DictionaryReadError::InvalidRecord(
                "entry offsets are not strictly increasing within entries",
            ));
        }
        self.read_range(offset..next)
    }

    pub fn read_definition(
        &mut self,
        key: &[u16],
    ) -> Result<Vec<[u16; WORDS_PER_CODELET]>, DictionaryReadError> {
        let range = self
            .definition_location(key)?
            .ok_or_else(|| DictionaryReadError::MissingEntry(key.to_vec()))?;
        self.read_range(range)
    }

    /// Decode every sentence in a codelet range, seeking by pack ordinal.
    ///
    /// Index sections use the v4 quote layout (raw continuation codelets keep
    /// their leading index word); entry bodies use the ordinary layout. The
    /// two are not interchangeable, so the caller says which range it is.
    pub(crate) fn sentences_in(
        &mut self,
        range: Range<u32>,
        indexed: bool,
    ) -> Result<Vec<super::reader_codec::Sentence>, DictionaryReadError> {
        if range.start > range.end || range.end > self.codelet_count {
            return Err(DictionaryReadError::InvalidRecord(
                "requested codelet range is outside the pack",
            ));
        }
        let mut sentences = Vec::new();
        let mut ordinal = range.start;
        while ordinal < range.end {
            let sentence = if indexed {
                read_sentence_v4(&mut self.source, ordinal, range.end)?
            } else {
                read_sentence(&mut self.source, ordinal, range.end)?
            };
            ordinal += sentence.consumed;
            sentences.push(sentence);
        }
        Ok(sentences)
    }

    pub(super) fn read_range(
        &mut self,
        range: Range<u32>,
    ) -> Result<Vec<[u16; WORDS_PER_CODELET]>, DictionaryReadError> {
        if range.start > range.end || range.end > self.codelet_count {
            return Err(DictionaryReadError::InvalidRecord(
                "requested codelet range is outside the pack",
            ));
        }
        let mut result = Vec::with_capacity((range.end - range.start) as usize);
        for ordinal in range {
            result.push(read_codelet(&mut self.source, ordinal)?);
        }
        Ok(result)
    }

    /// The codelet range a section occupies.
    pub fn section_range(&self, role: &str) -> Result<Range<u32>, DictionaryReadError> {
        Ok(self.section(role)?.range())
    }

    pub(super) fn section(&self, role: &str) -> Result<DictionarySection, DictionaryReadError> {
        self.sections
            .iter()
            .find(|section| section.role == role)
            .cloned()
            .ok_or_else(|| DictionaryReadError::MissingSection(role.into()))
    }
}

fn is_v4_entry_metadata(sentence: &Sentence, key: &[u16]) -> Result<bool, DictionaryReadError> {
    use super::reader_codec::{
        RecordSubject, expect_end, expect_word, take_record_subject, take_text, take_vector,
    };

    let words = &sentence.words;
    let mut cursor = 0;
    if take_record_subject(sentence, &mut cursor)? != RecordSubject::Name(key.to_vec()) {
        return Ok(false);
    }
    let text_shapes: [&[u16]; 3] = [
        &[ESSIVE, SOURCE, DESTINATION],
        &[ESSIVE, SOURCE, DESTINATION, INSTRUMENTAL],
        &[ESSIVE, SOURCE, DESTINATION, INSTRUMENTAL, COMITATIVE],
    ];
    for cases in text_shapes {
        let mut position = cursor;
        let matched = cases.iter().all(|case| {
            take_text(words, &mut position).is_ok()
                && expect_word(words, &mut position, *case).is_ok()
        });
        if matched && expect_end(words, &mut position).is_ok() {
            return Ok(true);
        }
    }
    for case in [SOURCE, COMITATIVE, DESTINATION, INSTRUMENTAL] {
        let mut position = cursor;
        if take_vector(words, &mut position).is_ok()
            && expect_word(words, &mut position, case).is_ok()
            && expect_end(words, &mut position).is_ok()
        {
            return Ok(true);
        }
    }
    {
        let mut position = cursor;
        if take_vector(words, &mut position).is_ok()
            && expect_word(words, &mut position, SOURCE).is_ok()
            && expect_end(words, &mut position).is_ok()
        {
            return Ok(true);
        }
    }
    for cases in [
        &[ESSIVE, SOURCE, DESTINATION][..],
        &[ESSIVE, INSTRUMENTAL, COMITATIVE][..],
    ] {
        let mut position = cursor;
        if take_vector(words, &mut position).is_ok()
            && expect_word(words, &mut position, cases[0]).is_ok()
            && take_text(words, &mut position).is_ok()
            && expect_word(words, &mut position, cases[1]).is_ok()
            && take_text(words, &mut position).is_ok()
            && expect_word(words, &mut position, cases[2]).is_ok()
            && expect_end(words, &mut position).is_ok()
        {
            return Ok(true);
        }
    }
    Ok(false)
}
