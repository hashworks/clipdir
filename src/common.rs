use anyhow::anyhow;
use clap::ArgMatches;
use std::fs::{self};
use std::io::{self};
use std::path::PathBuf;
use std::str;

pub fn get_storage_path(arg_matches: &ArgMatches) -> Result<&String, anyhow::Error> {
    arg_matches
        .try_get_one("storage-path")
        .map_err(|e| anyhow!("Failed to get value of argument 'storage-path': {}", e))?
        .ok_or(anyhow!("storage-path is required"))
}

pub fn get_clipboard_entries(dir: &str) -> Result<Vec<PathBuf>, io::Error> {
    let mut entries = fs::read_dir(dir)?
        .map(|res| res.map(|e| e.path()))
        .filter(|res| match res {
            Ok(path) => path.is_file(),
            Err(_) => false,
        })
        .collect::<Result<Vec<_>, io::Error>>()?;

    // Since the order of read_dir is is platform and filesystem dependent we sort the entries by name DESC
    entries.sort_by(|a, b| b.file_name().cmp(&a.file_name()));

    Ok(entries)
}

pub fn get_ext(buffer: &[u8]) -> &str {
    match infer::get(buffer) {
        Some(kind) => kind.extension(),
        None => {
            if str::from_utf8(buffer).is_ok() {
                "txt"
            } else {
                "bin"
            }
        }
    }
}

pub fn get_human_readable_size(size: u64) -> String {
    let units = ["B", "KiB", "MiB", "GiB", "TiB", "PiB", "EiB", "ZiB", "YiB"];
    let mut size = size as f64;
    let mut i = 0;
    while size >= 1024_f64 && i < units.len() {
        size /= 1024_f64;
        i += 1;
    }
    format!("{:.2} {}", size, units[i])
}

// *_dir functions from https://github.com/atuinsh/atuin/blob/v18.4.0/crates/atuin-common/src/utils.rs#L68

#[cfg(not(target_os = "windows"))]
pub fn home_dir() -> PathBuf {
    let home = std::env::var("HOME").expect("$HOME not found");
    PathBuf::from(home)
}

#[cfg(target_os = "windows")]
pub fn home_dir() -> PathBuf {
    let home = std::env::var("USERPROFILE").expect("%userprofile% not found");
    PathBuf::from(home)
}

pub fn data_dir() -> PathBuf {
    let data_dir = std::env::var("XDG_DATA_HOME")
        .map_or_else(|_| home_dir().join(".local").join("share"), PathBuf::from);

    data_dir.join("clipdir")
}
