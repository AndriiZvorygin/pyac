//! Entry blocks: the lines belonging to one entry, and the fields read from them.

/// One entry block: the `entry` sentence plus every record that follows it.
pub(super) struct EntryBlock {
    pub(super) key: String,
    pub(super) text: String,
    pub(super) components: Vec<String>,
    pub(super) key_layers: Vec<u16>,
    pub(super) component_layers: Vec<u16>,
    /// The block's dispatch records, lifted out when the split separates them.
    ///
    /// A ceremony signature says what an operation accepts; it changes when the
    /// API changes, not when the vocabulary does. Keeping it beside the entry
    /// means every vocabulary edit reflows the dispatch text too, and every
    /// signature change reflows a vocabulary shard.
    pub(super) dispatch: String,
}
impl EntryBlock {
    /// The half of this block that is vocabulary: the entry sentence plus its
    /// forms, components and citations, with dispatch records removed.
    pub(super) fn vocabulary_text(&self) -> String {
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
pub(super) fn is_dispatch_record(line: &str) -> bool {
    line.contains(" at text \"operation-signature\" be text ya")
        || line.contains(" at text \"operation-argument\" be text ya")
        || line.contains(" at text \"operation-result\" be text ya")
}
impl EntryBlock {
    /// Every component key this block names, in source order.
    pub(super) fn references(&self) -> Vec<String> {
        self.components.clone()
    }
}
/// Split a source document into entry blocks, preserving each block's raw text.
///
/// A block runs from its `at text "entry"` sentence to the line before the next
/// one. Metadata sentences before the first entry are not blocks.
pub(super) fn entry_blocks(source: &str) -> Vec<EntryBlock> {
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
pub(super) fn entry_key(line: &str) -> Option<String> {
    if !line.ends_with(" be text ya") || !line.contains(" at text \"entry\" ") {
        return None;
    }
    field(line, "su name tlat kwon ob text \"")
}
/// The component named by an `at text "component"` sentence.
pub(super) fn component_reference(line: &str) -> Option<String> {
    if !line.contains(" at text \"component\" be text ya") {
        return None;
    }
    field(line, " from text \"")
}
pub(super) fn field(line: &str, prefix: &str) -> Option<String> {
    let start = line.find(prefix)? + prefix.len();
    let rest = &line[start..];
    let end = rest.find('"')?;
    Some(rest[..end].to_owned())
}
pub(super) fn json(value: &str) -> String {
    serde_json::to_string(value).unwrap_or_else(|_| "\"\"".to_owned())
}
