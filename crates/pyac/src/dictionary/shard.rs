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
//! `.en.pya`; assembly accepts either extension on read, so a directory written
//! before the convention still assembles.

use std::collections::BTreeMap;
use std::fmt::{Display, Formatter};

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

/// One content shard as the manifest records it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ShardRecord {
    pub name: String,
    pub digest: String,
    pub entries: usize,
    pub max_layer: u16,
}

/// A split source: the manifest text plus each content shard's text.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ShardSet {
    pub manifest: String,
    pub shards: Vec<(String, String)>,
}

/// sha256 of the shard bytes, hex encoded, as the manifest records it.
pub fn shard_digest(text: &str) -> String {
    use sha2::{Digest, Sha256};
    let mut hasher = Sha256::new();
    hasher.update(text.as_bytes());
    hasher
        .finalize()
        .iter()
        .map(|byte| format!("{byte:02x}"))
        .collect()
}

/// Assemble a manifest and its content shards back into one source document.
///
/// Verifies every digest and entry count, and that no compound precedes a
/// component. Any mismatch is an error rather than a best-effort assembly, because
/// a shard set that has drifted must not compile into a plausible-looking volume.
pub fn assemble_shards(
    manifest: &str,
    contents: &BTreeMap<String, String>,
) -> Result<String, ShardError> {
    let records = parse_manifest(manifest)?;
    if records.is_empty() {
        return Err(ShardError::MissingManifest);
    }

    // Layers must ascend, so a compound's dependencies are already written. The
    // dispatch shard carries no vocabulary and declares layer 0 by convention, so
    // it is exempt from the ascent — it annotates entries that already exist.
    let mut previous_layer = 0u16;
    for record in &records {
        if record.name.starts_with("dispatch/") {
            continue;
        }
        if record.max_layer < previous_layer {
            return Err(ShardError::LayerOrder {
                previous: previous_layer,
                next: record.max_layer,
            });
        }
        previous_layer = record.max_layer;
    }

    let header = manifest
        .lines()
        .filter(|line| line.trim().starts_with("su name lyin kwon "))
        .collect::<Vec<_>>()
        .join("\n");

    let mut out = String::new();
    if !header.is_empty() {
        out.push_str(&header);
        out.push('\n');
    }

    let mut seen: BTreeMap<String, ()> = BTreeMap::new();
    for record in &records {
        let Some(body) = contents.get(&record.name) else {
            return Err(ShardError::MissingShard {
                name: record.name.clone(),
            });
        };
        let digest = shard_digest(body);
        if digest != record.digest {
            return Err(ShardError::DigestMismatch {
                name: record.name.clone(),
                expected: record.digest.clone(),
                actual: digest,
            });
        }
        // A vocabulary shard declares its entry count; the dispatch shard
        // declares how many signatures it carries, because its records annotate
        // entries declared elsewhere.
        let actual = if record.name.starts_with("dispatch/") {
            body.matches(" at text \"operation-signature\" be text ya")
                .count()
        } else {
            entry_blocks(body).len()
        };
        if actual != record.entries {
            return Err(ShardError::CountMismatch {
                name: record.name.clone(),
                expected: record.entries,
                actual,
            });
        }
        if record.name.starts_with("dispatch/") {
            // Dispatch names entries that must already exist, but nothing depends
            // on dispatch, so it needs no ordering beyond coming last.
            out.push_str(body);
            continue;
        }
        let blocks = entry_blocks(body);
        // Every component must already be present. This is the same rule the
        // volume writer enforces; catching it here names the shard at fault.
        for block in &blocks {
            for reference in block.references() {
                if !seen.contains_key(&reference) {
                    return Err(ShardError::DependencyOrder {
                        key: block.key.clone(),
                        missing: reference,
                    });
                }
            }
        }
        for block in &blocks {
            seen.insert(block.key.clone(), ());
        }
        out.push_str(body);
    }
    out.push_str("prah\n");
    Ok(out)
}

/// Read a manifest's shard rows.
pub fn parse_manifest(manifest: &str) -> Result<Vec<ShardRecord>, ShardError> {
    let mut records = Vec::new();
    let mut saw_header = false;
    for line in manifest.lines() {
        let line = line.trim();
        if line.is_empty() || line == "prah" {
            continue;
        }
        if line.starts_with("su name ") && line.ends_with(" be map def") {
            saw_header = true;
            continue;
        }
        if !line.starts_with("su text ") {
            continue;
        }
        let Some(rest) = line.strip_prefix("su text ") else {
            continue;
        };
        let Some((name_token, tail)) = rest.split_once(" ob text ") else {
            continue;
        };
        let Some(payload) = tail.strip_suffix(" ya") else {
            continue;
        };
        let name: String =
            serde_json::from_str(name_token).map_err(|_| ShardError::MalformedRow {
                name: name_token.to_owned(),
            })?;
        if name == "format" {
            continue;
        }
        let raw: String = serde_json::from_str(payload)
            .map_err(|_| ShardError::MalformedRow { name: name.clone() })?;
        let row: serde_json::Value = serde_json::from_str(&raw)
            .map_err(|_| ShardError::MalformedRow { name: name.clone() })?;
        records.push(ShardRecord {
            name: row
                .get("name")
                .and_then(|value| value.as_str())
                .unwrap_or_default()
                .to_owned(),
            digest: row
                .get("sha256")
                .and_then(|value| value.as_str())
                .unwrap_or_default()
                .to_owned(),
            entries: row
                .get("entries")
                .and_then(serde_json::Value::as_u64)
                .unwrap_or_default() as usize,
            max_layer: row
                .get("maxLayer")
                .and_then(serde_json::Value::as_u64)
                .unwrap_or_default() as u16,
        });
    }
    if !saw_header {
        return Err(ShardError::MissingManifest);
    }
    Ok(records)
}

/// A glyph layer word list, so an entry's layer can be computed.
///
/// Layers are cumulative, so membership is "any layer at or above this one lists
/// the word". The caller supplies them because they live beside the vocabulary
/// source in the `pyash` checkout rather than here.
#[derive(Debug, Clone, Default)]
pub struct LayerIndex {
    word_layer: HashMap<String, u16>,
}

impl LayerIndex {
    /// Build from `(layer, whitespace-separated words)` pairs.
    pub fn from_lists<I, S>(lists: I) -> Self
    where
        I: IntoIterator<Item = (u16, S)>,
        S: AsRef<str>,
    {
        let mut word_layer = HashMap::new();
        let mut sorted: Vec<(u16, String)> = lists
            .into_iter()
            .map(|(layer, text)| (layer, text.as_ref().to_owned()))
            .collect();
        sorted.sort_by_key(|(layer, _)| *layer);
        for (layer, text) in sorted {
            for word in text.split_whitespace() {
                word_layer.entry(word.to_owned()).or_insert(layer);
            }
        }
        Self { word_layer }
    }

    /// The lowest layer that can spell `word`, or [`BEYOND_LAST_LAYER`].
    pub fn layer_of(&self, word: &str) -> u16 {
        self.word_layer
            .get(word)
            .copied()
            .unwrap_or(BEYOND_LAST_LAYER)
    }
}

fn shard_name_for(block: &EntryBlock) -> (String, u16) {
    let is_atom = !block.key.contains(' ') && block.components.is_empty();
    if is_atom {
        return ("atoms".to_owned(), 0);
    }
    // An entry's layer is the highest layer among its key words and its
    // components: that is the point at which everything it needs exists.
    let layer = block
        .key_layers
        .iter()
        .chain(block.component_layers.iter())
        .copied()
        .max()
        .unwrap_or(BEYOND_LAST_LAYER);
    (format!("compounds-{layer}"), layer)
}

/// A filesystem stem for a dispatch file, named for its verb.
///
/// Every signature in the vocabulary uses its entry's own spelling as its verb —
/// measured, 96 entries and 96 distinct verbs with a 1:1 mapping — so the verb
/// stem and the key stem are the same string. Naming the file for the verb says
/// what the file is: the dispatch an operation named `<verb>` accepts.
///
/// Pyac spellings are lowercase letters, digits and spaces, so joining on `_` is
/// lossless enough to read and needs no escaping. The manifest records the true
/// key inside each row, so the stem is a convenience, not the identity.
fn verb_stem(verb: &str) -> String {
    verb.split_whitespace().collect::<Vec<_>>().join("_")
}

/// One entry block: the `entry` sentence plus every record that follows it.
struct EntryBlock {
    key: String,
    text: String,
    components: Vec<String>,
    key_layers: Vec<u16>,
    component_layers: Vec<u16>,
    /// The block's dispatch records, lifted out when the split separates them.
    ///
    /// A ceremony signature says what an operation accepts; it changes when the
    /// API changes, not when the vocabulary does. Keeping it beside the entry
    /// means every vocabulary edit reflows the dispatch text too, and every
    /// signature change reflows a vocabulary shard.
    dispatch: String,
}

impl EntryBlock {
    /// The half of this block that is vocabulary: the entry sentence plus its
    /// forms, components and citations, with dispatch records removed.
    fn vocabulary_text(&self) -> String {
        let mut out = String::new();
        for line in self.text.lines() {
            if is_dispatch_record(line) {
                continue;
            }
            out.push_str(line);
            out.push('\n');
        }
        out
    }
}

/// Whether a record line belongs to the ceremony dispatch API rather than the
/// vocabulary.
fn is_dispatch_record(line: &str) -> bool {
    line.contains(" at text \"operation-signature\" be text ya")
        || line.contains(" at text \"operation-argument\" be text ya")
        || line.contains(" at text \"operation-result\" be text ya")
}

impl EntryBlock {
    /// Every component key this block names, in source order.
    fn references(&self) -> Vec<String> {
        self.components.clone()
    }
}

/// Split a source document into entry blocks, preserving each block's raw text.
///
/// A block runs from its `at text "entry"` sentence to the line before the next
/// one. Metadata sentences before the first entry are not blocks.
fn entry_blocks(source: &str) -> Vec<EntryBlock> {
    let mut blocks: Vec<EntryBlock> = Vec::new();
    for line in source.lines() {
        let line = format!("{line}\n");
        if let Some(key) = entry_key(line.trim()) {
            blocks.push(EntryBlock {
                key,
                text: line,
                components: Vec::new(),
                key_layers: Vec::new(),
                component_layers: Vec::new(),
                dispatch: String::new(),
            });
            continue;
        }
        let Some(current) = blocks.last_mut() else {
            continue;
        };
        if is_dispatch_record(line.trim()) {
            current.dispatch.push_str(&line);
        }
        current.text.push_str(&line);
        if let Some(component) = component_reference(line.trim()) {
            current.components.push(component);
        }
    }
    blocks
}

/// The key of an `at text "entry"` sentence.
fn entry_key(line: &str) -> Option<String> {
    if !line.ends_with(" be text ya") || !line.contains(" at text \"entry\" ") {
        return None;
    }
    field(line, "su name tlat kwon ob text \"")
}

/// The component named by an `at text "component"` sentence.
fn component_reference(line: &str) -> Option<String> {
    if !line.contains(" at text \"component\" be text ya") {
        return None;
    }
    field(line, " from text \"")
}

fn field(line: &str, prefix: &str) -> Option<String> {
    let start = line.find(prefix)? + prefix.len();
    let rest = &line[start..];
    let end = rest.find('"')?;
    Some(rest[..end].to_owned())
}

fn json(value: &str) -> String {
    serde_json::to_string(value).unwrap_or_else(|_| "\"\"".to_owned())
}

use std::collections::HashMap;

/// Split one source document into a manifest plus content shards, with no glyph
/// layers available.
///
/// Every word resolves to [`BEYOND_LAST_LAYER`], so atoms form their own shard and
/// every compound lands in one group. Correct, but coarse: use
/// [`split_source_with_layers`] when the glyph lists are at hand, which is what
/// produces the intended per-layer split.
pub fn split_source(source: &str) -> Result<ShardSet, ShardError> {
    split_source_with_layers(source, &LayerIndex::default())
}

pub fn split_source_with_layers(source: &str, layers: &LayerIndex) -> Result<ShardSet, ShardError> {
    let blocks = entry_blocks(source);
    if blocks.is_empty() {
        return Err(ShardError::EmptySource);
    }
    let annotated: Vec<EntryBlock> = blocks
        .into_iter()
        .map(|mut block| {
            block.key_layers = block
                .key
                .split_whitespace()
                .map(|w| layers.layer_of(w))
                .collect();
            block.component_layers = block
                .components
                .iter()
                .map(|component| {
                    component
                        .split_whitespace()
                        .map(|w| layers.layer_of(w))
                        .max()
                        .unwrap_or(BEYOND_LAST_LAYER)
                })
                .collect();
            block
        })
        .collect();
    split_blocks(annotated, source)
}

fn split_blocks(blocks: Vec<EntryBlock>, source: &str) -> Result<ShardSet, ShardError> {
    let header = source
        .lines()
        .filter(|line| line.trim().starts_with("su name lyin kwon "))
        .collect::<Vec<_>>()
        .join("\n");

    let mut groups: BTreeMap<String, (Vec<EntryBlock>, u16)> = BTreeMap::new();
    // Dispatch is lifted out before grouping: it belongs to the API that changes
    // on its own schedule, not to the vocabulary. Kept per entry so an API change
    // touches one small file.
    let mut dispatch: BTreeMap<String, String> = BTreeMap::new();
    for block in blocks {
        if !block.dispatch.is_empty() {
            dispatch
                .entry(block.key.clone())
                .or_default()
                .push_str(&block.dispatch);
        }
        let (group, layer) = shard_name_for(&block);
        let slot = groups.entry(group).or_insert((Vec::new(), layer));
        slot.1 = slot.1.max(layer);
        slot.0.push(block);
    }

    let mut ordered: Vec<(String, Vec<EntryBlock>, u16)> = groups
        .into_iter()
        .map(|(name, (blocks, layer))| (name, blocks, layer))
        .collect();
    ordered.sort_by(|left, right| {
        let sort_key = |name: &str, layer: u16| {
            if name == "atoms" {
                (0u16, 0u16)
            } else {
                (1u16, layer)
            }
        };
        sort_key(&left.0, left.2)
            .cmp(&sort_key(&right.0, right.2))
            .then(left.0.cmp(&right.0))
    });

    let mut manifest_lines = vec![
        "su name pyac_dictionary_shards be map def".to_owned(),
        "su text \"format\" ob text \"pyac-dictionary-shards-v1\" ya".to_owned(),
    ];
    if !header.is_empty() {
        manifest_lines.push(header);
    }
    let mut shards = Vec::new();
    for (name, blocks, layer) in &ordered {
        // Vocabulary shards carry the entry and its records, minus dispatch.
        let body = blocks
            .iter()
            .map(|block| block.vocabulary_text())
            .collect::<String>();
        let digest = shard_digest(&body);
        let row = format!(
            "{{\"name\":{},\"sha256\":{},\"entries\":{},\"maxLayer\":{}}}",
            json(name),
            json(&digest),
            blocks.len(),
            layer
        );
        manifest_lines.push(format!("su text {} ob text {} ya", json(name), json(&row)));
        shards.push((name.clone(), body));
    }
    // Dispatch last, one file per verb. No vocabulary entry depends on a
    // signature, and each operation's signatures belong together. Grouped this
    // way the median file is ~3 KB instead of one 1.16 MB blob, and an API change
    // touches only the operations it actually changes.
    //
    // The map is keyed by entry, which is also the verb: a signature's verb is its
    // entry's spelling, so one operation has one file.
    for (key, body) in &dispatch {
        let number = body
            .matches(" at text \"operation-signature\" be text ya")
            .count();
        let digest = shard_digest(body);
        let name = format!("dispatch/{}", verb_stem(key));
        let row = format!(
            "{{\"name\":{},\"sha256\":{},\"entries\":{},\"maxLayer\":{}}}",
            json(&name),
            json(&digest),
            number,
            0
        );
        manifest_lines.push(format!("su text {} ob text {} ya", json(&name), json(&row)));
        shards.push((name, body.clone()));
    }
    manifest_lines.push("prah".to_owned());
    Ok(ShardSet {
        manifest: manifest_lines.join("\n") + "\n",
        shards,
    })
}
