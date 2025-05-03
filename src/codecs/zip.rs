use crate::codecs::Codec;
use crate::utils::ensure_directory_exists;
use crate::Result;
use log::{info};
use rayon::prelude::*;
use std::fs::{self, File};
use std::io::{self, Read, Seek, Write};
use std::path::{Path};
use std::time::Instant;
use sync_file::SyncFile;
use walkdir::{DirEntry, WalkDir};
use zip::write::{FileOptions, SimpleFileOptions};
use zip::{AesMode, ZipArchive, ZipWriter};
use zip::read::ZipFile;

/// Zip compression methods
#[derive(Clone, Copy, Debug)]
pub enum CompressionMethod {
    Deflated,
    Bzip2,
    Zstd,
}

impl Default for CompressionMethod {
    fn default() -> Self {
        Self::Zstd
    }
}

impl CompressionMethod {
    /// Create from string
    pub fn from_str(s: &str) -> Self {
        match s.to_lowercase().as_str() {
            "deflated" => Self::Deflated,
            "bzip2" => Self::Bzip2,
            "zstd" => Self::Zstd,
            _ => Self::default(),
        }
    }

    /// Convert to string
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Deflated => "deflated",
            Self::Bzip2 => "bzip2",
            Self::Zstd => "zstd",
        }
    }

    /// Convert to zip library compression method
    pub fn to_zip_method(&self) -> zip::CompressionMethod {
        match self {
            Self::Deflated => zip::CompressionMethod::Deflated,
            Self::Bzip2 => zip::CompressionMethod::Bzip2,
            Self::Zstd => zip::CompressionMethod::Zstd,
        }
    }
}

/// ZIP format implementation
pub struct ZipCodec {
    method: CompressionMethod,
    password: Option<String>,
}

impl ZipCodec {
    /// Create a new ZIP codec
    pub fn new(method: CompressionMethod, password: Option<String>) -> Self {
        Self { method, password }
    }

    /// Add a file to the zip archive
    fn zip_file<F: Read + Seek>(
        writer: &mut ZipWriter<File>,
        reader: &mut F,
        filename: String,
        base_options: FileOptions<()>,
        size: u64,
    ) -> Result<()> {
        info!("Writing file: {}", filename);

        let options = if size > u32::MAX as u64 {
            base_options.large_file(true)
        } else {
            base_options
        };

        writer.start_file(filename, options)?;

        if size > u32::MAX as u64 {
            let mut buffer = [0_u8; 8192];
            let mut written_size = 0;

            while written_size < size {
                let read_size = std::cmp::min(buffer.len() as u64, size - written_size);
                let bytes_read = reader.read(&mut buffer[0..read_size as usize])?;

                if bytes_read == 0 {
                    break;
                }

                writer.write_all(&buffer[0..bytes_read])?;
                written_size += bytes_read as u64;
            }
        } else {
            let mut buffer = Vec::with_capacity(size as usize);
            reader.read_to_end(&mut buffer)?;
            writer.write_all(&buffer)?;
        }

        Ok(())
    }

    /// Add a directory to the zip archive
    fn zip_dir(
        it: &mut dyn Iterator<Item = DirEntry>,
        prefix: String,
        writer: &mut ZipWriter<File>,
        options: FileOptions<()>,
    ) -> Result<()> {
        for entry in it {
            let path = entry.path();
            let outpath = path.strip_prefix(&prefix)?;
            let path_as_string = outpath.to_str().map(|e| e.to_owned()).unwrap_or_default();

            if path.is_file() {
                let mut file = File::open(path)?;
                let size = file.metadata()?.len();

                Self::zip_file(writer, &mut file, path_as_string, options, size)?;
            } else if !outpath.as_os_str().is_empty() {
                info!("Writing dir: {}", path_as_string);
                writer.add_directory(path_as_string, options)?;
            }
        }

        Ok(())
    }
}

impl Codec for ZipCodec {
    fn extract(&mut self, source: &[&Path], target: &Path) -> Result<()> {
        ensure_directory_exists(target)?;

        let start = Instant::now();
        
        let archive = ZipArchive::new(SyncFile::open(source[0])?)?;
        
        let total_files = archive.len();
        info!("Archive contains {} files", total_files);

        // Process files in parallel
        (0..archive.len())
            .into_par_iter()
            .try_for_each_with(archive, |archive, i| {
                let mut file;
                
                if let Some(ref password) = self.password {
                    file = archive.by_index_decrypt(i, password.as_bytes())?;
                } else {
                    file = archive.by_index(i)?;
                }
                
                let file_name = file.name().to_string();
                let filepath = match file.enclosed_name() {
                    Some(path) => path,
                    None => return Ok(()),
                };

                let outpath = target.join(filepath);

                if file.name().ends_with('/') {
                    info!("Creating directory: {}", file_name);
                    fs::create_dir_all(&outpath)?;
                } else {
                    info!("Extracting file: {}", file_name);
                    
                    if let Some(p) = outpath.parent() {
                        if !p.exists() {
                            fs::create_dir_all(p)?;
                        }
                    }

                    let mut outfile = File::create(&outpath)?;
                    let bytes_copied = io::copy(&mut file, &mut outfile)?;
                    info!("File extracted: {} ({} bytes)", file_name, bytes_copied);
                }

                Ok(()) as Result<()>
            })?;

        let elapsed = start.elapsed();
        info!(
            "Extraction completed successfully in {:?} ms / {:?} s",
            elapsed.as_millis(),
            elapsed.as_secs()
        );
        Ok(())
    }

    fn compress(&mut self, source: &[&Path], target: &Path) -> Result<()> {
        let start = Instant::now();

        // Ensure target directory exists
        if let Some(p) = target.parent() {
            if !p.exists() {
                fs::create_dir_all(p)?;
            }
        }

        // Setup compression options
        let zip_method = self.method.to_zip_method();

        let mut options = SimpleFileOptions::default()
            .compression_method(zip_method)
            .unix_permissions(0o755);

        if let Some(password) = &self.password {
            options = options.with_aes_encryption(AesMode::Aes256, password);
        }

        // Create zip writer
        let mut writer = ZipWriter::new(File::create(target)?);
        info!("Zip writer created");

        // Process each source path
        for item in source {
            if item.is_file() {
                let mut f = File::open(item)?;
                let filename = item
                    .file_name()
                    .and_then(|n| n.to_str())
                    .unwrap_or("unknown")
                    .to_string();
                let size = f.metadata()?.len();

                Self::zip_file(&mut writer, &mut f, filename, options, size)?;
            } else {
                let mut dir = WalkDir::new(item)
                    .follow_links(true)
                    .into_iter()
                    .filter_map(|e| e.ok());

                let prefix = item.to_str().unwrap_or("").to_string();

                Self::zip_dir(&mut dir, prefix, &mut writer, options)?;
            }
        }

        writer.finish()?;

        info!(
            "Compression completed in {:?} ms / {:?} s",
            start.elapsed().as_millis(),
            start.elapsed().as_secs()
        );

        Ok(())
    }
}
