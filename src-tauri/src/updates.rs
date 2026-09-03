//! GitHub release discovery and checksum-verified installation. No automatic execution.
use crate::{
    i18n,
    store::{atomic_write, err, Result},
};
use reqwest::blocking::Client;
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::{
    fs,
    io::{Read, Write},
    path::{Path, PathBuf},
    sync::{atomic::AtomicBool, Mutex},
    time::{Duration, Instant},
};
#[derive(Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Progress {
    pub status: String,
    pub version: String,
    pub current_version: String,
    pub downloaded: u64,
    pub total: u64,
    pub error: Option<String>,
}
impl Default for Progress {
    fn default() -> Self {
        Self {
            status: "idle".into(),
            version: String::new(),
            current_version: env!("CARGO_PKG_VERSION").into(),
            downloaded: 0,
            total: 0,
            error: None,
        }
    }
}
#[derive(Clone)]
pub struct Release {
    pub version: String,
    pub url: String,
    pub checksum_url: String,
    pub bytes: u64,
    pub name: String,
}
#[derive(Deserialize)]
struct Asset {
    name: String,
    browser_download_url: String,
    size: u64,
}
#[derive(Deserialize)]
struct ApiRelease {
    tag_name: String,
    draft: bool,
    prerelease: bool,
    assets: Vec<Asset>,
}
#[derive(Default)]
pub struct Updater {
    pub progress: Mutex<Progress>,
    pub busy: AtomicBool,
    release: Mutex<Option<Release>>,
    installer: Mutex<Option<PathBuf>>,
}
pub fn valid_repository(repository: &str) -> bool {
    let parts: Vec<_> = repository.split('/').collect();
    parts.len() == 2
        && parts.iter().all(|s| {
            !s.is_empty()
                && ![".", ".."].contains(s)
                && s.chars()
                    .all(|c| c.is_ascii_alphanumeric() || c == '-' || c == '_' || c == '.')
        })
}
fn client() -> Result<Client> {
    Client::builder()
        .user_agent("FastFileOCR")
        .connect_timeout(Duration::from_secs(10))
        .timeout(Duration::from_secs(3600))
        .build()
        .map_err(err)
}
fn parse_release(json: &[u8], repository: &str, current: &str) -> Result<Option<Release>> {
    let api: ApiRelease = serde_json::from_slice(json).map_err(err)?;
    let version = semver::Version::parse(api.tag_name.trim_start_matches('v')).map_err(err)?;
    if api.draft
        || api.prerelease
        || !version.pre.is_empty()
        || version <= semver::Version::parse(current).map_err(err)?
    {
        return Ok(None);
    }
    let name = format!("FastFileOCR_{}_x64-setup.exe", version);
    let exe = api
        .assets
        .iter()
        .find(|a| a.name == name)
        .ok_or_else(|| i18n::text("updateNoInstaller"))?;
    let sha = api
        .assets
        .iter()
        .find(|a| a.name == format!("{name}.sha256"))
        .ok_or_else(|| i18n::text("updateNoChecksum"))?;
    let prefix = format!("https://github.com/{repository}/releases/download/");
    if !exe.browser_download_url.starts_with(&prefix)
        || !sha.browser_download_url.starts_with(&prefix)
        || exe.size == 0
        || exe.size > 4 * 1024 * 1024 * 1024
    {
        return Err(i18n::text("updateInvalidAsset"));
    }
    Ok(Some(Release {
        version: version.to_string(),
        url: exe.browser_download_url.clone(),
        checksum_url: sha.browser_download_url.clone(),
        bytes: exe.size,
        name,
    }))
}
impl Updater {
    pub fn snapshot(&self) -> Progress {
        self.progress
            .lock()
            .unwrap_or_else(|e| e.into_inner())
            .clone()
    }
    pub fn check(&self, repository: &str) -> Result<()> {
        if !valid_repository(repository) {
            return Err(i18n::text("invalidRepository"));
        }
        let response = client()?
            .get(format!(
                "https://api.github.com/repos/{repository}/releases/latest"
            ))
            .header("Accept", "application/vnd.github+json")
            .send()
            .map_err(err)?
            .error_for_status()
            .map_err(err)?;
        let mut bytes = Vec::new();
        response
            .take(2 * 1024 * 1024)
            .read_to_end(&mut bytes)
            .map_err(err)?;
        let release = parse_release(&bytes, repository, env!("CARGO_PKG_VERSION"))?;
        let mut state = self.progress.lock().map_err(err)?;
        state.status = if release.is_some() {
            "available"
        } else {
            "current"
        }
        .into();
        state.version = release
            .as_ref()
            .map(|r| r.version.clone())
            .unwrap_or_default();
        state.error = None;
        *self.release.lock().map_err(err)? = release;
        *self.installer.lock().map_err(err)? = None;
        Ok(())
    }
    pub fn download(&self, root: &Path, notify: impl Fn(Progress)) -> Result<()> {
        let release = self
            .release
            .lock()
            .map_err(err)?
            .clone()
            .ok_or_else(|| i18n::text("updateCheckFirst"))?;
        let client = client()?;
        let mut checksum = String::new();
        client
            .get(&release.checksum_url)
            .send()
            .map_err(err)?
            .error_for_status()
            .map_err(err)?
            .take(8192)
            .read_to_string(&mut checksum)
            .map_err(err)?;
        let expected = checksum
            .split_whitespace()
            .next()
            .filter(|s| s.len() == 64 && s.chars().all(|c| c.is_ascii_hexdigit()))
            .ok_or_else(|| i18n::text("updateNoChecksum"))?
            .to_lowercase();
        let directory = root.join("updates");
        fs::create_dir_all(&directory).map_err(err)?;
        let final_path = directory.join(&release.name);
        let part = final_path.with_extension("part");
        {
            let mut state = self.progress.lock().map_err(err)?;
            state.status = "downloading".into();
            state.downloaded = 0;
            state.total = release.bytes;
            state.error = None;
        }
        notify(self.snapshot());
        let mut response = client
            .get(&release.url)
            .send()
            .map_err(err)?
            .error_for_status()
            .map_err(err)?;
        let mut file = fs::File::create(&part).map_err(err)?;
        let mut hash = Sha256::new();
        let mut received = 0u64;
        let mut buffer = vec![0; 256 * 1024];
        let mut last = Instant::now();
        loop {
            let n = response.read(&mut buffer).map_err(err)?;
            if n == 0 {
                break;
            }
            received += n as u64;
            if received > release.bytes {
                return Err(i18n::text("updateInvalidAsset"));
            }
            file.write_all(&buffer[..n]).map_err(err)?;
            hash.update(&buffer[..n]);
            if last.elapsed() > Duration::from_millis(150) {
                self.progress.lock().map_err(err)?.downloaded = received;
                notify(self.snapshot());
                last = Instant::now();
            }
        }
        file.sync_all().map_err(err)?;
        drop(file);
        if received != release.bytes || format!("{:x}", hash.finalize()) != expected {
            return Err(i18n::text("updateHashMismatch"));
        }
        if final_path.exists() {
            fs::remove_file(&final_path).map_err(err)?;
        }
        fs::rename(&part, &final_path).map_err(err)?;
        // Keep the expected hash to recheck immediately before running the installer.
        atomic_write(&final_path.with_extension("sha256"), expected.as_bytes())?;
        *self.installer.lock().map_err(err)? = Some(final_path);
        {
            let mut state = self.progress.lock().map_err(err)?;
            state.status = "ready".into();
            state.downloaded = received;
        }
        notify(self.snapshot());
        Ok(())
    }
    pub fn install(&self, language: &str) -> Result<()> {
        let path = self
            .installer
            .lock()
            .map_err(err)?
            .clone()
            .ok_or_else(|| i18n::text("updateNotReady"))?;
        let expected = fs::read_to_string(path.with_extension("sha256")).map_err(err)?;
        let mut file = fs::File::open(&path).map_err(err)?;
        let mut hash = Sha256::new();
        let mut buffer = vec![0; 1024 * 1024];
        loop {
            let n = file.read(&mut buffer).map_err(err)?;
            if n == 0 {
                break;
            }
            hash.update(&buffer[..n]);
        }
        if format!("{:x}", hash.finalize()) != expected {
            return Err(i18n::text("updateHashMismatch"));
        }
        // Deliberately interactive: installation and data-retention choices stay visible.
        let installer_language = match language {
            "ko" => "1042",
            "ja" => "1041",
            _ => "1033",
        };
        std::process::Command::new(&path)
            .arg(format!("/LANGUAGE={installer_language}"))
            .spawn()
            .map_err(err)?;
        Ok(())
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn validates_release_origin_version_and_checksum_pair() {
        let json = serde_json::json!({"tag_name":"v2.1.0","draft":false,"prerelease":false,"assets":[
            {"name":"FastFileOCR_2.1.0_x64-setup.exe","browser_download_url":"https://github.com/example/FastFileOCR/releases/download/v2.1.0/FastFileOCR_2.1.0_x64-setup.exe","size":500},
            {"name":"FastFileOCR_2.1.0_x64-setup.exe.sha256","browser_download_url":"https://github.com/example/FastFileOCR/releases/download/v2.1.0/a.sha256","size":100}]});
        let bytes = serde_json::to_vec(&json).unwrap();
        assert!(parse_release(&bytes, "example/FastFileOCR", "2.0.0")
            .unwrap()
            .is_some());
        assert!(parse_release(&bytes, "example/FastFileOCR", "2.1.0")
            .unwrap()
            .is_none());
        assert!(parse_release(&bytes, "other/repo", "2.0.0").is_err());
        assert!(!valid_repository("../repo"));
        assert!(!valid_repository("https://github.com/a/b"));
    }
}
