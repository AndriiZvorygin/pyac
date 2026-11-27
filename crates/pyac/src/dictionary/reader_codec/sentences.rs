//! Sentences: the words a codelet run carries, and where a case ends.

use super::super::reader::DictionaryReadError;
use crate::WORDS_PER_CODELET;
use crate::literal::decode_quote_header;
use std::io::{Read, Seek};

use super::header::read_codelet;

#[derive(Default)]
pub(crate) struct QuoteScan {
    explicit: Option<crate::literal::QuoteHeader>,
    remaining: usize,
}
/// A decoded sentence: its words, and for each word whether a grammatical case
/// boundary follows it. The boundary flags come from the codelet header bitmap,
/// so a decoder never has to look for a delimiter that could also appear inside
/// a value — which is what broke keys such as `na`, whose word is itself the
/// nominative marker `0x24DE`.
pub(crate) struct Sentence {
    pub words: Vec<u16>,
    pub boundaries: Vec<bool>,
    pub consumed: u32,
}
pub(crate) fn read_sentence<R: Read + Seek>(
    source: &mut R,
    start: u32,
    limit: u32,
) -> Result<Sentence, DictionaryReadError> {
    read_sentence_with_raw_width(source, start, limit, WORDS_PER_CODELET, false)
}
pub(crate) fn read_sentence_v4<R: Read + Seek>(
    source: &mut R,
    start: u32,
    limit: u32,
) -> Result<Sentence, DictionaryReadError> {
    read_sentence_with_raw_width(source, start, limit, WORDS_PER_CODELET - 1, true)
}
/// Whether slot `slot` of `codelet` holds the last word of a case.
///
/// The writer marks boundaries positively in a sentence's final codelet and
/// negatively in earlier ones (`pack_words` sets a bit when
/// `boundary == complete`), so the polarity depends on the completion bit.
pub(crate) fn slot_is_boundary(
    codelet: &[u16; WORDS_PER_CODELET],
    slot: usize,
    complete: bool,
) -> bool {
    let set = (codelet[0] >> slot) & 1 == 1;
    if complete { set } else { !set }
}
pub(crate) fn read_sentence_with_raw_width<R: Read + Seek>(
    source: &mut R,
    start: u32,
    limit: u32,
    raw_width: usize,
    indexed_raw: bool,
) -> Result<Sentence, DictionaryReadError> {
    let mut words = Vec::new();
    let mut boundaries = Vec::new();
    let mut scan = QuoteScan::default();
    let mut ordinal = start;
    loop {
        if ordinal >= limit {
            return Err(DictionaryReadError::InvalidRecord("sentence is truncated"));
        }
        let codelet = read_codelet(source, ordinal)?;
        ordinal += 1;
        let raw = scan_raw_text(&codelet, &mut scan);
        let used_slots = raw.map_or(WORDS_PER_CODELET - 1, |(_, slot)| slot);
        let complete = codelet[0] & 1 != 0;
        for slot in 1..=used_slots {
            words.push(codelet[slot]);
            boundaries.push(slot_is_boundary(&codelet, slot, complete));
        }
        if let Some((length, _)) = raw {
            let mut remaining = length;
            for _ in 0..length.div_ceil(raw_width) {
                if ordinal >= limit {
                    return Err(DictionaryReadError::InvalidRecord(
                        "raw text quote is truncated",
                    ));
                }
                let raw_codelet = read_codelet(source, ordinal)?;
                ordinal += 1;
                if indexed_raw && raw_codelet[0] != 0 {
                    return Err(DictionaryReadError::InvalidRecord(
                        "raw text continuation has a nonzero codelet index",
                    ));
                }
                let payload_start = usize::from(indexed_raw);
                let used = remaining.min(raw_width);
                for slot in payload_start..payload_start + used {
                    words.push(raw_codelet[slot]);
                    // Continuation payload is literal text, never a case boundary.
                    boundaries.push(false);
                }
                remaining -= used;
            }
        } else if complete {
            while words.last() == Some(&0) {
                words.pop();
                boundaries.pop();
            }
            return Ok(Sentence {
                words,
                boundaries,
                consumed: ordinal - start,
            });
        }
    }
}
pub(crate) fn scan_raw_text(
    codelet: &[u16; WORDS_PER_CODELET],
    scan: &mut QuoteScan,
) -> Option<(usize, usize)> {
    for slot in 1..WORDS_PER_CODELET {
        let word = codelet[slot];
        if scan.remaining > 0 {
            scan.remaining -= 1;
            continue;
        }
        if let Some(header) = scan.explicit.take() {
            let count = word as usize;
            if header.sort == 0 && header.scalar_bytes == 2 && count > 14 {
                return Some((count, slot));
            }
            scan.remaining = count * (header.scalar_bytes / 2);
            continue;
        }
        if let Some(header) = decode_quote_header(word) {
            if header.explicit_vector_length {
                scan.explicit = Some(header);
            } else if let Some(count) = header.vector_words {
                scan.remaining = count * (header.scalar_bytes / 2);
            }
        }
    }
    None
}
