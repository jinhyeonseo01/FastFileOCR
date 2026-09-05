use crate::{
    layout,
    state::*,
    store::{err, inside, Result},
};
use std::time::Instant;
use tauri::{Emitter, Manager};
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
        &state.runtimes,
        |progress| {
            let _ = app.emit("model-download", progress);
        },
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
        let result = crate::recognition::recognize_page(
            &state.engine,
            &path,
            &settings,
            detector.as_mut(),
            |text| {
                message(&state, text);
                changed(app);
            },
        );
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
