use std::{cmp, io};
use std::io::Write;
use crate::bit;
use crate::deflate::BlockType;

/// 默认的block大小
pub const DEFAULT_BLOCK_SIZE: usize = 1024 * 1024;

const MAX_NON_COMPRESSED_BLOCK_SIZE: usize = 0xFFFF;

pub struct EncodeOptions<E = lib_lz77::DefaultLz77Encoder> {
    block_size: usize,
    dynamic_huffman: bool,
    lz77: Option<E>,
}

impl Default for EncodeOptions<lib_lz77::DefaultLz77Encoder> {
    fn default() -> Self {
        Self::new()
    }
}

impl EncodeOptions<lib_lz77::DefaultLz77Encoder> {
    pub fn new() -> Self {
        EncodeOptions {
            block_size: DEFAULT_BLOCK_SIZE,
            dynamic_huffman: true,
            lz77: Some(lib_lz77::DefaultLz77Encoder::new()),
        }
    }
}

impl<E> EncodeOptions<E> where E: lib_lz77::Lz77Encode {
    pub fn with_lz77(lz77: E) -> Self {
        EncodeOptions {
            block_size: DEFAULT_BLOCK_SIZE,
            dynamic_huffman: true,
            lz77: Some(lz77),
        }
    }

    pub fn no_compression(mut self) -> Self {
        self.lz77 = None;
        self
    }
}

struct Block<E> {
    block_type: BlockType,
    block_size: usize,
    block_buf: BlockBuf<E>,
}

#[derive(Debug)]
enum BlockBuf<E> {
    Raw(RawBuf),
}

#[derive(Debug)]
struct RawBuf {
    buf: Vec<u8>,
}

impl RawBuf {
    fn new() -> Self {
        RawBuf { buf: Vec::new() }
    }

    fn append(&mut self, data: &[u8]) {
        self.buf.extend_from_slice(data);
    }

    fn len(&self) -> usize {
        self.buf.len()
    }

    fn flush<W>(&mut self, writer: &mut bit::BitWriter<W>) -> io::Result<()> where W: io::Write {
        let size = cmp::min(self.buf.len(), MAX_NON_COMPRESSED_BLOCK_SIZE);

        writer.flush()?;
        writer.as_inner_mut().write_all(&(size as u16).to_le_bytes())?;
        writer.as_inner_mut().write_all(&(!size as u16).to_le_bytes())?;
        self.buf.drain(0..size);

        Ok(())
    }
}