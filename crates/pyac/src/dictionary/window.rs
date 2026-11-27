//! A streaming window over the volume.
//!
//! The volume reader asks for one 32-byte codelet at a time, four million times,
//! each preceded by a seek. Against a plain `File` that is two syscalls per
//! codelet; decoding took 16.2 s where the same work from memory took 1.4 s.
//!
//! `BufReader` does not fix it. A seek discards its buffer, so every codelet
//! re-reads from the kernel, which measured *slower* than unbuffered. Holding the
//! whole 131 MB in memory does fix it, but streaming should not need the file.
//!
//! The moves are almost entirely forward and sequential — a sentence is walked
//! codelet by codelet — so a window only has to survive a seek that lands inside
//! it. `Window<R>` keeps a bounded buffer and, on `seek` to a position the buffer
//! already covers, just moves the cursor: no syscall. A miss refills once and then
//! serves the run that follows. Memory stays at one window, not one volume.
//!
//! Implemented as `Read + Seek` so the reader's existing seek-and-read code is
//! unchanged; only what it is layered over differs.

use std::io::{self, Read, Seek, SeekFrom};

/// How much a refill reads ahead. Large enough that a sentence's codelets and the
/// raw-text continuations that follow them are one read, small enough to stay a
/// cache rather than a copy of the volume.
const WINDOW_BYTES: usize = 1 << 20;

pub struct Window<R> {
    inner: R,
    buffer: Vec<u8>,
    /// Byte position of `buffer[0]` in the underlying stream.
    buffer_start: u64,
    /// Bytes of `buffer` holding data.
    buffer_len: usize,
    /// The caller's current byte position.
    position: u64,
    /// Total length, cached on first `End` seek because `open` asks for it.
    length: Option<u64>,
}

impl<R: Read + Seek> Window<R> {
    pub fn new(inner: R) -> Self {
        Self {
            inner,
            buffer: Vec::new(),
            buffer_start: 0,
            buffer_len: 0,
            position: 0,
            length: None,
        }
    }

    /// Whether `position` falls inside the currently buffered run.
    fn buffered(&self) -> bool {
        self.position >= self.buffer_start
            && self.position < self.buffer_start + self.buffer_len as u64
    }

    /// Load `WINDOW_BYTES` starting at `position`.
    ///
    /// One `read` rather than `read_exact`: a short read near the end of the
    /// volume is expected and is not an error. The caller detects exhaustion by
    /// the buffer coming back short.
    fn refill(&mut self) -> io::Result<()> {
        self.inner.seek(SeekFrom::Start(self.position))?;
        self.buffer.resize(WINDOW_BYTES, 0);
        let mut filled = 0;
        while filled < WINDOW_BYTES {
            match self.inner.read(&mut self.buffer[filled..]) {
                Ok(0) => break,
                Ok(count) => filled += count,
                Err(error) if error.kind() == io::ErrorKind::Interrupted => continue,
                Err(error) => return Err(error),
            }
        }
        self.buffer.truncate(filled);
        self.buffer_start = self.position;
        self.buffer_len = filled;
        Ok(())
    }

    fn total_length(&mut self) -> io::Result<u64> {
        if let Some(length) = self.length {
            return Ok(length);
        }
        let restore = self.position;
        let length = self.inner.seek(SeekFrom::End(0))?;
        self.length = Some(length);
        // The inner stream is now at the end; the next refill seeks anyway, but
        // keep the buffer honest about being unusable.
        self.buffer_len = 0;
        self.position = restore;
        Ok(length)
    }
}

impl<R: Read + Seek> Read for Window<R> {
    fn read(&mut self, out: &mut [u8]) -> io::Result<usize> {
        if out.is_empty() {
            return Ok(0);
        }
        if !self.buffered() {
            // At or past the end there is nothing to refill; report EOF rather
            // than issuing a read that can only return zero.
            if self.length.is_some_and(|length| self.position >= length) {
                return Ok(0);
            }
            self.refill()?;
            if !self.buffered() {
                return Ok(0);
            }
        }
        let offset = (self.position - self.buffer_start) as usize;
        let available = self.buffer_len - offset;
        let count = available.min(out.len());
        out[..count].copy_from_slice(&self.buffer[offset..offset + count]);
        self.position += count as u64;
        Ok(count)
    }
}

impl<R: Read + Seek> Seek for Window<R> {
    fn seek(&mut self, target: SeekFrom) -> io::Result<u64> {
        let position = match target {
            SeekFrom::Start(offset) => offset,
            SeekFrom::End(offset) => {
                let length = self.total_length()? as i64;
                length.saturating_add(offset).max(0) as u64
            }
            SeekFrom::Current(offset) => {
                (self.position as i64).saturating_add(offset).max(0) as u64
            }
        };
        // The whole point: a seek the window already covers is a cursor move.
        // `read_codelet` seeks to exactly the byte after the previous read, so
        // this is the common case and it costs nothing.
        self.position = position;
        Ok(position)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Cursor;

    fn window(bytes: &[u8]) -> Window<Cursor<Vec<u8>>> {
        Window::new(Cursor::new(bytes.to_vec()))
    }

    #[test]
    fn forward_seeks_within_the_window_do_not_reread() {
        let data: Vec<u8> = (0..255).cycle().take(4096).collect();
        let mut source = window(&data);
        // Read 32 bytes at a time, the codelet pattern, across a whole window.
        for offset in (0..4096).step_by(32) {
            source.seek(SeekFrom::Start(offset as u64)).expect("seek");
            let mut codelet = [0u8; 32];
            source.read_exact(&mut codelet).expect("read");
            assert_eq!(codelet, data[offset..offset + 32], "at {offset}");
        }
    }

    #[test]
    fn a_backward_seek_returns_the_earlier_bytes() {
        let data: Vec<u8> = (0..255).cycle().take(4096).collect();
        let mut source = window(&data);
        source.seek(SeekFrom::Start(2048)).expect("seek");
        let mut later = [0u8; 16];
        source.read_exact(&mut later).expect("read later");
        source.seek(SeekFrom::Start(8)).expect("seek back");
        let mut earlier = [0u8; 16];
        source.read_exact(&mut earlier).expect("read earlier");
        assert_eq!(earlier, data[8..24]);
        assert_eq!(later, data[2048..2064]);
    }

    #[test]
    fn a_seek_past_the_window_still_reads_correctly() {
        let data: Vec<u8> = (0..255).cycle().take(3 * WINDOW_BYTES + 512).collect();
        let mut source = window(&data);
        let far = 2 * WINDOW_BYTES + 100;
        source.seek(SeekFrom::Start(far as u64)).expect("seek");
        let mut bytes = [0u8; 64];
        source.read_exact(&mut bytes).expect("read");
        assert_eq!(bytes, data[far..far + 64]);
    }

    #[test]
    fn reading_at_the_end_reports_eof_instead_of_looping() {
        let data = vec![7u8; 100];
        let mut source = window(&data);
        source.seek(SeekFrom::Start(100)).expect("seek to end");
        let mut bytes = [0u8; 8];
        assert_eq!(source.read(&mut bytes).expect("read"), 0);
    }

    #[test]
    fn end_seek_reports_the_length_and_leaves_reads_working() {
        let data: Vec<u8> = (0..255).cycle().take(2000).collect();
        let mut source = window(&data);
        assert_eq!(source.seek(SeekFrom::End(0)).expect("length"), 2000);
        source.seek(SeekFrom::Start(10)).expect("seek");
        let mut bytes = [0u8; 4];
        source.read_exact(&mut bytes).expect("read");
        assert_eq!(bytes, data[10..14]);
    }

    #[test]
    fn a_read_spanning_the_window_edge_is_whole() {
        let data: Vec<u8> = (0..255).cycle().take(WINDOW_BYTES + 256).collect();
        let mut source = window(&data);
        source
            .seek(SeekFrom::Start((WINDOW_BYTES - 16) as u64))
            .expect("seek");
        let mut bytes = [0u8; 64];
        source.read_exact(&mut bytes).expect("read across edge");
        assert_eq!(bytes, data[WINDOW_BYTES - 16..WINDOW_BYTES + 48]);
    }
}
