use std::path::{Path, PathBuf};
use log::info;
use crate::{Result, ZipError};

pub struct VirtualEnv {
    pub path: PathBuf,
}

impl VirtualEnv {
    pub fn new(path: &Path) -> VirtualEnv {
        Self { path: path.to_path_buf() }
    }

    pub fn get_interpreter_path(&self) -> PathBuf {
        if cfg!(windows) {
            self.path.join("Scripts").join("python.exe")
        } else {
            self.path.join("bin").join("python")
        }
    }
}