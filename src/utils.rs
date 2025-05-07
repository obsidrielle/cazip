use std::{fs, thread};
use std::fs::File;
use std::io::{BufRead, BufReader, Read};
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
use crossbeam::channel::{unbounded, Sender};
use log::{error, info};
use tar::Archive;
use crate::{Result, ZipError};

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

pub enum OutputLine {
    Stdout(String),
    Stderr(String),
}

pub fn run_command_with_output_handling<F>(mut cmd: Command, mut handler: F) -> Result<()>
where F: FnMut(String) + Send + 'static {
    info!("Running command: {:?}", cmd);

    let mut child = cmd
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()?;

    let stdout = child.stdout.take().unwrap();
    let stderr = child.stderr.take().unwrap();

    let (tx, rx) = unbounded();

    let tx_stdout = tx.clone();

    let stdout_thread = thread::spawn(move || {
        let reader = BufReader::new(stdout);

        for line in reader.lines() {
            if let Ok(line) = line {
                tx_stdout.send(OutputLine::Stdout(line)).ok();
            }
        }
    });

    let stderr_thread = thread::spawn(move || {
        let reader = BufReader::new(stderr);

        for line in reader.lines() {
            if let Ok(line) = line {
                tx.send(OutputLine::Stderr(line)).ok();
            }
        }
    });

    let wait_thread = thread::spawn(move || {
        let status = child.wait().expect("Failed to wait on child");
        status.success()
    });

    for output in rx {
        match output {
            OutputLine::Stdout(line) => {
                info!("{}", line);
                handler(line);
            }
            OutputLine::Stderr(line) => {
                error!("{}", line);
                handler(line);
            }
        }

        // 检查命令是否已经完成
        if wait_thread.is_finished() {
            break;
        }
    }

    stdout_thread.join().unwrap();
    stderr_thread.join().unwrap();

    // let success = wait_thread.join().unwrap();
    // if success {
    //     return Err(ZipError::ExternalCommand(
    //         "Command failed".to_string()
    //     ));
    // }

    Ok(())
}