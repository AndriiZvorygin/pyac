//! The shard error and its display.

use std::fmt::{Display, Formatter};

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ShardError {
    /// The manifest is missing its header or has no shard rows.
    MissingManifest,
    /// A shard named by the manifest is absent from the directory.
    MissingShard { name: String },
    /// A declared digest does not match the shard's contents.
    DigestMismatch {
        name: String,
        expected: String,
        actual: String,
    },
    /// A shard's declared entry count does not match what it contains.
    CountMismatch {
        name: String,
        expected: usize,
        actual: usize,
    },
    /// Two shards declare overlapping layers, so order is ambiguous.
    LayerOrder { previous: u16, next: u16 },
    /// A compound appears before a component it names.
    DependencyOrder { key: String, missing: String },
    /// A shard row could not be read.
    MalformedRow { name: String },
    /// The source has no parseable entry blocks to split.
    EmptySource,
}
impl ShardError {
    pub fn code(&self) -> &'static str {
        match self {
            Self::MissingManifest => "missing-shard-manifest",
            Self::MissingShard { .. } => "missing-shard",
            Self::DigestMismatch { .. } => "shard-digest-mismatch",
            Self::CountMismatch { .. } => "shard-count-mismatch",
            Self::LayerOrder { .. } => "shard-layer-order",
            Self::DependencyOrder { .. } => "shard-dependency-order",
            Self::MalformedRow { .. } => "malformed-shard-row",
            Self::EmptySource => "empty-shard-source",
        }
    }
}
impl Display for ShardError {
    fn fmt(&self, formatter: &mut Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::MissingManifest => formatter
                .write_str("shard directory has no manifest with a profile header and shard rows"),
            Self::MissingShard { name } => {
                write!(
                    formatter,
                    "manifest names shard {name:?} but it is not present"
                )
            }
            Self::DigestMismatch {
                name,
                expected,
                actual,
            } => write!(
                formatter,
                "shard {name:?} digest is {actual}, manifest declares {expected}"
            ),
            Self::CountMismatch {
                name,
                expected,
                actual,
            } => write!(
                formatter,
                "shard {name:?} holds {actual} entries, manifest declares {expected}"
            ),
            Self::LayerOrder { previous, next } => write!(
                formatter,
                "shard layers are out of order: {next} follows {previous}"
            ),
            Self::DependencyOrder { key, missing } => write!(
                formatter,
                "compound {key:?} precedes its component {missing:?}"
            ),
            Self::MalformedRow { name } => {
                write!(formatter, "shard manifest row {name:?} is malformed")
            }
            Self::EmptySource => formatter.write_str("source has no entry blocks to split"),
        }
    }
}
impl std::error::Error for ShardError {}
