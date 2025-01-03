use anyhow::anyhow;
use chrono::Utc;
use clap::{crate_description, crate_name, crate_version, Arg, ArgMatches, Command};
use common::extract_required_arg_value;
use std::ffi::OsStr;
use std::fs::File;
use std::fs::{self};
use std::io::{self, BufReader, Read, Write};
use std::path::PathBuf;
use std::{env, str};

mod common;

// See man wl-clipboard
#[derive(Clone)]
enum ClipboardState {
    Nil,
    Sensitive,
    Clear,
    Data,
    Unknown,
}

impl ClipboardState {
    fn from_str(v: &str) -> Result<ClipboardState, String> {
        match v.to_lowercase().as_str() {
            "nil" => Ok(ClipboardState::Nil),
            "sensitive" => Ok(ClipboardState::Sensitive),
            "clear" => Ok(ClipboardState::Clear),
            "data" => Ok(ClipboardState::Data),
            _ => Ok(ClipboardState::Unknown),
        }
    }
}

fn main() -> anyhow::Result<()> {
    let matches = Command::new(crate_name!())
        .about(crate_description!())
        .version(crate_version!())
        .subcommand_required(true)
        .arg_required_else_help(true)
        .arg(
            Arg::new("storage-path")
                .long("storage-path")
                .global(true)
                .env("CLIPDIR_STORAGE_PATH")
                .value_parser(clap::value_parser!(PathBuf))
                .default_value(
                    common::data_dir(crate_name!())
                        .to_string_lossy()
                        .to_string(),
                ),
        )
        .subcommand(
            Command::new("store")
                .short_flag('s')
                .long_flag("store")
                .about("Store a clipboard entry by stdin")
                .arg(
                    Arg::new("state")
                        .long("state")
                        .env("CLIPBOARD_STATE")
                        .value_parser(ClipboardState::from_str)
                        .default_value("data"),
                )
                .arg(
                    Arg::new("byte-limit")
                        .long("byte-limit")
                        .env("CLIPDIR_BYTE_LIMIT")
                        .value_parser(clap::value_parser!(usize))
                        .default_value("5242880"),
                )
                .arg(
                    Arg::new("dedupe-search-limit")
                        .long("dedupe-search-limit")
                        .env("CLIPDIR_DEDUPE_SEARCH_LIMIT")
                        .value_parser(clap::value_parser!(usize))
                        .default_value("1000"),
                ),
        )
        .subcommand(
            Command::new("list")
                .short_flag('l')
                .long_flag("list")
                .arg(
                    Arg::new("preview-length")
                        .long("preview-length")
                        .env("CLIPDIR_PREVIEW_LENGTH")
                        .value_parser(clap::value_parser!(usize))
                        .default_value("100"),
                )
                .about("List clipboard entries prefixed with their id"),
        )
        .subcommand(
            Command::new("decode")
                .short_flag('d')
                .long_flag("decode")
                .about("Output a clipboard entry by dmenu stdin"),
        )
        .get_matches();

    match matches.subcommand() {
        Some(("store", arg_matches)) => {
            match extract_required_arg_value(arg_matches, "clipboard-state")? {
                ClipboardState::Nil | ClipboardState::Sensitive => Ok(()),
                ClipboardState::Clear => delete_latest(arg_matches),
                ClipboardState::Data | ClipboardState::Unknown => store(arg_matches),
            }
        }
        Some(("list", arg_matches)) => list(arg_matches),
        Some(("decode", arg_matches)) => decode(arg_matches),

        _ => unreachable!(),
    }
}

fn store(arg_matches: &ArgMatches) -> anyhow::Result<()> {
    let storage_path: &PathBuf = common::extract_required_arg_value(arg_matches, "storage-path")?;
    let size_limit: &usize = common::extract_required_arg_value(arg_matches, "byte-limit")?;
    let dedupe_search_limit: &usize =
        common::extract_required_arg_value(arg_matches, "dedupe-search-limit")?;

    fs::create_dir_all(storage_path)
        .map_err(|e| anyhow!("Failed to create clipboard directory: {}", e))?;

    let mut buffer = Vec::new();
    io::stdin()
        .read_to_end(&mut buffer)
        .map_err(|e| anyhow!("Failed to read new entry from stdin: {}", e))?;

    if buffer.trim_ascii().is_empty() {
        return Ok(());
    }

    if buffer.len() > *size_limit {
        return Err(anyhow!(
            "Clipboard entry size exceeds limit of {} bytes",
            size_limit
        ));
    }

    let ext = common::get_ext(&buffer);

    let filepath = storage_path.join(format!("{}.{}", Utc::now().timestamp_micros(), ext));

    let mut file =
        File::create(&filepath).map_err(|e| anyhow!("Failed to create clipboard file: {}", e))?;
    file.write_all(&buffer)
        .map_err(|e| anyhow!("Failed to write to clipboard file: {}", e))?;

    deduplicate_latest(storage_path, &buffer, dedupe_search_limit)?;

    Ok(())
}

fn deduplicate_latest(storage_path: &PathBuf, buffer: &[u8], limit: &usize) -> anyhow::Result<()> {
    let paths = common::get_clipboard_entries(storage_path)
        .map_err(|e| anyhow!("Failed to read clipboard directory: {}", e))?;

    for path in paths.iter().skip(1).take(*limit) {
        let file = File::open(path).map_err(|e| anyhow!("Failed to open clipboard file: {}", e))?;

        let mut reader = BufReader::new(file);
        let mut other_buffer = Vec::new();
        reader
            .read_to_end(&mut other_buffer)
            .map_err(|e| anyhow!("Failed to read clipboard file: {}", e))?;

        if buffer == other_buffer.as_slice() {
            fs::remove_file(path)
                .map_err(|e| anyhow!("Failed to delete duplicate clipboard file: {}", e))?;
        }
    }

    Ok(())
}

fn delete_latest(arg_matches: &ArgMatches) -> anyhow::Result<()> {
    let storage_path: &PathBuf = common::extract_required_arg_value(arg_matches, "storage-path")?;

    let paths = common::get_clipboard_entries(storage_path)
        .map_err(|e| anyhow!("Failed to read clipboard directory: {}", e))?;

    if let Some(path) = paths.first() {
        fs::remove_file(path).map_err(|e| anyhow!("Failed to delete clipboard file: {}", e))?;
    }

    Ok(())
}

fn list(arg_matches: &ArgMatches) -> anyhow::Result<()> {
    let storage_path: &PathBuf = common::extract_required_arg_value(arg_matches, "storage-path")?;
    let preview_length: &usize = common::extract_required_arg_value(arg_matches, "preview-length")?;

    let paths = common::get_clipboard_entries(storage_path)
        .map_err(|e| anyhow!("Failed to read clipboard directory: {}", e))?;

    for (i, path) in paths.iter().enumerate() {
        let ext = path
            .extension()
            .unwrap_or(OsStr::new("bin"))
            .to_string_lossy();

        let preview = if ext == "txt" {
            let file =
                File::open(path).map_err(|e| anyhow!("Failed to open clipboard file: {}", e))?;

            let mut reader = BufReader::new(file);
            let mut buffer = vec![0; *preview_length];

            let n = reader
                .read(&mut buffer[..])
                .map_err(|e| anyhow!("Failed to read clipboard file: {}", e))?;
            let buffer = &buffer[..n];

            let preview = if let Ok(utf8_str) = str::from_utf8(buffer) {
                utf8_str.to_string()
            } else {
                buffer.iter().map(|&b| b as char).collect()
            };

            preview
                .trim_ascii()
                .replace(if cfg!(windows) { "\r\n" } else { "\n" }, " ")
                .to_string()
        } else {
            let metadata = fs::metadata(path)
                .map_err(|e| anyhow!("Failed to read clipboard file metadata: {}", e))?;

            format!(
                "[[ binary data {} {} ]]",
                common::get_human_readable_size(metadata.len()),
                ext
            )
        };

        println!("{}\t{}", i, preview);
    }

    Ok(())
}

fn decode(arg_matches: &ArgMatches) -> anyhow::Result<()> {
    let storage_path: &PathBuf = common::extract_required_arg_value(arg_matches, "storage-path")?;

    // stdin from dmenu will be prefixed with the id, so we only take the numeric part
    let id = io::stdin()
        .lock()
        .bytes()
        .filter_map(|b| b.ok().map(|b| b as char))
        .take_while(|c| c.is_numeric())
        .collect::<String>()
        .parse::<usize>()
        .map_err(|e| anyhow!("Failed to parse id: {}", e))?;

    let paths = common::get_clipboard_entries(storage_path)
        .map_err(|e| anyhow!("Failed to read clipboard directory: {}", e))?;

    let path = paths
        .get(id)
        .ok_or_else(|| anyhow!("No clipboard entry with id {}", id))?;

    let file = File::open(path).map_err(|e| anyhow!("Failed to open clipboard file: {}", e))?;

    let mut reader = BufReader::new(file);

    io::copy(&mut reader, &mut io::stdout())
        .map_err(|e| anyhow!("Failed to write clipboard file to stdout: {}", e))?;

    Ok(())
}
