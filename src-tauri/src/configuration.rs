//! Versioned application preferences and the installer-selected data root.
use crate::{
    i18n,
    store::{atomic_write, err, Result, Settings},
};
use serde::{Deserialize, Serialize};
use std::{
    fs,
    path::{Path, PathBuf},
};
pub const MARKER: &str = ".fastfileocr-data";
#[derive(Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", default)]
pub struct Preferences {
    pub schema_version: u32,
    pub language: String,
    pub check_updates: bool,
    pub github_repository: String,
    pub project_root: Option<PathBuf>,
    pub scan: Settings,
}
impl Default for Preferences {
    fn default() -> Self {
        Self {
            schema_version: 1,
            language: "en".into(),
            check_updates: true,
            github_repository: option_env!("FASTFILEOCR_GITHUB_REPOSITORY")
                .unwrap_or("")
                .into(),
            project_root: None,
            scan: Settings::default(),
        }
    }
}
impl Preferences {
    pub fn validate(&self) -> Result<()> {
        if self.schema_version > 1 {
            return Err(i18n::text("futureSettings"));
        }
        if !["en", "ko", "ja"].contains(&self.language.as_str()) {
            return Err(i18n::text("invalidLanguage"));
        }
        if !self.github_repository.is_empty()
            && !crate::updates::valid_repository(&self.github_repository)
        {
            return Err(i18n::text("invalidRepository"));
        }
        self.scan.validate()
    }
    pub fn save(&self, root: &Path) -> Result<()> {
        self.validate()?;
        atomic_write(
            &root.join("settings.json"),
            &serde_json::to_vec_pretty(self).map_err(err)?,
        )
    }
}
#[cfg(windows)]
pub fn registry_value(name: &str) -> Option<String> {
    use windows_sys::Win32::System::Registry::*;
    let key: Vec<u16> = "Software\\FastFileOCR"
        .encode_utf16()
        .chain(Some(0))
        .collect();
    let value: Vec<u16> = name.encode_utf16().chain(Some(0)).collect();
    let mut bytes = 0u32;
    unsafe {
        if RegGetValueW(
            HKEY_CURRENT_USER,
            key.as_ptr(),
            value.as_ptr(),
            RRF_RT_REG_SZ,
            std::ptr::null_mut(),
            std::ptr::null_mut(),
            &mut bytes,
        ) != 0
        {
            return None;
        }
        let mut buffer = vec![0u16; (bytes as usize + 1) / 2];
        if RegGetValueW(
            HKEY_CURRENT_USER,
            key.as_ptr(),
            value.as_ptr(),
            RRF_RT_REG_SZ,
            std::ptr::null_mut(),
            buffer.as_mut_ptr().cast(),
            &mut bytes,
        ) != 0
        {
            return None;
        }
        Some(
            String::from_utf16_lossy(&buffer)
                .trim_end_matches('\0')
                .to_string(),
        )
    }
}
#[cfg(not(windows))]
pub fn registry_value(_name: &str) -> Option<String> {
    None
}
pub fn data_root(local: &Path) -> PathBuf {
    std::env::var_os("FASTFILEOCR_DATA_DIR")
        .map(PathBuf::from)
        .or_else(|| {
            registry_value("DataDir")
                .filter(|v| !v.trim().is_empty())
                .map(PathBuf::from)
        })
        .unwrap_or_else(|| local.join("FastFileOCR"))
}
pub fn ensure_owned(root: &Path) -> Result<()> {
    if !root.is_absolute() || root.parent().is_none() || root.components().count() < 3 {
        return Err(i18n::text("unsafeDataPath"));
    }
    if root.exists() {
        if root
            .symlink_metadata()
            .map_err(err)?
            .file_type()
            .is_symlink()
        {
            return Err(i18n::text("unsafeDataPath"));
        }
        #[cfg(windows)]
        {
            use std::os::windows::fs::MetadataExt;
            if root.symlink_metadata().map_err(err)?.file_attributes() & 0x400 != 0 {
                return Err(i18n::text("unsafeDataPath"));
            }
        }
        if root.join(MARKER).exists() {
            if fs::read_to_string(root.join(MARKER)).map_err(err)?.trim() != "FastFileOCR data v1" {
                return Err(i18n::text("unownedDataPath"));
            }
        } else if fs::read_dir(root).map_err(err)?.next().is_some() {
            return Err(i18n::text("unownedDataPath"));
        }
    }
    fs::create_dir_all(root).map_err(err)?;
    if !root.join(MARKER).exists() {
        atomic_write(&root.join(MARKER), b"FastFileOCR data v1\n")?;
    }
    Ok(())
}
pub fn load(root: &Path, legacy_config: &Path) -> Result<Preferences> {
    ensure_owned(root)?;
    let path = root.join("settings.json");
    let mut settings = if path.is_file() {
        serde_json::from_slice::<Preferences>(&fs::read(&path).map_err(err)?).map_err(err)?
    } else {
        let mut initial = Preferences::default();
        initial.language = registry_value("Language")
            .filter(|v| ["en", "ko", "ja"].contains(&v.as_str()))
            .unwrap_or_else(|| "en".into());
        // Upgrade preserves the old workspace reference; model migration is handled separately.
        if !root.join(".fresh-settings").exists() {
            if let Ok(bytes) = fs::read(legacy_config) {
                if let Ok(old) = serde_json::from_slice::<serde_json::Value>(&bytes) {
                    initial.project_root = old["projectRoot"].as_str().map(PathBuf::from);
                }
            }
        }
        initial
    };
    if settings.github_repository.is_empty() {
        settings.github_repository = option_env!("FASTFILEOCR_GITHUB_REPOSITORY")
            .unwrap_or("")
            .into();
    }
    settings.validate()?;
    settings.schema_version = 1;
    settings.save(root)?;
    let _ = fs::remove_file(root.join(".fresh-settings"));
    i18n::set_language(&settings.language);
    Ok(settings)
}
pub fn migrate_models(legacy: &Path, root: &Path) -> Result<()> {
    let destination = root.join("models");
    if destination.exists() || !legacy.is_dir() {
        return Ok(());
    }
    fn copy_tree(source: &Path, target: &Path) -> Result<()> {
        fs::create_dir_all(target).map_err(err)?;
        for entry in fs::read_dir(source).map_err(err)? {
            let entry = entry.map_err(err)?;
            let kind = entry.file_type().map_err(err)?;
            if kind.is_symlink() {
                continue;
            }
            if kind.is_dir() {
                copy_tree(&entry.path(), &target.join(entry.file_name()))?;
            } else if kind.is_file() {
                fs::copy(entry.path(), target.join(entry.file_name())).map_err(err)?;
            }
        }
        Ok(())
    }
    copy_tree(legacy, &destination)
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn migrations_preserve_workspace_and_reject_future_schema() {
        let dir = tempfile::tempdir().unwrap();
        let legacy = dir.path().join("legacy.json");
        fs::write(&legacy, br#"{"projectRoot":"C:/documents/project"}"#).unwrap();
        let root = dir.path().join("data");
        let p = load(&root, &legacy).unwrap();
        assert!(p.project_root.is_some());
        assert!(root.join(MARKER).is_file());
        let mut future = p.clone();
        future.schema_version = 99;
        assert!(future.validate().is_err());
    }
    #[test]
    fn fresh_start_skips_legacy_and_preserves_models() {
        let dir = tempfile::tempdir().unwrap();
        let root = dir.path().join("data");
        ensure_owned(&root).unwrap();
        fs::create_dir(root.join("models")).unwrap();
        fs::write(root.join("models/retained.part"), b"keep").unwrap();
        fs::write(root.join(".fresh-settings"), b"1").unwrap();
        let legacy = dir.path().join("legacy.json");
        fs::write(&legacy, br#"{"projectRoot":"C:/documents/old"}"#).unwrap();
        let preferences = load(&root, &legacy).unwrap();
        assert!(preferences.project_root.is_none());
        assert!(!root.join(".fresh-settings").exists());
        assert_eq!(
            fs::read(root.join("models/retained.part")).unwrap(),
            b"keep"
        );
    }
    #[test]
    fn rejects_invalid_ownership_marker() {
        let dir = tempfile::tempdir().unwrap();
        fs::write(dir.path().join(MARKER), "another app").unwrap();
        assert!(ensure_owned(dir.path()).is_err());
    }
    #[test]
    fn refuses_unmarked_nonempty_directory() {
        let dir = tempfile::tempdir().unwrap();
        fs::write(dir.path().join("personal.txt"), "keep").unwrap();
        assert!(ensure_owned(dir.path()).is_err());
        assert_eq!(
            fs::read_to_string(dir.path().join("personal.txt")).unwrap(),
            "keep"
        );
    }
}
