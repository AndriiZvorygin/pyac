//! Split the dictionary source into structural shards, and assemble them back.
//!
//! The source is 101 MB, over git's 100 MB hard per-file limit, so it cannot be
//! committed whole. The split is not arbitrary: it follows the vocabulary's own
//! dependency order.
//!
//! # Why atoms stay whole and compounds split by layer
//!
//! A compound may only name components that are themselves entries — the volume
//! writer rejects anything else (`component key does not resolve`). So atoms must
//! precede compounds, and a compound at layer *n* may only use words at layer
//! ≤ *n*. That ordering is already how the vocabulary was built: a word's layer is
//! the lowest glyph layer that can spell it, and an entry's layer is the maximum
//! over its own key and its components.
//!
//! Atoms total 17.9 MB on their own, well under the warning threshold, and every
//! compound depends on them, so they are not split further. Compounds are 83.6 MB
//! and split cleanly by layer.
//!
//! # The manifest
//!
//! `00-manifest.en.pya` carries the profile marker, the metadata sentence, and
//! one row per content shard naming its file, digest, entry count and maximum
//! layer. Assembly is: read the manifest, concatenate the named shards in order,
//! close with `prah`. A shard with no manifest entry is ignored, so a stray file
//! in the directory cannot silently change the vocabulary.
//!
//! # Two source forms, two extensions
//!
//! The records here are English-form Pyash (`su name … ob text … be text ya`),
//! so they carry `.en.pya`. The plain `.pya` extension is reserved for the glyph
//! Pyac form, which is the language, not a rendering of it. The splitter emits

mod blocks;
mod errors;
mod layers;
mod manifest;
mod names;
mod split;

pub use errors::ShardError;
pub use layers::LayerIndex;
pub use manifest::{ShardRecord, ShardSet, assemble_shards, parse_manifest, shard_digest};
pub use names::{
    BEYOND_LAST_LAYER, EN_SOURCE_EXT, LAYERS, MANIFEST_FILE, PYAC_SOURCE_EXT,
    REFERENCE_SOURCE_FILE, shard_candidates, shard_file_name,
};
pub use split::{split_source, split_source_with_layers};
