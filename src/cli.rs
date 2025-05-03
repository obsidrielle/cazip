pub(crate) use crate::{codecs, codecs::Format, Result};
use clap::Parser;
use log::{debug, info};
use std::path::{Path, PathBuf};
use crate::ZipError;

#[derive(Parser)]
#[command(version, about, long_about = None)]
pub struct Cli {
    /// The name of compressed files without the suffix.
    /// Not required when list_filetree is enabled
    #[arg(required_unless_present = "list_filestree")]
    pub target: Option<PathBuf>,

    /// The name of source files with suffix and archives.
    pub source: Vec<PathBuf>,

    /// Format: zip, gz, 7z, xz
    #[arg(short, long)]
    pub format: Option<Format>,

    /// Compression algorithm: deflate, bzip2, zstd
    #[arg(short, long)]
    pub method: Option<String>,

    /// Password for encryption
    #[arg(short, long)]
    pub password: Option<String>,

    /// Extract mode
    #[arg(short, long)]
    pub unzip: bool,

    /// Enable debug output
    #[arg(short, long)]
    pub debug: bool,

    /// Use command line tools instead of Rust backend
    #[arg(short = 'e', long)]
    pub use_external: bool,

    /// Volume size in MB for split archives (only for zip and 7z)
    #[arg(short = 'v', long)]
    pub volume_size: Option<usize>,

    /// List the file tree (doesn't require target)
    #[arg(short, long)]
    pub list_filestree: bool,

    /// Extracts parts of files from archive.
    #[arg(long, value_delimiter = ',')]
    pub files: Option<Vec<String>>,
}

impl Cli {
    /// Identify format from file extension if not specified
    pub fn identify_format(&mut self) -> Result<Format> {
        if self.format.is_some() {
            return Ok(self.format.unwrap());
        }

        let path_to_check = if self.unzip && !self.list_filestree {
            // Use first source file for extraction or listing
            &self.source[0]
        } else {
            // Use target for compression
            self.target.as_ref().unwrap()
        };

        if let Some(ext) = path_to_check.extension() {
            let ext_str = ext.to_string_lossy().to_lowercase();
            Ok(Format::from(ext_str.as_str()))
        } else {
            // Default to ZIP if no extension is provided
            Ok(Format::Zip)
        }
    }

    /// Process command and execute compression or extraction
    pub fn execute(mut self) -> Result<()> {
        // Identify format from extension if not specified
        let format = self.identify_format()?;
        self.format = Some(format);
        
        if self.files.is_some() && (!self.unzip || !self.use_external) {
            return Err(ZipError::Other("File option (-p) must be used with unzip (-u) and use-external (-e)".to_string()));
        }

        if self.list_filestree {
            if self.source.is_empty() {
                // 尝试使用 target 作为源
                if let Some(target) = &self.target {
                    self.source.push(target.clone());
                } else {
                    return Err(crate::ZipError::Other("When using list mode (-l), you must specify at least one file to list".to_string()));
                }
            }
            return self.list_archive_contents();
        }
        
        // Validate inputs
        if self.source.is_empty()  {
            return Err(crate::ZipError::Other("No source files specified".to_string()));
        }

        // For list_filestree operation, we don't need a target
        if !(self.list_filestree && self.unzip) && self.target.is_none() {
            return Err(crate::ZipError::Other("Target path is required unless listing file tree".to_string()));
        }
        
        if self.debug {
            self.log_debug_info();
        }

          // Get target path (safe to unwrap since we validated above)
        let target = self.target.as_ref().unwrap();

        // Create codec factory based on configuration
        let codec_factory = codecs::CodecFactory::new(
            format,
            self.method.as_deref(),
            self.password.clone(),
            self.volume_size,
            self.use_external,
        );

        // Get actual codec implementation
        let mut codec = codec_factory.create_codec()?;

        // Source paths
        let source_paths: Vec<&Path> = self.source.iter().map(|p| p.as_path()).collect();

        println!("{:?}", self.files);

        // Perform operation
        if self.unzip {
            if let Some(parts) = self.files {
                println!("parts");
                codec.extract_parts(&source_paths, target, &parts)
            } else {
                codec.extract(&source_paths, target)
            }
        } else {
            codec.compress(&source_paths, target)
        }
    }

    fn log_debug_info(&self) {
        let yes = "✓";
        let no = "✗";

        let source_paths = self.source.iter()
            .map(|p| std::path::absolute(p).unwrap_or_else(|_| p.to_path_buf()))
            .collect::<Vec<_>>();

        debug!("Source: {:?}", source_paths);

        if let Some(target) = &self.target {
            if let Ok(abs_target) = std::path::absolute(target) {
                debug!("Target: {:?}", abs_target);
            } else {
                debug!("Target: {:?}", target);
            }
        } else {
            debug!("Target: None (list mode)");
        }

        debug!("Compress: {}", if !self.unzip { yes } else { no });

        if !self.unzip && self.format.is_some() {
            debug!("Format: {:?}", self.format.unwrap());
            debug!("Method: {:?}", self.method.as_deref().unwrap_or("default"));
        }

        if let Some(ref password) = self.password {
            debug!("Password: {}, encryption: AES256", password);
        }
    }

    fn list_archive_contents(&self) -> Result<()> {
        let format = self.format.unwrap();

        let contents = crate::file_tree::list_archive_contents_json(
            &self.source[0],
            format.into(),
            self.debug
        )?;

        println!("{}", contents);
        Ok(())
    }
}