//! The codelet builder: the buffer the encoder writes into.
//!
//! The crate is `no_std` with no allocator, so the builder writes into a
//! caller-supplied codelet buffer and reports how many codelets it filled. Slot 0
//! of each codelet is the summary word carrying the boundary bitmap and the
//! completion bit; slots 1..=15 are payload.

use super::errors::*;
use super::numbers::*;
use super::words::*;
/// A codelet under construction, writing straight into the caller's buffer.
pub(super) struct Builder<'a> {
    codelets: &'a mut [[u32; CODELET_WIDTH]],
    count: usize,
    summary: u32,
    used: usize,
}
impl<'a> Builder<'a> {
    pub(super) fn new(codelets: &'a mut [[u32; CODELET_WIDTH]]) -> Self {
        Self {
            codelets,
            count: 1,
            summary: 0,
            used: 0,
        }
    }

    pub(super) fn current(&mut self) -> &mut [u32; CODELET_WIDTH] {
        &mut self.codelets[self.count - 1]
    }

    /// Append a payload word, optionally marking it a boundary.
    pub(super) fn push(&mut self, word: u32, boundary: bool) -> Result<(), ParseError> {
        if self.used == CODELET_PAYLOAD {
            if self.count == self.codelets.len() {
                return Err(ParseError::CodeletCapacity);
            }
            self.seal();
            self.count += 1;
            self.summary = 0;
            self.used = 0;
        }
        let slot = self.used + 1;
        if boundary {
            self.summary |= 1 << slot;
        }
        self.current()[slot] = word;
        self.used += 1;
        Ok(())
    }

    /// Write the summary word for a codelet that continues into a later one.
    ///
    /// The writer marks boundaries **negatively** on an incomplete codelet: a
    /// slot that closes a field has its bit clear and every other used slot has
    /// its bit set. `reader_codec::slot_is_boundary` reads exactly that (`!set`
    /// when incomplete). The completion bit stays clear. The final codelet uses
    /// the positive form instead; see [`Builder::finish`].
    pub(super) fn seal(&mut self) {
        let used = if self.used == 0 {
            0
        } else {
            ((1u32 << (self.used + 1)) - 1) & !1
        };
        self.current()[0] = used & !self.summary;
    }

    /// Seal the indexed codelet and open a raw continuation region.
    ///
    /// A text quote longer than fourteen units carries its units in codelets whose
    /// every slot is payload — no summary word. `pack_words` chunks them sixteen
    /// words at a time and `read_sentence_with_raw_width` reads sixteen per codelet,
    /// so slot 0 is data here.
    pub(super) fn begin_raw_region(&mut self) -> Result<(), ParseError> {
        self.seal();
        self.advance_blank()
    }

    /// Close a raw region: start a fresh indexed codelet for the words that follow.
    pub(super) fn end_raw_region(&mut self) -> Result<(), ParseError> {
        self.advance_blank()
    }

    /// Append a raw payload word, starting a new codelet when the current fills.
    pub(super) fn push_raw(&mut self, word: u32) -> Result<(), ParseError> {
        if self.used == CODELET_WIDTH {
            if self.count == self.codelets.len() {
                return Err(ParseError::CodeletCapacity);
            }
            self.count += 1;
            self.used = 0;
            let codelet = self.current();
            for slot in codelet.iter_mut() {
                *slot = 0;
            }
        }
        let used = self.used;
        self.current()[used] = word;
        self.used += 1;
        Ok(())
    }

    /// Move to a fresh, zeroed codelet with an empty summary.
    pub(super) fn advance_blank(&mut self) -> Result<(), ParseError> {
        if self.count == self.codelets.len() {
            return Err(ParseError::CodeletCapacity);
        }
        self.count += 1;
        self.used = 0;
        self.summary = 0;
        let codelet = self.current();
        for slot in codelet.iter_mut() {
            *slot = 0;
        }
        Ok(())
    }

    /// Fold one digit into the numeric literal already on the codelet.
    ///
    /// The literal's words were pushed when `do` opened it, so the value word is
    /// the last one written. `digits` counts how many have been folded already,
    /// tracked explicitly because leading zeros carry no information in the value.
    pub(super) fn accumulate_number(
        &mut self,
        digit: u32,
        format: NumberFormat,
        digits: &mut u32,
    ) -> Result<(), ParseError> {
        let last = match self.count.checked_sub(1) {
            Some(index) => index,
            None => return Err(ParseError::CodeletCapacity),
        };
        let slot = self.used;
        if slot == 0 {
            return Err(ParseError::Empty);
        }
        let value = self.codelets[last][slot];
        let folded = if format.big_endian {
            value.wrapping_mul(format.base).wrapping_add(digit)
        } else {
            // Little endian: the first digit is least significant, so each new
            // digit is scaled by the base raised to the count already folded.
            let scale = format.base.saturating_pow(*digits);
            value.wrapping_add(digit.wrapping_mul(scale))
        };
        self.codelets[last][slot] = folded;
        *digits = digits.saturating_add(1);
        Ok(())
    }

    /// Seal the last codelet, marking it complete.
    pub(super) fn finish(mut self) -> usize {
        let summary = self.summary | 1;
        self.current()[0] = summary;
        self.count
    }
}
