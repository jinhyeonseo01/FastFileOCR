use crate::{
    configuration, download, export, i18n, import,
    scanning::scan_worker,
    state::*,
    store::{err, inside, Result, Settings, Store},
};
use base64::Engine as _;
use std::{
    fs,
    path::{Path, PathBuf},
    sync::{atomic::Ordering, Arc},
};
use tauri::{Emitter, Manager, State};
#[tauri::command]
pub(crate) async fn snapshot(app: tauri::AppHandle) -> Result<Snapshot> {
    tauri::async_runtime::spawn_blocking(move || snapshot_value(&app.state::<AppState>()))
        .await
        .map_err(err)?
}

#[tauri::command]
pub(crate) async fn create_project(
    app: tauri::AppHandle,
    parent: String,
    name: String,
) -> Result<()> {
    tauri::async_runtime::spawn_blocking(move || {
        let state = app.state::<AppState>();
        let _busy = begin(&state)?;
        let mut store = Store::create(Path::new(&parent), name)?;
        store.project.settings = state.preferences.lock().map_err(err)?.scan.clone();
        store.save()?;
        remember(&state, &store.root)?;
        *state.store.lock().map_err(err)? = store;
        message(&state, crate::i18n::text("createdProject"));
        Ok(())
    })
    .await
    .map_err(err)?
}
#[tauri::command]
pub(crate) async fn open_project(app: tauri::AppHandle, directory: String) -> Result<()> {
    tauri::async_runtime::spawn_blocking(move || {
        let state = app.state::<AppState>();
        let _busy = begin(&state)?;
        let store = Store::open(PathBuf::from(directory))?;
        remember(&state, &store.root)?;
        *state.store.lock().map_err(err)? = store;
        message(&state, crate::i18n::text("openedProject"));
        Ok(())
    })
    .await
    .map_err(err)?
}
#[tauri::command]
pub(crate) async fn import_paths(app: tauri::AppHandle, paths: Vec<String>) -> Result<Vec<String>> {
    tauri::async_runtime::spawn_blocking(move || {
        let state = app.state::<AppState>();
        let busy = begin(&state)?;
        let mut errors = vec![];
        for (index, path) in paths.iter().enumerate() {
            message(
                &state,
                crate::i18n::f(
                    "importing",
                    &[(index + 1).to_string(), (paths.len()).to_string()],
                ),
            );
            changed(&app);
            let result = import::import_file(
                &mut *state.store.lock().map_err(err)?,
                Path::new(path),
                &state.resources,
            );
            if let Err(e) = result {
                errors.push(format!(
                    "{}: {e}",
                    Path::new(path)
                        .file_name()
                        .unwrap_or_default()
                        .to_string_lossy()
                ));
            }
        }
        message(
            &state,
            if errors.is_empty() {
                crate::i18n::text("imported")
            } else {
                crate::i18n::text("importPartial")
            },
        );
        drop(busy);
        changed(&app);
        Ok(errors)
    })
    .await
    .map_err(err)?
}
#[tauri::command]
pub(crate) async fn paste_image(app: tauri::AppHandle) -> Result<()> {
    tauri::async_runtime::spawn_blocking(move || {
        let state = app.state::<AppState>();
        let _busy = begin(&state)?;
        import::import_clipboard(&mut *state.store.lock().map_err(err)?)?;
        message(&state, crate::i18n::text("pasted"));
        Ok(())
    })
    .await
    .map_err(err)?
}
#[tauri::command]
pub(crate) fn update_settings(state: State<AppState>, settings: Settings) -> Result<()> {
    let _busy = begin(&state)?;
    settings.validate()?;
    state.runtimes.reset();
    let mut store = state.store.lock().map_err(err)?;
    *state.downloads.lock().map_err(err)? = Arc::new(download::Downloads::for_model(
        &state.data_root.join("models"),
        &settings.model_id,
        settings.use_layout,
    )?);
    let mut preferences = state.preferences.lock().map_err(err)?;
    preferences.scan = settings.clone();
    preferences.save(&state.data_root)?;
    store.project.settings = settings;
    store.save()
}
#[tauri::command]
pub(crate) fn edit_page(state: State<AppState>, page_id: String, markdown: String) -> Result<()> {
    let _busy = begin(&state)?;
    if markdown.len() > 4 * 1024 * 1024 {
        return Err(crate::i18n::text("resultTooLarge").into());
    }
    let mut store = state.store.lock().map_err(err)?;
    store.page_mut(&page_id)?.edit(markdown);
    store.save_result(&page_id)
}
#[tauri::command]
pub(crate) fn remove_page(state: State<AppState>, page_id: String) -> Result<()> {
    let _busy = begin(&state)?;
    let mut store = state.store.lock().map_err(err)?;
    store.page(&page_id)?;
    store.project.pages.retain(|p| p.id != page_id);
    store.save()
}
#[tauri::command]
pub(crate) async fn preview(app: tauri::AppHandle, page_id: String) -> Result<String> {
    tauri::async_runtime::spawn_blocking(move || {
        let state = app.state::<AppState>();
        let path = {
            let store = state.store.lock().map_err(err)?;
            inside(&store.root, &store.page(&page_id)?.image)?
        };
        Ok(format!(
            "data:image/jpeg;base64,{}",
            base64::engine::general_purpose::STANDARD.encode(fs::read(path).map_err(err)?)
        ))
    })
    .await
    .map_err(err)?
}
#[tauri::command]
pub(crate) fn cancel_scan(state: State<AppState>) {
    state.downloads().cancel();
    state.runtimes.cancel();
    state.engine.cancel();
    message(&state, crate::i18n::text("stopping"));
}

#[tauri::command]
pub(crate) fn pause_download(app: tauri::AppHandle) {
    let state = app.state::<AppState>();
    state.current_download().pause();
    let _ = app.emit("model-download", state.current_download().snapshot());
}
#[tauri::command]
pub(crate) fn resume_download(state: State<AppState>) {
    state.current_download().resume();
}

#[tauri::command]
pub(crate) fn start_scan(app: tauri::AppHandle, page_ids: Option<Vec<String>>) -> Result<()> {
    let state = app.state::<AppState>();
    let busy = begin(&state)?;
    let ids = {
        let store = state.store.lock().map_err(err)?;
        store.project.settings.validate()?;
        let ids =
            page_ids.unwrap_or_else(|| store.project.pages.iter().map(|p| p.id.clone()).collect());
        for id in &ids {
            store.page(id)?;
        }
        if ids.is_empty() {
            return Err(crate::i18n::text("noPages").into());
        }
        // Process each requested page once, always in document order, including completed pages.
        store
            .project
            .pages
            .iter()
            .filter(|p| ids.contains(&p.id))
            .map(|p| p.id.clone())
            .collect::<Vec<_>>()
    };
    state.engine.cancel.store(false, Ordering::SeqCst);
    state.downloads().reset();
    state.runtimes.reset();
    // Transfer ownership of the busy state to the worker.
    std::mem::forget(busy);
    tauri::async_runtime::spawn_blocking(move || {
        let state = app.state::<AppState>();
        let guard = Busy(&state.busy);
        let result =
            std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| scan_worker(&app, &ids)));
        state.engine.stop();
        let outcome = match result {
            Ok(v) => v,
            Err(_) => Err(crate::i18n::text("scanCrashed").into()),
        };
        if let Ok(mut store) = state.store.lock() {
            for page in &mut store.project.pages {
                if page.status == "processing" {
                    page.status = "queued".into();
                }
            }
            if let Err(e) = store.save() {
                message(&state, crate::i18n::f("saveFailed", &[(e).to_string()]));
                drop(guard);
                changed(&app);
                return;
            }
        }
        match outcome {
            Err(e) if !state.engine.cancelled() => message(&state, e),
            _ if state.engine.cancelled() => message(&state, crate::i18n::text("scanCancelled")),
            _ => message(&state, crate::i18n::text("scanCompleted")),
        }
        drop(guard);
        changed(&app);
    });
    Ok(())
}
#[tauri::command]
pub(crate) fn export_document(state: State<AppState>, path: String, format: String) -> Result<()> {
    let _busy = begin(&state)?;
    export::save(
        &state.store.lock().map_err(err)?.project,
        Path::new(&path),
        &format,
    )
}
#[tauri::command]
pub(crate) fn copy_text(text: String) -> Result<()> {
    arboard::Clipboard::new()
        .map_err(err)?
        .set_text(text)
        .map_err(err)
}
#[tauri::command]
pub(crate) fn open_folder(state: State<AppState>) -> Result<()> {
    let root = state.store.lock().map_err(err)?.root.clone();
    #[cfg(windows)]
    {
        use std::os::windows::process::CommandExt;
        std::process::Command::new("explorer.exe")
            .arg(root)
            .creation_flags(0x08000000)
            .spawn()
            .map_err(err)?;
    }
    Ok(())
}
#[tauri::command]
pub(crate) fn update_preferences(
    state: State<AppState>,
    mut preferences: configuration::Preferences,
) -> Result<()> {
    let _busy = begin(&state)?;
    preferences.validate()?;
    let mut current = state.preferences.lock().map_err(err)?;
    // The frontend edits app preferences only; workspace and OCR settings have dedicated commands.
    preferences.project_root = current.project_root.clone();
    preferences.scan = current.scan.clone();
    preferences.save(&state.data_root)?;
    i18n::set_language(&preferences.language);
    *current = preferences;
    Ok(())
}
pub(crate) fn update_operation(app: tauri::AppHandle, download: bool) -> Result<()> {
    let state = app.state::<AppState>();
    state
        .updater
        .busy
        .compare_exchange(false, true, Ordering::SeqCst, Ordering::SeqCst)
        .map_err(|_| i18n::text("busy"))?;
    {
        let mut progress = state.updater.progress.lock().map_err(err)?;
        progress.status = if download { "downloading" } else { "checking" }.into();
        progress.error = None;
    }
    changed(&app);
    tauri::async_runtime::spawn_blocking(move || {
        let state = app.state::<AppState>();
        let guard = Busy(&state.updater.busy);
        let result = if download {
            state.updater.download(&state.data_root, |p| {
                let _ = app.emit("app-update", p);
            })
        } else {
            let repository = state
                .preferences
                .lock()
                .unwrap_or_else(|e| e.into_inner())
                .github_repository
                .clone();
            state.updater.check(&repository)
        };
        if let Err(e) = result {
            let mut p = state
                .updater
                .progress
                .lock()
                .unwrap_or_else(|e| e.into_inner());
            p.status = "error".into();
            p.error = Some(e);
        }
        drop(guard);
        let _ = app.emit("app-update", state.updater.snapshot());
        changed(&app);
    });
    Ok(())
}
#[tauri::command]
pub(crate) fn check_updates(app: tauri::AppHandle) -> Result<()> {
    update_operation(app, false)
}
#[tauri::command]
pub(crate) fn download_update(app: tauri::AppHandle) -> Result<()> {
    update_operation(app, true)
}
#[tauri::command]
pub(crate) fn install_update(app: tauri::AppHandle) -> Result<()> {
    let state = app.state::<AppState>();
    let _busy = begin(&state)?;
    state.store.lock().map_err(err)?.save()?;
    let language = state.preferences.lock().map_err(err)?.language.clone();
    state.updater.install(&language)?;
    app.exit(0);
    Ok(())
}
#[tauri::command]
pub(crate) fn open_data_folder(state: State<AppState>) -> Result<()> {
    std::process::Command::new("explorer.exe")
        .arg(&state.data_root)
        .spawn()
        .map_err(err)?;
    Ok(())
}
