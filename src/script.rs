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

        let venv_dir = self.env.interpreter.parent().unwrap();
        let activate_path = venv_dir.join("activate");

        // 使用shell执行命令，先激活环境再运行脚本
        let mut command = Command::new("sh");
        command
            .arg("-c")
            .arg(format!(
                "source {} && python \"{}\"",
                activate_path.to_string_lossy(),
                self.env.script_path.to_string_lossy()
            ))
            .env("CAZIP_SOURCE_FILES", self.env.source_files.iter()
                .map(|path| path.to_string_lossy().into())
                .collect::<Vec<String>>()
                .join(","))
            .env("CAZIP_IS_EXTRACT", self.env.is_extract.to_string());
        
        run_command_with_output_handling(command, move |_line| {})
    }
}