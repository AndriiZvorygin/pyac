//! Name spans: which words of a field are the typed name.
#![allow(unsafe_code)]

use super::super::*;
use super::fields::is_class_word;

pub(crate) fn name_span(
    tokens: &SentenceView<'_, impl VliwWord>,
    field: SentenceField,
    typed_number: bool,
    name: &mut NameSpan,
) -> u32 {
    let Some(end) = field.start.checked_add(field.length) else {
        return ERROR_MALFORMED;
    };
    if field.length < 2 || end > tokens.len() {
        return ERROR_MALFORMED;
    }
    let name_start = if typed_number {
        // A typed local is `<class words…> [gi] <name>`: the class word says what
        // it holds and `gi` marks the name. The class word leads.
        let mut index = field.start;
        while index < end && is_class_word(sentence_at!(*tokens, index)) {
            index += 1;
        }
        if index == field.start {
            return ERROR_MALFORMED;
        }
        if index < end && sentence_at!(*tokens, index) == NAME_TYPE {
            index += 1;
        }
        index
    } else {
        if sentence_at!(*tokens, field.start) != NAME_TYPE {
            return ERROR_MALFORMED;
        }
        if sentence_at!(*tokens, field.start + 1) == NAME_TYPE {
            return ERROR_MALFORMED;
        }
        let mut marker = field.start + 2;
        while marker < end {
            if sentence_at!(*tokens, marker) == NAME_TYPE {
                return ERROR_MALFORMED;
            }
            marker += 1;
        }
        field.start + 1
    };
    if name_start >= end || !is_root_name(tokens, name_start, end) {
        return ERROR_MALFORMED;
    }
    *name = NameSpan {
        start: name_start,
        length: end - name_start,
    };
    0
}
/// Whether every word in `start..end` is a root word.
///
/// A name's payload is one or more roots, never a grammar word: `gi` followed by
/// a pronoun is not a name, because the pronoun is serving its grammatical
/// purpose.
pub(crate) fn is_root_name(
    tokens: &SentenceView<'_, impl VliwWord>,
    start: usize,
    end: usize,
) -> bool {
    start < end
        && (start..end).all(|index| {
            matches!(
                word_code::classify_identity(sentence_at!(*tokens, index) as u16),
                Some(word_code::WordClass::LongRoot) | Some(word_code::WordClass::ShortRoot)
            )
        })
}
pub(crate) fn typed_name_span(
    tokens: &SentenceView<'_, impl VliwWord>,
    field: SentenceField,
    name: &mut NameSpan,
    kind: &mut u32,
) -> u32 {
    let Some(end) = field.start.checked_add(field.length) else {
        return ERROR_MALFORMED;
    };
    if field.length < 2 || end > tokens.len() {
        return ERROR_MALFORMED;
    }

    // Leading class words say what kind of value follows, then `gi` optionally
    // marks a name. A field with no class word is an untyped name.
    let mut index = field.start;
    let mut has_vector = false;
    let mut has_float = false;
    let mut has_boolean = false;
    let mut has_number = false;
    while index < end && is_class_word(sentence_at!(*tokens, index)) {
        match sentence_at!(*tokens, index) {
            VECTOR_TYPE => has_vector = true,
            FLOAT_TYPE => has_float = true,
            BOOLEAN_TYPE => has_boolean = true,
            _ => has_number = true,
        }
        index += 1;
    }
    if index == field.start {
        // No class word: an ordinary name.
        *kind = 0;
        return name_span(tokens, field, false, name);
    }
    if index < end && sentence_at!(*tokens, index) == NAME_TYPE {
        index += 1;
    }
    if index >= end || !is_root_name(tokens, index, end) {
        return ERROR_MALFORMED;
    }
    *kind = if has_vector && has_float {
        VALUE_VECTOR_FLOAT
    } else if has_float {
        VALUE_FLOAT
    } else if has_boolean {
        VALUE_BOOLEAN
    } else if has_number {
        VALUE_NUMBER
    } else {
        return ERROR_UNSUPPORTED;
    };
    *name = NameSpan {
        start: index,
        length: end - index,
    };
    0
}
pub(crate) fn typed_output_name(
    tokens: &SentenceView<'_, impl VliwWord>,
    field: SentenceField,
    name: &mut NameSpan,
    kind: &mut u32,
) -> u32 {
    let error = typed_name_span(tokens, field, name, kind);
    if error != 0 {
        return error;
    }
    if *kind == 0 {
        return ERROR_MALFORMED;
    }
    0
}
