pub mod command_line;
pub mod gzip;
pub mod sevenz;
pub mod xz;
pub mod zip;

use crate::{Result, ZipError};
use serde::{Deserialize, Serialize};
use std::path::Path;
use std::time::Instant;
use log::info;

use self::command_line::CommandLineCodec;
use self::gzip::GzipCodec;
use self::sevenz::SevenZCodec;
use self::xz::XzCodec;
use self::zip::{CompressionMethod, ZipCodec};

/// Compression format types
#[derive(Serialize, Deserialize, Clone, Copy, Debug, PartialEq)]
pub enum Format {
    Zip,
    Gz,
    SevenZ,
    Xz,
}

impl From<&str> for Format {
    fn from(value: &str) -> Self {
        match value {
            "zip" => Self::Zip,
            "gz" => Self::Gz,
            "7z" => Self::SevenZ,
            "xz" => Self::Xz,
            _ => Self::Zip,
        }
    }
}

impl Into<&str> for Format {
    fn into(self) -> &'static str {
        match self {
            Self::Zip => "zip",
            Self::Gz => "gz",
            Self::SevenZ => "7z",
            Self::Xz => "xz",
        }
    }
}

/// Trait for compression/decompression operations
pub trait Codec {
    /// Extract files from an archive
    fn extract(&mut self, source: &[&Path], target: &Path) -> Result<()>;

    fn extract_parts(&mut self, source: &[&Path], target: &Path, parts: &[String]) -> Result<()> {
        Err(ZipError::UnsupportedOperation(
            "Extracting specific parts is not supported for this format".to_string()
        ))
    }
    
    /// Compress files into an archive
    fn compress(&mut self, source: &[&Path], target: &Path) -> Result<()>;
}

/// Factory for creating codec instances
pub struct CodecFactory {
    format: Format,
    method: Option<String>,
    password: Option<String>,
    volume_size: Option<usize>,
    use_external: bool,
}

impl CodecFactory {
    /// Create a new codec factory
    pub fn new(
        format: Format,
        method: Option<&str>,
        password: Option<String>,
        volume_size: Option<usize>,
        use_external: bool,
    ) -> Self {
        Self {
            format,
            method: method.map(String::from),
            password,
            volume_size,
            use_external,
        }
    }

    /// Create appropriate codec based on configuration
    pub fn create_codec(&self) -> Result<Box<dyn Codec>> {
        // If external tools are requested, use command line codec
        if self.use_external {
            return Ok(Box::new(CommandLineCodec::new(
                self.format,
                self.method.as_deref(),
                self.password.clone(),
                self.volume_size,
            )));
        }

        // Create native Rust codec based on format
        match self.format {
            Format::Zip => {
                let method = self.method
                    .as_deref()
                    .map(CompressionMethod::from_str)
                    .unwrap_or_default();

                Ok(Box::new(ZipCodec::new(method, self.password.clone())))
            },
            Format::Gz => Ok(Box::new(GzipCodec::new())),
            Format::SevenZ => Ok(Box::new(SevenZCodec::new(self.password.clone()))),
            Format::Xz => {
                // Use 12 threads by default
                Ok(Box::new(XzCodec::new(0, 12)))
            },
        }
    }
}

/// Helper function to time codec operations
pub fn time_operation<F, T>(operation: &str, f: F) -> Result<T>
where
    F: FnOnce() -> Result<T>,
{
    let start = Instant::now();
    let result = f()?;

    info!(
        "{} completed in {:?} ms / {:?} s",
        operation,
        start.elapsed().as_millis(),
        start.elapsed().as_secs()
    );

    Ok(result)
}