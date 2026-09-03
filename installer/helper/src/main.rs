#![cfg_attr(windows, windows_subsystem = "windows")]
use fastfileocr_installer_support::{self as data, Error};
use std::{
    collections::BTreeMap,
    fs,
    path::{Path, PathBuf},
};

fn main() {
    let mut args = std::env::args_os().skip(1);
    let operation = args.next().unwrap_or_default();
    let mut options = BTreeMap::new();
    while let Some(key) = args.next() {
        if let Some(value) = args.next() {
            options.insert(key.to_string_lossy().into_owned(), value);
        } else {
            std::process::exit(2);
        }
    }
    let path = |key: &str| options.get(key).map(PathBuf::from).unwrap_or_default();
    let result_file = path("--result");
    if result_file.as_os_str().is_empty() {
        std::process::exit(2);
    }
    let root = path("--root");
    let app = path("--app");
    let flag = |key: &str| options.get(key).is_some_and(|v| v == "1");
    let result = match operation.to_str() {
        Some("resolve") => data::resolve(&root, &app),
        Some("prepare") => data::prepare(&root, &app, flag("--fresh")),
        Some("remove") => data::remove(&root, flag("--data"), flag("--documents")).map(|_| root),
        _ => Err(Error::Unsafe),
    };
    let (status, resolved) = match &result {
        Ok(path) => ("ok", path.as_path()),
        Err(Error::Unsafe) => ("unsafe", Path::new("")),
        Err(Error::Write) => ("write", Path::new("")),
    };
    // NSIS ReadINIStr supports UTF-16, including non-ASCII data-folder names.
    let report = format!(
        "[Result]\r\nStatus={status}\r\nRoot={}\r\n",
        resolved.display()
    );
    let bytes: Vec<u8> = std::iter::once(0xfeffu16)
        .chain(report.encode_utf16())
        .flat_map(u16::to_le_bytes)
        .collect();
    if fs::write(result_file, bytes).is_err() {
        std::process::exit(2);
    }
    std::process::exit(if result.is_ok() { 0 } else { 1 });
}
