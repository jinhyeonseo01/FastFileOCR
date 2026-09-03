//! Publish verified runtime archives as one complete, versioned directory.
use crate::{
    download::{Downloads, ModelFile, Progress},
    i18n,
    store::{err, Result},
};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::{
    collections::BTreeMap,
    fs,
    io::{Read, Write},
    path::Path,
};
const RECEIPT: &str = ".fastfileocr-runtime.json";
const OWNER: &str = "FastFileOCR runtime v1";
#[derive(Deserialize, Serialize)]
struct FileRecord {
    name: String,
    bytes: u64,
    sha256: String,
}
#[derive(Deserialize, Serialize)]
struct Receipt {
    owner: String,
    archives: Vec<String>,
    files: Vec<FileRecord>,
}
fn plain_file(name: &str) -> bool {
    !name.is_empty() && !name.contains(['/', '\\', ':']) && name != "." && name != ".."
}
fn regular(path: &Path) -> bool {
    let Ok(meta) = path.symlink_metadata() else {
        return false;
    };
    #[cfg(windows)]
    {
        use std::os::windows::fs::MetadataExt;
        if meta.file_attributes() & 0x400 != 0 {
            return false;
        }
    }
    meta.is_file() && !meta.file_type().is_symlink()
}
fn digest(path: &Path, checkpoint: &impl Fn() -> Result<()>) -> Result<String> {
    let mut file = fs::File::open(path).map_err(err)?;
    let mut hash = Sha256::new();
    let mut buffer = vec![0; 1024 * 1024];
    loop {
        checkpoint()?;
        let n = file.read(&mut buffer).map_err(err)?;
        if n == 0 {
            break;
        }
        hash.update(&buffer[..n]);
    }
    Ok(format!("{:x}", hash.finalize()))
}
fn required(device: &str) -> Vec<String> {
    [
        "llama-server.exe",
        "llama.dll",
        "mtmd.dll",
        "ggml.dll",
        "ggml-base.dll",
        "vcruntime140.dll",
    ]
    .into_iter()
    .map(str::to_string)
    .chain([format!("ggml-{device}.dll")])
    .collect()
}
pub(super) fn valid(
    root: &Path,
    device: &str,
    archives: &[ModelFile],
    checkpoint: &impl Fn() -> Result<()>,
) -> Result<bool> {
    if !regular(&root.join(RECEIPT)) {
        return Ok(false);
    }
    let Ok(receipt) =
        serde_json::from_slice::<Receipt>(&fs::read(root.join(RECEIPT)).map_err(err)?)
    else {
        return Ok(false);
    };
    if receipt.owner != OWNER
        || receipt.archives
            != archives
                .iter()
                .map(|f| f.sha256.clone())
                .collect::<Vec<_>>()
        || required(device)
            .iter()
            .any(|name| !receipt.files.iter().any(|f| &f.name == name))
    {
        return Ok(false);
    }
    for record in receipt.files {
        if !plain_file(&record.name)
            || !regular(&root.join(&record.name))
            || fs::metadata(root.join(&record.name)).map_err(err)?.len() != record.bytes
            || digest(&root.join(&record.name), checkpoint)? != record.sha256
        {
            return Ok(false);
        }
    }
    Ok(true)
}
fn unpack(path: &Path, output: &Path, checkpoint: &impl Fn() -> Result<()>) -> Result<()> {
    let mut archive = zip::ZipArchive::new(fs::File::open(path).map_err(err)?).map_err(err)?;
    let mut total = 0u64;
    for index in 0..archive.len() {
        checkpoint()?;
        let mut entry = archive.by_index(index).map_err(err)?;
        let enclosed = entry
            .enclosed_name()
            .ok_or_else(|| i18n::text("runtimeUnsafeArchive"))?;
        if entry.name().contains(['\\', ':'])
            || entry.unix_mode().is_some_and(|m| m & 0o170000 == 0o120000)
        {
            return Err(i18n::text("runtimeUnsafeArchive"));
        }
        if entry.is_dir() {
            continue;
        }
        let name = enclosed
            .file_name()
            .and_then(|n| n.to_str())
            .ok_or_else(|| i18n::text("runtimeUnsafeArchive"))?
            .to_owned();
        if !plain_file(&name) {
            return Err(i18n::text("runtimeUnsafeArchive"));
        }
        if name != "llama-server.exe" && !name.to_ascii_lowercase().ends_with(".dll") {
            continue;
        }
        total = total
            .checked_add(entry.size())
            .ok_or_else(|| i18n::text("runtimeUnsafeArchive"))?;
        if total > 4 * 1024 * 1024 * 1024 {
            return Err(i18n::text("runtimeUnsafeArchive"));
        }
        let target = output.join(name);
        let mut file = fs::OpenOptions::new()
            .write(true)
            .create_new(true)
            .open(&target)
            .map_err(err)?;
        let mut buffer = vec![0; 1024 * 1024];
        loop {
            checkpoint()?;
            let n = entry.read(&mut buffer).map_err(err)?;
            if n == 0 {
                break;
            }
            file.write_all(&buffer[..n]).map_err(err)?;
        }
        file.sync_all().map_err(err)?;
    }
    Ok(())
}
pub(super) fn install(
    root: &Path,
    resources: &Path,
    device: &str,
    archives: &[ModelFile],
    downloads: &Downloads,
    notify: &impl Fn(Progress),
) -> Result<()> {
    let parent = root
        .parent()
        .ok_or_else(|| i18n::text("runtimeCacheError"))?;
    fs::create_dir_all(parent).map_err(err)?;
    let stage = tempfile::Builder::new()
        .prefix(".install-")
        .tempdir_in(parent)
        .map_err(err)?;
    let checkpoint = || downloads.checkpoint(notify);
    for archive in archives {
        unpack(
            &downloads.directory().join(&archive.name),
            stage.path(),
            &checkpoint,
        )?;
    }
    for item in fs::read_dir(resources.join("runtime/msvc")).map_err(err)? {
        checkpoint()?;
        let item = item.map_err(err)?;
        if item
            .path()
            .extension()
            .is_some_and(|v| v.eq_ignore_ascii_case("dll"))
            && regular(&item.path())
        {
            let target = stage.path().join(item.file_name());
            if !target.exists() {
                fs::copy(item.path(), target).map_err(err)?;
            }
        }
    }
    if required(device)
        .iter()
        .any(|name| !regular(&stage.path().join(name)))
    {
        return Err(i18n::text("runtimeIncomplete"));
    }
    let mut records = BTreeMap::new();
    for entry in fs::read_dir(stage.path()).map_err(err)? {
        let entry = entry.map_err(err)?;
        let name = entry.file_name().to_string_lossy().into_owned();
        records.insert(
            name.clone(),
            FileRecord {
                name,
                bytes: entry.metadata().map_err(err)?.len(),
                sha256: digest(&entry.path(), &checkpoint)?,
            },
        );
    }
    let receipt = Receipt {
        owner: OWNER.into(),
        archives: archives.iter().map(|f| f.sha256.clone()).collect(),
        files: records.into_values().collect(),
    };
    fs::write(
        stage.path().join(RECEIPT),
        serde_json::to_vec_pretty(&receipt).map_err(err)?,
    )
    .map_err(err)?;
    checkpoint()?;
    if root.exists() {
        // Only replace an app-owned runtime directory, never an arbitrary occupied folder.
        let old = fs::read(root.join(RECEIPT))
            .ok()
            .and_then(|v| serde_json::from_slice::<Receipt>(&v).ok());
        if !old.is_some_and(|r| r.owner == OWNER) {
            return Err(i18n::text("runtimeCacheError"));
        }
        fs::remove_dir_all(root).map_err(err)?;
    }
    fs::rename(stage.path(), root).map_err(err)?;
    Ok(())
}
#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Cursor;
    fn archive(path: &Path, entries: &[(&str, &[u8])]) {
        let mut zip = zip::ZipWriter::new(Cursor::new(Vec::new()));
        for (name, value) in entries {
            zip.start_file(*name, zip::write::SimpleFileOptions::default())
                .unwrap();
            zip.write_all(value).unwrap();
        }
        fs::write(path, zip.finish().unwrap().into_inner()).unwrap();
    }
    #[test]
    fn extraction_rejects_traversal_and_duplicate_runtime_names() {
        let dir = tempfile::tempdir().unwrap();
        let output = dir.path().join("out");
        fs::create_dir(&output).unwrap();
        let zip = dir.path().join("test.zip");
        archive(&zip, &[("../escape.dll", b"bad")]);
        assert!(unpack(&zip, &output, &|| Ok(())).is_err());
        assert!(!dir.path().join("escape.dll").exists());
        archive(&zip, &[("a/engine.dll", b"one"), ("b/engine.dll", b"two")]);
        assert!(unpack(&zip, &output, &|| Ok(())).is_err());
    }
    #[test]
    fn verified_cache_survives_restart_and_detects_corruption() {
        let dir = tempfile::tempdir().unwrap();
        let resources = dir.path().join("resources");
        fs::create_dir_all(resources.join("runtime/msvc")).unwrap();
        fs::write(resources.join("runtime/msvc/vcruntime140.dll"), b"crt").unwrap();
        let archive_path = dir.path().join("engine.zip");
        let names = required("vulkan");
        let entries: Vec<_> = names
            .iter()
            .filter(|n| n.as_str() != "vcruntime140.dll")
            .map(|n| (n.as_str(), b"engine".as_slice()))
            .collect();
        archive(&archive_path, &entries);
        let file = ModelFile {
            name: "engine.zip".into(),
            bytes: fs::metadata(&archive_path).unwrap().len(),
            sha256: digest(&archive_path, &|| Ok(())).unwrap(),
            url: Some("https://example.invalid/engine.zip".into()),
            repository: None,
            revision: None,
            layout: false,
        };
        let downloads = Downloads::for_files(dir.path().into(), vec![file.clone()]);
        let root = dir.path().join("version/vulkan");
        install(
            &root,
            &resources,
            "vulkan",
            &[file.clone()],
            &downloads,
            &|_| {},
        )
        .unwrap();
        assert!(valid(&root, "vulkan", &[file.clone()], &|| Ok(())).unwrap());
        fs::write(root.join("llama-server.exe"), b"broken").unwrap();
        assert!(!valid(&root, "vulkan", &[file.clone()], &|| Ok(())).unwrap());
        install(
            &root,
            &resources,
            "vulkan",
            &[file.clone()],
            &downloads,
            &|_| {},
        )
        .unwrap();
        assert!(valid(&root, "vulkan", &[file], &|| Ok(())).unwrap());
    }
}
