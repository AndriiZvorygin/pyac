//! Render a decoded volume as Pyac `.pya`: the codelet sentences themselves.
//!
//! The English form (`render.rs`) writes `su name tlat kwon ob text "…" at text
//! "entry" be text ya` — a JSON-ish rendering that needs English role words. This
//! writes the Pyac the codelets are:
//!
//! ```text
//! gi <key> na <value> <case> … hwus li
//! ```
//!
//! The record kind is implied by its case sequence, exactly what `decode_entry`
//! matches, so no `"entry"`/`"lexical-form"` tag appears. Text is a
//! `zi.kyit. <text> .kyit.zi` frame; vectors are `ve <n> <lanes…>`; numbers stay
//! typed literals. Every value is algorithmic — no lookup table, no ISO-code table.
//!
//! # Keys that spell a grammar word
//!
//! A key can be `li`, `na`, `ka`, `ve`, `pu` — words that are also the case, mood
//! or directive words the record grammar uses. Written bare, such a key would close
//! a field. The word quote `wo` marks a word as data: the key `li` is written
//! `wo li`, so it is name payload and not the realis mood.
//!
//! # Gate
//!
//! `dictionary_pyac.rs` renders the published volume, parses the text back, and
//! requires the same codelets. `pyac_record_coverage.rs` measures the whole volume.

use std::fmt::Write;

use super::render::RenderError;
use super::volume::{DictionaryVolume, DictionaryVolumeEntry};
use super::words::{
    COMITATIVE, DESTINATION, ESSIVE, INSTRUMENTAL, NAME_MARKER, NOMINATIVE, REALIS, SOURCE,
    TEXT_VERB, WORD_QUOTE,
};

/// The vector grammar word, `ve`.
const VECTOR_WORD: u16 = 0x2ede;
/// The kind root of a `zi` text frame, `kyit`.
const TEXT_KIND: &str = "kyit";

/// Words that the record grammar reads as structure, so a *key* spelled this way
/// must be word-quoted to stay data.
fn is_structural(word: u16) -> bool {
    matches!(
        word,
        NAME_MARKER
            | NOMINATIVE
            // The word quote itself: a key `wo` is written `wo wo` (`giwowo`), which
            // is the phrase for quoting the word quote.
            | WORD_QUOTE
            | super::words::ACCUSATIVE
            | ESSIVE
            | SOURCE
            | DESTINATION
            | INSTRUMENTAL
            | COMITATIVE
            | super::words::LOCATIVE
            | REALIS
            | TEXT_VERB
            | VECTOR_WORD
            | 0x295e // deontic
            | 0x29be // conditional
            | 0x213e // epistemic
            | 0x21de // interrogative
            | 0x227e // directive
            | 0x5217 // declarative
            | 0x4e27 // paragraph
            | 0x327e // do, the 16-bit width word
            | 0x309e // po
            | 0x289e // pu
            | 0x6b57 // tl6h
            | 0x6d57 // tc6h
            | 0x4067 // fyih
    )
}

fn spell(word: u16) -> String {
    let mut buffer = [0u8; 8];
    match pyac_vliw_core::word_code::decode(word, &mut buffer) {
        Ok(length) => String::from_utf8_lossy(&buffer[..length]).into_owned(),
        Err(_) => format!("0x{word:04x}"),
    }
}

/// A key word, word-quoted (`wo <word>`) when it is a structural word.
fn key_word(word: u16) -> String {
    if is_structural(word) {
        format!("{} {}", spell(WORD_QUOTE), spell(word))
    } else {
        spell(word)
    }
}

fn record_head(key: &[u16]) -> Result<String, RenderError> {
    if key.is_empty() {
        return Err(RenderError::EmptyKey);
    }
    let mut out = format!("{}", spell(NAME_MARKER));
    for word in key {
        let mut buffer = [0u8; 8];
        if pyac_vliw_core::word_code::decode(*word, &mut buffer).is_err() {
            return Err(RenderError::UnspellableKey { hnuc: *word });
        }
        out.push(' ');
        out.push_str(&key_word(*word));
    }
    out.push(' ');
    out.push_str(&spell(NOMINATIVE));
    Ok(out)
}

/// A text value as `zi.kyit.<payload>.kyit.zi`, escaping the close marker.
fn frame(text: &str) -> String {
    let close = format!(".{TEXT_KIND}.zi");
    let mut out = String::with_capacity(text.len() + 16);
    let mut rest = text;
    while let Some(position) = rest.find(&close) {
        out.push_str(&rest[..position].replace('\\', "\\\\"));
        out.push('\\');
        out.push_str(&close);
        rest = &rest[position + close.len()..];
    }
    out.push_str(&rest.replace('\\', "\\\\"));
    format!("zi.{TEXT_KIND}.{out}.{TEXT_KIND}.zi")
}

fn vector(lanes: &[u16]) -> String {
    let mut out = format!("{} {}", spell(VECTOR_WORD), lanes.len());
    for lane in lanes {
        let _ = write!(out, " {lane}");
    }
    out
}

/// Render the whole volume as Pyac records, one per line.
pub fn render_pyac_source(volume: &DictionaryVolume) -> Result<String, RenderError> {
    let mut out = String::new();
    for entry in &volume.entries {
        render_entry(entry, &mut out)?;
    }
    Ok(out)
}

fn render_entry(entry: &DictionaryVolumeEntry, out: &mut String) -> Result<(), RenderError> {
    let head = record_head(&entry.key)?;
    let tail = format!("{} {}", spell(TEXT_VERB), spell(REALIS));
    for form in &entry.lexical_forms {
        let _ = writeln!(
            out,
            "{head} {} {} {} {} {} {} {tail}",
            frame(&form.language),
            spell(ESSIVE),
            frame(&form.surface),
            spell(SOURCE),
            frame(&form.normalized),
            spell(DESTINATION)
        );
    }
    for link in &entry.source_links {
        let _ = writeln!(
            out,
            "{head} {} {} {} {} {} {} {} {} {tail}",
            frame(&link.dataset),
            spell(ESSIVE),
            frame(&link.release),
            spell(SOURCE),
            frame(&link.identifier),
            spell(DESTINATION),
            frame(&link.relation),
            spell(INSTRUMENTAL)
        );
    }
    for gloss in &entry.source_glosses {
        let _ = writeln!(
            out,
            "{head} {} {} {} {} {} {} {} {} {} {} {tail}",
            frame(&gloss.language),
            spell(ESSIVE),
            frame(&gloss.dataset),
            spell(SOURCE),
            frame(&gloss.release),
            spell(DESTINATION),
            frame(&gloss.identifier),
            spell(INSTRUMENTAL),
            frame(&gloss.text),
            spell(COMITATIVE)
        );
    }
    for component in &entry.component_keys {
        let _ = writeln!(out, "{head} {} {} {tail}", vector(component), spell(SOURCE));
    }
    for task in &entry.task_keys {
        let _ = writeln!(out, "{head} {} {} {tail}", vector(task), spell(COMITATIVE));
    }
    for reference in &entry.references {
        let _ = writeln!(
            out,
            "{head} {} {} {tail}",
            vector(reference),
            spell(DESTINATION)
        );
    }
    for signature in &entry.operation_signatures {
        // The dispatch vector: verb, mood, then one slot per argument.
        let _ = writeln!(
            out,
            "{head} {} {} {tail}",
            vector(&dispatch_words(signature)),
            spell(INSTRUMENTAL)
        );
        for argument in &signature.arguments {
            let _ = writeln!(
                out,
                "{head} {} {} {} {} {} {} {tail}",
                vector(&slot_words(argument)),
                spell(ESSIVE),
                frame(&argument.tag_label),
                spell(SOURCE),
                frame(&argument.descriptive_specification),
                spell(DESTINATION)
            );
        }
        for result in &signature.results {
            let _ = writeln!(
                out,
                "{head} {} {} {} {} {} {} {tail}",
                vector(&result.noun_class),
                spell(ESSIVE),
                frame(&result.tag_label),
                spell(INSTRUMENTAL),
                frame(&result.descriptive_specification),
                spell(COMITATIVE)
            );
        }
    }
    Ok(())
}

/// The dispatch key words of a signature, in `parse_signature` order: verb, mood,
/// then each argument as case-length, case, class-length, class.
fn dispatch_words(signature: &super::operation::OperationSignature) -> Vec<u16> {
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

/// One argument slot as the `swih ve` descriptor `parse_slot` reads.
fn slot_words(argument: &super::operation::OperationParameter) -> Vec<u16> {
    let mut words = Vec::new();
    words.push(argument.case_type.grammatical_case.len() as u16);
    words.extend(argument.case_type.grammatical_case.iter().copied());
    words.push(argument.case_type.noun_class.len() as u16);
    words.extend(argument.case_type.noun_class.iter().copied());
    words
}
