use crate::{Code, Lz77Encode, Sink, MAX_LENGTH, MAX_WINDOW_SIZE};
use std::cmp;
use std::collections::HashMap;

#[derive(Debug)]
pub struct DefaultLz77Encoder {
    window_size: u16,
    max_length: u16,
    buf: Vec<u8>,
}

impl DefaultLz77Encoder {
    pub fn new() -> Self {
        DefaultLz77EncoderBuilder::new().build()
    }

    pub fn with_window_size(window_size: u16) -> Self {
        DefaultLz77EncoderBuilder::new()
            .window_size(cmp::min(window_size, MAX_WINDOW_SIZE))
            .build()
    }
}

impl Default for DefaultLz77Encoder {
    fn default() -> Self {
        Self::new()
    }
}

impl Lz77Encode for DefaultLz77Encoder {
    fn encode<S>(&mut self, buf: &[u8], sink: S)
    where
        S: Sink,
    {
        self.buf.extend_from_slice(buf);

        if self.buf.len() >= self.window_size as usize * 8 {
            self.flush(sink);
        }
    }

    fn flush<S>(&mut self, mut sink: S)
    where
        S: Sink,
    {
        let mut prefix_table = PrefixTable::new(self.buf.len());
        let mut i = 0;
        let end = cmp::max(3, self.buf.len()) - 3;
        while i < end {
            let key = prefix(&self.buf[i..]);
            let matched = prefix_table.insert(key, i as u32);
            if let Some(j) = matched.map(|j| j as usize) {
                let distance = i - j;
                if distance <= self.window_size as usize {
                    let length = 3 + longest_common_prefix(
                        &self.buf,
                        i + 3,
                        j + 3,
                        self.max_length as usize,
                    );
                    sink.consume(Code::Pointer {
                        length,
                        backward_distance: distance as u16,
                    });
                    for k in (i..).take(length as usize).skip(1) {
                        if k >= end {
                            break;
                        }
                        prefix_table.insert(prefix(&self.buf[k..]), k as u32);
                    }
                    i += length as usize;
                    continue;
                }
            }
            sink.consume(Code::Literal(self.buf[i]));
            i += 1;
        }
        for b in &self.buf[i..] {
            sink.consume(Code::Literal(*b));
        }
        self.buf.clear();
    }

    fn window_size(&self) -> u16 {
        self.window_size
    }
}

#[inline]
fn prefix(input_buf: &[u8]) -> [u8; 3] {
    let buf: &[u8] = &input_buf[..3];
    [buf[0], buf[1], buf[2]]
}

#[inline]
fn longest_common_prefix(buf: &[u8], i: usize, j: usize, max: usize) -> u16 {
    buf[i..]
        .iter()
        .take(max - 3)
        .zip(&buf[j..])
        .take_while(|&(x, y)| x == y)
        .count() as u16
}

#[derive(Debug)]
enum PrefixTable {
    Small(HashMap<[u8; 3], u32>),
    Large(LargePrefixTable),
}

impl PrefixTable {
    fn new(bytes: usize) -> Self {
        if bytes < super::MAX_WINDOW_SIZE as usize {
            PrefixTable::Small(HashMap::new())
        } else {
            PrefixTable::Large(LargePrefixTable::new())
        }
    }

    #[inline]
    fn insert(&mut self, prefix: [u8; 3], position: u32) -> Option<u32> {
        match *self {
            PrefixTable::Small(ref mut x) => x.insert(prefix, position),
            PrefixTable::Large(ref mut x) => x.insert(prefix, position),
        }
    }
}

#[derive(Debug)]
struct LargePrefixTable {
    table: Vec<Vec<(u8, u32)>>,
}

impl LargePrefixTable {
    fn new() -> Self {
        LargePrefixTable {
            table: (0..=0xFFFF).map(|_| Vec::new()).collect(),
        }
    }

    #[inline]
    fn insert(&mut self, prefix: [u8; 3], position: u32) -> Option<u32> {
        let p0 = prefix[0] as usize;
        let p1 = prefix[1] as usize;
        let p2 = prefix[2];

        let i = (p0 << 8) + p1;
        let positions = &mut self.table[i];
        for &mut (key, ref mut value) in positions.iter_mut() {
            if key == p2 {
                let old = *value;
                *value = position;
                return Some(old);
            }
        }
        positions.push((p2, position));
        None
    }
}

#[derive(Debug)]
pub struct DefaultLz77EncoderBuilder {
    window_size: u16,
    max_length: u16,
}

impl DefaultLz77EncoderBuilder {
    pub fn new() -> Self {
        Self {
            window_size: MAX_WINDOW_SIZE,
            max_length: MAX_LENGTH,
        }
    }

    pub fn window_size(mut self, window_size: u16) -> Self {
        self.window_size = window_size;
        self
    }

    pub fn max_length(mut self, max_length: u16) -> Self {
        self.max_length = max_length;
        self
    }

    pub fn build(self) -> DefaultLz77Encoder {
        DefaultLz77Encoder {
            window_size: self.window_size,
            max_length: self.max_length,
            buf: vec![],
        }
    }
}

impl Default for DefaultLz77EncoderBuilder {
    fn default() -> Self {
        Self::new()
    }
}