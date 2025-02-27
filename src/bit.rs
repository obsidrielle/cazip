use std::io;

#[derive(Debug)]
pub struct BitWriter<W> {
    inner: W,
    buf: u32,
    end: u8,
}

impl<W> BitWriter<W>
where
    W: io::Write,
{
    pub fn new(inner: W) -> Self {
        BitWriter {
            inner,
            buf: 0,
            end: 0,
        }
    }

    #[inline(always)]
    pub fn write_bit(&mut self, bit: bool) -> io::Result<()> {
        self.write_bits(1, bit as u16)
    }

    #[inline(always)]
    pub fn write_bits(&mut self, bitwidth: u8, bits: u16) -> io::Result<()> {
        debug_assert!(bitwidth < 16);
        debug_assert!(self.end + bitwidth <= 32);
        self.buf |= u32::from(bits) << self.end;
        self.end += bitwidth;
        self.flush_if_needed()
    }

    pub fn flush(&mut self) -> io::Result<()> {
        while self.end > 0 {
            self.inner.write_all(&[self.buf as u8])?;
            self.buf >>= 8;
            self.end = self.end.saturating_sub(8);
        }
        self.inner.flush()?;
        Ok(())
    }

    #[inline(always)]
    fn flush_if_needed(&mut self) -> io::Result<()> {
        if self.end >= 16 {
            self.inner.write_all(&(self.buf as u16).to_le_bytes())?;
            self.end -= 16;
            self.buf >>= 16;
        }
        Ok(())
    }
}

impl<W> BitWriter<W> {
    pub fn as_inner_ref(&self) -> &W {
        &self.inner
    }
    pub fn as_inner_mut(&mut self) -> &mut W {
        &mut self.inner
    }
    pub fn into_inner(self) -> W {
        self.inner
    }
}

#[derive(Debug)]
pub struct BitReader<R> {
    inner: R,
    last_read: u32,
    offset: u8,
    last_error: Option<io::Error>,
}

impl<R> BitReader<R>
where
    R: io::Read,
{
    pub fn new(inner: R) -> Self {
        BitReader {
            inner,
            last_read: 0,
            offset: 32,
            last_error: None,
        }
    }

    #[inline(always)]
    pub fn set_last_error(&mut self, e: io::Error) {
        self.last_error = Some(e);
    }

    #[inline(always)]
    pub fn check_last_error(&mut self) -> io::Result<()> {
        if let Some(e) = self.last_error.take() {
            Err(e)
        } else {
            Ok(())
        }
    }

    #[inline(always)]
    pub fn read_bit(&mut self) -> io::Result<bool> {
        self.read_bits(1).map(|b| b != 0)
    }

    #[inline(always)]
    pub fn read_bits(&mut self, bitwidth: u8) -> io::Result<u16> {
        let v = self.read_bits_unchecked(bitwidth);
        self.check_last_error().map(|_| v)
    }

    #[inline(always)]
    pub fn read_bits_unchecked(&mut self, bitwidth: u8) -> u16 {
        let bits = self.peek_bits_unchecked(bitwidth);
        self.skip_bits(bitwidth);
        bits
    }

    #[inline(always)]
    pub fn peek_bits_unchecked(&mut self, bitwidth: u8) -> u16 {
        debug_assert!(bitwidth <= 16);

        while 32 < self.offset + bitwidth {
            if self.last_error.is_some() {
                return 0;
            }
            if let Err(e) = self.fill_next_u8() {
                self.last_error = Some(e);
                return 0;
            }
        }

        debug_assert!(self.offset < 32 || bitwidth == 0);
        let bits = self.last_read.wrapping_shr(u32::from(self.offset)) as u16;
        bits & ((1 << bitwidth) - 1)
    }

    #[inline(always)]
    pub fn skip_bits(&mut self, bitwidth: u8) {
        debug_assert!(self.last_error.is_some() || 32 - self.offset >= bitwidth);
        self.offset += bitwidth;
    }

    #[inline(always)]
    fn fill_next_u8(&mut self) -> io::Result<()> {
        self.offset -= 8;
        self.last_read >>= 8;

        let mut buf = [0; 1];
        self.inner.read_exact(&mut buf)?;
        let next = u32::from(buf[0]);
        self.last_read |= next << (32 - 8);
        Ok(())
    }

    #[inline]
    pub(crate) fn state(&self) -> BitReaderState {
        BitReaderState {
            last_read: self.last_read,
            offset: self.offset,
        }
    }

    #[inline]
    pub(crate) fn restore_state(&mut self, state: BitReaderState) {
        self.last_read = state.last_read;
        self.offset = state.offset;
    }
}
impl<R> BitReader<R> {
    pub fn reset(&mut self) {
        self.offset = 32;
    }
    pub fn as_inner_ref(&self) -> &R {
        &self.inner
    }
    pub fn as_inner_mut(&mut self) -> &mut R {
        &mut self.inner
    }
    pub fn into_inner(self) -> R {
        self.inner
    }
}

#[derive(Debug, Clone, Copy)]
pub(crate) struct BitReaderState {
    last_read: u32,
    offset: u8,
}