use crate::codecs::Codec;
use crate::utils::{ensure_directory_exists, ensure_extension};
use crate::Result;
use crate::ZipError;
use log::debug;
use sevenz_rust2::{SevenZArchiveEntry, SevenZWriter};
use std::fs::File;
use std::path::Path;

/// 7-Zip codec implementation
pub struct SevenZCodec {
    password: Option<String>,
}

impl SevenZCodec {
    /// Create a new 7-Zip codec
    pub fn new(password: Option<String>) -> Self {
        Self { password }
    }
}

impl Codec for SevenZCodec {
    fn extract(&mut self, source: &[&Path], target: &Path) -> Result<()> {
        ensure_directory_exists(target)?;

        match &self.password {
            Some(password) => {
                return Err(ZipError::UnsupportedOperation(
                    "Password-protected 7z extraction not implemented in native mode".to_string()
                ));
            }
            None => {
                return Err(ZipError::UnsupportedOperation(
                    "7z extraction not implemented in native mode".to_string()
                ));
            }
        }
    }

    fn compress(&mut self, source: &[&Path], target: &Path, _exclude: Option<&[&Path]>) -> Result<()> {
        let target = ensure_extension(target, "7z");
        ensure_directory_exists(target.parent().unwrap_or(Path::new(".")))?;

        let mut sz_writer = SevenZWriter::create(target.as_path())?;

        for src in source {
            debug!("Writing {:?}", src);

            let name = if src.is_file() {
                src.file_name()
                    .and_then(|n| n.to_str())
                    .unwrap_or("unknown")
                    .to_string()
            } else {
                "".to_string()
            };

            if src.is_file() {
                sz_writer.push_archive_entry(
                    SevenZArchiveEntry::from_path(src, name),
                    Some(File::open(src)?),
                )?;
            } else {
                // TODO: Implement directory handling for 7z
                return Err(ZipError::UnsupportedOperation(
                    "Directory compression with 7z not yet implemented".to_string()
                ));
            }
        }

        sz_writer.finish()?;
        Ok(())
    }
}