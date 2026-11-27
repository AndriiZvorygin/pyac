//! The source error and its display.

use std::fmt::{Display, Formatter};

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SourceError {
    Unsupported {
        line: usize,
        text: String,
    },
    Orphan {
        line: usize,
        key: String,
    },
    Duplicate {
        line: usize,
        key: String,
    },
    UnknownWord {
        line: usize,
        word: String,
    },
    /// A sentence with no `at` clause — a definition sentence. Rejected.
    DefinitionSentence {
        line: usize,
    },
    MissingMetadata,
    EmptyText {
        line: usize,
        field: &'static str,
    },
}
impl SourceError {
    pub fn code(&self) -> &'static str {
        match self {
            Self::Unsupported { .. } => "unsupported-source-sentence",
            Self::Orphan { .. } => "orphan-entry-record",
            Self::Duplicate { .. } => "duplicate-entry-block",
            Self::UnknownWord { .. } => "unknown-key-word",
            Self::DefinitionSentence { .. } => "unsupported-definition-sentence",
            Self::MissingMetadata => "missing-source-metadata",
            Self::EmptyText { .. } => "empty-source-text",
        }
    }
}
impl Display for SourceError {
    fn fmt(&self, formatter: &mut Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Unsupported { line, text } => {
                write!(
                    formatter,
                    "line {line}: unsupported source sentence: {text}"
                )
            }
            Self::Orphan { line, key } => write!(
                formatter,
                "line {line}: record for {key:?} precedes its entry block"
            ),
            Self::Duplicate { line, key } => {
                write!(formatter, "line {line}: duplicate entry block for {key:?}")
            }
            Self::UnknownWord { line, word } => {
                write!(formatter, "line {line}: key word has no HNUC: {word}")
            }
            Self::DefinitionSentence { line } => write!(
                formatter,
                "line {line}: the accepted source has no definition sentences; \
                 carry the lowered codelets as a pre-lowered definition instead"
            ),
            Self::MissingMetadata => formatter.write_str(
                "dictionary source requires profile and metadata sentences for lyin kwon",
            ),
            Self::EmptyText { line, field } => {
                write!(formatter, "line {line}: {field} must be non-empty text")
            }
        }
    }
}
impl std::error::Error for SourceError {}
