//! Recognition orchestration shared by the desktop app and native validation tools.
use crate::{
    layout, models,
    store::{err, Result, Settings},
};
pub struct PageRecognition {
    pub raw: String,
    pub markdown: String,
    pub regions: Vec<layout::Region>,
    pub warning: Option<String>,
}
pub fn recognize_page(
    engine: &crate::engine::Engine,
    path: &std::path::Path,
    settings: &Settings,
    detector: Option<&mut layout::Detector>,
    notify: impl Fn(String),
) -> Result<PageRecognition> {
    let adapter = models::get(&settings.model_id)?;
    let mut regions = vec![];
    let mut warning = None;
    if let Some(detector) = detector {
        notify(crate::i18n::text("detecting"));
        let image = image::open(&path).map_err(err)?;
        regions = detector.detect(&image)?;
        let found = regions.clone();
        let mut raw = Vec::new();
        let mut markdown = Vec::new();
        for region in &mut regions {
            if engine.cancelled() {
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
            notify(crate::i18n::f(
                "recognizingRegion",
                &[
                    (region.order).to_string(),
                    (found.len()).to_string(),
                    (region.label).to_string(),
                ],
            ));
            let crop = layout::crop(&image, region.bbox);
            let temp = tempfile::Builder::new()
                .suffix(".png")
                .tempfile()
                .map_err(err)?;
            crop.save_with_format(temp.path(), image::ImageFormat::Png)
                .map_err(err)?;
            let (text, notice) = engine.recognize(temp.path().into(), &options)?;
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
    let (raw, notice) = engine.recognize(path.into(), settings)?;
    let (markdown, table_notice) = adapter.normalize(&raw, &settings.mode);
    Ok(PageRecognition {
        raw,
        markdown,
        regions,
        warning: notice.or(table_notice).or(warning),
    })
}
