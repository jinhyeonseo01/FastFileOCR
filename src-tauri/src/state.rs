use crate::{
    configuration, download,
    engine::Engine,
    models,
    store::{err, Project, Result, Store},
    updates,
};
use serde::Serialize;
use std::{
    path::{Path, PathBuf},
    sync::{
        atomic::{AtomicBool, Ordering},
        Arc, Mutex,
    },
};
use tauri::Emitter;
pub struct AppState {
    pub(crate) store: Mutex<Store>,
    pub(crate) engine: Engine,
    pub(crate) runtimes: crate::runtimes::Runtimes,
    pub(crate) downloads: Mutex<Arc<download::Downloads>>,
    pub(crate) preferences: Mutex<configuration::Preferences>,
    pub(crate) data_root: PathBuf,
    pub(crate) updater: updates::Updater,
    pub(crate) busy: AtomicBool,
    pub(crate) message: Mutex<String>,
    pub(crate) resources: PathBuf,
    pub(crate) logs: PathBuf,
}
impl AppState {
    pub(crate) fn current_download(&self) -> Arc<download::Downloads> {
        self.runtimes.active().unwrap_or_else(|| self.downloads())
    }
    pub(crate) fn downloads(&self) -> Arc<download::Downloads> {
        self.downloads
            .lock()
            .unwrap_or_else(|e| e.into_inner())
            .clone()
    }
}
#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct Snapshot {
    pub(crate) project: Project,
    pub(crate) directory: String,
    pub(crate) busy: bool,
    pub(crate) message: String,
    pub(crate) resources_ready: bool,
    pub(crate) download: download::Progress,
    pub(crate) preferences: configuration::Preferences,
    pub(crate) data_root: String,
    pub(crate) models: Vec<models::Descriptor>,
    pub(crate) update: updates::Progress,
}
pub(crate) fn snapshot_value(state: &AppState) -> Result<Snapshot> {
    let store = state.store.lock().map_err(err)?;
    Ok(Snapshot {
        project: store.project.clone(),
        directory: store.root.to_string_lossy().into(),
        busy: state.busy.load(Ordering::SeqCst),
        message: state.message.lock().map_err(err)?.clone(),
        resources_ready: ["runtime/cpu/llama-server.exe", "runtime/pdfium/pdfium.dll"]
            .iter()
            .all(|p| state.resources.join(p).is_file()),
        download: state.current_download().snapshot(),
        preferences: state.preferences.lock().map_err(err)?.clone(),
        data_root: state.data_root.to_string_lossy().into(),
        models: models::descriptors(),
        update: state.updater.snapshot(),
    })
}
pub(crate) fn changed(app: &tauri::AppHandle) {
    let _ = app.emit("workspace-changed", ());
}
pub(crate) fn message(state: &AppState, text: impl Into<String>) {
    if let Ok(mut m) = state.message.lock() {
        *m = text.into();
    }
}
pub(crate) struct Busy<'a>(pub(crate) &'a AtomicBool);
impl Drop for Busy<'_> {
    fn drop(&mut self) {
        self.0.store(false, Ordering::SeqCst);
    }
}
pub(crate) fn begin(state: &AppState) -> Result<Busy<'_>> {
    state
        .busy
        .compare_exchange(false, true, Ordering::SeqCst, Ordering::SeqCst)
        .map_err(|_| crate::i18n::text("busy"))?;
    Ok(Busy(&state.busy))
}
pub(crate) fn remember(state: &AppState, root: &Path) -> Result<()> {
    let mut preferences = state.preferences.lock().map_err(err)?;
    preferences.project_root = Some(root.into());
    preferences.save(&state.data_root)
}
