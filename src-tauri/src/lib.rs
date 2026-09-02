pub mod document;
pub mod download;
pub mod engine;
pub mod export;
pub mod import;
pub mod layout;
pub mod store;
pub mod table;

use base64::Engine as _;
use engine::Engine;
use serde::Serialize;
use std::{
    fs,
    path::{Path, PathBuf},
    sync::{
        atomic::{AtomicBool, Ordering},
        Mutex,
    },
    time::Instant,
};
use store::{atomic_write, err, inside, Project, Result, Settings, Store};
use tauri::{Emitter, Manager, State};

pub struct AppState {
    store: Mutex<Store>,
    engine: Engine,
    downloads: download::Downloads,
    busy: AtomicBool,
    message: Mutex<String>,
    resources: PathBuf,
    config: PathBuf,
    logs: PathBuf,
}
#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct Snapshot {
    project: Project,
    directory: String,
    busy: bool,
    message: String,
    resources_ready: bool,
    download: download::Progress,
}
fn snapshot_value(state: &AppState) -> Result<Snapshot> {
    let store = state.store.lock().map_err(err)?;
    Ok(Snapshot {
        project: store.project.clone(),
        directory: store.root.to_string_lossy().into(),
        busy: state.busy.load(Ordering::SeqCst),
        message: state.message.lock().map_err(err)?.clone(),
        resources_ready: ["runtime/cpu/llama-server.exe", "runtime/pdfium/pdfium.dll"]
            .iter()
            .all(|p| state.resources.join(p).is_file()),
        download: state.downloads.snapshot(),
    })
}
fn changed(app: &tauri::AppHandle) {
    let _ = app.emit("workspace-changed", ());
}
fn message(state: &AppState, text: impl Into<String>) {
    if let Ok(mut m) = state.message.lock() {
        *m = text.into();
    }
}
struct Busy<'a>(&'a AtomicBool);
impl Drop for Busy<'_> {
    fn drop(&mut self) {
        self.0.store(false, Ordering::SeqCst);
    }
}
fn begin(state: &AppState) -> Result<Busy<'_>> {
    state
        .busy
        .compare_exchange(false, true, Ordering::SeqCst, Ordering::SeqCst)
        .map_err(|_| "진행 중인 작업이 있습니다.")?;
    Ok(Busy(&state.busy))
}
fn remember(state: &AppState, root: &Path) -> Result<()> {
    atomic_write(
        &state.config,
        &serde_json::to_vec(&serde_json::json!({"projectRoot":root})).map_err(err)?,
    )
}
#[tauri::command]
async fn snapshot(app: tauri::AppHandle) -> Result<Snapshot> {
    tauri::async_runtime::spawn_blocking(move || snapshot_value(&app.state::<AppState>()))
        .await
        .map_err(err)?
}

#[tauri::command]
async fn create_project(app: tauri::AppHandle, parent: String, name: String) -> Result<()> {
    tauri::async_runtime::spawn_blocking(move || {
        let state = app.state::<AppState>();
        let _busy = begin(&state)?;
        let store = Store::create(Path::new(&parent), name)?;
        remember(&state, &store.root)?;
        *state.store.lock().map_err(err)? = store;
        message(&state, "새 작업을 만들었습니다.");
        Ok(())
    })
    .await
    .map_err(err)?
}
#[tauri::command]
async fn open_project(app: tauri::AppHandle, directory: String) -> Result<()> {
    tauri::async_runtime::spawn_blocking(move || {
        let state = app.state::<AppState>();
        let _busy = begin(&state)?;
        let store = Store::open(PathBuf::from(directory))?;
        remember(&state, &store.root)?;
        *state.store.lock().map_err(err)? = store;
        message(&state, "작업을 열었습니다.");
        Ok(())
    })
    .await
    .map_err(err)?
}
#[tauri::command]
async fn import_paths(app: tauri::AppHandle, paths: Vec<String>) -> Result<Vec<String>> {
    tauri::async_runtime::spawn_blocking(move || {
        let state = app.state::<AppState>();
        let busy = begin(&state)?;
        let mut errors = vec![];
        for (index, path) in paths.iter().enumerate() {
            message(
                &state,
                format!("문서를 가져오는 중 · {}/{}", index + 1, paths.len()),
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
                "문서를 추가했습니다."
            } else {
                "일부 파일을 추가하지 못했습니다."
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
async fn paste_image(app: tauri::AppHandle) -> Result<()> {
    tauri::async_runtime::spawn_blocking(move || {
        let state = app.state::<AppState>();
        let _busy = begin(&state)?;
        import::import_clipboard(&mut *state.store.lock().map_err(err)?)?;
        message(&state, "클립보드 캡처를 추가했습니다.");
        Ok(())
    })
    .await
    .map_err(err)?
}
#[tauri::command]
fn update_settings(state: State<AppState>, settings: Settings) -> Result<()> {
    let _busy = begin(&state)?;
    settings.validate()?;
    let mut store = state.store.lock().map_err(err)?;
    store.project.settings = settings;
    store.save()
}
#[tauri::command]
fn edit_page(state: State<AppState>, page_id: String, markdown: String) -> Result<()> {
    let _busy = begin(&state)?;
    if markdown.len() > 4 * 1024 * 1024 {
        return Err("페이지 결과는 최대 4MB입니다.".into());
    }
    let mut store = state.store.lock().map_err(err)?;
    store.page_mut(&page_id)?.edit(markdown);
    store.save_result(&page_id)
}
#[tauri::command]
fn remove_page(state: State<AppState>, page_id: String) -> Result<()> {
    let _busy = begin(&state)?;
    let mut store = state.store.lock().map_err(err)?;
    store.page(&page_id)?;
    store.project.pages.retain(|p| p.id != page_id);
    store.save()
}
#[tauri::command]
async fn preview(app: tauri::AppHandle, page_id: String) -> Result<String> {
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
fn cancel_scan(state: State<AppState>) {
    state.downloads.cancel();
    state.engine.cancel();
    message(&state, "스캔 중단 중…");
}

#[tauri::command]
fn pause_download(app: tauri::AppHandle) {
    let state = app.state::<AppState>();
    state.downloads.pause();
    let _ = app.emit("model-download", state.downloads.snapshot());
}
#[tauri::command]
fn resume_download(state: State<AppState>) {
    state.downloads.resume();
}

struct PageRecognition {
    raw: String,
    markdown: String,
    regions: Vec<layout::Region>,
    warning: Option<String>,
}
fn recognize_page(
    state: &AppState,
    app: &tauri::AppHandle,
    path: PathBuf,
    settings: &Settings,
    detector: Option<&mut layout::Detector>,
) -> Result<PageRecognition> {
    let mut regions = vec![];
    let mut warning = None;
    if let Some(detector) = detector {
        message(state, "페이지의 영역과 읽기 순서를 분석하고 있습니다…");
        changed(app);
        let image = image::open(&path).map_err(err)?;
        regions = detector.detect(&image)?;
        let found = regions.clone();
        let mut raw = Vec::new();
        let mut markdown = Vec::new();
        for region in &mut regions {
            if state.engine.cancelled() {
                return Err("취소됨".into());
            }
            if region.label == "image"
                && found
                    .iter()
                    .any(|inner| inner.label != "image" && layout::contains(region, inner))
            {
                region.status = "skipped".into();
                continue;
            }
            let mut options = settings.clone();
            if settings.mode == "document" {
                options.mode = match region.label.as_str() {
                    "table" => "table",
                    "formula" => "formula",
                    _ => "text",
                }
                .into();
            }
            region.ocr_mode = options.mode.clone();
            message(
                state,
                format!(
                    "영역별 OCR · {}/{} · {}",
                    region.order,
                    found.len(),
                    region.label
                ),
            );
            changed(app);
            let crop = layout::crop(&image, region.bbox);
            let temp = tempfile::Builder::new()
                .suffix(".jpg")
                .tempfile()
                .map_err(err)?;
            crop.save_with_format(temp.path(), image::ImageFormat::Jpeg)
                .map_err(err)?;
            let (text, notice) = state.engine.recognize(temp.path().into(), &options)?;
            let (mut normalized, table_notice) = table::normalize(&text, &options.mode);
            if settings.mode == "document" && !normalized.trim().is_empty() {
                if region.label == "doc_title" {
                    normalized = format!("# {}", normalized.trim());
                } else if region.label == "paragraph_title" {
                    normalized = format!("## {}", normalized.trim());
                }
            }
            region.raw_text = text.clone();
            region.markdown = normalized.clone();
            region.status = "done".into();
            region.warning = notice.or(table_notice);
            if region.warning.is_some() {
                warning = Some(
                    "일부 영역의 인식 결과에 주의 사항이 있습니다. 구조 탭에서 확인하세요.".into(),
                );
            }
            raw.push(text);
            if !normalized.trim().is_empty() {
                markdown.push(normalized);
            }
        }
        if !regions.is_empty() {
            return Ok(PageRecognition {
                raw: raw.join("\n\n"),
                markdown: markdown.join("\n\n"),
                regions,
                warning,
            });
        }
        warning=Some("영역을 찾지 못해 전체 페이지로 인식했습니다. 탐지 모델이 만화 말풍선이나 특수 레이아웃을 놓칠 수 있습니다.".into());
    }
    let (raw, notice) = state.engine.recognize(path, settings)?;
    let (markdown, table_notice) = table::normalize(&raw, &settings.mode);
    Ok(PageRecognition {
        raw,
        markdown,
        regions,
        warning: notice.or(table_notice).or(warning),
    })
}
fn scan_worker(app: &tauri::AppHandle, ids: &[String]) -> Result<()> {
    let state = app.state::<AppState>();
    let settings = state.store.lock().map_err(err)?.project.settings.clone();
    message(&state, "필요한 모델을 확인하고 있습니다…");
    changed(app);
    state.downloads.ensure(settings.use_layout, |progress| {
        let _ = app.emit("model-download", progress);
    })?;
    if state.engine.cancelled() {
        return Ok(());
    }
    message(&state, "OCR 엔진을 준비하고 있습니다…");
    changed(app);
    let device = state.engine.prepare(
        &state.resources,
        state.downloads.directory(),
        &state.logs,
        &settings.device,
    )?;
    let mut detector = if settings.use_layout {
        Some(layout::Detector::load(
            &state.resources,
            state.downloads.directory(),
        )?)
    } else {
        None
    };
    for (index, page_id) in ids.iter().enumerate() {
        if state.engine.cancelled() {
            break;
        }
        let path = {
            let mut store = state.store.lock().map_err(err)?;
            let path = inside(&store.root, &store.page(page_id)?.image)?;
            let p = store.page_mut(page_id)?;
            p.status = "processing".into();
            p.error = None;
            p.warning = None;
            store.save()?;
            path
        };
        message(
            &state,
            format!("전체 페이지 스캔 · {}/{} · {device}", index + 1, ids.len()),
        );
        changed(app);
        let start = Instant::now();
        let result = recognize_page(&state, app, path, &settings, detector.as_mut());
        let mut store = state.store.lock().map_err(err)?;
        let p = store.page_mut(page_id)?;
        p.elapsed_ms = start.elapsed().as_millis() as u64;
        if state.engine.cancelled() {
            p.status = "queued".into();
            store.save()?;
            break;
        }
        match result {
            Ok(result) => {
                p.recognized_with = Some(settings.clone());
                p.raw_text = result.raw;
                p.edit(result.markdown);
                p.regions = result.regions;
                p.status = "done".into();
                p.warning = result.warning;
                store.save_result(page_id)?;
            }
            Err(e) => {
                p.status = "error".into();
                p.error = Some(e);
                store.save()?;
            }
        }
        drop(store);
        changed(app);
    }
    Ok(())
}
#[tauri::command]
fn start_scan(app: tauri::AppHandle, page_ids: Option<Vec<String>>) -> Result<()> {
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
            return Err("스캔할 페이지가 없습니다.".into());
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
    state.downloads.reset();
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
            Err(_) => Err("OCR 작업이 예기치 않게 중단되었습니다.".into()),
        };
        if let Ok(mut store) = state.store.lock() {
            for page in &mut store.project.pages {
                if page.status == "processing" {
                    page.status = "queued".into();
                }
            }
            if let Err(e) = store.save() {
                message(&state, format!("작업 저장 실패: {e}"));
                drop(guard);
                changed(&app);
                return;
            }
        }
        match outcome {
            Err(e) if !state.engine.cancelled() => message(&state, e),
            _ if state.engine.cancelled() => message(
                &state,
                "스캔을 중단했습니다. 완료한 결과는 저장되어 있습니다.",
            ),
            _ => message(&state, "스캔을 마쳤습니다. 페이지별 결과를 확인하세요."),
        }
        drop(guard);
        changed(&app);
    });
    Ok(())
}
#[tauri::command]
fn export_document(state: State<AppState>, path: String, format: String) -> Result<()> {
    let _busy = begin(&state)?;
    export::save(
        &state.store.lock().map_err(err)?.project,
        Path::new(&path),
        &format,
    )
}
#[tauri::command]
fn copy_text(text: String) -> Result<()> {
    arboard::Clipboard::new()
        .map_err(err)?
        .set_text(text)
        .map_err(err)
}
#[tauri::command]
fn open_folder(state: State<AppState>) -> Result<()> {
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
            let data = app.path().app_data_dir()?;
            fs::create_dir_all(&data)?;
            let config = data.join("settings.json");
            let previous = fs::read(&config)
                .ok()
                .and_then(|b| serde_json::from_slice::<serde_json::Value>(&b).ok())
                .and_then(|v| v["projectRoot"].as_str().map(PathBuf::from));
            let mut notice = "문서를 추가하고 전체 스캔을 시작하세요.".to_string();
            let store = match previous.map(Store::open) {
                Some(Ok(s)) => s,
                previous => {
                    if let Some(Err(e)) = previous {
                        notice = format!("이전 작업을 열지 못했습니다: {e}");
                    }
                    let parent = app
                        .path()
                        .document_dir()
                        .unwrap_or_else(|_| data.clone())
                        .join("Glyph");
                    Store::create(&parent, "새 문서".into()).map_err(std::io::Error::other)?
                }
            };
            #[cfg(debug_assertions)]
            let resources = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("resources");
            #[cfg(not(debug_assertions))]
            let resources = app.path().resource_dir()?.join("resources");
            let use_layout = store.project.settings.use_layout;
            let state = AppState {
                store: Mutex::new(store),
                engine: Engine::default(),
                downloads: download::Downloads::new(
                    &app.path().app_local_data_dir()?.join("models"),
                    use_layout,
                ),
                busy: AtomicBool::new(false),
                message: Mutex::new(notice),
                resources,
                config,
                logs: data.join("logs"),
            };
            remember(&state, &state.store.lock().unwrap().root).map_err(std::io::Error::other)?;
            app.manage(state);
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            snapshot,
            create_project,
            open_project,
            import_paths,
            paste_image,
            update_settings,
            edit_page,
            remove_page,
            preview,
            cancel_scan,
            pause_download,
            resume_download,
            start_scan,
            export_document,
            copy_text,
            open_folder
        ])
        .build(tauri::generate_context!())
        .expect("Glyph 앱을 시작하지 못했습니다.")
        .run(|app, event| {
            if matches!(event, tauri::RunEvent::Exit) {
                app.state::<AppState>().downloads.cancel();
                app.state::<AppState>().engine.cancel();
            }
        });
}
