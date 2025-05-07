use crate::codecs::Codec;
use crate::utils::ensure_directory_exists;
use crate::Result;
use flate2::{bufread, Compression, GzBuilder};
use std::fs::File;
use std::io::{self, BufReader, Read, Write};
use std::path::Path;

/// GZip codec implementation
pub struct GzipCodec;

impl GzipCodec {
    /// Create a new GZip codec
    pub fn new() -> Self {
        Self
    }
}

impl Codec for GzipCodec {
    fn extract(&mut self, source: &[&Path], target: &Path) -> Result<()> {
        ensure_directory_exists(target.parent().unwrap_or(Path::new(".")))?;

        let reader = BufReader::new(File::open(source[0])?);
        let mut outfile = File::create(target)?;
        let mut decoder = bufread::GzDecoder::new(reader);

        io::copy(&mut decoder, &mut outfile)?;

        Ok(())
    }

    fn compress(&mut self, source: &[&Path], target: &Path, _exclude: Option<&[&Path]>) -> Result<()> {
        ensure_directory_exists(target.parent().unwrap_or(Path::new(".")))?;

        // GZip only compresses a single file
        let source_file = source[0];

        let f = File::create(target)?;
        let mut s = File::open(source_file)?;

        let filename = source_file
            .file_name()
            .and_then(|n| n.to_str())
            .unwrap_or("unknown");

        let mut gz = GzBuilder::new()
            .filename(filename)
            .write(f, Compression::default());

        let mut buffer = Vec::new();
        s.read_to_end(&mut buffer)?;

        gz.write_all(&buffer)?;
        gz.finish()?;

        Ok(())
    }
}
