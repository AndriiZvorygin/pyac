//! Walking a sentence: where it starts, ends, and what it spans.
#![allow(unsafe_code)]

use super::super::*;
use super::fields::FieldBuilder;

#[derive(Clone, Copy)]
pub(crate) struct SentenceView<'a, W: VliwWord> {
    words: &'a [W],
    start_word_index: usize,
    token_count: usize,
}
/// A borrowed Phrase-like span over codelet payloads and their header marks.
/// Reads stay on the source page; parsing does not flatten sentences into scratch arrays.
impl<'a, W: VliwWord> SentenceView<'a, W> {
    pub(crate) fn new(
        words: &'a [W],
        start_codelet: usize,
        token_count: usize,
    ) -> SentenceView<'a, W> {
        SentenceView {
            words,
            start_word_index: start_codelet * CODELET_WORDS + 1,
            token_count,
        }
    }

    pub(crate) fn word_index(&self, index: usize) -> usize {
        Self::word_index_from(self.start_word_index, index)
    }

    pub(crate) fn word_index_from(start_word_index: usize, index: usize) -> usize {
        let start_codelet = start_word_index / CODELET_WORDS;
        let start_slot = start_word_index % CODELET_WORDS;
        let first_codelet_words = CODELET_WORDS - start_slot;
        if index < first_codelet_words {
            start_word_index + index
        } else {
            let remaining = index - first_codelet_words;
            (start_codelet + 1 + remaining / (CODELET_WORDS - 1)) * CODELET_WORDS
                + remaining % (CODELET_WORDS - 1)
                + 1
        }
    }

    pub(crate) fn word_at_word_index(&self, start_word_index: usize, index: usize) -> u32 {
        let source_index = Self::word_index_from(start_word_index, index);
        // SAFETY: callers validate this index against the collected span.
        unsafe { input_word_unchecked(self.words, source_index) }
    }

    pub(crate) fn word_span_is_valid(&self, start_word_index: usize, token_count: usize) -> bool {
        let start_slot = start_word_index % CODELET_WORDS;
        if start_slot == 0 || token_count == 0 {
            return false;
        }
        Self::word_index_from(start_word_index, token_count - 1) < self.words.len()
    }

    pub(crate) fn len(&self) -> usize {
        self.token_count
    }

    pub(crate) fn token(&self, index: usize) -> Option<u32> {
        if index >= self.token_count {
            return None;
        }
        let value = self.word_at_word_index(self.start_word_index, index);
        (value <= MAX_U16).then_some(value)
    }

    pub(crate) fn token_unchecked(&self, index: usize) -> u32 {
        let source_index = self.word_index(index);
        // SAFETY: callers validate the logical index against the collected
        // span, whose codelets and payload words were validated on collection.
        unsafe { input_word_unchecked(self.words, source_index) }
    }

    pub(crate) fn token_or_first(&self, index: usize) -> u32 {
        if index < self.token_count {
            self.token_unchecked(index)
        } else if self.token_count > 0 {
            self.token_unchecked(0)
        } else {
            0
        }
    }

    /// Head word of the codelet that holds logical token `index`.
    pub(crate) fn header_for(&self, index: usize) -> u32 {
        let source_index = self.word_index(index);
        word(self.words, source_index / CODELET_WORDS, 0)
    }

    /// Slot position (1..CODELET_WORDS) of logical token `index`.
    pub(crate) fn slot_for(&self, index: usize) -> usize {
        self.word_index(index) % CODELET_WORDS
    }

    /// Read a payload word by its absolute source index.
    #[cfg(test)]
    pub(crate) fn source_word(&self, source_index: usize) -> u32 {
        // SAFETY: callers pass an index validated by the collected sentence span.
        unsafe { input_word_unchecked(self.words, source_index) }
    }

    pub(crate) fn boundary(&self, index: usize) -> bool {
        if index >= self.token_count {
            return false;
        }
        let source_index = self.word_index(index);
        let codelet = source_index / CODELET_WORDS;
        let slot = source_index % CODELET_WORDS;
        let header = word(self.words, codelet, 0);
        let complete = header & 1 != 0;
        (header & (1 << slot) != 0) == complete
    }

    #[cfg(test)]
    pub(crate) fn boundary_unchecked(&self, index: usize) -> bool {
        let source_index = self.word_index(index);
        let codelet = source_index / CODELET_WORDS;
        let slot = source_index % CODELET_WORDS;
        let source_index = codelet * CODELET_WORDS;
        // SAFETY: callers validate the logical index against the collected span.
        let header = unsafe { input_word_unchecked(self.words, source_index) };
        let complete = header & 1 != 0;
        (header & (1 << slot) != 0) == complete
    }
}
/// Validate one sentence and return its payload length and next codelet offset.
/// Scalar span metadata avoids materializing a token buffer on the GPU path.
pub(crate) fn collect_sentence(
    words: &[impl VliwWord],
    codelet_count: usize,
    start: usize,
) -> Result<(usize, usize), u32> {
    collect_sentence_at(words, codelet_count, 0, codelet_count, start)
}
pub(crate) fn collect_sentence_at(
    words: &[impl VliwWord],
    total_codelet_count: usize,
    base_codelet: usize,
    window_codelet_count: usize,
    start: usize,
) -> Result<(usize, usize), u32> {
    if start >= window_codelet_count
        || base_codelet > total_codelet_count
        || window_codelet_count > total_codelet_count - base_codelet
        || total_codelet_count > words.len() / CODELET_WORDS
    {
        return Err(ERROR_MALFORMED);
    }
    let mut token_count = 0;
    let mut codelet = start;
    loop {
        if codelet >= window_codelet_count {
            return Err(ERROR_MALFORMED);
        }
        let absolute_codelet = base_codelet + codelet;
        let header = word(words, absolute_codelet, 0);
        let complete = header & 1 != 0;
        let mut last_boundary = 0;
        let mut slot = 1;
        while slot < CODELET_WORDS {
            if (header & (1 << slot) != 0) == complete {
                last_boundary = slot;
            }
            slot += 1;
        }
        if complete && last_boundary == 0 {
            return Err(ERROR_MALFORMED);
        }
        let last_slot = if complete {
            last_boundary
        } else {
            CODELET_WORDS - 1
        };
        slot = 1;
        while slot <= last_slot {
            let value = word(words, absolute_codelet, slot);
            if value > MAX_U16 {
                return Err(ERROR_MALFORMED);
            }
            token_count += 1;
            slot += 1;
        }
        codelet += 1;
        if complete {
            return Ok((token_count, codelet));
        }
    }
}
pub(crate) fn parse_sentence(
    tokens: &SentenceView<'_, impl VliwWord>,
    parsed: &mut ParsedSentence,
) -> u32 {
    parse_sentence_range(tokens, 0, tokens.len(), parsed)
}
pub(crate) fn parse_sentence_range(
    tokens: &SentenceView<'_, impl VliwWord>,
    start: usize,
    end: usize,
    parsed: &mut ParsedSentence,
) -> u32 {
    if start >= end || end > tokens.len() {
        return ERROR_MALFORMED;
    }
    // Clause markers are not phrase boundaries, so a sentence without them can
    // be decoded from the head bitmap alone. This pass cannot fail; it only
    // chooses the decode form, so error order is unchanged.
    let mut has_clause = false;
    let mut index = start;
    while index < end {
        let token = tokens.token_unchecked(index);
        if token == DEPENDENT_CLAUSE || token == CLAUSE_TAIL {
            has_clause = true;
            break;
        }
        index += 1;
    }
    let mut builder = FieldBuilder::new(start);
    let error = if has_clause {
        builder.parse_with_clauses(tokens, start, end)
    } else {
        builder.parse_from_boundaries(tokens, start, end)
    };
    if error != 0 {
        return error;
    }
    builder.finish(parsed)
}
pub(crate) fn clause_tail(
    tokens: &SentenceView<'_, impl VliwWord>,
    start: usize,
    end: usize,
) -> Option<usize> {
    if start >= end || end > tokens.len() {
        return None;
    }
    let mut depth = 0usize;
    let mut index = start;
    while index < end {
        let token = tokens.token_unchecked(index);
        if token == DEPENDENT_CLAUSE {
            depth += 1;
        } else if token == CLAUSE_TAIL {
            if depth == 0 {
                return None;
            }
            depth -= 1;
            if depth == 0 {
                return Some(index);
            }
        }
        index += 1;
    }
    None
}
pub(crate) fn embedded_clause_range(
    tokens: &SentenceView<'_, impl VliwWord>,
    field: SentenceField,
) -> Result<(usize, usize), u32> {
    let Some(end) = field.start.checked_add(field.length) else {
        return Err(ERROR_MALFORMED);
    };
    if field.length < 3 || end > tokens.len() {
        return Err(ERROR_MALFORMED);
    }
    if tokens.token_unchecked(field.start) != DEPENDENT_CLAUSE {
        return Err(ERROR_UNSUPPORTED);
    }
    let Some(tail) = clause_tail(tokens, field.start, end) else {
        return Err(ERROR_MALFORMED);
    };
    if tail + 1 != end {
        return Err(ERROR_MALFORMED);
    }
    Ok((field.start + 1, tail))
}
pub(crate) fn next_sentence_is_paragraph_end(
    words: &[impl VliwWord],
    codelet_count: usize,
    base_codelet: usize,
    start: usize,
    routine_end: usize,
) -> Result<bool, u32> {
    if start >= routine_end {
        return Ok(false);
    }
    let (token_count, _) = collect_sentence_at(
        words,
        base_codelet + codelet_count,
        base_codelet,
        codelet_count,
        start,
    )?;
    let tokens = SentenceView::new(words, base_codelet + start, token_count);
    let mut next = ParsedSentence::empty();
    let error = parse_sentence(&tokens, &mut next);
    if error != 0 {
        return Err(error);
    }
    Ok(next.mood == MOOD_PARAGRAPH)
}
pub(crate) fn contains_marked_token(tokens: &SentenceView<'_, impl VliwWord>, target: u32) -> bool {
    let mut index = 0;
    while index < tokens.len() {
        if tokens.boundary(index) && sentence_at!(*tokens, index) == target {
            return true;
        }
        index += 1;
    }
    false
}
