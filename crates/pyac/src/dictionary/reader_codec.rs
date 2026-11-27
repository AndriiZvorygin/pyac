//! Reading the volume's records back out of codelets.
//!
//! A sentence is a run of codelets holding words and a boundary bitmap; a record
//! is a sentence whose words follow one of the volume's record shapes. This layer
//! turns words into values and knows nothing about entries.

mod header;
mod records;
mod sections;
mod sentences;
mod subjects;
mod values;

pub(crate) use header::{read_codelet, validate_header};
pub(crate) use records::{parse_location, parse_marker, parse_metadata, parse_toc, parse_toc_v4};
pub(crate) use sections::{validate_sections, validate_sections_v4};
pub(crate) use sentences::{Sentence, read_sentence, read_sentence_v4};
pub(crate) use subjects::{RecordSubject, take_record_subject};
pub(crate) use values::{
    expect_end, expect_word, subject_end, take_text, take_uint32, take_uint32_vector, take_vector,
};
