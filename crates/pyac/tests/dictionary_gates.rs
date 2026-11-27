//! Every gate that reads the whole dictionary, in one test binary.
//!
//! These used to be nine separate binaries — `dictionary_compile`,
//! `dictionary_roundtrip`, `dictionary_render`, `dictionary_selfhost`, and so on.
//! Cargo runs test binaries one at a time, and each one independently assembled the
//! 96 MB authored source or decoded the 131 MB published volume. The work was the
//! same in every process; only the assertions differed.
//!
//! One binary lets `fixtures` derive each artifact once and every gate read it.
//! The gates stay separate `#[test]`s, so `cargo test --test dictionary_gates pyac`
//! still runs one, and a failure still names the single thing that broke.
//!
//! Layout, kept under the 16 KB per-file limit:
//!
//! - `fixtures` — the artifacts, each derived on first use
//! - `volume`   — decode/encode and render/compile over the published volume
//! - `source`   — the bootstrap: the authored source compiles to it, deterministically
//! - `glyph`    — the Pyac renderer's per-entry gate
//! - `records`  — how much of the record surface the tablet parser reproduces
//! - `tablet`   — pinned codelets for one text record and one vector record
//! - `shard`    — split/assemble over the committed shards
//! - `dispatch` — signatures separate from vocabulary, one verb one file
//!
//! A gate that needs an artifact the checkout does not have skips rather than
//! fails: a clean clone must be green. See `common::reference_volume_or_skip`.

mod common;

#[path = "dictionary_gates/fixtures.rs"]
mod fixtures;

#[path = "dictionary_gates/dispatch.rs"]
mod dispatch;
#[path = "dictionary_gates/glyph.rs"]
mod glyph;
#[path = "dictionary_gates/records.rs"]
mod records;
#[path = "dictionary_gates/shard.rs"]
mod shard;
#[path = "dictionary_gates/source.rs"]
mod source;
#[path = "dictionary_gates/tablet.rs"]
mod tablet;
#[path = "dictionary_gates/volume.rs"]
mod volume;
