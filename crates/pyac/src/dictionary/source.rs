//! Parse the authored dictionary `.en.pya` source into a build request.
//!
//! This is the bootstrap step: `pyac-dictionary compile` reads the source
//! directly, so regenerating the volume needs no JavaScript. It replaces
//! `program/library/dictionary/source.mjs` from the `pyash` checkout.
//!
//! # The source is records, not programs
//!
//! Every line of `reference.en.pya` is one of:
//!
//! ```text
//! su name lyin kwon ob text "…" be text ya                       profile marker
//! su name lyin kwon ob text "…" as num 1 from text "…" …         profile metadata
//! su name tlat kwon ob text "<key>" at text "entry" be text ya   entry block
//! su name tlat kwon ob text "<key>" as text "en" from … at text "lexical-form" be text ya
//! ```
//!
//! There are no definition sentences in the accepted reference source — 730,376
//! of its 730,379 lines are records, and the other three are the two profile
//! sentences and the closing `prah`. That matters: the JavaScript parser's
//! fallback lowers definition sentences to VLIW, and that path is never taken
//! here. A port that reproduced it would be porting dead code.
//!
//! A sentence without an `at` clause is rejected rather than dropped, so the day
//! a definition sentence appears the build says so instead of quietly producing a
//! thinner volume.
//!
//! # Clause order
//!
//! The source writes **case then value**: `ob text "b6" at text "entry"`. This is
//! the opposite of the encoded volume, which writes value then case word. Reading
//! the source as if it were the volume silently parses nothing.
//!
//! # Records are positional within an entry block
//!
//! `at text "entry"` opens an entry; every later record attaches to the most
//! recently opened one, until the next `entry`. A record before any entry block
//! is an error, not a defaulted empty entry.

use std::collections::HashMap;
use std::fmt::{Display, Formatter};

mod document;
mod errors;
mod keys;
mod lexing;
mod records;
mod signatures;
mod values;

pub use document::parse_source;
pub use errors::SourceError;
pub use records::{
    ArgumentRecord, DictionarySource, EntryRecord, LexicalFormRecord, ResultRecord,
    SignatureRecord, SourceGlossRecord, SourceLinkRecord, VolumeMetadataRecord,
};
