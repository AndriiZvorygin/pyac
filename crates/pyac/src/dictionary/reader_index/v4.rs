use super::super::OperationSignatureKey;
use super::super::reader::{DictionaryReadError, DictionarySection};
use super::super::reader_codec::{
    RecordSubject, Sentence, expect_end, expect_word, read_codelet, read_sentence_v4,
    take_record_subject, take_text, take_uint32, take_uint32_vector,
};
use super::super::words::{ESSIVE, LOCATIVE, SOURCE};
use std::io::{Read, Seek};

use super::types::{DirectIndexKey, DirectIndexLookup, DirectIndexRecord};

pub(crate) fn lookup_v4<R: Read + Seek>(
    source: &mut R,
    section: &DictionarySection,
    lookup: DirectIndexLookup<'_>,
) -> Result<Vec<u32>, DictionaryReadError> {
    Ok(lookup_v4_record(source, section, lookup)?
        .map(|record| record.offsets)
        .unwrap_or_default())
}
/// The whole index row a lookup lands on, not just its offsets.
///
/// A caller that needs the entry's *end* wants `record.next`, which the row
/// already carries. Reading the offsets alone and then searching again for the
/// successor costs a second full binary search over the index — that is what
/// `location` used to do, 65,598 times in a full read-back.
pub(crate) fn lookup_v4_record<R: Read + Seek>(
    source: &mut R,
    section: &DictionarySection,
    lookup: DirectIndexLookup<'_>,
) -> Result<Option<DirectIndexRecord>, DictionaryReadError> {
    let mut low = section.codelet_start;
    let mut high = section.codelet_start + section.codelet_count;
    while low < high {
        let probe = low + (high - low) / 2;
        let Some(record) = find_next_record(source, section, probe, high)? else {
            high = probe;
            continue;
        };
        match compare_lookup(&record.key, lookup) {
            std::cmp::Ordering::Less => low = record.next,
            std::cmp::Ordering::Equal => return Ok(Some(record)),
            std::cmp::Ordering::Greater => high = probe,
        }
    }

    let section_end = section.codelet_start + section.codelet_count;
    let mut ordinal = low;
    while ordinal < high && ordinal < section_end {
        let sentence = read_sentence_v4(source, ordinal, section_end)?;
        let record = parse_v4_record(&sentence, ordinal + sentence.consumed)?;
        match compare_lookup(&record.key, lookup) {
            std::cmp::Ordering::Less => ordinal = record.next,
            std::cmp::Ordering::Equal => return Ok(Some(record)),
            std::cmp::Ordering::Greater => break,
        }
    }
    Ok(None)
}
/// The canonical index row for `key`, with the offset its entry body ends at.
///
/// `next` is the offset of the *following* canonical row, which is the first
/// codelet after this entry's body. `DirectIndexRecord::next` is a position in the
/// index section rather than an entry offset, so the successor row is read from
/// there — one sequential sentence, not a second binary search over the index.
pub(crate) fn canonical_row<R: Read + Seek>(
    source: &mut R,
    section: &DictionarySection,
    key: &[u16],
    entries_end: u32,
) -> Result<Option<(u32, u32)>, DictionaryReadError> {
    let Some(record) = lookup_v4_record(source, section, DirectIndexLookup::Canonical(key))? else {
        return Ok(None);
    };
    let start = *record
        .offsets
        .first()
        .ok_or_else(|| invalid("canonical index row carries no offset"))?;
    let section_end = section.codelet_start + section.codelet_count;
    let end = if record.next < section_end {
        let sentence = read_sentence_v4(source, record.next, section_end)?;
        let successor = parse_v4_record(&sentence, record.next + sentence.consumed)?;
        match successor.key {
            DirectIndexKey::Canonical(_) => *successor
                .offsets
                .first()
                .ok_or_else(|| invalid("canonical index row carries no offset"))?,
            _ => return Err(invalid("canonical index contains a noncanonical row")),
        }
    } else {
        entries_end
    };
    Ok(Some((start, end)))
}
pub(crate) fn next_canonical_offset_after<R: Read + Seek>(
    source: &mut R,
    section: &DictionarySection,
    wanted: u32,
) -> Result<Option<u32>, DictionaryReadError> {
    let mut low = section.codelet_start;
    let mut high = section.codelet_start + section.codelet_count;
    while low < high {
        let probe = low + (high - low) / 2;
        let Some((record_start, record)) =
            find_canonical_record_at_or_before(source, section, probe)?
        else {
            high = probe;
            continue;
        };
        let offset = match record.key {
            DirectIndexKey::Canonical(_) => record.offsets[0],
            _ => return Err(invalid("canonical index contains a noncanonical row")),
        };
        if offset <= wanted {
            low = record.next;
        } else {
            high = record_start;
        }
    }
    let section_end = section.codelet_start + section.codelet_count;
    if low < section_end {
        let sentence = read_sentence_v4(source, low, section_end)?;
        let record = parse_v4_record(&sentence, low + sentence.consumed)?;
        let offset = match record.key {
            DirectIndexKey::Canonical(_) => record.offsets[0],
            _ => return Err(invalid("canonical index contains a noncanonical row")),
        };
        if offset > wanted {
            return Ok(Some(offset));
        }
    }
    Ok(None)
}
pub(crate) fn find_canonical_record_at_or_before<R: Read + Seek>(
    source: &mut R,
    section: &DictionarySection,
    probe: u32,
) -> Result<Option<(u32, DirectIndexRecord)>, DictionaryReadError> {
    let mut start = probe;
    while start > section.codelet_start {
        if read_codelet(source, start - 1)?[0] & 1 != 0 {
            break;
        }
        start -= 1;
    }
    let section_end = section.codelet_start + section.codelet_count;
    let sentence = read_sentence_v4(source, start, section_end)?;
    let record = parse_v4_record(&sentence, start + sentence.consumed)?;
    if !matches!(record.key, DirectIndexKey::Canonical(_)) {
        return Err(invalid("canonical index contains a noncanonical row"));
    }
    Ok(Some((start, record)))
}
pub(crate) fn find_next_record<R: Read + Seek>(
    source: &mut R,
    section: &DictionarySection,
    probe: u32,
    search_end: u32,
) -> Result<Option<DirectIndexRecord>, DictionaryReadError> {
    let section_end = section.codelet_start + section.codelet_count;
    let mut ordinal = probe;
    while ordinal < search_end {
        let starts_here = ordinal == section.codelet_start
            || (ordinal > section.codelet_start && read_codelet(source, ordinal - 1)?[0] & 1 != 0);
        if starts_here {
            match read_sentence_v4(source, ordinal, section_end) {
                Ok(sentence) => {
                    if let Ok(record) = parse_v4_record(&sentence, ordinal + sentence.consumed) {
                        return Ok(Some(record));
                    }
                }
                Err(DictionaryReadError::Io(error)) => {
                    return Err(DictionaryReadError::Io(error));
                }
                Err(_) => {}
            }
        }
        ordinal += 1;
    }
    Ok(None)
}
pub(crate) fn parse_v4_record(
    sentence: &Sentence,
    next: u32,
) -> Result<DirectIndexRecord, DictionaryReadError> {
    let words = &sentence.words;
    let mut cursor = 0;
    let subject = take_record_subject(sentence, &mut cursor)?;
    let key = match subject {
        RecordSubject::Name(name) => {
            let offset = take_uint32(words, &mut cursor)?;
            expect_word(words, &mut cursor, LOCATIVE)?;
            DirectIndexKey::Canonical(name).with_offsets(vec![offset], next)
        }
        RecordSubject::Text(surface) => {
            let first = take_text(words, &mut cursor)?;
            expect_word(words, &mut cursor, ESSIVE)?;
            let (key, offsets) = if words.get(cursor).copied() == Some(0x0e1d) {
                let release = take_text(words, &mut cursor)?;
                expect_word(words, &mut cursor, SOURCE)?;
                let dataset = first;
                let offsets = take_offsets(words, &mut cursor)?;
                expect_word(words, &mut cursor, LOCATIVE)?;
                (DirectIndexKey::Source(surface, dataset, release), offsets)
            } else {
                let offsets = take_offsets(words, &mut cursor)?;
                expect_word(words, &mut cursor, LOCATIVE)?;
                (
                    DirectIndexKey::Surface(surface.encode_utf16().collect(), first),
                    offsets,
                )
            };
            key.with_offsets(offsets, next)
        }
        RecordSubject::Uint16Vector(signature) => {
            let offsets = take_offsets(words, &mut cursor)?;
            expect_word(words, &mut cursor, LOCATIVE)?;
            DirectIndexKey::Operation(signature).with_offsets(offsets, next)
        }
    };
    expect_end(words, &mut cursor)?;
    Ok(key)
}
pub(crate) fn take_offsets(
    words: &[u16],
    cursor: &mut usize,
) -> Result<Vec<u32>, DictionaryReadError> {
    let offsets = if words.get(*cursor).copied() == Some(0x901d) {
        vec![take_uint32(words, cursor)?]
    } else {
        take_uint32_vector(words, cursor)?
    };
    if offsets.is_empty() || offsets.windows(2).any(|pair| pair[0] >= pair[1]) {
        return Err(invalid(
            "index offsets must be non-empty and strictly increasing",
        ));
    }
    Ok(offsets)
}
pub(crate) fn compare_lookup(
    key: &DirectIndexKey,
    lookup: DirectIndexLookup<'_>,
) -> std::cmp::Ordering {
    match (key, lookup) {
        (DirectIndexKey::Canonical(key), DirectIndexLookup::Canonical(needle)) => {
            key.as_slice().cmp(needle)
        }
        (DirectIndexKey::Surface(surface, language), DirectIndexLookup::Surface(lang, form)) => {
            surface
                .as_slice()
                .cmp(form.encode_utf16().collect::<Vec<_>>().as_slice())
                .then_with(|| language.as_str().cmp(lang))
        }
        (
            DirectIndexKey::Source(identifier, dataset, release),
            DirectIndexLookup::Source(needle_dataset, needle_release, needle_identifier),
        ) => (identifier.as_str(), dataset.as_str(), release.as_str()).cmp(&(
            needle_identifier,
            needle_dataset,
            needle_release,
        )),
        (DirectIndexKey::Operation(signature), DirectIndexLookup::Operation(needle)) => signature
            .as_slice()
            .cmp(operation_key_words(needle).as_slice()),
        _ => std::cmp::Ordering::Greater,
    }
}
pub(crate) fn operation_key_words(key: &OperationSignatureKey) -> Vec<u16> {
    let mut words = Vec::new();
    append_sequence(&mut words, key.verb());
    append_sequence(&mut words, key.mood());
    words.push(key.arguments().len() as u16);
    for argument in key.arguments() {
        append_sequence(&mut words, &argument.grammatical_case);
        append_sequence(&mut words, &argument.noun_class);
    }
    words
}
pub(crate) fn append_sequence(output: &mut Vec<u16>, sequence: &[u16]) {
    output.push(sequence.len() as u16);
    output.extend(sequence);
}
pub(crate) fn invalid(reason: &'static str) -> DictionaryReadError {
    DictionaryReadError::InvalidRecord(reason)
}
