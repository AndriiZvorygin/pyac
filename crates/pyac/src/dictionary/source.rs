//! Parse the authored dictionary `.pya` source into a build request.
//!
//! This is the bootstrap step: `pyac-dictionary compile` reads the source
//! directly, so regenerating the volume needs no JavaScript. It replaces
//! `program/library/dictionary/source.mjs` from the `pyash` checkout.
//!
//! # The source is records, not programs
//!
//! Every line of `reference.pya` is one of:
//!
//! ```text
//! su name lyin kwon ob text "…" be text ya                       profile marker
//! su name lyin kwon ob text "…" as num 1 from text "…" …         profile metadata
//! su name tlat kwon ob text "<key>" at text "entry" be text ya   entry block
//! su name tlat kwon ob text "<key>" as text "en" from … at text "lexical-form" be text ya
//! ```
//!
//! There are no definition sentences in the accepted reference source — 730,376
//! of its 730,379 lines are records, and the other three are the two profile
//! sentences and the closing `prah`. That matters: the JavaScript parser's
//! fallback lowers definition sentences to VLIW, and that path is never taken
//! here. A port that reproduced it would be porting dead code.
//!
//! A sentence without an `at` clause is rejected rather than dropped, so the day
//! a definition sentence appears the build says so instead of quietly producing a
//! thinner volume.
//!
//! # Clause order
//!
//! The source writes **case then value**: `ob text "b6" at text "entry"`. This is
//! the opposite of the encoded volume, which writes value then case word. Reading
//! the source as if it were the volume silently parses nothing.
//!
//! # Records are positional within an entry block
//!
//! `at text "entry"` opens an entry; every later record attaches to the most
//! recently opened one, until the next `entry`. A record before any entry block
//! is an error, not a defaulted empty entry.

use std::collections::HashMap;
use std::fmt::{Display, Formatter};

use pyac_vliw_core::word_code;

/// One entry as the source declares it, before encoding.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct EntryRecord {
    pub key: Vec<u16>,
    pub lexical_forms: Vec<LexicalFormRecord>,
    pub source_links: Vec<SourceLinkRecord>,
    pub source_glosses: Vec<SourceGlossRecord>,
    pub component_keys: Vec<Vec<u16>>,
    pub task_keys: Vec<Vec<u16>>,
    pub references: Vec<Vec<u16>>,
    pub operation_signatures: Vec<SignatureRecord>,
    pub definition: Option<Vec<Vec<u16>>>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LexicalFormRecord {
    pub language: String,
    pub surface: String,
    pub normalized: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SourceLinkRecord {
    pub dataset: String,
    pub release: String,
    pub identifier: String,
    pub relation: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SourceGlossRecord {
    pub language: String,
    pub dataset: String,
    pub release: String,
    pub identifier: String,
    pub text: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SignatureRecord {
    pub verb: Vec<u16>,
    pub mood: Vec<u16>,
    pub arguments: Vec<ArgumentRecord>,
    pub results: Vec<ResultRecord>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ArgumentRecord {
    pub grammatical_case: Vec<u16>,
    pub noun_class: Vec<u16>,
    pub tag_label: String,
    pub descriptive_specification: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ResultRecord {
    pub noun_class: Vec<u16>,
    pub tag_label: String,
    pub descriptive_specification: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct VolumeMetadataRecord {
    pub kind: String,
    pub title: String,
    pub revision: u32,
    pub languages: Vec<String>,
    pub normalization_policy: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DictionarySource {
    pub metadata: VolumeMetadataRecord,
    pub entries: Vec<EntryRecord>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SourceError {
    Unsupported { line: usize, text: String },
    Orphan { line: usize, key: String },
    Duplicate { line: usize, key: String },
    UnknownWord { line: usize, word: String },
    /// A sentence with no `at` clause — a definition sentence. Rejected.
    DefinitionSentence { line: usize },
    MissingMetadata,
    EmptyText { line: usize, field: &'static str },
}

impl SourceError {
    pub fn code(&self) -> &'static str {
        match self {
            Self::Unsupported { .. } => "unsupported-source-sentence",
            Self::Orphan { .. } => "orphan-entry-record",
            Self::Duplicate { .. } => "duplicate-entry-block",
            Self::UnknownWord { .. } => "unknown-key-word",
            Self::DefinitionSentence { .. } => "unsupported-definition-sentence",
            Self::MissingMetadata => "missing-source-metadata",
            Self::EmptyText { .. } => "empty-source-text",
        }
    }
}

impl Display for SourceError {
    fn fmt(&self, formatter: &mut Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Unsupported { line, text } => {
                write!(formatter, "line {line}: unsupported source sentence: {text}")
            }
            Self::Orphan { line, key } => write!(
                formatter,
                "line {line}: record for {key:?} precedes its entry block"
            ),
            Self::Duplicate { line, key } => {
                write!(formatter, "line {line}: duplicate entry block for {key:?}")
            }
            Self::UnknownWord { line, word } => {
                write!(formatter, "line {line}: key word has no HNUC: {word}")
            }
            Self::DefinitionSentence { line } => write!(
                formatter,
                "line {line}: the accepted source has no definition sentences; \
                 carry the lowered codelets as a pre-lowered definition instead"
            ),
            Self::MissingMetadata => formatter.write_str(
                "dictionary source requires profile and metadata sentences for lyin kwon",
            ),
            Self::EmptyText { line, field } => {
                write!(formatter, "line {line}: {field} must be non-empty text")
            }
        }
    }
}

impl std::error::Error for SourceError {}

/// One clause: the case word and the single value that follows it.
#[derive(Debug, Clone)]
enum ClauseValue {
    Text(String),
    Number(u32),
    Vector(Vec<u16>),
}

/// Parse the whole source into a build request.
pub fn parse_source(source: &str) -> Result<DictionarySource, SourceError> {
    let mut kind: Option<String> = None;
    let mut title: Option<String> = None;
    let mut revision: Option<u32> = None;
    let mut languages: Vec<String> = Vec::new();
    let mut policy: Option<String> = None;

    let mut entries: Vec<EntryRecord> = Vec::new();
    let mut by_key_name: HashMap<String, usize> = HashMap::new();

    for (index, raw) in source.lines().enumerate() {
        let line_number = index + 1;
        let line = raw.trim();
        if line.is_empty() || line == "prah" {
            continue;
        }
        let clauses = parse_line(line, line_number)?;

        let subject = match clauses.get("su") {
            Some(ClauseValue::Text(name)) => name.clone(),
            _ => {
                return Err(SourceError::Unsupported {
                    line: line_number,
                    text: line.to_owned(),
                });
            }
        };

        if subject == "lyin kwon" {
            match (&clauses.get("as"), &clauses.get("ob")) {
                // Metadata: `ob <title> as num <revision> from <languages> to <NFC> with <version>`
                (Some(ClauseValue::Number(value)), Some(ClauseValue::Text(t))) => {
                    revision = Some(*value);
                    title = Some(t.clone());
                    if let Some(ClauseValue::Text(list)) = clauses.get("from") {
                        languages = list
                            .split(',')
                            .map(|item| item.trim().to_owned())
                            .filter(|item| !item.is_empty())
                            .collect();
                    }
                    if let Some(ClauseValue::Text(value)) = clauses.get("to") {
                        policy = Some(value.clone());
                    }
                }
                // Marker: `ob <kind>`
                (_, Some(ClauseValue::Text(t))) => kind = Some(t.clone()),
                _ => {
                    return Err(SourceError::Unsupported {
                        line: line_number,
                        text: line.to_owned(),
                    });
                }
            }
            continue;
        }

        if subject != "tlat kwon" {
            return Err(SourceError::Unsupported {
                line: line_number,
                text: line.to_owned(),
            });
        }

        let record_kind = match clauses.get("at") {
            Some(ClauseValue::Text(value)) => value.clone(),
            _ => return Err(SourceError::DefinitionSentence { line: line_number }),
        };
        let key_name = match clauses.get("ob") {
            Some(ClauseValue::Text(value)) if !value.trim().is_empty() => value.clone(),
            _ => {
                return Err(SourceError::EmptyText {
                    line: line_number,
                    field: "entry key",
                });
            }
        };

        if record_kind == "entry" {
            if by_key_name.contains_key(&key_name) {
                return Err(SourceError::Duplicate {
                    line: line_number,
                    key: key_name,
                });
            }
            let key = key_for_name(&key_name, line_number)?;
            entries.push(EntryRecord {
                key,
                ..EntryRecord::default()
            });
            by_key_name.insert(key_name, entries.len() - 1);
            continue;
        }

        let Some(&position) = by_key_name.get(&key_name) else {
            return Err(SourceError::Orphan {
                line: line_number,
                key: key_name,
            });
        };
        apply_record(&mut entries[position], &record_kind, &clauses, line_number)?;
    }

    let (Some(kind), Some(title), Some(revision)) = (kind, title, revision) else {
        return Err(SourceError::MissingMetadata);
    };
    if entries.is_empty() {
        return Err(SourceError::MissingMetadata);
    }
    Ok(DictionarySource {
        metadata: VolumeMetadataRecord {
            kind,
            title,
            revision,
            languages,
            normalization_policy: policy.unwrap_or_else(|| "NFC".to_owned()),
        },
        entries,
    })
}

fn apply_record(
    entry: &mut EntryRecord,
    kind: &str,
    clauses: &HashMap<&'static str, ClauseValue>,
    line: usize,
) -> Result<(), SourceError> {
    let text = |case: &'static str, field: &'static str| -> Result<String, SourceError> {
        match clauses.get(case) {
            Some(ClauseValue::Text(value)) if !value.trim().is_empty() => Ok(value.clone()),
            _ => Err(SourceError::EmptyText { line, field }),
        }
    };
    let vector = |case: &'static str| -> Option<Vec<u16>> {
        match clauses.get(case) {
            Some(ClauseValue::Vector(words)) => Some(words.clone()),
            _ => None,
        }
    };

    match kind {
        "lexical-form" => entry.lexical_forms.push(LexicalFormRecord {
            language: text("as", "lexical language")?,
            surface: text("from", "lexical surface")?,
            normalized: text("to", "normalized lexical surface")?,
        }),
        "source-link" => entry.source_links.push(SourceLinkRecord {
            dataset: text("as", "source dataset")?,
            release: text("from", "source release")?,
            identifier: text("to", "source identifier")?,
            relation: text("by", "source relation")?,
        }),
        "source-gloss" => entry.source_glosses.push(SourceGlossRecord {
            language: text("as", "source gloss language")?,
            dataset: text("from", "source gloss dataset")?,
            release: text("to", "source gloss release")?,
            identifier: text("by", "source gloss identifier")?,
            text: text("with", "source gloss text")?,
        }),
        "component" => {
            let name = text("from", "component key")?;
            entry.component_keys.push(key_for_name(&name, line)?);
        }
        "task" => {
            let name = text("with", "task key")?;
            entry.task_keys.push(key_for_name(&name, line)?);
        }
        "reference" => {
            let name = text("to", "reference key")?;
            entry.references.push(key_for_name(&name, line)?);
        }
        "operation-signature" => {
            let Some(words) = vector("from") else {
                return Err(SourceError::Unsupported {
                    line,
                    text: "operation-signature requires a from vector".to_owned(),
                });
            };
            entry.operation_signatures.push(parse_signature(&words, line)?);
        }
        "operation-argument" => {
            let Some(words) = vector("as") else {
                return Err(SourceError::Unsupported {
                    line,
                    text: "operation-argument requires an as vector".to_owned(),
                });
            };
            let Some(signature) = entry.operation_signatures.last_mut() else {
                return Err(SourceError::Orphan {
                    line,
                    key: "operation-argument".to_owned(),
                });
            };
            let (grammatical_case, noun_class) = parse_slot(&words, line)?;
            let label = text("from", "operation argument name")?;
            let description = text("to", "operation argument description")?;
            // Fill the argument the signature's dispatch key already declared.
            // Pushing would double the list and leave the declared one unlabelled
            // — the defect the volume decoder had.
            let Some(argument) = signature.arguments.iter_mut().find(|argument| {
                argument.grammatical_case == grammatical_case
                    && argument.noun_class == noun_class
                    && argument.tag_label.is_empty()
            }) else {
                return Err(SourceError::Unsupported {
                    line,
                    text: "operation-argument does not match an argument in its signature"
                        .to_owned(),
                });
            };
            argument.tag_label = label;
            argument.descriptive_specification = description;
        }
        "operation-result" => {
            let Some(words) = vector("as") else {
                return Err(SourceError::Unsupported {
                    line,
                    text: "operation-result requires an as vector".to_owned(),
                });
            };
            let Some(signature) = entry.operation_signatures.last_mut() else {
                return Err(SourceError::Orphan {
                    line,
                    key: "operation-result".to_owned(),
                });
            };
            signature.results.push(ResultRecord {
                noun_class: words,
                tag_label: text("from", "operation result name")?,
                descriptive_specification: text("to", "operation result description")?,
            });
        }
        other => {
            return Err(SourceError::Unsupported {
                line,
                text: format!("unsupported dictionary record type: {other}"),
            });
        }
    }
    Ok(())
}

/// Structural words the source uses as keys, which are not spellings the codec
/// can encode.
///
/// `fromindex` is the multiplicative-case atom (`multiplicative_case_`, 0x6357):
/// a role name in the grammar, not a Pyac word. It appears as a key component in
/// the reference source, so the parser must resolve it the way the encoder does.
/// The same table lives in `program/library/vliw/sentence.mjs`
/// (`ROLE_ENTRIES`), and this is the minimal subset that keys need.
const STRUCTURAL_KEYS: [(&str, u16); 1] = [
    // `multiplicative_case_`. A case word, not a spellable word, so the codec
    // cannot derive it.
    ("fromindex", 0x6357),
];

/// Resolve one key word: structural roles first, then the codec.
fn key_word(word: &str) -> Option<u16> {
    if let Some((_, value)) = STRUCTURAL_KEYS.iter().find(|(name, _)| *name == word) {
        return Some(*value);
    }
    word_code::encode(word).ok()
}

/// Resolve a Pyac spelling to its HNUC key. The codec is the only identity.
fn key_for_name(name: &str, line: usize) -> Result<Vec<u16>, SourceError> {
    let words: Vec<&str> = name.split_whitespace().collect();
    if words.is_empty() {
        return Err(SourceError::EmptyText {
            line,
            field: "entry key",
        });
    }
    words
        .into_iter()
        .map(|word| {
            key_word(word).ok_or_else(|| SourceError::UnknownWord {
                line,
                word: word.to_owned(),
            })
        })
        .collect()
}

/// Every case word the source uses, including the two that are not cases in the
/// encoded volume: `su` carries the subject and `name` names it.
const CASES: [&str; 10] = ["su", "ob", "as", "from", "to", "by", "with", "at", "be", "name"];

/// Split one sentence into `su` plus its case clauses.
///
/// The source writes each case word immediately before its value, so a clause
/// runs from a case word to the next one. Quoted text is skipped, so a value that
/// contains a word like `from` is never mistaken for a case.
fn parse_line(line: &str, line_number: usize) -> Result<HashMap<&'static str, ClauseValue>, SourceError> {
    let unsupported = || SourceError::Unsupported {
        line: line_number,
        text: line.to_owned(),
    };

    // Tokenise into (case word | bare token) while respecting quotes.
    let mut tokens: Vec<String> = Vec::new();
    let mut current = String::new();
    let mut in_text = false;
    let mut escaped = false;
    for character in line.chars() {
        if in_text {
            current.push(character);
            if escaped {
                escaped = false;
            } else if character == '\\' {
                escaped = true;
            } else if character == '"' {
                in_text = false;
            }
            continue;
        }
        if character == '"' {
            in_text = true;
            current.push(character);
            continue;
        }
        if character.is_whitespace() {
            if !current.is_empty() {
                tokens.push(std::mem::take(&mut current));
            }
            continue;
        }
        current.push(character);
    }
    if !current.is_empty() {
        tokens.push(current);
    }
    if in_text {
        return Err(unsupported());
    }
    // Drop the trailing mood: `ya`.
    if tokens.last().is_some_and(|token| token == "ya") {
        tokens.pop();
    }
    if tokens.first().is_some_and(|token| token == "be") {
        tokens.drain(..2);
    }

    let mut clauses: HashMap<&'static str, ClauseValue> = HashMap::new();
    let mut index = 0usize;
    while index < tokens.len() {
        let token = tokens[index].clone();
        // `be` opens the closing verb phrase, which this parser does not model.
        if token == "be" {
            break;
        }
        let Some(case) = CASES.iter().find(|candidate| **candidate == token) else {
            return Err(unsupported());
        };
        index += 1;
        let Some(value) = tokens.get(index) else {
            return Err(unsupported());
        };
        let (parsed, consumed) = parse_value(value, &tokens, index, *case, line_number)?;
        clauses.insert(*case, parsed);
        index += consumed;
    }
    Ok(clauses)
}

/// Parse one clause value starting at `index`, returning how many tokens it used.
fn parse_value(
    token: &str,
    tokens: &[String],
    index: usize,
    case: &'static str,
    line: usize,
) -> Result<(ClauseValue, usize), SourceError> {
    let value = match token {
        "text" => {
            let raw = tokens.get(index + 1).ok_or(SourceError::Unsupported {
                line,
                text: token.to_owned(),
            })?;
            let parsed: String = serde_json::from_str(raw).map_err(|_| SourceError::Unsupported {
                line,
                text: raw.to_owned(),
            })?;
            (ClauseValue::Text(parsed), 2)
        }
        "num" => {
            let raw = tokens.get(index + 1).ok_or(SourceError::Unsupported {
                line,
                text: token.to_owned(),
            })?;
            let parsed: u32 = raw.parse().map_err(|_| SourceError::Unsupported {
                line,
                text: raw.to_owned(),
            })?;
            (ClauseValue::Number(parsed), 2)
        }
        "ve" => {
            // `ve <count> <words…>` — the count is redundant with the run length,
            // so consume exactly it.
            let count_raw = tokens.get(index + 1).ok_or(SourceError::Unsupported {
                line,
                text: token.to_owned(),
            })?;
            let count: usize = count_raw.parse().map_err(|_| SourceError::Unsupported {
                line,
                text: count_raw.to_owned(),
            })?;
            let mut words = Vec::with_capacity(count);
            for offset in 0..count {
                let raw = tokens.get(index + 2 + offset).ok_or(SourceError::Unsupported {
                    line,
                    text: token.to_owned(),
                })?;
                let parsed = parse_vector_word(raw).ok_or_else(|| SourceError::Unsupported {
                    line,
                    text: raw.to_owned(),
                })?;
                words.push(parsed);
            }
            (ClauseValue::Vector(words), count + 2)
        }
        // A bare word after a name case is the name itself. `su name lyin kwon`
        // carries a two-word name, so consume until the next case word.
        other if case == "su" || case == "name" => {
            let mut words: Vec<&str> = vec![other];
            let mut consumed = 1usize;
            while let Some(next) = tokens.get(index + consumed) {
                if CASES.contains(&next.as_str()) || next == "be" {
                    break;
                }
                words.push(next);
                consumed += 1;
            }
            // `su name X` declares X: `name` is the subject's noun class, not
            // part of the name itself.
            if words.first() == Some(&"name") && words.len() > 1 {
                words.remove(0);
            }
            (ClauseValue::Text(words.join(" ")), consumed)
        }
        other => (ClauseValue::Text(other.to_owned()), 1),
    };
    Ok(value)
}

/// One uint16 in a `ve` run.
///
/// Decimal first, because that is what the source writes: `10590` is the number
/// ten thousand five hundred and ninety, not an oversized hex literal. A `0x`
/// prefix or a value too large for decimal is read as hex, matching the
/// JavaScript parser this replaces.
fn parse_vector_word(word: &str) -> Option<u16> {
    let trimmed = word.trim();
    if let Some(hex) = trimmed.strip_prefix("0x").or_else(|| trimmed.strip_prefix("0X")) {
        return u16::from_str_radix(hex, 16).ok();
    }
    if let Ok(value) = trimmed.parse::<u16>() {
        return Some(value);
    }
    u16::from_str_radix(trimmed, 16).ok()
}

fn take_sequence(words: &[u16], cursor: &mut usize) -> Option<Vec<u16>> {
    let count = *words.get(*cursor)? as usize;
    *cursor += 1;
    let end = cursor.checked_add(count)?;
    let sequence = words.get(*cursor..end)?.to_vec();
    *cursor = end;
    Some(sequence)
}

fn parse_signature(words: &[u16], line: usize) -> Result<SignatureRecord, SourceError> {
    let mut cursor = 0usize;
    let unsupported = || SourceError::Unsupported {
        line,
        text: "operation signature has an invalid dispatch key".to_owned(),
    };
    let verb = take_sequence(words, &mut cursor).ok_or_else(unsupported)?;
    let mood = take_sequence(words, &mut cursor).ok_or_else(unsupported)?;
    let count = *words.get(cursor).ok_or_else(unsupported)? as usize;
    cursor += 1;
    let mut arguments = Vec::with_capacity(count);
    for _ in 0..count {
        let grammatical_case = take_sequence(words, &mut cursor).ok_or_else(unsupported)?;
        let noun_class = take_sequence(words, &mut cursor).ok_or_else(unsupported)?;
        arguments.push(ArgumentRecord {
            grammatical_case,
            noun_class,
            tag_label: String::new(),
            descriptive_specification: String::new(),
        });
    }
    if cursor != words.len() {
        return Err(unsupported());
    }
    Ok(SignatureRecord {
        verb,
        mood,
        arguments,
        results: Vec::new(),
    })
}

fn parse_slot(words: &[u16], line: usize) -> Result<(Vec<u16>, Vec<u16>), SourceError> {
    let mut cursor = 0usize;
    let unsupported = || SourceError::Unsupported {
        line,
        text: "operation argument descriptor has trailing words".to_owned(),
    };
    let grammatical_case = take_sequence(words, &mut cursor).ok_or_else(unsupported)?;
    let noun_class = take_sequence(words, &mut cursor).ok_or_else(unsupported)?;
    if cursor != words.len() {
        return Err(unsupported());
    }
    Ok((grammatical_case, noun_class))
}
