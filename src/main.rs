use clap::Parser;
use env_logger::Env;
use log::error;
use std::process;
use std::path::{Path, PathBuf};
use thiserror::Error;

pub mod codecs;
pub mod file_tree;
pub mod utils;
mod cli;

/// Result type for zip operations
pub type Result<T> = std::result::Result<T, ZipError>;

/// Error type for zip operations
#[derive(Error, Debug)]
pub enum ZipError {
    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),
    #[error("Zip error: {0}")]
    Zip(#[from] zip::result::ZipError),
    #[error("Path prefix error: {0}")]
    StripPrefix(#[from] std::path::StripPrefixError),
    #[error("7z error: {0}")]
    SevenZ(#[from] sevenz_rust2::Error),
    #[error("XZ error: {0}")]
    Xz(#[from] xz2::stream::Error),
    #[error("Unknown format")]
    UnknownFormat,
    #[error("Unsupported operation: {0}")]
    UnsupportedOperation(String),
    #[error("External command error: {0}")]
    ExternalCommand(String),
    #[error("Other error: {0}")]
    Other(String),
}

fn main() {
    let env = Env::default()
        .filter_or("RUST_LOG", "info")
        .write_style_or("RUST_LOG_STYLE", "always");

    env_logger::init_from_env(env);

    let cli = cli::Cli::parse();

    if let Err(e) = cli.execute() {
        error!("Error: {}", e);
        process::exit(1);
    }
}