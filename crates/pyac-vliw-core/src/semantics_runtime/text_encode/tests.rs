//! Word-boundary rules, checked against the shapes the parser derives.
use super::super::text_encode::*;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_cv_word_is_two_glyphs() {
        assert_eq!(first_word_length(b"doka"), Some(2));
    }

    #[test]
    fn a_cvt_word_is_three_glyphs() {
        assert_eq!(first_word_length(b"no7ka"), Some(3));
    }

    #[test]
    fn a_ccvc_word_is_four_glyphs() {
        assert_eq!(first_word_length(b"plustu"), Some(4));
    }

    #[test]
    fn a_ccvt_word_is_five_glyphs() {
        assert_eq!(first_word_length(b"kse7m"), Some(5));
    }

    #[test]
    fn a_vowel_initial_word_asks_the_caller_to_advance() {
        assert_eq!(first_word_length(b"ipla"), Some(0));
    }

    #[test]
    fn separators_are_not_letters() {
        assert!(!is_letter(b' '));
        assert!(is_letter(b'k'));
        assert!(is_letter(b'a'));
        assert!(is_letter(b'7'));
    }
}
