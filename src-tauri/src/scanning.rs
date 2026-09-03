use crate::{
    layout, models,
    state::*,
    store::{err, inside, Result, Settings},
};
use std::{path::PathBuf, time::Instant};
use tauri::{Emitter, Manager};
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
    let adapter = models::get(&settings.model_id)?;
    let mut regions = vec![];
    let mut warning = None;
    if let Some(detector) = detector {
        message(state, crate::i18n::text("detecting"));
        changed(app);
        let image = image::open(&path).map_err(err)?;
        regions = detector.detect(&image)?;
        let found = regions.clone();
        let mut raw = Vec::new();
        let mut markdown = Vec::new();
        for region in &mut regions {
            if state.engine.cancelled() {
                return Err(crate::i18n::text("cancelledOperation").into());
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
            options.mode = adapter.region_mode(&settings.mode, &region.label);
            region.ocr_mode = options.mode.clone();
            message(
                state,
                crate::i18n::f(
                    "recognizingRegion",
                    &[
                        (region.order).to_string(),
                        (found.len()).to_string(),
                        (region.label).to_string(),
                    ],
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
            let (mut normalized, table_notice) = adapter.normalize(&text, &options.mode);
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
                warning = Some(crate::i18n::text("regionWarnings").into());
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
        warning = Some(crate::i18n::text("noRegions").into());
    }
    let (raw, notice) = state.engine.recognize(path, settings)?;
    let (markdown, table_notice) = adapter.normalize(&raw, &settings.mode);
    Ok(PageRecognition {
        raw,
        markdown,
        regions,
        warning: notice.or(table_notice).or(warning),
    })
}
pub(crate) fn scan_worker(app: &tauri::AppHandle, ids: &[String]) -> Result<()> {
    let state = app.state::<AppState>();
    let settings = state.store.lock().map_err(err)?.project.settings.clone();
    message(&state, crate::i18n::text("checkingModels"));
    changed(app);
    state.downloads().ensure(settings.use_layout, |progress| {
        let _ = app.emit("model-download", progress);
    })?;
    if state.engine.cancelled() {
        return Ok(());
    }
    message(&state, crate::i18n::text("preparingEngine"));
    changed(app);
    let device = state.engine.prepare(
        &state.resources,
        state.downloads().directory(),
        &state.logs,
        &settings.device,
        &settings.model_id,
    )?;
    let mut detector = if settings.use_layout {
        Some(layout::Detector::load(
            &state.resources,
            state.downloads().directory(),
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
            crate::i18n::f(
                "recognizingPage",
                &[
                    (index + 1).to_string(),
                    (ids.len()).to_string(),
                    (device).to_string(),
                ],
            ),
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
