use crate::codecs::{Codec, Format};
use crate::utils::{ensure_directory_exists, is_tar_file};
use crate::{Result, ZipError};
use log::{error, info};
use std::fs;
use std::io::{self, BufRead, BufReader};
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
use std::thread;
use std::time::Instant;

/// External command line tools implementation
pub struct CommandLineCodec {
    format: Format,
    method: Option<String>,
    password: Option<String>,
    volume_size: Option<usize>,
}

impl CommandLineCodec {
    /// Create a new command line codec
    pub fn new(
        format: Format,
        method: Option<&str>,
        password: Option<String>,
        volume_size: Option<usize>,
    ) -> Self {
        Self {
            format,
            method: method.map(String::from),
            password,
            volume_size,
        }
    }

    /// Run a command with logging
    fn run_command_with_logging(mut cmd: Command) -> Result<()> {
        info!("Running command: {:?}", cmd);

        let mut child = cmd
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .spawn()?;

        let stdout = child.stdout.take().unwrap();
        let stderr = child.stderr.take().unwrap();

        let stdout_thread = thread::spawn(move || {
            let reader = BufReader::new(stdout);

            for line in reader.lines() {
                if let Ok(line) = line {
                    info!("{}", line);
                }
            }
        });

        let stderr_thread = thread::spawn(move || {
            let reader = BufReader::new(stderr);

            for line in reader.lines() {
                if let Ok(line) = line {
                    error!("{}", line);
                }
            }
        });

        let status = child.wait()?;

        stdout_thread.join().unwrap();
        stderr_thread.join().unwrap();

        if !status.success() {
            return Err(ZipError::ExternalCommand(format!(
                "Command failed with status: {}",
                status
            )));
        }

        Ok(())
    }
}

impl Codec for CommandLineCodec {
    fn extract(&mut self, source: &[&Path], target: &Path) -> Result<()> {
        let start = Instant::now();

        ensure_directory_exists(target)?;

        match self.format {
            Format::Zip => {
                let mut cmd = Command::new("unzip");

                if let Some(ref pwd) = self.password {
                    cmd.arg("-P").arg(pwd);
                }

                cmd.arg("-o");
                cmd.arg(source[0]);
                cmd.arg("-d").arg(target);

                Self::run_command_with_logging(cmd)?;
            }
            Format::SevenZ => {
                let mut cmd = Command::new("7z");
                cmd.arg("x");

                if let Some(ref pwd) = self.password {
                    cmd.arg(format!("-p{{{}}}", pwd));
                }

                cmd.arg("-mmt12");
                cmd.arg("-y");
                cmd.arg("-bb3");
                cmd.arg(source[0]);
                cmd.arg(format!("-o{}", target.display()));

                Self::run_command_with_logging(cmd)?;
            }
            Format::Xz => {
                let source_path = source[0];
                let filename = source_path.to_string_lossy();

                if filename.ends_with(".tar.xz") || source_path.extension().map_or(false, |ext| ext == "txz") {
                    // For tar.xz files
                    let mut cmd = Command::new("tar");
                    cmd.arg("-xvf");
                    cmd.arg(source_path);
                    cmd.arg("-C").arg(target);

                    Self::run_command_with_logging(cmd)?;
                } else {
                    // For .xz files (single file compression)
                    let mut cmd = Command::new("xz");
                    cmd.arg("-d");
                    cmd.arg("-v");
                    cmd.arg("-k");
                    cmd.arg(source_path);

                    Self::run_command_with_logging(cmd)?;

                    // Get the uncompressed file path
                    let source_stem = source_path.file_stem().unwrap_or_default();
                    let source_dir = source_path.parent().unwrap_or(Path::new("."));
                    let uncompressed = source_dir.join(source_stem);

                    println!("Uncompressed tar file: {}", uncompressed.to_string_lossy());
                    
                    if uncompressed.extension().map_or(false, |ext| ext == "tar")
                        || uncompressed.to_string_lossy().ends_with(".tar") || is_tar_file(uncompressed.as_path())
                    {
                        // If it's a .tar file, extract it
                        let mut cmd = Command::new("tar");
                        cmd.arg("-xvf");
                        cmd.arg(&uncompressed);
                        cmd.arg("-C").arg(target);

                        Self::run_command_with_logging(cmd)?;
                    } else {
                        // Otherwise just copy it
                        let target_file = if target.is_dir() {
                            target.join(source_stem)
                        } else {
                            target.to_path_buf()
                        };

                        info!("Copying uncompressed file to: {:?}", target_file);
                        fs::copy(uncompressed, target_file)?;
                    }
                }
            }
            Format::Gz => {
                return Err(ZipError::UnsupportedOperation(
                    "GZ extraction via command line not implemented".to_string()
                ));
            }
        }

        info!(
            "Extraction completed in {:?} ms / {:?} s",
            start.elapsed().as_millis(),
            start.elapsed().as_secs()
        );

        Ok(())
    }

    fn extract_parts(&mut self, source: &[&Path], target: &Path, parts: &[String]) -> Result<()> {
        let start = Instant::now();

        ensure_directory_exists(target)?;

        match self.format {
            Format::Zip => {
                let mut cmd = Command::new("unzip");

                if let Some(ref pwd) = self.password {
                    cmd.arg("-P").arg(pwd);
                }

                cmd.arg("-o");
                cmd.arg(source[0]);
                cmd.arg("-d").arg(target);

                for part in parts {
                    let mut part_str = part.clone();
                    
                    if !part_str.contains(".") && !part_str.ends_with("*") {
                        part_str.push_str("/*");
                    }
                    
                    cmd.arg(&part_str);
                }
                
                Self::run_command_with_logging(cmd)?;
            }
            Format::SevenZ => {
                let mut cmd = Command::new("7z");
                cmd.arg("x");

                if let Some(ref pwd) = self.password {
                    cmd.arg(format!("-p{{{}}}", pwd));
                }

                cmd.arg("-mmt12");
                cmd.arg("-y");
                cmd.arg("-bb3");
                cmd.arg(source[0]);
                cmd.arg(format!("-o{}", target.display()));

                for part in parts {
                    cmd.arg(part);
                }
                
                Self::run_command_with_logging(cmd)?;
            }
            Format::Xz => {
                let source_path = source[0];
                let filename = source_path.to_string_lossy();

                if filename.ends_with(".tar.xz") || source_path.extension().map_or(false, |ext| ext == "txz") {
                    // For tar.xz files
                    let mut cmd = Command::new("tar");
                    cmd.arg("-xvf");
                    cmd.arg(source_path);
                    cmd.arg("-C").arg(target);

                    for part in parts {
                        cmd.arg(part);
                    }
                    
                    Self::run_command_with_logging(cmd)?;
                } else {
                    // For .xz files (single file compression)
                    let mut cmd = Command::new("xz");
                    cmd.arg("-d");
                    cmd.arg("-v");
                    cmd.arg("-k");
                    cmd.arg(source_path);

                    Self::run_command_with_logging(cmd)?;

                    // Get the uncompressed file path
                    let source_stem = source_path.file_stem().unwrap_or_default();
                    let source_dir = source_path.parent().unwrap_or(Path::new("."));
                    let uncompressed = source_dir.join(source_stem);

                    println!("Uncompressed tar file: {}", uncompressed.to_string_lossy());

                    if uncompressed.extension().map_or(false, |ext| ext == "tar")
                        || uncompressed.to_string_lossy().ends_with(".tar") || is_tar_file(uncompressed.as_path())
                    {
                        // If it's a .tar file, extract it
                        let mut cmd = Command::new("tar");
                        cmd.arg("-xvf");
                        cmd.arg(&uncompressed);
                        cmd.arg("-C").arg(target);

                        for part in parts {
                            cmd.arg(part);
                        }
                        
                        Self::run_command_with_logging(cmd)?;
                    } else {
                        // Otherwise just copy it
                        let target_file = if target.is_dir() {
                            target.join(source_stem)
                        } else {
                            target.to_path_buf()
                        };

                        info!("Copying uncompressed file to: {:?}", target_file);
                        fs::copy(uncompressed, target_file)?;
                    }
                }
            }
            Format::Gz => {
                return Err(ZipError::UnsupportedOperation(
                    "GZ extraction via command line not implemented".to_string()
                ));
            }
        }

        info!(
            "Extraction completed in {:?} ms / {:?} s",
            start.elapsed().as_millis(),
            start.elapsed().as_secs()
        );

        Ok(())
    }
    
    fn compress(&mut self, source: &[&Path], target: &Path, exclude: Option<&[&Path]>) -> Result<()> {
        let start = Instant::now();

        if let Some(parent) = target.parent() {
            if !parent.exists() {
                fs::create_dir_all(parent)?;
            }
        }

        match self.format {
            Format::Zip => {
                let mut cmd = Command::new("zip");
                cmd.arg("-r");
                cmd.arg("-v");
                
                // Set compression level if using deflate
                if self.method.as_deref() == Some("deflated") {
                    cmd.arg("-9");
                }

                if let Some(ref pwd) = self.password {
                    cmd.arg("-e");
                    cmd.arg("-P").arg(pwd);
                }

                if let Some(size_mb) = self.volume_size {
                    cmd.arg("-s").arg(format!("{}m", size_mb));
                }

                if let Some(exclude_paths) = exclude {
                    for path in exclude_paths {
                        if let Some(file_name) = path.file_name() {
                            cmd.arg(format!("-x={}", file_name.to_string_lossy()));
                        }
                    }
                }
                
                cmd.arg(target);
                
                // Find the parent directory to use as base
                if !source.is_empty() {
                    let first_path = source[0];
                    if let Some(parent_dir) = first_path.parent() {
                        println!("{:?}", parent_dir);
                        if format!("{:?}", parent_dir) != "\"\"" {
                            cmd.current_dir(parent_dir);
                            // cmd.arg("-cd").arg(parent_dir);

                            // Add all files relative to the parent directory
                            for path in source {
                                if let Some(file_name) = path.file_name() {
                                    cmd.arg(file_name);
                                }
                            }
                        } else {
                            for path in source {
                                cmd.arg(path);
                            }
                        }
                    } else {
                        // Just add all files directly
                        for path in source {
                            cmd.arg(path);
                        }
                    }
                }

                Self::run_command_with_logging(cmd)?;
            }
            Format::SevenZ => {
                let mut cmd = Command::new("7z");
                cmd.arg("a");
                cmd.arg("-mmt12");
                cmd.arg("-bb3");

                if let Some(ref pwd) = self.password {
                    cmd.arg(format!("-p{{{}}}", pwd));
                }

                if let Some(size_mb) = self.volume_size {
                    cmd.arg(format!("-v{}m", size_mb));
                }

                if let Some(exclude_paths) = exclude {
                    for path in exclude_paths {
                        cmd.arg(format!("-xr!{}", path.to_string_lossy()));
                    }
                }
                
                cmd.arg(target);

                for path in source {
                    cmd.arg(path);
                }

                Self::run_command_with_logging(cmd)?;
            }
            Format::Xz => {
                if source.len() > 1 || source[0].is_dir() {
                    // Create a tar archive first
                    let tar_path = target.with_extension("tar");

                    let mut tar_cmd = Command::new("tar");
                    tar_cmd.arg("-cvf");
                    tar_cmd.arg(&tar_path);

                    if let Some(exclude_paths) = exclude {
                        for path in exclude_paths {
                            tar_cmd.arg("--exclude");
                            tar_cmd.arg(path);
                        }
                    }
                    
                    for path in source {
                        tar_cmd.arg(path);
                    }

                    
                    Self::run_command_with_logging(tar_cmd)?;

                    // Compress the tar file with xz
                    let mut xz_cmd = Command::new("xz");
                    xz_cmd.arg("-f");
                    xz_cmd.arg("-v");
                    xz_cmd.arg("-T").arg("12");
                    xz_cmd.arg(&tar_path);

                    Self::run_command_with_logging(xz_cmd)?;

                    // Rename if needed
                    let xz_path = tar_path.with_extension("tar.xz");
                    if xz_path != target {
                        info!("Renaming {:?} to {:?}", xz_path, target);
                        fs::rename(xz_path, target)?;
                    }
                } else {
                    // Compress a single file
                    let mut cmd = Command::new("xz");
                    cmd.arg("-k");
                    cmd.arg("-f");
                    cmd.arg("-v");
                    cmd.arg("-T").arg("12");
                    cmd.arg(source[0]);

                    Self::run_command_with_logging(cmd)?;

                    // Move the compressed file to the target path
                    let compressed = source[0].with_extension("xz");
                    if compressed != target {
                        info!("Renaming {:?} to {:?}", compressed, target);
                        fs::rename(compressed, target)?;
                    }
                }
            }
            Format::Gz => {
                return Err(ZipError::UnsupportedOperation(
                    "GZ compression via command line not implemented".to_string()
                ));
            }
        }

        info!(
            "Compression completed in {:?} ms / {:?} s",
            start.elapsed().as_millis(),
            start.elapsed().as_secs()
        );

        Ok(())
    }
}