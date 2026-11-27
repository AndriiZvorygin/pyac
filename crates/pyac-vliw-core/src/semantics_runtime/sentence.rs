//! Reading a sentence out of a codelet run.
//!
//! A sentence is a span of words with fields; a field is a value and the case that
//! names it. The three parts here are separable: finding the span, accumulating
//! fields, and deciding which words inside a field are its name.

#[path = "sentence/fields.rs"]
pub(crate) mod fields;
#[path = "sentence/names.rs"]
mod names;
#[path = "sentence/view.rs"]
mod view;

pub(crate) use names::{name_span, typed_name_span, typed_output_name};
pub(crate) use view::{
    SentenceView, collect_sentence, collect_sentence_at, contains_marked_token,
    embedded_clause_range, next_sentence_is_paragraph_end, parse_sentence, parse_sentence_range,
};
