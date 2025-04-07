use crate::ZipResult;
use anyhow::Context;
use clap::Parser;
use std::ffi::{OsStr, OsString};
use std::{fs, io, path, thread};
use std::fs::File;
use std::io::{BufRead, BufReader, Read, Seek, Write};
use std::iter::Zip;
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
use std::ptr::read;
use std::time::Instant;
use flate2::{bufread, Compression, GzBuilder};
use log::{debug, error, info};
use sync_file::SyncFile;
use walkdir::{DirEntry, WalkDir};
use zip::write::{FileOptions, SimpleFileOptions};
use zip::{AesMode, CompressionMethod, ZipArchive, ZipWriter};
use rayon::prelude::*;
use sevenz_rust2;
use sevenz_rust2::{SevenZArchiveEntry, SevenZWriter};
use tar::{Archive, Builder};
use xz2::read::XzDecoder;
use xz2::stream::{Action, Check, MtStreamBuilder, Stream};
use xz2::write::XzEncoder;

#[derive(Parser)]
#[command(version, about, long_about = None)]
pub struct Cli {
    /// The name of compressed files without the suffix.
    target: PathBuf,
    /// The name of source files with suffix and archives.
    source: Vec<PathBuf>,
    /// format: zip, gz, 7z, xz
    #[arg(short, long)]
    format: Option<Format>,
    /// Compression algorithm: deflate, deflate64, bzip2, zstd
    #[arg(short, long)]
    method: Option<Method>,
    /// password
    #[arg(short, long)]
    password: Option<String>,
    /// Extract mode
    #[arg(short, long)]
    unzip: bool,
    /// Enable debug output (extra messages)
    #[arg(short, long)]
    debug: bool,
    /// Use command line tools instead of Rust backend
    #[arg(short = 'e', long)]
    use_external: bool,
    /// Volume size in MB for split archives (only for zip and 7z)
    #[arg(short = 'v', long)]
    volume_size: Option<usize>,
}

#[derive(Clone, Copy)]
enum Format {
    Zip,
    Gz,
    SevenZ,
    Xz,
}

impl Default for Format {
    fn default() -> Self {
        Self::Zip
    }
}

#[derive(Clone, Copy)]
enum Method {
    Deflated,
    Bzip2,
    Zstd,
}

impl Default for Method {
    fn default() -> Self {
        Self::Zstd
    }
}

impl From<String> for Format {
    fn from(value: String) -> Self {
        match value.as_str() {
            "zip" => Self::Zip,
            "Gz" => Self::Gz,
            "7z" => Self::SevenZ,
            "xz" => Self::Xz,
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
            Format::Xz => "xz",
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
    fn identify_format(&mut self) {
        if self.format.is_none() && !self.unzip {
            self.format = Some(Format::from(self.target.extension().unwrap().to_string_lossy().to_string()));
        }
        if self.format.is_none() && self.unzip {
            self.format = Some(Format::from(self.source[0].extension().unwrap().to_string_lossy().to_string()));
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
        let format = self.format.unwrap();

        let mut codec: Box<dyn LosslessCodec> = match format.clone() {
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
                Box::new(SevenZCodec{
                    password: self.password.clone(),
                })
            }
            Format::Xz => {
                Box::new(XZCodec {
                    compression_level: 0,
                    threads: 12,
                })
            }
        };

        if self.use_external {
            codec = Box::new(CommandLineCodec::new(
                format,
                self.method,
                self.password.clone(),
                self.volume_size,
            ))
        }

        if !self.unzip { codec.compress(source, target)?; } else { codec.extract(source, target)?; }
        Ok(())
    }
}

struct GzipCodec;

impl LosslessCodec for GzipCodec {
    fn extract(&mut self, source: Vec<&Path>, target: &Path) -> ZipResult<()> {
        let reader = BufReader::new(File::open(source[0])?);
        let mut outfile = File::create(&target)?;
        let mut decoder = bufread::GzDecoder::new(reader);

        io::copy(&mut decoder, &mut outfile)?;

        Ok(())
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

struct SevenZCodec {
    password: Option<String>,
}

impl LosslessCodec for SevenZCodec {
    fn extract(&mut self, source: Vec<&Path>, target: &Path) -> ZipResult<()> {
        match self.password {
            Some(ref password) => {
                todo!()
            }
            None => {
                todo!()
            }

        }

        Ok(())
    }

    fn compress(&mut self, source: Vec<&Path>, target: &Path) -> ZipResult<()> {
        let target = ensure_extension(target, "7z");
        let mut sz_writer = SevenZWriter::create(target.as_path())?;

        debug!("writing {:?}", source[0]);

        let src = source[0];
        let name = "sample".to_string();

        sz_writer.push_archive_entry(
            SevenZArchiveEntry::from_path(src, name),
            Some(File::open(src)?),
        )?;

        let src = source[2];
        let name = "sample/1.txt".to_string();

        sz_writer.push_archive_entry(
            SevenZArchiveEntry::from_path(src, name),
            Some(File::open(src)?),
        )?;

        let src = source[1];
        let name = "1.txt".to_string();

        sz_writer.push_archive_entry(
            SevenZArchiveEntry::from_path(src, name),
            Some(File::open(src)?),
        )?;

        sz_writer.finish()?;
        Ok(())
    }
}

struct XZCodec {
    compression_level: u32,
    threads: u32,
}

impl XZCodec {
    pub fn new(level: u32, threads: u32) -> Self {
        Self {
            compression_level: level.clamp(0, 9),
            threads,
        }
    }
}

impl LosslessCodec for XZCodec {
    fn extract(&mut self, source: Vec<&Path>, target: &Path) -> ZipResult<()> {
        let tar_xz = SyncFile::open(source[0])?;
        let tar = XzDecoder::new(tar_xz);
        let mut archive = Archive::new(tar);

        let time_start = Instant::now();

        for entry_result in archive.entries()? {
            let mut entry = entry_result?;
            let entry_path = entry.path()?.into_owned();

            info!("Extracting: {:?}", entry_path);

            if let Err(e) = entry.unpack_in(target) {
                eprintln!("Error extracting {:?}: {}", entry_path, e);
            }
        }

        info!("Extraction process completed");
        info!("Time costed: {:?} Millis, {:?} Secs", time_start.elapsed().as_millis(), time_start.elapsed().as_secs());
        Ok(())
    }

    fn compress(&mut self, source: Vec<&Path>, target: &Path) -> ZipResult<()> {
        let target_file = File::create(target)?;
        info!("Creating target file: {:?}", target);

        let mut mt_stream = MtStreamBuilder::new();
        let _ = mt_stream
            .threads(12)
            .block_size(4 * 1024 * 1024)
            .preset(self.compression_level);
        let memusage = mt_stream.memusage();
        info!("approximate mem usage: {:?} MB / {:?} GB", memusage as f64 / 1024.0 / 1024.0,
            memusage as f64 / 1024.0 / 1024.0 / 1024.0);
        info!("Using 12 threads");

        let mt_stream = mt_stream.encoder()?;
        info!("Compression level: {:?}", self.compression_level);

        // let xz_encoder = XzEncoder::new_stream(target_file, mt_stream);
        let xz_encoder = XzEncoder::new(target_file, 0);
        let mut builder = Builder::new(xz_encoder);
        info!("Creating XZ writer");

        let time_start = Instant::now();

        for source_path in source {
            let name_in_archive = source_path.file_name().unwrap();

            if source_path.is_dir() {
                info!("Writing directory: {:?}", source_path);
                builder.append_dir_all(name_in_archive, source_path).unwrap()
            } else {
                info!("Writing file: {:?}", source_path);
                builder.append_path_with_name(source_path, name_in_archive).unwrap()
            }
        }

        let finished = builder.into_inner()?;
        finished.finish()?;

        info!("Compress completed");
        info!("Time costed: {:?} Millis, {:?} Secs", time_start.elapsed().as_millis(), time_start.elapsed().as_secs());
        Ok(())
    }
}

#[derive(Default)]
struct CommandLineCodec {
    format: Format,
    method: Option<Method>,
    password: Option<String>,
    // Just for Zip and 7z
    volume_size: Option<usize>,
}

impl CommandLineCodec {
    fn new(format: Format, method: Option<Method>, password: Option<String>, volume_size: Option<usize>) -> Self {
        Self {
            format,
            method,
            password,
            volume_size,
        }
    }

    fn build_path_args<'a>(paths: &'a [&'a Path]) -> Vec<&'a str> {
        paths.iter().map(|p| p.to_str().unwrap()).collect()
    }

    fn parse_file_operations<P: AsRef<Path>>(paths: &[P]) -> Vec<String> {
        let mut operations = Vec::new();

        for path in paths {
            let path_ref = path.as_ref();
            if path_ref.is_dir() {
                operations.push(format!("Writing directory: {:?}", path_ref));

                if let Ok(entries) = fs::read_dir(path_ref) {
                    for entry in entries.filter_map(Result::ok) {
                        let entry_path = entry.path();
                        let subpath_operations = Self::parse_file_operations(&[entry_path]);
                        operations.extend(subpath_operations);
                    }
                }
            } else {
                operations.push(format!("Writing file: {:?}", path_ref));
            }
        }

        operations
    }

    fn run_command_with_logging(mut cmd: Command) -> ZipResult<()> {
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
                info!("{}", line.unwrap())
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
            error!("Command failed with status: {}", status);
            return Err(io::Error::new(
                io::ErrorKind::Other,
                format!("Command failed with status: {}", status)
            ).into());
        }

        Ok(())
    }
}

impl LosslessCodec for CommandLineCodec {
    fn extract(&mut self, source: Vec<&Path>, target: &Path) -> ZipResult<()> {
        let start = Instant::now();

        if !target.exists() {
            fs::create_dir_all(target)?;
        }

        match self.format {
            Format::Zip => {
                let mut cmd = Command::new("unzip");

                if let Some(ref pwd) = self.password {
                    cmd.arg("-P").arg(pwd);
                }

                cmd.arg("-v");
                cmd.arg("-o");
                cmd.arg(source[0]);
                cmd.arg("-d").arg(target);

                Self::run_command_with_logging(cmd)?;
            }
            Format::SevenZ => {
                let mut cmd = Command::new("7z");
                cmd.arg("x");

                if let Some(ref pwd) = self.password {
                    cmd.arg("-p").arg(pwd);
                }

                cmd.arg("-mmt12");
                cmd.arg("-y");
                cmd.arg("-bb3");
                cmd.arg(source[0]);
                cmd.arg(format!("-o{:?}", target));

                Self::run_command_with_logging(cmd)?;
            }
            Format::Xz => {
                if source[0].extension().map_or(false, |ext| ext == "tar.xz" || ext == "txz")
                    || source[0].to_string_lossy().ends_with(".tar.xz") {
                    // For tar.xz, we can extract it directly
                    let mut cmd = Command::new("tar");
                    cmd.arg("-xvf");
                    cmd.arg(source[0]);
                    cmd.arg("-C").arg(target);

                    Self::run_command_with_logging(cmd)?;
                } else {
                    // For no tar.xz, we should extract it, then check if it is tar
                    let mut cmd = Command::new("xz");
                    cmd.arg("-d");
                    cmd.arg("-v");
                    cmd.arg("-k");
                    cmd.arg(source[0]);

                    Self::run_command_with_logging(cmd)?;

                    let source_stem = source[0].file_stem().unwrap();
                    let source_dir = source[0].parent().unwrap();
                    let uncompressed = source_dir.join(source_stem);

                    if uncompressed.extension().map_or(false, |ext| ext == "tar")
                        || uncompressed.to_string_lossy().ends_with(".tar") {
                        let mut cmd = Command::new("tar");
                        cmd.arg("-xvf");
                        cmd.arg(&uncompressed);
                        cmd.arg("-C").arg(target);

                        Self::run_command_with_logging(cmd)?
                    } else {
                        // if no, just copy it
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
                todo!()
            }
        }

        info!("Extraction completed in {:?} ms / {:?} s",
            start.elapsed().as_millis(), start.elapsed().as_secs());
        Ok(())
    }

    fn compress(&mut self, source: Vec<&Path>, target: &Path) -> ZipResult<()> {
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

                if let Some(Method::Deflated) = self.method {
                    cmd.arg("-9");
                }

                if let Some(ref pwd) = self.password {
                    cmd.arg("-e");
                    cmd.arg("-P").arg(pwd);
                }

                if let Some(size_mb) = self.volume_size {
                    cmd.arg("-s").arg(format!("{}m", size_mb));
                }

                cmd.arg(target);
                for path in source.iter() {
                    cmd.arg(path);
                }

                Self::run_command_with_logging(cmd)?;
            }
            Format::SevenZ => {
                let mut cmd = Command::new("7z");
                cmd.arg("a");
                cmd.arg("-mmt12");
                cmd.arg("-bb3");

                if let Some(ref pwd) = self.password {
                    cmd.arg("-p").arg(pwd);
                }

                if let Some(size_mb) = self.volume_size {
                    cmd.arg(format!("-v{}m", size_mb));
                }

                cmd.arg(target);
                for path in source {
                    cmd.arg(path);
                }
                Self::run_command_with_logging(cmd)?;
            }
            Format::Xz => {
                if source.len() > 1 || source[0].is_dir() {
                    let tar_path = target.with_extension("tar");

                    let mut tar_cmd = Command::new("tar");
                    tar_cmd.arg("-cvf");
                    tar_cmd.arg(&tar_path);

                    for path in source {
                        tar_cmd.arg(path);
                    }

                    Self::run_command_with_logging(tar_cmd)?;

                    let mut xz_cmd = Command::new("xz");
                    xz_cmd.arg("-f");
                    xz_cmd.arg("-v");
                    xz_cmd.arg("-T").arg("12");
                    xz_cmd.arg(&tar_path);

                    Self::run_command_with_logging(xz_cmd)?;

                    let xz_path = tar_path.with_extension("tar.xz");
                    if xz_path != target {
                        info!("Renaming {:?} to {:?}", xz_path, target);
                        fs::rename(xz_path, target)?;
                    }
                } else {
                    let mut cmd = Command::new("xz");
                    cmd.arg("-k");
                    cmd.arg("-f");
                    cmd.arg("-v");
                    cmd.arg("-T").arg("12");
                    cmd.arg(source[0]);

                    let compressed = source[0].with_extension("xz");
                    if compressed != target {
                        info!("Copy {:?} to {:?}", compressed, target);
                        fs::rename(compressed, target)?;
                    }
                }
            }
            Format::Gz => {
                todo!()
            }
        }
        Ok(())
    }
}
fn ensure_extension(p: &Path, extension: &str) -> PathBuf {
    if p.extension().map(|ext| ext != extension).unwrap_or(true) {
        let mut new_name = p.file_name().unwrap_or_default().to_os_string();
        new_name.push(".".to_string() + extension);
        p.with_file_name(new_name)
    } else {
        p.to_path_buf()
    }
}

