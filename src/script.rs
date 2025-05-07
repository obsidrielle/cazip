use std::path::{Path, PathBuf};
use std::process::Command;
use std::time::Duration;
use log::info;
use crate::{Result};
use crate::utils::{run_command_with_output_handling};

pub struct ScriptEnvironment {
    pub source_files: Vec<PathBuf>,
    pub interpreter: PathBuf,
    pub is_extract: bool,
    pub script_path: PathBuf,
}

pub struct ScriptRunner {
    env: ScriptEnvironment,
}

impl ScriptRunner {
    pub fn new(
        source_files: Vec<PathBuf>,
        interpreter: &Path,
        is_extract: bool,
        script_path: &Path,
    ) -> Result<Self> {
        let temp_dir = tempfile::tempdir()?;

        Ok(Self {
            env: ScriptEnvironment {
                source_files,
                interpreter: interpreter.to_path_buf(),
                is_extract,
                script_path: script_path.to_path_buf(),
            }
        })
    }

    pub fn run(&self) -> Result<()> {
        info!("Running processor script");
        
        let mut command = Command::new(&self.env.interpreter);

        command
            .arg(&self.env.script_path)
            .env("CAZIP_SOURCE_FILES", self.env.source_files.iter()
                .map(|path| path.to_string_lossy().into())
                .collect::<Vec<String>>()
                .join(","))
            .env("CAZIP_IS_EXTRACT", self.env.is_extract.to_string());
        
        run_command_with_output_handling(command, move |_line| {})
    }
}