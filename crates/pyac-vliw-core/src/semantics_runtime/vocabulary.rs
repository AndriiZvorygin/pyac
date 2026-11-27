//! The HNUC words the runtime compares against, and the limits it enforces.
//!
//! Each constant names one word of the instruction set by its spelling, so a
//! reader can match the number against the spec.

pub const CODELET_WORDS: usize = 16;

pub const MAX_VECTOR_LANES: usize = 16;

pub const RESULT_WORDS: usize = 32;

pub const MAX_PROGRAMLET_SECTIONS: usize = 64;

pub const MAX_PROGRAMLET_SECTION_NAME_WORDS: usize = 8;

pub const MAX_PROGRAMLET_ROUTINES: usize = 64;

/// Scratch capacity for interned identifiers and dispatch signature words.
pub const MAX_PROGRAMLET_SYMBOL_WORDS: usize = 960;

pub const MAX_PROGRAMLET_LOCAL_SLOTS: usize = 64;

pub const MAX_PROGRAMLET_CALL_DEPTH: usize = 8;

pub const MAX_PROGRAMLET_VECTOR_VALUE_WORDS: usize =
    MAX_PROGRAMLET_CALL_DEPTH * MAX_PROGRAMLET_VECTOR_LOCAL_SLOTS * RESULT_WORDS;

pub(crate) const MAX_BOOLEAN_EXPRESSION_DEPTH: usize = 8;

pub(crate) const PROGRAMLET_EXECUTION_COMPLETE: u32 = u32::MAX;

pub(crate) const MAX_U16: u32 = 65_535;

pub(crate) const SYMBOL_NAME_SEPARATOR: u32 = MAX_U16 + 1;

pub const MAX_PROGRAMLET_ARGUMENTS: usize = 8;

pub(crate) const MAX_SENTENCE_FIELDS: usize = MAX_PROGRAMLET_ARGUMENTS + 2;

pub const VALUE_UNSIGNED: u32 = 1;

pub const VALUE_SIGNED: u32 = 2;

pub const VALUE_DATE: u32 = 3;

pub const VALUE_VECTOR_U16: u32 = 4;

pub const VALUE_VECTOR_U32: u32 = 5;

pub const VALUE_VECTOR_BOOL: u32 = 6;

pub const VALUE_BOOLEAN: u32 = 7;

pub(crate) const VALUE_NUMBER: u32 = 8;

pub const VALUE_FLOAT32: u32 = 9;

pub const VALUE_FIXED16: u32 = 10;

pub const VALUE_VECTOR_FLOAT32: u32 = 11;

pub const VALUE_VECTOR_FLOAT64: u32 = 12;

pub const VALUE_FLOAT64: u32 = 13;

pub(crate) const VALUE_FLOAT: u32 = 14;

pub(crate) const VALUE_VECTOR_FLOAT: u32 = 15;

pub(crate) const MAX_PROGRAMLET_VECTOR_LOCAL_SLOTS: usize = 8;

pub const ERROR_UNSUPPORTED: u32 = 1;

pub const ERROR_DIVISION_BY_ZERO: u32 = 2;

pub const ERROR_MALFORMED: u32 = 3;

pub const ERROR_VECTOR_WIDTH: u32 = 4;

pub const ERROR_BRANCH: u32 = 5;

pub const ERROR_FRAME_CAPACITY: u32 = 6;

pub const ERROR_CONSTANT_WRITE: u32 = 7;

pub const ERROR_STEP_BUDGET: u32 = 8;

pub const ERROR_PROGRAM_CAPACITY: u32 = 9;

pub const ERROR_CALL_DEPTH: u32 = 10;

pub const ERROR_FLOAT_DOMAIN: u32 = 11;

pub const ERROR_FIXED16_DOMAIN: u32 = 12;

pub const ERROR_DUPLICATE_SIGNATURE: u32 = 13;

pub(crate) const NUMBER_QUOTE: u32 = 0x881d;

pub(crate) const DATE_QUOTE: u32 = 0x981d;

pub(crate) const QUOTED_GRAMMAR: u32 = 0x229e;

pub(crate) const ACCUSATIVE: u32 = 0x245e;

pub(crate) const NOMINATIVE: u32 = 0x24de;

pub(crate) const SOURCE: u32 = 0x313e;

pub(crate) const DESTINATION: u32 = 0x243e;

pub(crate) const LOCATIVE: u32 = 0x615f;

pub(crate) const MULTIPLICATIVE: u32 = 0x6357;

pub(crate) const COMITATIVE: u32 = 0x490f;

pub(crate) const INSTRUMENTAL: u32 = 0x287e;

pub(crate) const DATIVE: u32 = 0x207e;

pub(crate) const DEONTIC: u32 = 0x295e;

pub(crate) const REALIS: u32 = 0x217e;

pub(crate) const CONDITIONAL: u32 = 0x29be;

pub(crate) const NUMBER_VERB: u32 = 0xea30;

pub(crate) const BOOLEAN_VERB: u32 = 0x7271;

pub(crate) const EQUALLY: u32 = 0x4809;

pub(crate) const TINY: u32 = 0x6c0d;

pub(crate) const GIANT: u32 = 0x69b3;

pub(crate) const NEGATORY_QUANTIFIER: u32 = 0x30de;

pub(crate) const AND_GRAMMAR: u32 = 0x24be;

pub(crate) const OR_GRAMMAR: u32 = 0x263e;

pub(crate) const TRUTH: u32 = 0x6909;

pub(crate) const LIE: u32 = 0x680d;

pub(crate) const DEPENDENT_CLAUSE: u32 = 0x257e;

pub(crate) const CLAUSE_TAIL: u32 = 0x305e;

pub(crate) const PLUS: u32 = 0x8a64;

pub(crate) const SUBTRACT: u32 = 0xabd2;

pub(crate) const MULTIPLY: u32 = 0xec90;

pub(crate) const DIVIDE: u32 = 0x8893;

pub(crate) const XOR: u32 = 0xa810;

pub(crate) const EXPONENTIAL: u32 = 0x8c29;

pub(crate) const REMAINS: u32 = 0xe82a;

pub(crate) const INVERT: u32 = 0xaad2;

pub(crate) const QUOTE_DENOTE: u32 = 0x001d;

pub(crate) const NAME_TYPE: u32 = 0x225e;

pub(crate) const NUMBER_TYPE: u32 = 0x327e;

pub(crate) const BOOLEAN_TYPE: u32 = 0x538f;

pub(crate) const VECTOR_TYPE: u32 = 0x2b73;

pub(crate) const FLOAT_TYPE: u32 = 0xaa6c;

pub(crate) const PAUCAL_WIDTH: u32 = 0x309e;

pub(crate) const MULTAL_WIDTH: u32 = 0x6b57;

pub(crate) const PLURAL_WIDTH: u32 = 0x289e;

/// `fyih`, the floating-point class word. Distinct from `flut` (a lexical root).
pub(crate) const FLOAT_CLASS: u32 = 0x4067;

/// `kcot` gather and `kyut` computer: the storage-section verb pair.
pub(crate) const STORAGE_GATHER: u32 = 0xaca2;

pub(crate) const STORAGE_COMPUTER: u32 = 0xaa02;

pub(crate) const VARIABLE: u32 = 0xc8c4;

pub(crate) const PERMANENTLY: u32 = 0x0b50;

pub(crate) const CEREMONY: u32 = 0xe8c9;

pub(crate) const SECTION_MODULE: u32 = 0xec01;

pub(crate) const SECTION_PART: u32 = 0x6a84;

pub(crate) const MOOD_DEFINITION: u32 = 0x5217;

pub(crate) const MOOD_PARAGRAPH: u32 = 0x4e27;

pub(crate) const MOOD_RETURN: u32 = 0xa828;

pub(crate) const ESSIVE: u32 = 0x414f;

pub(crate) const FROM_INDEX: u32 = 0x6357;

pub(crate) const TO_INDEX: u32 = 0x4e4f;

pub(crate) const GENITIVE_CASE: u32 = 0x215e;

pub(crate) const THIS_WORD: u32 = 0x4a57;

pub const DEFAULT_PROGRAMLET_STEP_BUDGET: u32 = 65_536;

/// Whether `width_word` names a width that can hold `value`.
///
/// The width words are the quantity class: `po` one byte, `do` two, `pu` four,
/// `tl6h` eight. A value that does not fit is malformed, not clamped — silent
/// truncation would make the width word decorative.
pub(crate) fn width_fits(width_word: u32, value: u32) -> bool {
    match width_word {
        PAUCAL_WIDTH => value <= u8::MAX as u32,
        NUMBER_TYPE => value <= u16::MAX as u32,
        PLURAL_WIDTH | MULTAL_WIDTH => true,
        _ => false,
    }
}

/// Whether `word` is a leading type tag on a value.
///
/// These are the quantity width words plus the boolean class word: each one says
/// "what follows is a value of this kind", never "this is a name".
pub(crate) fn is_width_word(word: u32) -> bool {
    matches!(
        word,
        PAUCAL_WIDTH | NUMBER_TYPE | PLURAL_WIDTH | MULTAL_WIDTH | BOOLEAN_TYPE
    )
}
