//! Building a sentence field by field as words are read.
#![allow(unsafe_code)]

use super::super::*;
use super::names::is_root_name;

pub(crate) struct FieldBuilder {
    fields: [SentenceField; MAX_SENTENCE_FIELDS],
    field_count: usize,
    seen_roles: u32,
    value_start: usize,
    verb_start: usize,
    verb_length: usize,
    mood: u32,
}
impl FieldBuilder {
    pub(super) const fn new(start: usize) -> Self {
        Self {
            fields: [SentenceField::EMPTY; MAX_SENTENCE_FIELDS],
            field_count: 0,
            seen_roles: 0,
            value_start: start,
            verb_start: 0,
            verb_length: 0,
            mood: 0,
        }
    }

    pub(crate) fn accept_boundary(
        &mut self,
        tokens: &SentenceView<'_, impl VliwWord>,
        token: u32,
        offset: usize,
        end: usize,
    ) -> u32 {
        let role_bit = match token {
            NOMINATIVE => 1 << 0,
            ACCUSATIVE => 1 << 1,
            ESSIVE => 1 << 2,
            DESTINATION => 1 << 3,
            SOURCE => 1 << 4,
            COMITATIVE => 1 << 5,
            INSTRUMENTAL => 1 << 6,
            FROM_INDEX => 1 << 7,
            TO_INDEX => 1 << 8,
            LOCATIVE => 1 << 9,
            _ => 0,
        };
        if role_bit != 0 {
            let field_index = self.field_count;
            if self.value_start >= offset
                || field_index >= MAX_SENTENCE_FIELDS
                || self.seen_roles & role_bit != 0
                || validate_field_tags(tokens, self.value_start, offset) != 0
            {
                return ERROR_MALFORMED;
            }
            array_put!(
                &mut self.fields,
                field_index,
                SentenceField {
                    role: token,
                    start: self.value_start,
                    length: offset - self.value_start,
                }
            );
            self.seen_roles |= role_bit;
            self.field_count = field_index + 1;
            self.value_start = offset + 1;
            return 0;
        }
        if matches!(
            token,
            MOOD_DEFINITION | REALIS | DEONTIC | CONDITIONAL | MOOD_RETURN | MOOD_PARAGRAPH
        ) {
            let verb_length = offset - self.value_start;
            if self.mood != 0
                || (verb_length == 0 && token != MOOD_RETURN)
                || (verb_length != 0 && token == MOOD_RETURN)
                || offset + 1 != end
            {
                return ERROR_MALFORMED;
            }
            self.verb_start = self.value_start;
            self.verb_length = verb_length;
            self.mood = token;
            return 0;
        }
        ERROR_UNSUPPORTED
    }

    /// Clause-free decode: the head bitmap is the field map, so only boundary
    /// slots are read and value words are never decoded.
    pub(crate) fn parse_from_boundaries(
        &mut self,
        tokens: &SentenceView<'_, impl VliwWord>,
        start: usize,
        end: usize,
    ) -> u32 {
        let mut offset = start;
        while offset < end {
            let header = tokens.header_for(offset);
            let complete = header & 1 != 0;
            let mut slot = tokens.slot_for(offset);
            while slot < CODELET_WORDS && offset < end {
                if (header & (1 << slot) != 0) == complete {
                    let token = tokens.token_unchecked(offset);
                    let error = self.accept_boundary(tokens, token, offset, end);
                    if error != 0 || self.mood != 0 {
                        return error;
                    }
                }
                slot += 1;
                offset += 1;
            }
        }
        0
    }

    /// Clause-aware decode. Clause markers may sit anywhere, so they are
    /// recognized before the boundary test, exactly as the previous parser did.
    pub(crate) fn parse_with_clauses(
        &mut self,
        tokens: &SentenceView<'_, impl VliwWord>,
        start: usize,
        end: usize,
    ) -> u32 {
        let mut depth = 0usize;
        let mut index = start;
        while index < end {
            let token = tokens.token_unchecked(index);
            if token == DEPENDENT_CLAUSE {
                depth += 1;
                index += 1;
                continue;
            }
            if token == CLAUSE_TAIL {
                if depth == 0 {
                    return ERROR_MALFORMED;
                }
                depth -= 1;
                index += 1;
                continue;
            }
            if depth == 0 && tokens.boundary(index) {
                let error = self.accept_boundary(tokens, token, index, end);
                if error != 0 || self.mood != 0 {
                    return error;
                }
            }
            index += 1;
        }
        0
    }

    pub(crate) fn finish(&self, parsed: &mut ParsedSentence) -> u32 {
        if self.mood == 0 {
            return ERROR_MALFORMED;
        }
        let mut result = ParsedSentence::empty();
        let mut index = 0;
        while index < self.field_count {
            array_put!(&mut result.fields, index, array_at!(self.fields, index));
            index += 1;
        }
        result.field_count = self.field_count;
        result.verb_start = self.verb_start;
        result.verb_length = self.verb_length;
        result.mood = self.mood;
        *parsed = result;
        0
    }
}
/// The `gi` rule, and nothing more.
///
/// `gi` marks a name, so it may appear at most once per field, only followed by
/// root words, and optionally preceded by class words describing the kind of
/// name:
///
/// ```text
/// gi <root…>               a name
/// dlek flut gi <hzit>      a vector-float name
/// ```
///
/// `gi sa` is rejected, because `sa` is a pronoun doing its grammatical job
/// rather than a name. A field with no `gi` is not policed here: it may be a
/// pronoun, a genitive construction, a quoted value, or a nested clause, and
/// those are validated by the code that understands them.
pub(crate) fn validate_field_tags(
    tokens: &SentenceView<'_, impl VliwWord>,
    start: usize,
    end: usize,
) -> u32 {
    if start >= end {
        return ERROR_MALFORMED;
    }
    // A field holding a nested clause is a sentence in its own right, and is
    // validated when that clause is parsed.
    if (start..end)
        .any(|index| matches!(sentence_at!(*tokens, index), DEPENDENT_CLAUSE | CLAUSE_TAIL))
    {
        return 0;
    }
    // A quoted value is its own encoding, not a name.
    if numeric::quote_header(sentence_at!(*tokens, start)).valid != 0 {
        return 0;
    }

    let mut name_tag = None;
    for index in start..end {
        if sentence_at!(*tokens, index) == NAME_TYPE {
            if name_tag.is_some() {
                // A name has one payload, so one tag.
                return ERROR_MALFORMED;
            }
            name_tag = Some(index);
        }
    }
    let Some(name_index) = name_tag else {
        return 0;
    };

    // Everything before the tag must be a class word: an untyped `gi` has none.
    let mut index = start;
    while index < name_index && is_class_word(sentence_at!(*tokens, index)) {
        index += 1;
    }
    if index != name_index {
        return ERROR_MALFORMED;
    }
    // The payload is one or more roots, never a grammar word.
    if !is_root_name(tokens, name_index + 1, end) {
        return ERROR_MALFORMED;
    }
    0
}
/// Whether `word` names the kind of value that follows it.
///
/// Width words, the boolean class, and the vector/float class words: each says
/// "what follows is a value of this kind", which is why none of them is a name.
pub(crate) fn is_class_word(word: u32) -> bool {
    is_width_word(word) || matches!(word, VECTOR_TYPE | FLOAT_TYPE)
}
