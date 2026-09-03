//! CPU ships with the app. GPU engines are pinned, resumable downloads retained with user data.
mod cache;
pub mod hardware;
use crate::{
    download::{Downloads, ModelFile, Progress},
    i18n,
    store::{err, Result},
};
use serde::Deserialize;
use std::{
    collections::HashMap,
    fs,
    path::{Path, PathBuf},
    sync::{
        atomic::{AtomicBool, Ordering},
        Arc, Mutex,
    },
};
#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Manifest {
    pub version: String,
    pub cuda_driver_minimum: i32,
    pub files: HashMap<String, Vec<ModelFile>>,
}
pub fn manifest() -> Manifest {
    serde_json::from_str(include_str!("manifest.json")).expect("Bundled runtime manifest")
}
pub struct Runtimes {
    root: PathBuf,
    active: Mutex<Option<Arc<Downloads>>>,
    cancelled: AtomicBool,
}
impl Runtimes {
    pub fn new(data_root: &Path) -> Self {
        Self {
            root: data_root.join("runtimes"),
            active: Mutex::new(None),
            cancelled: AtomicBool::new(false),
        }
    }
    pub fn active(&self) -> Option<Arc<Downloads>> {
        self.active
            .lock()
            .unwrap_or_else(|e| e.into_inner())
            .clone()
    }
    pub fn reset(&self) {
        self.cancelled.store(false, Ordering::SeqCst);
        *self.active.lock().unwrap_or_else(|e| e.into_inner()) = None;
    }
    pub fn cancel(&self) {
        self.cancelled.store(true, Ordering::SeqCst);
        if let Some(dl) = self.active() {
            dl.cancel();
        }
    }
    pub fn ensure(
        &self,
        resources: &Path,
        device: &str,
        notify: impl Fn(Progress),
    ) -> Result<PathBuf> {
        if self.cancelled.load(Ordering::SeqCst) {
            return Err(i18n::text("cancelledOperation"));
        }
        if device == "cpu" {
            return Ok(resources.join("runtime/cpu"));
        }
        let spec = manifest();
        let files = spec
            .files
            .get(device)
            .filter(|_| ["cuda", "vulkan"].contains(&device))
            .ok_or_else(|| i18n::text("unsupportedDevice"))?
            .clone();
        let version_root = self.root.join(&spec.version);
        let root = version_root.join(device);
        for path in [
            &self.root,
            &version_root,
            &root,
            &version_root.join("downloads"),
            &version_root.join("downloads").join(device),
        ] {
            if let Ok(meta) = path.symlink_metadata() {
                #[cfg(windows)]
                {
                    use std::os::windows::fs::MetadataExt;
                    if meta.file_attributes() & 0x400 != 0 {
                        return Err(i18n::text("runtimeCacheError"));
                    }
                }
                if meta.file_type().is_symlink() || !meta.is_dir() {
                    return Err(i18n::text("runtimeCacheError"));
                }
            }
        }
        let dl = Arc::new(Downloads::for_files(
            version_root.join("downloads").join(device),
            files.clone(),
        ));
        *self.active.lock().map_err(err)? = Some(dl.clone());
        if self.cancelled.load(Ordering::SeqCst) {
            dl.cancel();
        }
        let result = (|| {
            dl.stage("checking", &notify);
            if cache::valid(&root, device, &files, &|| dl.checkpoint(&notify))? {
                dl.stage("ready", &notify);
                return Ok(());
            }
            fs::create_dir_all(&version_root).map_err(err)?;
            dl.ensure(true, &notify)?;
            dl.stage("extracting", &notify);
            cache::install(&root, resources, device, &files, &dl, &notify)?;
            dl.stage("ready", &notify);
            // Keep installed engines across app updates, without retaining a second compressed copy.
            for file in &files {
                let _ = fs::remove_file(dl.directory().join(&file.name));
            }
            Ok(())
        })();
        dl.report_error(&result, &notify);
        result.map(|_| root)
    }
}
