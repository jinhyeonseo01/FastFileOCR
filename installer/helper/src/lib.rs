//! Data lifecycle shared by NSIS and its isolated test harness.
//! No registry access: the installer owns presentation and passes explicit paths.
use std::{
    fs,
    io::Write,
    path::{Component, Path, PathBuf},
    time::{SystemTime, UNIX_EPOCH},
};

pub const MARKER: &str = ".fastfileocr-data";
const OWNER: &str = "FastFileOCR data v1";
pub type Result<T> = std::result::Result<T, Error>;
#[derive(Debug)]
pub enum Error {
    Unsafe,
    Write,
}
impl From<std::io::Error> for Error {
    fn from(_: std::io::Error) -> Self {
        Self::Write
    }
}
fn linked(meta: &fs::Metadata) -> bool {
    #[cfg(windows)]
    {
        use std::os::windows::fs::MetadataExt;
        if meta.file_attributes() & 0x400 != 0 {
            return true;
        }
    }
    meta.file_type().is_symlink()
}
fn path_checked(path: &Path) -> Result<PathBuf> {
    if !path.is_absolute() || path.components().any(|c| matches!(c, Component::ParentDir)) {
        return Err(Error::Unsafe);
    }
    #[cfg(windows)]
    if !matches!(path.components().next(), Some(Component::Prefix(p)) if matches!(p.kind(), std::path::Prefix::Disk(_)))
    {
        return Err(Error::Unsafe);
    }
    let path: PathBuf = path.components().collect();
    for ancestor in path.ancestors() {
        match fs::symlink_metadata(ancestor) {
            Ok(meta) if linked(&meta) || !meta.is_dir() => return Err(Error::Unsafe),
            Ok(_) => (),
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => (),
            Err(_) => return Err(Error::Unsafe),
        }
    }
    Ok(path)
}
fn same(a: &Path, b: &Path) -> bool {
    #[cfg(windows)]
    {
        a.to_string_lossy()
            .eq_ignore_ascii_case(&b.to_string_lossy())
    }
    #[cfg(not(windows))]
    {
        a == b
    }
}
fn within(path: &Path, parent: &Path) -> bool {
    #[cfg(windows)]
    {
        Path::new(&path.to_string_lossy().to_lowercase())
            .starts_with(parent.to_string_lossy().to_lowercase())
    }
    #[cfg(not(windows))]
    {
        path.starts_with(parent)
    }
}
fn protected(path: &Path) -> bool {
    for key in [
        "WINDIR",
        "SystemRoot",
        "ProgramFiles",
        "ProgramFiles(x86)",
        "ProgramW6432",
    ] {
        if let Some(value) = std::env::var_os(key) {
            if within(path, Path::new(&value)) {
                return true;
            }
        }
    }
    for key in ["USERPROFILE", "LOCALAPPDATA", "APPDATA", "PUBLIC"] {
        if let Some(value) = std::env::var_os(key) {
            if same(path, Path::new(&value)) {
                return true;
            }
        }
    }
    false
}
fn no_links_tree(path: &Path) -> Result<()> {
    let meta = fs::symlink_metadata(path)?;
    if linked(&meta) {
        return Err(Error::Unsafe);
    }
    if meta.is_dir() {
        for item in fs::read_dir(path)? {
            no_links_tree(&item?.path())?;
        }
    }
    Ok(())
}
fn populated(path: &Path) -> Result<bool> {
    if !path.exists() {
        return Ok(false);
    }
    Ok(fs::read_dir(path)?.next().transpose()?.is_some())
}
pub fn owned(root: &Path) -> Result<()> {
    let root = path_checked(root)?;
    if root.parent().is_none() || protected(&root) {
        return Err(Error::Unsafe);
    }
    no_links_tree(&root)?;
    if fs::read_to_string(root.join(MARKER))?.trim() != OWNER {
        return Err(Error::Unsafe);
    }
    Ok(())
}
/// Resolve a general parent to a dedicated child without claiming unrelated files.
pub fn resolve(selected: &Path, app: &Path) -> Result<PathBuf> {
    let selected = path_checked(selected)?;
    if protected(&selected) {
        return Err(Error::Unsafe);
    }
    let app = path_checked(app)?;
    let mut root = selected.clone();
    if same(&selected, &app) {
        root.push("Data");
    } else if within(&app, &selected)
        || (!selected.join(MARKER).exists()
            && (selected.parent().is_none() || populated(&selected)?))
    {
        root.push(
            if selected
                .file_name()
                .is_some_and(|n| n.to_string_lossy().eq_ignore_ascii_case("FastFileOCR"))
            {
                "Data"
            } else {
                "FastFileOCR"
            },
        );
    }
    let root = path_checked(&root)?;
    if root.parent().is_none() || protected(&root) || within(&app, &root) {
        return Err(Error::Unsafe);
    }
    if root.exists() {
        no_links_tree(&root)?;
        if populated(&root)? {
            owned(&root)?;
        }
    }
    Ok(root)
}
pub fn prepare(selected: &Path, app: &Path, fresh: bool) -> Result<PathBuf> {
    let root = resolve(selected, app)?;
    fs::create_dir_all(&root)?;
    let nonce = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map_err(|_| Error::Write)?
        .as_nanos();
    let probe = root.join(format!(".write-check-{nonce}"));
    fs::OpenOptions::new()
        .create_new(true)
        .write(true)
        .open(&probe)?
        .write_all(b"ok")?;
    fs::remove_file(probe)?;
    if !root.join(MARKER).exists() {
        fs::write(root.join(MARKER), OWNER)?;
    }
    owned(&root)?;
    if fresh {
        let settings = root.join("settings.json");
        if settings.exists() {
            let backups = root.join("settings-backups");
            fs::create_dir_all(&backups)?;
            fs::rename(settings, backups.join(format!("{nonce}.json")))?;
        }
    }
    Ok(root)
}
pub fn remove(root: &Path, data: bool, documents: bool) -> Result<()> {
    if !data && !documents {
        return Ok(());
    }
    owned(root)?;
    let mut names = Vec::new();
    if data {
        names.extend([
            "settings.json",
            "settings-backups",
            "models",
            "runtimes",
            "logs",
            "updates",
        ]);
    }
    if documents {
        names.push("workspaces");
    }
    for name in names {
        let child = root.join(name);
        match fs::symlink_metadata(&child) {
            Ok(meta) if meta.is_dir() => fs::remove_dir_all(child)?,
            Ok(_) => fs::remove_file(child)?,
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => (),
            Err(e) => return Err(e.into()),
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    struct Fixture(PathBuf);
    impl Fixture {
        fn new() -> Self {
            let id = SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap()
                .as_nanos();
            let path = std::env::temp_dir()
                .join(format!("fastfileocr-installer-{}-{id}", std::process::id()));
            fs::create_dir_all(&path).unwrap();
            Self(path)
        }
        fn path(&self, child: &str) -> PathBuf {
            self.0.join(child)
        }
    }
    impl Drop for Fixture {
        fn drop(&mut self) {
            let _ = fs::remove_dir_all(&self.0);
        }
    }
    #[test]
    fn fresh_backs_up_settings_and_keeps_models_and_documents() {
        let f = Fixture::new();
        let root = prepare(&f.path("한국어 日本語 data"), &f.path("app"), false).unwrap();
        fs::write(root.join("settings.json"), r#"{"language":"ko"}"#).unwrap();
        for name in ["models", "runtimes", "workspaces"] {
            fs::create_dir(root.join(name)).unwrap();
            fs::write(root.join(name).join("keep.part"), "retained").unwrap();
        }
        prepare(&root, &f.path("app"), true).unwrap();
        assert!(!root.join("settings.json").exists());
        let backup = fs::read_dir(root.join("settings-backups"))
            .unwrap()
            .next()
            .unwrap()
            .unwrap()
            .path();
        assert_eq!(fs::read_to_string(backup).unwrap(), r#"{"language":"ko"}"#);
        assert!(root.join("models/keep.part").exists());
        assert!(root.join("runtimes/keep.part").exists());
        assert!(root.join("workspaces/keep.part").exists());
    }
    #[test]
    fn populated_parent_uses_a_child_and_never_adopts_unrelated_data() {
        let f = Fixture::new();
        fs::create_dir(f.path("parent")).unwrap();
        fs::write(f.path("parent/personal.txt"), "keep").unwrap();
        assert_eq!(
            resolve(&f.path("parent"), &f.path("app")).unwrap(),
            f.path("parent/FastFileOCR")
        );
        fs::create_dir(f.path("parent/FastFileOCR")).unwrap();
        fs::write(f.path("parent/FastFileOCR/personal.txt"), "keep").unwrap();
        assert!(prepare(&f.path("parent"), &f.path("app"), false).is_err());
        assert!(!f.path("parent/.fastfileocr-data").exists());
    }
    #[test]
    fn deletion_choices_are_independent_and_preserve_unmanaged_files() {
        let f = Fixture::new();
        for (data, docs) in [(false, false), (true, false), (false, true), (true, true)] {
            let root = prepare(
                &f.path(&format!("data-{data}-{docs}")),
                &f.path("app"),
                false,
            )
            .unwrap();
            fs::write(root.join("settings.json"), "settings").unwrap();
            fs::write(root.join("personal.txt"), "personal").unwrap();
            for name in ["models", "runtimes", "workspaces"] {
                fs::create_dir(root.join(name)).unwrap();
            }
            remove(&root, data, docs).unwrap();
            assert_eq!(root.join("settings.json").exists(), !data);
            assert_eq!(root.join("models").exists(), !data);
            assert_eq!(root.join("runtimes").exists(), !data);
            assert_eq!(root.join("workspaces").exists(), !docs);
            assert!(root.join("personal.txt").exists());
            assert!(root.join(MARKER).exists());
        }
    }
    #[test]
    fn rejects_bad_marker_traversal_and_directory_links() {
        let f = Fixture::new();
        assert!(resolve(Path::new("relative"), &f.path("app")).is_err());
        assert!(resolve(&f.path("a/../b"), &f.path("app")).is_err());
        fs::create_dir(f.path("bad")).unwrap();
        fs::write(f.path("bad/.fastfileocr-data"), "other").unwrap();
        assert!(remove(&f.path("bad"), true, true).is_err());
        let root = prepare(&f.path("owned"), &f.path("app"), false).unwrap();
        #[cfg(windows)]
        {
            // Junctions do not require Developer Mode or symlink privileges.
            let status = std::process::Command::new("cmd")
                .args(["/c", "mklink", "/J"])
                .arg(root.join("models"))
                .arg(f.path("bad"))
                .output()
                .unwrap();
            assert!(status.status.success());
        }
        #[cfg(not(windows))]
        std::os::unix::fs::symlink(f.path("bad"), root.join("models")).unwrap();
        assert!(remove(&root, true, true).is_err());
        assert!(resolve(&root.join("models/new"), &f.path("app")).is_err());
        assert!(f.path("bad/.fastfileocr-data").exists());
        #[cfg(windows)]
        fs::remove_dir(root.join("models")).unwrap();
    }
    #[test]
    fn an_empty_parent_of_the_app_resolves_consistently_before_and_after_install() {
        let f = Fixture::new();
        let parent = f.path("shared");
        let app = parent.join("application");
        let resolved = resolve(&parent, &app).unwrap();
        assert_eq!(resolved, parent.join("FastFileOCR"));
        fs::create_dir_all(&app).unwrap();
        assert_eq!(resolve(&parent, &app).unwrap(), resolved);
        assert!(resolve(&parent, &parent.join("FastFileOCR")).is_err());
    }
    #[test]
    fn reinstall_keeps_settings_and_app_folder_gets_a_data_child() {
        let f = Fixture::new();
        let root = prepare(&f.path("app"), &f.path("app"), false).unwrap();
        assert_eq!(root, f.path("app/Data"));
        fs::write(root.join("settings.json"), "saved").unwrap();
        prepare(&root, &f.path("app"), false).unwrap();
        assert_eq!(
            fs::read_to_string(root.join("settings.json")).unwrap(),
            "saved"
        );
    }
}
