//! The artifacts every dictionary gate reads, derived once per test binary.
//!
//! A gate binary is a process. Before this, nine of them each assembled the 96 MB
//! authored source, or decoded the 131 MB published volume, or encoded it again —
//! and cargo runs test binaries one at a time, so that work added up rather than
//! overlapping. `dictionary_selfhost` alone spent 12 s on a source three of its
//! tests each rebuilt.
//!
//! Everything here is a pure function of two files that do not change during a
//! run, so it is computed on first use and held. A test binary may be filtered to
//! one gate; untouched fixtures are never built.
//!
//! Presence is not assumed. The published volume may be absent on a clone, and the
//! authored source may be absent in a packaging check, so each accessor returns
//! `Option` and the gates skip — see `common::reference_volume_or_skip`.

#![allow(dead_code)]

use std::collections::BTreeMap;
use std::path::{Path, PathBuf};
use std::sync::OnceLock;

use pyac::dictionary::{
    DictionarySource, DictionaryVolume, LayerIndex, assemble_shards, encode_dictionary,
    parse_manifest, parse_source, read_volume, split_source_with_layers,
};

/// The crate that owns this test.
///
/// `CARGO_MANIFEST_DIR` is `crates/pyac`; the workspace root is two levels up, and
/// both artifacts live there rather than beside the crate.
pub fn workspace_root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .ancestors()
        .nth(2)
        .map(Path::to_path_buf)
        .expect("the crate lives two levels below the workspace root")
}

/// The assembled authored source, read once.
///
/// `PYAC_REFERENCE_SOURCE` overrides with a single file, which is what a caller
/// uses when it has rendered the volume and wants the gates to run against that
/// render instead of the committed shards.
pub fn source() -> Option<&'static str> {
    static SOURCE: OnceLock<Option<String>> = OnceLock::new();
    SOURCE.get_or_init(read_source).as_deref()
}

fn read_source() -> Option<String> {
    if let Some(explicit) = std::env::var_os("PYAC_REFERENCE_SOURCE") {
        return std::fs::read_to_string(PathBuf::from(explicit)).ok();
    }
    let root = workspace_root();
    // The whole file is generated and git-ignored; the shard directory is what the
    // tree ships, so it is the fallback rather than the other way round.
    if let Ok(text) = std::fs::read_to_string(root.join(pyac::dictionary::REFERENCE_SOURCE_FILE)) {
        return Some(text);
    }
    assemble_committed_shards().ok()
}

/// The committed shards, assembled through the manifest.
fn assemble_committed_shards() -> Result<String, String> {
    let directory = workspace_root().join("dictionary/shard");
    let manifest = std::fs::read_to_string(directory.join(pyac::dictionary::MANIFEST_FILE))
        .map_err(|error| error.to_string())?;
    let records = parse_manifest(&manifest).map_err(|error| error.code().to_owned())?;
    let mut contents = BTreeMap::new();
    for record in records {
        let body = pyac::dictionary::shard_candidates(&record.name)
            .into_iter()
            .find_map(|file| std::fs::read_to_string(directory.join(file)).ok())
            .ok_or_else(|| format!("shard {} is named by the manifest but absent", record.name))?;
        contents.insert(record.name, body);
    }
    assemble_shards(&manifest, &contents).map_err(|error| error.code().to_owned())
}

/// The authored source, parsed once.
pub fn parsed() -> Option<&'static DictionarySource> {
    static PARSED: OnceLock<Option<DictionarySource>> = OnceLock::new();
    PARSED
        .get_or_init(|| Some(parse_source(source()?).ok()?))
        .as_ref()
}

/// The authored source compiled, as the `compile` subcommand would.
///
/// This is source -> volume -> bytes with the library owning every step, so the
/// gates compare against the real `compile` rather than a re-implementation.
pub fn compiled_source() -> Option<&'static [u8]> {
    static COMPILED: OnceLock<Option<Vec<u8>>> = OnceLock::new();
    COMPILED
        .get_or_init(|| {
            let volume = pyac::dictionary::source_to_volume(parsed()?.clone()).ok()?;
            encode_dictionary(&volume).ok()
        })
        .as_deref()
}

/// The published volume, decoded once.
pub fn volume() -> Option<&'static DictionaryVolume> {
    static VOLUME: OnceLock<Option<DictionaryVolume>> = OnceLock::new();
    VOLUME
        .get_or_init(|| Some(read_volume(std::fs::File::open(reference_volume()?).ok()?).ok()?))
        .as_ref()
}

/// The published volume's bytes, read once.
pub fn published_bytes() -> Option<&'static [u8]> {
    static BYTES: OnceLock<Option<Vec<u8>>> = OnceLock::new();
    BYTES
        .get_or_init(|| std::fs::read(reference_volume()?).ok())
        .as_deref()
}

/// Where the published volume is, validated.
pub fn reference_volume() -> Option<&'static Path> {
    static PATH: OnceLock<Option<PathBuf>> = OnceLock::new();
    PATH.get_or_init(super::common::reference_volume).as_deref()
}

/// The authored source rendered back to `.en.pya`, produced once.
pub fn rendered_source() -> Option<&'static str> {
    static RENDERED: OnceLock<Option<String>> = OnceLock::new();
    RENDERED
        .get_or_init(|| Some(pyac::dictionary::decode_source(volume()?).ok()?))
        .as_deref()
}

/// The authored source split into shards with the configured layer lists.
pub fn split() -> Option<&'static pyac::dictionary::ShardSet> {
    static SPLIT: OnceLock<Option<pyac::dictionary::ShardSet>> = OnceLock::new();
    SPLIT
        .get_or_init(|| Some(split_source_with_layers(source()?, layer_index()).ok()?))
        .as_ref()
}

/// The glyph layer lists, when the caller points at them.
///
/// They live beside the vocabulary in the `pyash` checkout, not here, so the path
/// is configured rather than assumed; without them every compound lands in one
/// shard and the layer assertions have nothing to check.
pub fn layer_index() -> &'static LayerIndex {
    static INDEX: OnceLock<LayerIndex> = OnceLock::new();
    INDEX.get_or_init(|| {
        let Ok(directory) = std::env::var("PYAC_LAYER_LISTS") else {
            return LayerIndex::default();
        };
        let mut lists = Vec::new();
        for layer in pyac::dictionary::LAYERS {
            let path = PathBuf::from(&directory).join(format!("{layer}GlyphWordList.txt"));
            if let Ok(text) = std::fs::read_to_string(path) {
                lists.push((layer, text));
            }
        }
        LayerIndex::from_lists(lists)
    })
}

/// Report the first differing byte, which is what makes a byte-identity failure
/// diagnosable rather than just "not equal".
pub fn assert_identical(actual: &[u8], expected: &[u8], what: &str) {
    if actual == expected {
        return;
    }
    let first = actual
        .iter()
        .zip(expected.iter())
        .position(|(left, right)| left != right);
    panic!(
        "{what}: expected {} bytes, got {}, first difference at {first:?}",
        expected.len(),
        actual.len()
    );
}
