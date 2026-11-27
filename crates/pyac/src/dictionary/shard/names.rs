//! File names and the layer vocabulary the splitter assigns shards by.

/// The English-form Pyash source extension.
pub const EN_SOURCE_EXT: &str = "en.pya";
/// The glyph Pyac source extension, which the English form is never written as.
pub const PYAC_SOURCE_EXT: &str = "pya";
/// The name of the manifest file within a shard directory.
pub const MANIFEST_FILE: &str = "00-manifest.en.pya";
/// The flat file a single-shard or whole-dictionary source is named.
///
/// The shipped vocabulary is a shard directory, so this names the assembled
/// whole rather than a committed file.
pub const REFERENCE_SOURCE_FILE: &str = "reference.en.pya";
/// The file name for one content shard.
pub fn shard_file_name(name: &str) -> String {
    format!("{name}.{EN_SOURCE_EXT}")
}
/// Paths to try for a shard named `name`, in read order.
///
/// `.en.pya` first because that is what the splitter emits, then the bare `.pya`
/// the directory may still carry from before the extension convention.
pub fn shard_candidates(name: &str) -> [String; 2] {
    [shard_file_name(name), format!("{name}.{PYAC_SOURCE_EXT}")]
}
/// The glyph layers the vocabulary is organised by, in ascending order.
///
/// From `pyac_complexity_layers.pya`: each layer adds glyphs to the previous, so
/// the lists nest exactly. A word belongs to the lowest layer that spells it.
pub const LAYERS: [u16; 5] = [16, 19, 24, 28, 32];
/// The layer assigned to a word that no glyph layer can spell. It sorts last so
/// such a compound still lands after everything it could depend on.
pub const BEYOND_LAST_LAYER: u16 = 33;
