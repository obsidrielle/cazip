pub(crate) use crate::{codecs, codecs::Format, Result};
use clap::{Parser, Subcommand};
use log::{debug, info};
use std::path::{Path, PathBuf};
use std::time::Duration;
use crate::script::ScriptRunner;
use crate::venv::VirtualEnv;
use crate::ZipError;

#[derive(Parser)]
#[command(version, about, long_about = None)]
pub struct Cli {
    #[command(subcommand)]
    pub command: Commands,

    /// Enable debug output
    #[arg(short, long)]
    pub debug: bool,
}

#[derive(Subcommand)]
pub enum Commands {
    /// 压缩文件
    #[command(alias = "c")]
    Compress {
        /// 压缩后的文件路径
        target: PathBuf,

        /// 要压缩的源文件路径
        source: Vec<PathBuf>,

        /// 压缩格式: zip, gz, 7z, xz
        #[arg(short, long)]
        format: Option<Format>,

        /// 压缩算法: deflate, bzip2, zstd
        #[arg(short, long)]
        method: Option<String>,

        /// 加密密码
        #[arg(short, long)]
        password: Option<String>,

        /// 使用命令行工具而不是Rust后端
        #[arg(short = 'e', long)]
        use_external: bool,

        /// 分卷大小(MB)，仅适用于zip和7z
        #[arg(short = 'v', long)]
        volume_size: Option<usize>,
    },

    /// 解压文件
    #[command(alias = "e")]
    Extract {
        /// 解压目标目录
        target: PathBuf,

        /// 要解压的源文件
        source: Vec<PathBuf>,

        /// 压缩格式: zip, gz, 7z, xz
        #[arg(short, long)]
        format: Option<Format>,

        /// 加密密码
        #[arg(short, long)]
        password: Option<String>,

        /// 使用命令行工具而不是Rust后端
        #[arg(short = 'e', long)]
        use_external: bool,

        /// 从压缩包中提取指定文件
        #[arg(long, value_delimiter = ',', requires = "use_external")]
        files: Option<Vec<String>>,
    },

    /// 执行脚本处理文件
    #[command(alias = "s")]
    Script {
        /// 源文件路径
        source: Vec<PathBuf>,

        /// 脚本文件路径
        #[arg(long)]
        script_file: PathBuf,

        /// 虚拟环境目录
        #[arg(long)]
        virtual_env_dir: Option<PathBuf>,

        /// 是否解压模式
        #[arg(short, long)]
        unzip: bool,
    },

    /// 列出压缩包内容
    #[command(alias = "l")]
    List {
        /// 压缩包文件路径
        source: PathBuf,

        /// 压缩格式: zip, gz, 7z, xz
        #[arg(short, long)]
        format: Option<Format>,
    },
}

impl Cli {
    /// 从文件扩展名识别格式
    fn identify_format(format_opt: &Option<Format>, path: &Path, is_extract: bool) -> Result<Format> {
        if let Some(format) = format_opt {
            return Ok(*format);
        }

        if let Some(ext) = path.extension() {
            let ext_str = ext.to_string_lossy().to_lowercase();
            Ok(Format::from(ext_str.as_str()))
        } else {
            // 如果没有提供扩展名，默认为ZIP
            Ok(Format::Zip)
        }
    }

    /// 记录调试信息
    fn log_debug_info(
        source: &[PathBuf],
        target: Option<&PathBuf>,
        is_compress: bool,
        format: Option<Format>,
        method: Option<&str>,
        password: Option<&String>
    ) {
        let yes = "✓";
        let no = "✗";

        let source_paths = source.iter()
            .map(|p| std::path::absolute(p).unwrap_or_else(|_| p.to_path_buf()))
            .collect::<Vec<_>>();

        debug!("Source: {:?}", source_paths);

        if let Some(target_path) = target {
            if let Ok(abs_target) = std::path::absolute(target_path) {
                debug!("Target: {:?}", abs_target);
            } else {
                debug!("Target: {:?}", target_path);
            }
        } else {
            debug!("Target: None (list mode)");
        }

        debug!("Compress: {}", if is_compress { yes } else { no });

        if is_compress && format.is_some() {
            debug!("Format: {:?}", format.unwrap());
            debug!("Method: {:?}", method.unwrap_or("default"));
        }

        if let Some(pwd) = password {
            debug!("Password: {}, encryption: AES256", pwd);
        }
    }

    /// 验证输入参数
    fn validate_source_not_empty(source: &[PathBuf]) -> Result<()> {
        if source.is_empty() {
            return Err(ZipError::Other("No source files specified".to_string()));
        }
        Ok(())
    }

    /// 执行压缩操作
    fn execute_compress(
        target: PathBuf,
        source: Vec<PathBuf>,
        format_opt: Option<Format>,
        method: Option<String>,
        password: Option<String>,
        use_external: bool,
        volume_size: Option<usize>,
        debug: bool
    ) -> Result<()> {
        Self::validate_source_not_empty(&source)?;

        let format = Self::identify_format(&format_opt, &target, false)?;

        if debug {
            Self::log_debug_info(
                &source,
                Some(&target),
                true,
                Some(format),
                method.as_deref(),
                password.as_ref()
            );
        }

        // 创建编解码器工厂
        let codec_factory = codecs::CodecFactory::new(
            format,
            method.as_deref(),
            password,
            volume_size,
            use_external,
        );

        // 获取实际编解码器实现
        let mut codec = codec_factory.create_codec()?;

        // 源路径
        let source_paths: Vec<&Path> = source.iter().map(|p| p.as_path()).collect();

        // 执行压缩
        codec.compress(&source_paths, &target, None)
    }

    /// 执行解压操作
    fn execute_extract(
        target: PathBuf,
        source: Vec<PathBuf>,
        format_opt: Option<Format>,
        password: Option<String>,
        use_external: bool,
        files: Option<Vec<String>>,
        debug: bool
    ) -> Result<()> {
        Self::validate_source_not_empty(&source)?;

        let format = Self::identify_format(&format_opt, &source[0], true)?;

        if debug {
            Self::log_debug_info(
                &source,
                Some(&target),
                false,
                Some(format),
                None,
                password.as_ref()
            );
        }

        // 创建编解码器工厂
        let codec_factory = codecs::CodecFactory::new(
            format,
            None,
            password,
            None,
            use_external,
        );

        // 获取实际编解码器实现
        let mut codec = codec_factory.create_codec()?;

        // 源路径
        let source_paths: Vec<&Path> = source.iter().map(|p| p.as_path()).collect();

        // 执行解压
        if let Some(parts) = files {
            codec.extract_parts(&source_paths, &target, &parts)
        } else {
            codec.extract(&source_paths, &target)
        }
    }

    /// 执行脚本操作
    fn execute_script(
        source: Vec<PathBuf>,
        script_file: PathBuf,
        virtual_env_dir: Option<PathBuf>,
        unzip: bool,
    ) -> Result<()> {
        // 设置虚拟环境
        let virtual_env = virtual_env_dir.map(|dir| VirtualEnv::new(dir.as_path()));

        // 创建脚本运行器
        let script = ScriptRunner::new(
            source.clone(),
            virtual_env.as_ref().unwrap().get_interpreter_path().as_path(),
            unzip,
            &script_file,
        )?;

        script.run()
    }

    /// 列出压缩包内容
    fn execute_list(
        source: PathBuf,
        format_opt: Option<Format>,
        debug: bool
    ) -> Result<()> {
        let format = Self::identify_format(&format_opt, &source, true)?;

        if debug {
            Self::log_debug_info(
                &[source.clone()],
                None,
                false,
                Some(format),
                None,
                None
            );
        }

        let contents = crate::file_tree::list_archive_contents_json(
            &source,
            format.into(),
            debug
        )?;

        println!("{}", contents);
        Ok(())
    }

    /// 执行命令
    pub fn execute(self) -> Result<()> {
        match self.command {
            Commands::Compress {
                target,
                source,
                format,
                method,
                password,
                use_external,
                volume_size
            } => {
                Self::execute_compress(
                    target,
                    source,
                    format,
                    method,
                    password,
                    use_external,
                    volume_size,
                    self.debug
                )
            },

            Commands::Extract {
                target,
                source,
                format,
                password,
                use_external,
                files
            } => {
                Self::execute_extract(
                    target,
                    source,
                    format,
                    password,
                    use_external,
                    files,
                    self.debug
                )
            },

            Commands::Script {
                source,
                script_file,
                virtual_env_dir,
                unzip,
            } => {
                Self::execute_script(
                    source,
                    script_file,
                    virtual_env_dir,
                    unzip,
                )
            },

            Commands::List {
                source,
                format
            } => {
                Self::execute_list(
                    source,
                    format,
                    self.debug
                )
            },
        }
    }
}