//! The parse error, and the codelet geometry callers need to size a buffer.

/// Payload slots per codelet; slot 0 is the summary word.
pub const CODELET_PAYLOAD: usize = 15;
/// A full codelet, summary word included.
pub const CODELET_WIDTH: usize = CODELET_PAYLOAD + 1;
/// Why a sentence could not be encoded.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ParseError {
    /// A glyph that begins no word.
    BadGlyph,
    /// A word-shaped run the codec could not encode.
    Unencodable,
    /// The caller's codelet buffer filled up.
    CodeletCapacity,
    /// Nothing to parse.
    Empty,
}
