//! Spelling a key name into HNUC words.

use super::errors::SourceError;
use pyac_vliw_core::word_code;

/// Structural words the source uses as keys, which are not spellings the codec
/// can encode.
///
/// `fromindex` is the multiplicative-case atom (`multiplicative_case_`, 0x6357):
/// a role name in the grammar, not a Pyac word. It appears as a key component in
/// the reference source, so the parser must resolve it the way the encoder does.
/// The same table lives in `program/library/vliw/sentence.mjs`
/// (`ROLE_ENTRIES`), and this is the minimal subset that keys need.
pub(super) const STRUCTURAL_KEYS: [(&str, u16); 1] = [
    // `multiplicative_case_`. A case word, not a spellable word, so the codec
    // cannot derive it.
    ("fromindex", 0x6357),
];
/// Resolve one key word: structural roles first, then the codec.
pub(super) fn key_word(word: &str) -> Option<u16> {
    if let Some((_, value)) = STRUCTURAL_KEYS.iter().find(|(name, _)| *name == word) {
        return Some(*value);
    }
    word_code::encode(word).ok()
}
/// Resolve a Pyac spelling to its HNUC key. The codec is the only identity.
pub(super) fn key_for_name(name: &str, line: usize) -> Result<Vec<u16>, SourceError> {
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
