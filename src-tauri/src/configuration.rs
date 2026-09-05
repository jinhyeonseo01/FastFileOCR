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
    // Display-only. Legacy settings and IPC input cannot override the build's update source.
    #[serde(skip_deserializing, default = "configured_repository")]
    pub github_repository: String,
    pub project_root: Option<PathBuf>,
    pub scan: Settings,
}
fn configured_repository() -> String {
    crate::updates::default_repository().into()
}
impl Default for Preferences {
    fn default() -> Self {
        Self {
            schema_version: 1,
            language: "en".into(),
            check_updates: true,
            github_repository: crate::updates::default_repository().into(),
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
pub fn load(root: &Path) -> Result<Preferences> {
    ensure_owned(root)?;
    let path = root.join("settings.json");
    let mut settings = if path.is_file() {
        serde_json::from_slice::<Preferences>(&fs::read(&path).map_err(err)?).map_err(err)?
    } else {
        let mut initial = Preferences::default();
        initial.language = registry_value("Language")
            .filter(|v| ["en", "ko", "ja"].contains(&v.as_str()))
            .unwrap_or_else(|| "en".into());
        initial
    };
    settings.validate()?;
    settings.schema_version = 1;
    settings.save(root)?;
    i18n::set_language(&settings.language);
    Ok(settings)
}
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn saved_preferences_preserve_workspace_and_use_the_build_repository() {
        let dir = tempfile::tempdir().unwrap();
        let root = dir.path().join("data");
        ensure_owned(&root).unwrap();
        let saved = Preferences {
            project_root: Some(PathBuf::from("C:/documents/project")),
            github_repository: "example/FastFileOCR".into(),
            ..Preferences::default()
        };
        saved.save(&root).unwrap();
        let mut loaded = load(&root).unwrap();
        assert_eq!(loaded.project_root, saved.project_root);
        assert_eq!(
            loaded.github_repository,
            crate::updates::default_repository()
        );
        loaded.schema_version = 99;
        assert!(loaded.validate().is_err());
    }
    #[test]
    fn blank_repository_uses_default_without_changing_saved_settings() {
        let dir = tempfile::tempdir().unwrap();
        ensure_owned(dir.path()).unwrap();
        let mut saved = Preferences::default();
        saved.github_repository.clear();
        saved.language = "ja".into();
        saved.scan.use_layout = false;
        saved.save(dir.path()).unwrap();
        let loaded = load(dir.path()).unwrap();
        assert_eq!(
            loaded.github_repository,
            crate::updates::default_repository()
        );
        assert_eq!(loaded.language, "ja");
        assert!(!loaded.scan.use_layout);
    }
    #[test]
    fn repository_from_ipc_or_legacy_settings_cannot_override_update_source() {
        for repository in ["other/project", "", "https://example.invalid"] {
            let input = serde_json::json!({"githubRepository":repository, "language":"ko", "checkUpdates":false});
            let decoded: Preferences = serde_json::from_value(input).unwrap();
            assert_eq!(
                decoded.github_repository,
                crate::updates::default_repository()
            );
            assert_eq!(decoded.language, "ko");
            assert!(!decoded.check_updates);
        }
    }
    #[test]
    fn fresh_settings_preserve_models_and_documents() {
        let dir = tempfile::tempdir().unwrap();
        ensure_owned(dir.path()).unwrap();
        for name in ["models", "runtimes", "workspaces"] {
            fs::create_dir(dir.path().join(name)).unwrap();
            fs::write(dir.path().join(name).join("retained.part"), b"keep").unwrap();
        }
        let loaded = load(dir.path()).unwrap();
        assert!(loaded.project_root.is_none());
        assert_eq!(
            loaded.github_repository,
            crate::updates::default_repository()
        );
        for name in ["models", "runtimes", "workspaces"] {
            assert_eq!(
                fs::read(dir.path().join(name).join("retained.part")).unwrap(),
                b"keep"
            );
        }
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
