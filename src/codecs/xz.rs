use crate::codecs::Codec;
use crate::utils::ensure_directory_exists;
use crate::{Result, ZipError};
use log::info;
use std::fs::{self, File};
use std::io;
use std::path::Path;
use std::time::Instant;
use tar::{Archive, Builder};
use xz2::read::XzDecoder;
use xz2::write::XzEncoder;

/// XZ codec implementation
pub struct XzCodec {
    compression_level: u32,
    threads: u32,
}

impl XzCodec {
    /// Create a new XZ codec
    pub fn new(level: u32, threads: u32) -> Self {
        Self {
            compression_level: level.clamp(0, 9),
            threads,
        }
    }
}

impl Codec for XzCodec {
    fn extract(&mut self, source: &[&Path], target: &Path) -> Result<()> {
        ensure_directory_exists(target)?;

        let tar_xz = File::open(source[0])?;
        let tar = XzDecoder::new(tar_xz);
        let mut archive = Archive::new(tar);

        let time_start = Instant::now();

        for entry_result in archive.entries()? {
            let mut entry = entry_result?;
            let entry_path = entry.path()?.to_owned().to_str().unwrap().to_string();

            info!("Extracting: {:?}", entry_path);

            if let Err(e) = entry.unpack_in(target) {
                return Err(ZipError::Other(format!("Error extracting {:?}: {}", entry_path, e)));
            }
        }

        info!("Extraction process completed");
        info!(
            "Time costed: {:?} Millis, {:?} Secs",
            time_start.elapsed().as_millis(),
            time_start.elapsed().as_secs()
        );

        Ok(())
    }

    fn compress(&mut self, source: &[&Path], target: &Path) -> Result<()> {
        ensure_directory_exists(target.parent().unwrap_or(Path::new(".")))?;

        let target_file = File::create(target)?;
        info!("Creating target file: {:?}", target);

        // Use simple XzEncoder with specified compression level
        let xz_encoder = XzEncoder::new(target_file, self.compression_level);
        let mut builder = Builder::new(xz_encoder);

        info!("Creating XZ writer with compression level: {}", self.compression_level);
        info!("Using {} threads", self.threads);

        let time_start = Instant::now();

        for source_path in source {
            let name_in_archive = source_path
                .file_name()
                .ok_or_else(|| io::Error::new(io::ErrorKind::InvalidInput, "Invalid source path"))?;

            if source_path.is_dir() {
                info!("Writing directory: {:?}", source_path);
                builder.append_dir_all(name_in_archive, source_path)?;
            } else {
                info!("Writing file: {:?}", source_path);
                builder.append_path_with_name(source_path, name_in_archive)?;
            }
        }

        let finished = builder.into_inner()?;
        finished.finish()?;

        info!("Compression completed");
        info!(
            "Time costed: {:?} Millis, {:?} Secs",
            time_start.elapsed().as_millis(),
            time_start.elapsed().as_secs()
        );

        Ok(())
    }
}