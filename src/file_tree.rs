use std::io::{Error, ErrorKind};
use std::path::Path;
use std::process::Command;
use chrono::{DateTime, NaiveDateTime, Utc};
use log::info;
use serde::{Deserialize, Serialize};
use regex::Regex;

#[derive(Serialize, Deserialize, Debug)]
pub struct FileEntry {
    name: String,
    path: String,
    size: u64,
    compressed_size: Option<u64>,
    modified_time: Option<String>,
    is_directory: bool,
    permissions: Option<String>,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct ArchiveContents {
    format: crate::cli::Format,
    total_files: usize,
    total_size: u64,
    files: Vec<FileEntry>,
}

// 列出ZIP文件内容并转为JSON
fn list_zip_contents_json(archive_path: &Path, debug: bool) -> Result<String, Error> {
    let output = Command::new("unzip")
        .arg("-l")
        .arg(archive_path)
        .output()?;

    if debug {
        println!("Command executed: unzip -l {}", archive_path.display());
    }

    if !output.status.success() {
        let error_msg = String::from_utf8_lossy(&output.stderr);
        return Err(Error::new(ErrorKind::Other, format!("Failed to list ZIP contents: {}", error_msg)));
    }

    let content = String::from_utf8_lossy(&output.stdout);

    let mut files = Vec::new();
    let mut total_size: u64 = 0;

    // Length   Date    Time    Name
    // ------   ----    ----    ----
    // 123456  01-02-03 12:34   path/to/file.txt
    let lines: Vec<&str> = content.lines().collect();
    let data_lines = if lines.len() > 4 {
        &lines[3..lines.len()-2]
    } else {
        &[]
    };

    for line in data_lines {
        let line = line.split_whitespace().map(|e| e.trim()).collect::<Vec<&str>>();

        let size = line[0].parse::<u64>().unwrap_or(0);
        let path = Path::new(line[3]);
        let name = path.file_name().unwrap().to_string_lossy().into_owned();
        let date = line[1];
        let time = line[2];

        files.push(FileEntry {
            name,
            path: line[3].to_string(),
            modified_time: Some(format!("{} {}", date, time)),
            size,
            compressed_size: None,
            is_directory: path.is_dir(),
            permissions: None,
        });

        total_size += size;
    }

    let archive_contents = ArchiveContents {
        format: "zip".into(),
        total_files: files.len(),
        total_size,
        files,
    };

    Ok(serde_json::to_string_pretty(&archive_contents)?)
}

fn list_7z_contents_json(archive_path: &Path, debug: bool) -> Result<String, Error> {
    let output = Command::new("7z")
        .arg("l")
        .arg("-slt")
        .arg(archive_path)
        .output()?;

    if debug {
        info!("Command executed: 7z l -slt {}", archive_path.display());
    }

    if !output.status.success() {
        let error_msg = String::from_utf8_lossy(&output.stderr);
        return Err(Error::new(ErrorKind::Other, format!("Failed to list 7Z contents: {}", error_msg)));
    }

    let content = String::from_utf8_lossy(&output.stdout);


    let mut files = Vec::new();
    let mut total_size: u64 = 0;
    let mut current_entry: Option<FileEntry> = None;


    for line in content.lines() {
        if line.trim().is_empty() {
            if let Some(entry) = current_entry.take() {
                total_size += entry.size;
                files.push(entry);
            }
            continue;
        }

        if let Some(pos) = line.find(" = ") {
            let key = line[0..pos].trim();
            let value = line[pos+3..].trim();

            match key {
                "Path" => {
                    if current_entry.is_none() {
                        let path = Path::new(value);
                        current_entry = Some(FileEntry {
                            name: path.file_name().map_or(value.to_string(), |n| n.to_string_lossy().to_string()),
                            path: value.to_string(),
                            size: 0,
                            compressed_size: None,
                            modified_time: None,
                            is_directory: false,
                            permissions: None,
                        });
                    } else if let Some(entry) = &mut current_entry {
                        let path = Path::new(value);
                        entry.name = path.file_name().map_or(value.to_string(), |n| n.to_string_lossy().to_string());
                        entry.path = value.to_string();
                    }
                },
                "Size" => {
                    if let Some(entry) = &mut current_entry {
                        entry.size = value.parse().unwrap_or(0);
                    }
                },
                "Packed Size" => {
                    if let Some(entry) = &mut current_entry {
                        entry.compressed_size = Some(value.parse().unwrap_or(0));
                    }
                },
                "Modified" => {
                    if let Some(entry) = &mut current_entry {
                        if let Ok(dt) = NaiveDateTime::parse_from_str(value, "%Y-%m-%d %H:%M:%S") {
                            entry.modified_time = Some(DateTime::<Utc>::from_naive_utc_and_offset(dt, Utc).to_rfc3339());
                        }
                    }
                },
                "Folder" => {
                    if let Some(entry) = &mut current_entry {
                        entry.is_directory = value == "+" || value.to_lowercase() == "true";
                    }
                },
                "Attributes" => {
                    if let Some(entry) = &mut current_entry {
                        entry.permissions = Some(value.to_string());
                    }
                },
                _ => {}
            }
        }
    }

    if let Some(entry) = current_entry.take() {
        total_size += entry.size;
        files.push(entry);
    }

    let archive_contents = ArchiveContents {
        format: "7z".into(),
        total_files: files.len(),
        total_size,
        files,
    };

    Ok(serde_json::to_string_pretty(&archive_contents)?)
}


fn list_tar_contents_json(archive_path: &Path, format: &str, debug: bool) -> Result<String, Error> {
    let output = Command::new("tar")
        .arg("--list")
        .arg("--verbose")
        .arg("-f")
        .arg(archive_path)
        .output()?;

    if debug {
        println!("Command executed: tar --list --verbose -f {}", archive_path.display());
    }

    if !output.status.success() {
        let error_msg = String::from_utf8_lossy(&output.stderr);
        return Err(Error::new(ErrorKind::Other, format!("Failed to list {} contents: {}", format, error_msg)));
    }

    let content = String::from_utf8_lossy(&output.stdout);

    let mut files = Vec::new();
    let mut total_size: u64 = 0;

    // -rw-r--r-- user/group 123456 2023-01-01 12:34 path/to/file.txt
    // drwxr-xr-x user/group 0      2023-01-01 12:34 path/to/dir/

    for line in content.lines() {
        let parts: Vec<&str> = line.split_whitespace().collect();
        if parts.len() >= 6 {
            let permissions = parts[0];
            let size_str = parts[2];
            let size: u64 = size_str.parse().unwrap_or(0);

            let date = parts[3];
            let time = parts[4];
            let datetime_str = format!("{} {}", date, time);

            let datetime = NaiveDateTime::parse_from_str(&datetime_str, "%Y-%m-%d %H:%M")
                .or_else(|_| NaiveDateTime::parse_from_str(&datetime_str, "%Y-%m-%d %H:%M:%S"))
                .ok()
                .map(|dt| DateTime::<Utc>::from_naive_utc_and_offset(dt, Utc).to_rfc3339());


            let path_str = parts[5..].join(" ");
            let path = Path::new(&path_str);
            let is_directory = permissions.starts_with('d') || path_str.ends_with('/');

            files.push(FileEntry {
                name: path.file_name().map_or(path_str.clone(), |n| n.to_string_lossy().to_string()),
                path: path_str,
                size,
                compressed_size: None,
                modified_time: datetime,
                is_directory,
                permissions: Some(permissions.to_string()),
            });

            total_size += size;
        }
    }

    let archive_contents = ArchiveContents {
        format: format.into(),
        total_files: files.len(),
        total_size,
        files,
    };

    Ok(serde_json::to_string_pretty(&archive_contents)?)
}

pub fn list_archive_contents_json(archive_path: &Path, format: &str, debug: bool) -> Result<String, Error> {
    println!("format: {}", format);
    match format.to_lowercase().as_str() {
        "zip" => list_zip_contents_json(archive_path, debug),
        "7z" => list_7z_contents_json(archive_path, debug),
        "gz" | "tar.gz" | "tgz" => list_tar_contents_json(archive_path, "gz", debug),
        "xz" | "tar.xz" => list_tar_contents_json(archive_path, "xz", debug),
        _ => Err(Error::new(ErrorKind::Other, format!("Unsupported archive format: {}", format))),
    }
}

#[cfg(test)]
mod tests {
    use std::process::Command;

    #[test]
    fn test_zip_contents() {
        let output = Command::new("unzip")
            .arg("-l")
            .arg("/home/cagliostro/archive.zip")
            .output().unwrap();

        if !output.status.success() {
            println!("{:?}", String::from_utf8_lossy(&output.stderr))
        }

        let content = String::from_utf8_lossy(&output.stdout);
        let lines = content.lines().collect::<Vec<_>>();

        println!("{:?}", lines);
    }
}