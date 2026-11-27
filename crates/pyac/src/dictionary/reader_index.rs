//! The in-band index: a key in, the entry offsets that answer it.
//!
//! v4 volumes carry a sorted direct index per lookup kind; v2/v3 volumes have no
//! such index and are scanned instead. The two paths answer the same question.

mod legacy;
mod types;
mod v4;

pub(super) use legacy::{lookup_legacy_operation, lookup_legacy_source, lookup_legacy_surface};
pub(super) use types::DirectIndexLookup;
pub(super) use v4::{canonical_row, lookup_v4, next_canonical_offset_after};
