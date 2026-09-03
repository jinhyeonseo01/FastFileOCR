pub mod configuration;
pub mod document;
pub mod download;
pub mod engine;
pub mod export;
pub mod i18n;
pub mod import;
pub mod layout;
pub mod models;
pub mod runtimes;
pub mod store;
pub mod table;
pub mod updates;

mod commands;
mod scanning;
mod state;
use engine::Engine;
use state::{remember, AppState};
use std::{
    path::PathBuf,
    sync::{atomic::AtomicBool, Arc, Mutex},
};
use store::Store;
use tauri::Manager;
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_single_instance::init(|app, _, _| {
            if let Some(window) = app.get_webview_window("main") {
                let _ = window.unminimize();
                let _ = window.set_focus();
            }
        }))
        .plugin(tauri_plugin_dialog::init())
        .setup(|app| {
            let local = std::env::var_os("LOCALAPPDATA")
                .map(PathBuf::from)
                .unwrap_or(app.path().local_data_dir()?);
            let data = configuration::data_root(&local);
            let preferences = configuration::load(&data).map_err(std::io::Error::other)?;
            let mut notice = i18n::text("initialMessage");
            let store = match preferences.project_root.clone().map(Store::open) {
                Some(Ok(s)) => s,
                previous => {
                    if let Some(Err(e)) = previous {
                        notice = i18n::f("previousOpenError", &[e]);
                    }
                    let mut s =
                        Store::create(&data.join("workspaces"), i18n::translated("newDocument"))
                            .map_err(std::io::Error::other)?;
                    s.project.settings = preferences.scan.clone();
                    s.save().map_err(std::io::Error::other)?;
                    s
                }
            };
            #[cfg(debug_assertions)]
            let resources = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("resources");
            #[cfg(not(debug_assertions))]
            let resources = app.path().resource_dir()?.join("resources");
            let downloads = download::Downloads::for_model(
                &data.join("models"),
                &store.project.settings.model_id,
                store.project.settings.use_layout,
            )
            .map_err(std::io::Error::other)?;
            let state = AppState {
                store: Mutex::new(store),
                engine: Engine::default(),
                runtimes: runtimes::Runtimes::new(&data),
                downloads: Mutex::new(Arc::new(downloads)),
                preferences: Mutex::new(preferences),
                data_root: data.clone(),
                updater: updates::Updater::default(),
                busy: AtomicBool::new(false),
                message: Mutex::new(notice),
                resources,
                logs: data.join("logs"),
            };
            remember(&state, &state.store.lock().unwrap().root).map_err(std::io::Error::other)?;
            app.manage(state);
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            commands::snapshot,
            commands::create_project,
            commands::open_project,
            commands::import_paths,
            commands::paste_image,
            commands::update_settings,
            commands::edit_page,
            commands::remove_page,
            commands::preview,
            commands::cancel_scan,
            commands::pause_download,
            commands::resume_download,
            commands::start_scan,
            commands::export_document,
            commands::copy_text,
            commands::open_folder,
            commands::update_preferences,
            commands::check_updates,
            commands::download_update,
            commands::install_update,
            commands::open_data_folder
        ])
        .build(tauri::generate_context!())
        .expect("FastFileOCR startup failed")
        .run(|app, event| {
            if matches!(event, tauri::RunEvent::Exit) {
                app.state::<AppState>().downloads().cancel();
                app.state::<AppState>().runtimes.cancel();
                app.state::<AppState>().engine.cancel();
            }
        });
}
