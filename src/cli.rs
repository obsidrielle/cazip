use crate::ZipResult;
use anyhow::Context;
use clap::Parser;
use std::ffi::OsString;
use std::{fs, io, path};
use std::fs::File;
use std::io::{Read, Seek, Write};
use std::iter::Zip;
use std::path::{Path, PathBuf};
use std::ptr::read;
use std::time::Instant;
use flate2::{Compression, GzBuilder};
use log::{debug, info};
use sync_file::SyncFile;
use walkdir::{DirEntry, WalkDir};
use zip::write::{FileOptions, SimpleFileOptions};
use zip::{AesMode, CompressionMethod, ZipArchive, ZipWriter};
use rayon::prelude::*;

#[derive(Parser)]
#[command(version, about, long_about = None)]
pub struct Cli {
    /// The name of compressed files without the suffix.
    target: PathBuf,
    /// The name of source files with suffix and archives.
    source: Vec<PathBuf>,
    /// format: zip, gz
    #[arg(short, long)]
    format: Option<Format>,
    /// Compression algorithm: deflate, deflate64, bzip2, zstd
    #[arg(short, long)]
    method: Option<Method>,
    /// password
    #[arg(short, long)]
    password: Option<String>,
    #[arg(short, long)]
    unzip: bool,
    #[arg(short, long)]
    debug: bool,
}

#[derive(Clone, Copy)]
enum Format {
    Zip,
    Gz,
    SevenZ,
}

#[derive(Clone, Copy)]
enum Method {
    Deflated,
    Bzip2,
    Zstd,
}

impl From<String> for Format {
    fn from(value: String) -> Self {
        match value.as_str() {
            "zip" => Self::Zip,
            "Gz" => Self::Gz,
            "7z" => Self::SevenZ,
            _ => Self::Zip,
        }
    }
}

impl Into<CompressionMethod> for Method {
    fn into(self) -> CompressionMethod {
        match self {
            Method::Deflated => CompressionMethod::Deflated,
            Method::Bzip2 => CompressionMethod::Bzip2,
            Method::Zstd => CompressionMethod::Zstd,
        }
    }
}

impl Into<&str> for Method {
    fn into(self) -> &'static str {
        match self {
            Method::Deflated => "deflated",
            Method::Bzip2 => "bzip2",
            Method::Zstd => "zstd",
        }
    }
}

impl Into<&str> for Format {
    fn into<'a>(self) -> &'static str {
        match self {
            Format::Zip => "zip",
            Format::Gz => "gzip",
            Format::SevenZ => "7z",
        }
    }
}

impl From<String> for Method {
    fn from(value: String) -> Self {
        match value.as_str() {
            "deflated" => Method::Deflated,
            "bzip2" => Method::Bzip2,
            "zstd" => Method::Zstd,
            _ => Method::Deflated,
        }
    }
}

trait LosslessCodec {
    fn extract(&mut self, source: Vec<&Path>, target: &Path) -> ZipResult<()>;
    fn compress(&mut self, source: Vec<&Path>, target: &Path) -> ZipResult<()>;
}

struct ZipCodec {
    method: Method,
    password: Option<String>,
}

impl LosslessCodec for ZipCodec {
    fn extract(&mut self, source: Vec<&Path>, target: &Path) -> ZipResult<()> {
        let mut archive = ZipArchive::new(SyncFile::open(source[0])?)?;

        (0..archive.len())
            .into_par_iter()
            .for_each_with(archive, |archive, i| {
                let mut file = archive.by_index(i).unwrap();
                let filepath = file
                    .enclosed_name()
                    .unwrap();

                let outpath = target.join(filepath);

                if file.name().ends_with('/') {
                    fs::create_dir_all(&outpath).unwrap();
                } else {
                    if let Some(p) = outpath.parent() {
                        if !p.exists() {
                            fs::create_dir_all(&p).unwrap();
                        }
                    }

                    let mut outfile = File::create(&outpath).unwrap();
                    io::copy(&mut file, &mut outfile).unwrap();
                }
            });

        Ok(())
    }

    fn compress(&mut self, source: Vec<&Path>, target: &Path) -> ZipResult<()> {
        let prefix = target.parent();

        let start = Instant::now();

        if let Some(p) = prefix {
            if !p.exists() {
                fs::create_dir_all(p)?;
            }
        }

        let method: CompressionMethod = self.method.into();

        let mut options = SimpleFileOptions::default()
            .compression_method(method)
            .unix_permissions(0o755);

        if let Some(password) = &self.password {
            options = options.with_aes_encryption(AesMode::Aes256, password.as_str())
        }

        let mut writer = ZipWriter::new(File::create(&target)?);

        info!("zip writer created");

        for item in source {
            if item.is_file() {
                let mut f = File::open(item)?;
                let filename = item.file_name().unwrap().to_str().unwrap().to_string();
                let size = f.metadata()?.len();

                Self::zip_file(&mut writer, &mut f, filename.to_string(), options, size)?;
            } else {
                let mut dir = WalkDir::new(item)
                    .follow_links(true)
                    .into_iter()
                    .filter_map(|e| e.ok());
                let prefix = item.to_str().unwrap().to_string();

                Self::zip_dir(&mut dir, prefix, &mut writer, options)?;
            }
        }

        writer.finish()?;

        info!("time costed: {:?} ms / {:?} s", start.elapsed().as_millis(), start.elapsed().as_secs());
        Ok(())
    }
}

impl ZipCodec {
    fn zip_file<F: Read + Seek>(
        writer: &mut ZipWriter<File>,
        reader: &mut F,
        filename: String,
        base_options: FileOptions<()>,
        size: u64,
    ) -> ZipResult<()> {
        let mut options = base_options;

        info!("writing file: {}", filename);

        writer.start_file(filename, options)?;

        if size > u32::MAX as u64 {
            options = base_options.large_file(true);

            let mut buffer = [0_u8; 8192];
            let mut written_size = 0;

            while written_size < size {
                written_size += 8192;

                reader.read(&mut buffer)?;
                writer.write_all(&buffer)?;
            }
        } else {
            let mut buffer = vec![];

            reader.read_to_end(&mut buffer)?;
            writer.write_all(&buffer)?;
        }

        Ok(())
    }

    fn zip_dir(
        it: &mut dyn Iterator<Item = DirEntry>,
        prefix: String,
        writer: &mut ZipWriter<File>,
        options: FileOptions<()>,
    ) -> ZipResult<()> {
        for entry in it {
            let path = entry.path();
            let outpath = path.strip_prefix(&prefix)?;
            let path_as_string = outpath.to_str().map(|e| e.to_owned()).unwrap();

            if path.is_file() {
                let mut file = File::open(path)?;
                let size = file.metadata()?.len();

                Self::zip_file(writer, &mut file, path_as_string, options, size)?;
            } else if !outpath.as_os_str().is_empty() {
                info!("writing dir: {:?}", path_as_string);
                writer.add_directory(path_as_string, options)?;
            }
        }

        Ok(())
    }
}


impl Cli {
    /// identify the format unless it has been specified.
    fn identify_format(&mut self) {
        if self.format.is_none() {
            self.format = Some(Format::from(self.target.extension().unwrap().to_string_lossy().to_string()));
        }
    }

    fn set_default(&mut self) {
        if self.method.is_none() {
            self.method = Some(Method::Zstd);
        }
    }

    pub fn finish(mut self) -> ZipResult<()> {
        self.identify_format();
        self.set_default();

        if self.debug {
            let yes = "✓";
            let no = "✗";

            let source_path = self.source.iter()
                .map(|e| path::absolute(e.as_path()).unwrap())
                .collect::<Vec<_>>();

            debug!("source: {:?}", source_path);

            debug!("target: {:?}", path::absolute(self.target.as_path())?);
            debug!("compress: {}", if !self.unzip { yes } else { no });

            if !self.unzip {
                debug!("format: {:?}", <Format as Into<&str>>::into(self.format.unwrap()));
                debug!("method: {:?}", <Method as Into<&str>>::into(self.method.unwrap()));
            }

           if self.password.is_some() {
               debug!("password: {}, encryption: AES256", self.password.as_ref().unwrap());
           }
        }

        let source = self.source.iter().map(|e| e.as_path()).collect::<Vec<_>>();
        let target = self.target.as_path();

        let mut codec: Box<dyn LosslessCodec> = match self.format.unwrap() {
            Format::Zip => {
                Box::new(ZipCodec {
                    password: self.password.clone(),
                    method: self.method.unwrap().clone(),
                })
            }
            Format::Gz => {
                Box::new(GzipCodec{})
            }
            Format::SevenZ => {
                Box::new(SevenZCodec{})
            }
        };

        codec.compress(source, target)?;

        Ok(())
    }
}

struct GzipCodec;

impl LosslessCodec for GzipCodec {
    fn extract(&mut self, source: Vec<&Path>, target: &Path) -> ZipResult<()> {
        todo!()
    }

    fn compress(&mut self, source: Vec<&Path>, target: &Path) -> ZipResult<()> {
        let source = source[0];

        let f = File::create(target)?;
        let mut s = File::open(source)?;

        let mut gz = GzBuilder::new()
            .filename(source.file_name().unwrap().to_str().unwrap())
            .write(f, Compression::default());

        let mut buffer = vec![];
        s.read_to_end(&mut buffer)?;

        gz.write_all(&buffer)?;
        gz.finish()?;

        Ok(())
    }
}

// fn gzip_extract(
//     source: &Path,
//     target: &Path,
// ) -> ZipResult<()> {
//
// }

fn tar_append<T>(
    builder: &mut tar::Builder<T>,
    source: &Path,
) -> ZipResult<()> where T: Write + Seek {
    match source.is_dir() {
        true => builder.append_dir(source, source)?,
        false => builder.append_path(source)?,
    }

    Ok(())
}

fn tar_extract(
    source: &Path,
    target: &Path,
) -> ZipResult<()> {
    let mut archive = tar::Archive::new(File::create(source)?);
    let prefix = PathBuf::from(target);

    for file in archive.entries()? {
        let mut f = file?;
        let path = f.path()?;

        let mut outpath = prefix.clone();
        outpath.push(&path);

        info!("extracting: {}", path.display());

        if path.is_dir() {
            fs::create_dir_all(&outpath)?;
        } else {
            if let Some(p) = outpath.parent() {
                if !p.exists() {
                    fs::create_dir_all(p)?;
                }
            }

            let mut outfile = File::create(&outpath)?;
            io::copy(&mut f, &mut outfile)?;
        }
    }

    Ok(())
}

struct SevenZCodec;

impl LosslessCodec for SevenZCodec {
    fn extract(&mut self, source: Vec<&Path>, target: &Path) -> ZipResult<()> {
        todo!()
    }

    fn compress(&mut self, source: Vec<&Path>, target: &Path) -> ZipResult<()> {
        todo!()
    }
}