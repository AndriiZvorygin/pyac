//! Render a decoded dictionary volume back to `.en.pya` source text.
//!
//! This is the inverse of `source.rs::parse_source`. Together they close the loop
//! `source -> volume -> source` in Rust, with no JSON in between:
//!
//! ```text
//! reference.pyac --decode--> model --render--> reference.en.pya --compile--> reference.pyac
//! ```
//!
//! The gate is byte identity: `dictionary_render.rs` renders the published volume,
//! recompiles it, and requires the same bytes back. An approximate renderer would
//! lose entries a round trip silently, which is the defect the byte gate exists to
//! catch.
//!
//! # Keys
//!
//! A key is a sequence of HNUCs and the source writes it as its spelling, so each
//! HNUC goes back through the word codec. A key that does not spell is a defect in
//! the volume, not something to paper over, so it is an error rather than a hex
//! fallback.
//!
//! # Text
//!
//! Every text value is a JSON string literal — the same `text "..."` form
//! `parse_value` reads with `serde_json`. That is what makes a surface such as
//! `प\u{94d}राप\u{94d}य` or `"quoted"` survive: `{:?}` would emit Rust escapes and
//! non-ASCII, which the parser rejects.

use std::fmt::Write;

use super::operation::OperationSignature;
use super::volume::{DictionaryVolume, DictionaryVolumeEntry};

/// Why a volume could not be rendered.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum RenderError {
    /// An HNUC in a key has no spelling.
    UnspellableKey { hnuc: u16 },
    /// A key is empty, so it has no spelling at all.
    EmptyKey,
}

impl std::fmt::Display for RenderError {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::UnspellableKey { hnuc } => {
                write!(formatter, "key word 0x{hnuc:04x} has no glyph spelling")
            }
            Self::EmptyKey => formatter.write_str("entry key is empty"),
        }
    }
}

impl std::error::Error for RenderError {}

/// The profile marker kind the source opens with.
const SOURCE_MARKER: &str = "pyac-vliw-dictionary/3";

/// Render a whole volume to `.en.pya` source text.
pub fn render_source(volume: &DictionaryVolume) -> Result<String, RenderError> {
    let mut out = String::new();
    let _ = writeln!(
        out,
        "su name lyin kwon ob text {} be text ya",
        quote(&volume.metadata.kind)
    );
    let _ = writeln!(
        out,
        "su name lyin kwon ob text {} as num {} from text {} to text {} with text {} be text ya",
        quote(&volume.metadata.title),
        volume.metadata.revision,
        quote(&volume.metadata.languages.join(",")),
        quote(&volume.metadata.normalization_policy),
        quote(SOURCE_MARKER)
    );
    for entry in &volume.entries {
        render_entry(entry, &mut out)?;
    }
    out.push_str("prah\n");
    Ok(out)
}

fn render_entry(entry: &DictionaryVolumeEntry, out: &mut String) -> Result<(), RenderError> {
    let key = quote(&spell_key(&entry.key)?);
    let _ = writeln!(
        out,
        "su name tlat kwon ob text {key} at text \"entry\" be text ya"
    );
    for form in &entry.lexical_forms {
        let _ = writeln!(
            out,
            "su name tlat kwon ob text {key} as text {} from text {} to text {} at text \"lexical-form\" be text ya",
            quote(&form.language),
            quote(&form.surface),
            quote(&form.normalized)
        );
    }
    for link in &entry.source_links {
        let _ = writeln!(
            out,
            "su name tlat kwon ob text {key} as text {} from text {} to text {} by text {} at text \"source-link\" be text ya",
            quote(&link.dataset),
            quote(&link.release),
            quote(&link.identifier),
            quote(&link.relation)
        );
    }
    for gloss in &entry.source_glosses {
        let _ = writeln!(
            out,
            "su name tlat kwon ob text {key} as text {} from text {} to text {} by text {} with text {} at text \"source-gloss\" be text ya",
            quote(&gloss.language),
            quote(&gloss.dataset),
            quote(&gloss.release),
            quote(&gloss.identifier),
            quote(&gloss.text)
        );
    }
    for component in &entry.component_keys {
        let _ = writeln!(
            out,
            "su name tlat kwon ob text {key} from text {} at text \"component\" be text ya",
            quote(&spell_key(component)?)
        );
    }
    for task in &entry.task_keys {
        let _ = writeln!(
            out,
            "su name tlat kwon ob text {key} with text {} at text \"task\" be text ya",
            quote(&spell_key(task)?)
        );
    }
    for reference in &entry.references {
        let _ = writeln!(
            out,
            "su name tlat kwon ob text {key} to text {} at text \"reference\" be text ya",
            quote(&spell_key(reference)?)
        );
    }
    for signature in &entry.operation_signatures {
        let _ = writeln!(
            out,
            "su name tlat kwon ob text {key} from ve {} {} at text \"operation-signature\" be text ya",
            signature_words(signature).len(),
            word_list(&signature_words(signature))
        );
        for argument in &signature.arguments {
            let slot = slot_words(argument);
            let _ = writeln!(
                out,
                "su name tlat kwon ob text {key} as ve {} {} from text {} to text {} at text \"operation-argument\" be text ya",
                slot.len(),
                word_list(&slot),
                quote(&argument.tag_label),
                quote(&argument.descriptive_specification)
            );
        }
        for result in &signature.results {
            let _ = writeln!(
                out,
                "su name tlat kwon ob text {key} as ve {} {} from text {} to text {} at text \"operation-result\" be text ya",
                result.noun_class.len(),
                word_list(&result.noun_class),
                quote(&result.tag_label),
                quote(&result.descriptive_specification)
            );
        }
    }
    if let Some(definition) = &entry.definition {
        let words: Vec<u16> = definition.iter().flatten().copied().collect();
        let _ = writeln!(
            out,
            "su name tlat kwon ob text {key} from ve {} {} at text \"definition\" be text ya",
            words.len(),
            word_list(&words)
        );
    }
    Ok(())
}

/// The dispatch key words of a signature, in the order `parse_signature` reads.
fn signature_words(signature: &OperationSignature) -> Vec<u16> {
    let mut words = Vec::new();
    words.push(signature.verb.len() as u16);
    words.extend(signature.verb.iter().copied());
    words.push(signature.mood.len() as u16);
    words.extend(signature.mood.iter().copied());
    words.push(signature.arguments.len() as u16);
    for argument in &signature.arguments {
        words.push(argument.case_type.grammatical_case.len() as u16);
        words.extend(argument.case_type.grammatical_case.iter().copied());
        words.push(argument.case_type.noun_class.len() as u16);
        words.extend(argument.case_type.noun_class.iter().copied());
    }
    words
}

/// One argument slot as the `as ve` descriptor `parse_slot` reads.
fn slot_words(argument: &super::operation::OperationParameter) -> Vec<u16> {
    let mut words = Vec::new();
    words.push(argument.case_type.grammatical_case.len() as u16);
    words.extend(argument.case_type.grammatical_case.iter().copied());
    words.push(argument.case_type.noun_class.len() as u16);
    words.extend(argument.case_type.noun_class.iter().copied());
    words
}

fn word_list(words: &[u16]) -> String {
    words
        .iter()
        .map(u16::to_string)
        .collect::<Vec<_>>()
        .join(" ")
}

/// Spell an HNUC key back to the source's space-joined glyph form.
fn spell_key(key: &[u16]) -> Result<String, RenderError> {
    if key.is_empty() {
        return Err(RenderError::EmptyKey);
    }
    let mut out = String::new();
    for hnuc in key {
        let mut buffer = [0u8; 8];
        let length = pyac_vliw_core::word_code::decode(*hnuc, &mut buffer)
            .map_err(|_| RenderError::UnspellableKey { hnuc: *hnuc })?;
        if !out.is_empty() {
            out.push(' ');
        }
        out.push_str(&String::from_utf8_lossy(&buffer[..length]));
    }
    Ok(out)
}

fn quote(value: &str) -> String {
    serde_json::to_string(value).expect("a string always serializes")
}
