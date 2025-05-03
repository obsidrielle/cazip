use std::fs;
use std::fs::File;
use std::io::Read;
use std::path::{Path, PathBuf};
use tar::Archive;
use crate::Result;

/// Ensure a directory exists, creating it if necessary
pub fn ensure_directory_exists(path: &Path) -> Result<()> {
    if !path.exists() {
        fs::create_dir_all(path)?;
    }
    Ok(())
}

/// Ensure a path has the correct extension
pub fn ensure_extension(p: &Path, extension: &str) -> PathBuf {
    if p.extension().map_or(true, |ext| ext != extension) {
        let mut new_name = p.file_name().unwrap_or_default().to_os_string();
        new_name.push(".".to_string() + extension);
        p.with_file_name(new_name)
    } else {
        p.to_path_buf()
    }
}

pub fn is_tar_file(path: &Path) -> bool {
    if !path.exists() {
        return false;
    }
    
    if let Ok(file) = File::open(path) {
        let mut archive = Archive::new(file);
        match archive.entries() {
            Ok(_) => true,
            Err(_) => false,
        } 
    } else {
        false
    }
}