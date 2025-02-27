use thiserror::Error;

#[derive(Error, Debug)]
pub enum ZipError {
    #[error("{0}")]
    IoError(#[from] std::io::Error),
    #[error("{0}")]
    ZipError(#[from] zip::result::ZipError),
    #[error("{0}")]
    StripPrefixError(#[from] std::path::StripPrefixError),
    #[error("{0}")]
    AnyhowError(#[from] anyhow::Error),
}