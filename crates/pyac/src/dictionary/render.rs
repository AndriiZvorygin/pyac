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

/// How a text value is written.
#[derive(Clone, Copy, PartialEq, Eq)]
enum TextStyle {
    /// English form: a JSON string after the literal word `text`.
    English,
    /// Glyph form: a `zi.kyit. … .kyit.zi` frame, no `text` keyword.
    Glyph,
}

/// Render a whole volume to `.pya` source text.
///
/// The glyph form spells keys as glyphs and carries every text value in a
/// `zi.kyit. … .kyit.zi` frame, so the source is the language and not a JSON
/// rendering of it. Numbers stay as `num N`; that is a typed decimal literal, not
/// a `zi` frame, which is reserved for strings and binary.
pub fn render_glyph_source(volume: &DictionaryVolume) -> Result<String, RenderError> {
    render(volume, TextStyle::Glyph)
}

/// Render a whole volume to `.en.pya` source text.
pub fn render_source(volume: &DictionaryVolume) -> Result<String, RenderError> {
    render(volume, TextStyle::English)
}

fn render(volume: &DictionaryVolume, style: TextStyle) -> Result<String, RenderError> {
    let mut out = String::new();
    let _ = writeln!(
        out,
        "su name lyin kwon ob {} be text ya",
        text(&volume.metadata.kind, style)
    );
    let _ = writeln!(
        out,
        "su name lyin kwon ob {} as num {} from {} to {} with {} be text ya",
        text(&volume.metadata.title, style),
        volume.metadata.revision,
        text(&volume.metadata.languages.join(","), style),
        text(&volume.metadata.normalization_policy, style),
        text(SOURCE_MARKER, style)
    );
    for entry in &volume.entries {
        render_entry(entry, style, &mut out)?;
    }
    out.push_str("prah\n");
    Ok(out)
}

fn render_entry(
    entry: &DictionaryVolumeEntry,
    style: TextStyle,
    out: &mut String,
) -> Result<(), RenderError> {
    let key = text(&spell_key(&entry.key)?, style);
    let _ = writeln!(
        out,
        "su name tlat kwon ob {key} at {} be text ya",
        text("entry", style)
    );
    for form in &entry.lexical_forms {
        let _ = writeln!(
            out,
            "su name tlat kwon ob {key} as {} from {} to {} at {} be text ya",
            text(&form.language, style),
            text(&form.surface, style),
            text(&form.normalized, style),
            text("lexical-form", style)
        );
    }
    for link in &entry.source_links {
        let _ = writeln!(
            out,
            "su name tlat kwon ob {key} as {} from {} to {} by {} at {} be text ya",
            text(&link.dataset, style),
            text(&link.release, style),
            text(&link.identifier, style),
            text(&link.relation, style),
            text("source-link", style)
        );
    }
    for gloss in &entry.source_glosses {
        let _ = writeln!(
            out,
            "su name tlat kwon ob {key} as {} from {} to {} by {} with {} at {} be text ya",
            text(&gloss.language, style),
            text(&gloss.dataset, style),
            text(&gloss.release, style),
            text(&gloss.identifier, style),
            text(&gloss.text, style),
            text("source-gloss", style)
        );
    }
    for component in &entry.component_keys {
        let _ = writeln!(
            out,
            "su name tlat kwon ob {key} from {} at {} be text ya",
            text(&spell_key(component)?, style),
            text("component", style)
        );
    }
    for task in &entry.task_keys {
        let _ = writeln!(
            out,
            "su name tlat kwon ob {key} with {} at {} be text ya",
            text(&spell_key(task)?, style),
            text("task", style)
        );
    }
    for reference in &entry.references {
        let _ = writeln!(
            out,
            "su name tlat kwon ob {key} to {} at {} be text ya",
            text(&spell_key(reference)?, style),
            text("reference", style)
        );
    }
    for signature in &entry.operation_signatures {
        let _ = writeln!(
            out,
            "su name tlat kwon ob {key} from ve {} {} at {} be text ya",
            signature_words(signature).len(),
            word_list(&signature_words(signature)),
            text("operation-signature", style)
        );
        for argument in &signature.arguments {
            let slot = slot_words(argument);
            let _ = writeln!(
                out,
                "su name tlat kwon ob {key} as ve {} {} from {} to {} at {} be text ya",
                slot.len(),
                word_list(&slot),
                text(&argument.tag_label, style),
                text(&argument.descriptive_specification, style),
                text("operation-argument", style)
            );
        }
        for result in &signature.results {
            let _ = writeln!(
                out,
                "su name tlat kwon ob {key} as ve {} {} from {} to {} at {} be text ya",
                result.noun_class.len(),
                word_list(&result.noun_class),
                text(&result.tag_label, style),
                text(&result.descriptive_specification, style),
                text("operation-result", style)
            );
        }
    }
    if let Some(definition) = &entry.definition {
        let words: Vec<u16> = definition.iter().flatten().copied().collect();
        let _ = writeln!(
            out,
            "su name tlat kwon ob {key} from ve {} {} at {} be text ya",
            words.len(),
            word_list(&words),
            text("definition", style)
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

/// Write a text value in the selected form.
///
/// English is `text "<json>"`. Glyph is a `zi` frame whose kind root is `kyit`
/// and whose payload is the raw text between two dots:
///
/// ```text
/// zi.kyit.<raw text>.kyit.zi
/// ```
///
/// Only two characters need escaping: a backslash becomes `\\`, and a literal
/// closing marker `.kyit.zi` inside the payload gets a backslash before its dot.
/// Everything else — spaces, punctuation, periods — stays as written, so the
/// payload reads as the text it is. The parser reverses both.
fn text(value: &str, style: TextStyle) -> String {
    match style {
        TextStyle::English => format!("text {}", quote(value)),
        TextStyle::Glyph => format!("zi.{KIND}.{}.{KIND}.zi", escape_frame(value)),
    }
}

/// The kind root of a text frame, glyph `kyit`.
const KIND: &str = "kyit";

/// The closing marker a payload must not contain unescaped.
const CLOSE: &str = ".kyit.zi";

fn escape_frame(value: &str) -> String {
    let mut out = String::with_capacity(value.len());
    let mut rest = value;
    while let Some(position) = rest.find(CLOSE) {
        // Escape the backslash and the leading dot of any embedded close marker.
        let head = &rest[..position];
        out.push_str(&head.replace('\\', "\\\\"));
        out.push_str("\\.kyit.zi");
        rest = &rest[position + CLOSE.len()..];
    }
    out.push_str(&rest.replace('\\', "\\\\"));
    out
}
