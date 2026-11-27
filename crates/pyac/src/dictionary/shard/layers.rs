//! Glyph layers: how deep a compound sits, and which shard that puts it in.

use super::blocks::EntryBlock;
use super::names::BEYOND_LAST_LAYER;
use std::collections::HashMap;

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
pub(super) fn shard_name_for(block: &EntryBlock) -> (String, u16) {
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
pub(super) fn verb_stem(verb: &str) -> String {
    verb.split_whitespace().collect::<Vec<_>>().join("_")
}
