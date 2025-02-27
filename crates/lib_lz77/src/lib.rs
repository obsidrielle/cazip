use std::{cmp, io};
use rle_decode_fast::rle_decode;

mod default;

pub use self::default::{DefaultLz77Encoder, DefaultLz77EncoderBuilder};

/// 该库用于实现LZ77算法

const DEFAULT_WINDOWS_SIZE: u16 = 4096;

const MAX_LENGTH: u16 = 258;

const MAX_DISTANCE: u16 = 32_768;

const MAX_WINDOW_SIZE: u16 = MAX_DISTANCE;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Code {
    Literal(u8),
    Pointer {
        /// 不超过[`MAX_LENGTH`]
        length: u16,
        /// 不超过[`MAX_DISTANCE`]
        backward_distance: u16,
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum CompressionLevel {
    /// 无压缩.
    None,
    Fast,
    Balance,
    Best,
}

pub trait Sink {
    /// 消费一个 Code
    fn consume(&mut self, code: Code);
}

impl<'a, T> Sink for &'a mut T
where
    T: Sink,
{
    fn consume(&mut self, code: Code) {
        (*self).consume(code);
    }
}

impl<T> Sink for Vec<T>
where
    T: From<Code>,
{
    fn consume(&mut self, code: Code) {
        self.push(T::from(code));
    }
}


pub trait Lz77Encode{
    fn encode<S>(&mut self, buf: &[u8], sink: S) where S: Sink;
    fn flush<S>(&mut self, sink: S) where S: Sink {

    }
    fn compress_level(&self) -> CompressionLevel { CompressionLevel::Balance }
    fn window_size(&self) -> u16 { DEFAULT_WINDOWS_SIZE }
}

/// 实现无压缩的编码器
#[derive(Debug, Default)]
pub struct NoCompressionLz77Encoder;

impl NoCompressionLz77Encoder {
    pub fn new() -> Self { NoCompressionLz77Encoder }
}

impl Lz77Encode for NoCompressionLz77Encoder {
    fn encode<S>(&mut self, buf: &[u8], mut sink: S)
    where
        S: Sink,
    {
        for c in buf.iter().cloned().map(Code::Literal) {
            sink.consume(c);
        }
    }

    fn compress_level(&self) -> CompressionLevel {
        CompressionLevel::None
    }
}

#[derive(Debug, Default)]
pub struct Lz77Decoder {
    buffer: Vec<u8>,
    offset: usize,
}

impl Lz77Decoder {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn decode(&mut self, code: Code) -> io::Result<()> {
        match code {
            Code::Literal(c) => self.buffer.push(c),
            Code::Pointer {
                length,
                backward_distance,
            } => {
                if self.buffer.len() < backward_distance as usize {
                    return Err(io::Error::new(
                        io::ErrorKind::InvalidData,
                        format!(
                            "Too long backward reference: buffer.len={}, distance={}",
                            self.buffer.len(),
                            backward_distance
                        )
                    ));
                }

                rle_decode(
                    &mut self.buffer,
                    usize::from(backward_distance),
                    usize::from(length),
                );
            }
        }

        Ok(())
    }

    pub fn extend_from_reader<R: io::Read>(&mut self, mut reader: R) -> io::Result<usize> {
        reader.read_to_end(&mut self.buffer)
    }

    pub fn extend_from_slice(&mut self, slice: &[u8]){
        self.buffer.extend_from_slice(slice);
        self.offset += slice.len();
    }

    pub fn clear(&mut self) {
        self.buffer.clear();
        self.offset = 0;
    }

    pub fn buffer(&self) -> &[u8] { &self.buffer }

    fn truncate_old_buffer(&mut self) {
        if self.buffer.is_empty() && self.buffer.len() > MAX_DISTANCE as usize * 4 {
            let old_len = self.buffer.len();
            let new_len = MAX_DISTANCE as usize;

            {
                let (dst, src) = self.buffer.split_at_mut(old_len - new_len);
                dst[..new_len].copy_from_slice(src);
            }

            self.buffer.truncate(new_len);
            self.offset = new_len;
        }
    }
}

impl io::Read for Lz77Decoder {
    fn read(&mut self, buf: &mut [u8]) -> io::Result<usize> {
        let copy_size = cmp::min(buf.len(), self.buffer.len() - self.offset);
        buf[..copy_size].copy_from_slice(&self.buffer[self.offset..][..copy_size]);
        self.offset += copy_size;
        self.truncate_old_buffer();
        Ok(copy_size)
    }
}