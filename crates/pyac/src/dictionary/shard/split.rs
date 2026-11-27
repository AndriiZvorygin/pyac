//! The splitter itself: a document in, shards out.

use super::blocks::{EntryBlock, entry_blocks, json};
use super::errors::ShardError;
use super::layers::{LayerIndex, shard_name_for, verb_stem};
use super::manifest::{ShardSet, shard_digest};
use super::names::BEYOND_LAST_LAYER;
use std::collections::BTreeMap;

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
pub(super) fn split_blocks(blocks: Vec<EntryBlock>, source: &str) -> Result<ShardSet, ShardError> {
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
