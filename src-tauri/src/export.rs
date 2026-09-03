use crate::{
    document::{plain_text, safe_html},
    store::{atomic_write, err, Project, Result},
};
use serde_json::json;
use std::path::Path;
pub fn render(project: &Project, format: &str) -> Result<String> {
    let pages = project
        .pages
        .iter()
        .filter(|p| !p.markdown.is_empty())
        .collect::<Vec<_>>();
    if format == "json" {
        return serde_json::to_string_pretty(&json!({
            "schemaVersion": 2, "generator": concat!("FastFileOCR ",env!("CARGO_PKG_VERSION")), "model": crate::models::get(&project.settings.model_id)?.descriptor().name, "modelId": project.settings.model_id,
            "project": {"id":project.id,"name":project.name,"updatedAt":project.updated_at},
            "structureSource":"model_output_markdown_or_html", "coordinatesAvailable":project.pages.iter().any(|p|!p.regions.is_empty()),
            "coordinateSystem":"normalized_page_image_pixels", "regionDetector":"PP-DocLayoutV3", "bboxFormat":"left,top,right,bottom",
            "pages": project.pages.iter().map(|p| json!({
                "id":p.id,"sourceName":p.name,"sourcePage":p.source_page,
                "width":p.width,"height":p.height,"status":p.status,
                "rawText":p.raw_text,"markdown":p.markdown,"text":if p.recognized_with.as_ref().is_some_and(|s| s.mode == "text") { p.markdown.clone() } else { plain_text(&p.markdown) },
                "blocks":p.blocks,"settings":p.recognized_with,"regions":p.regions,
                "regionTextMatchesDocument":p.regions.iter().filter(|r|!r.markdown.trim().is_empty()).map(|r|r.markdown.as_str()).collect::<Vec<_>>().join("\n\n")==p.markdown,
                "warning":p.warning,"error":p.error,"elapsedMs":p.elapsed_ms
            })).collect::<Vec<_>>()
        }))
        .map_err(err);
    }
    if pages.is_empty() {
        return Err(crate::i18n::text("noExport").into());
    }
    match format {
        "md" => Ok(pages
            .iter()
            .map(|p| p.markdown.as_str())
            .collect::<Vec<_>>()
            .join("\n\n---\n\n")),
        "txt" => Ok(pages
            .iter()
            .map(|p| {
                if p.recognized_with.as_ref().is_some_and(|s| s.mode == "text") {
                    p.markdown.clone()
                } else {
                    plain_text(&p.markdown)
                }
            })
            .collect::<Vec<_>>()
            .join("\n\n\u{000c}\n\n")),
        "html" => {
            let sections = pages
                .iter()
                .map(|p| {
                    format!(
                        "<section class=\"page\">{}</section>",
                        safe_html(&p.markdown)
                    )
                })
                .collect::<Vec<_>>()
                .join("\n");
            let title = project
                .name
                .replace('&', "&amp;")
                .replace('<', "&lt;")
                .replace('>', "&gt;")
                .replace('"', "&quot;");
            Ok(format!("<!doctype html><html><meta charset=\"utf-8\"><meta name=\"viewport\" content=\"width=device-width,initial-scale=1\"><meta http-equiv=\"Content-Security-Policy\" content=\"default-src 'none'; style-src 'unsafe-inline'\"><title>{title}</title><style>body{{max-width:900px;margin:48px auto;padding:0 24px;font:16px/1.8 system-ui;color:#243e59}}table{{border-collapse:collapse;width:100%}}td,th{{border:1px solid #d4e3f2;padding:8px;text-align:left}}pre{{white-space:pre-wrap;background:#f2f7fd;padding:16px}}.page{{break-after:page;margin-bottom:64px}}@media print{{body{{margin:0}}}}</style><body>{sections}</body></html>"))
        }
        _ => Err(crate::i18n::text("invalidExport").into()),
    }
}
pub fn save(project: &Project, path: &Path, format: &str) -> Result<()> {
    if path
        .extension()
        .and_then(|x| x.to_str())
        .map(|x| x.to_lowercase())
        != Some(format.into())
    {
        return Err(crate::i18n::f("exportExtension", &[(format).to_string()]));
    }
    atomic_write(path, render(project, format)?.as_bytes())
}
#[cfg(test)]
mod tests {
    use super::*;
    use crate::store::{Page, Store};
    #[test]
    fn structured_export_records_incomplete_pages_and_raw_response() {
        let dir = tempfile::tempdir().unwrap();
        let mut s = Store::create(dir.path(), "test".into()).unwrap();
        let mut p = Page::new(
            "원본.png".into(),
            "sources/a.png".into(),
            1,
            "pages/a.jpg".into(),
            "pages/a-thumb.jpg".into(),
            100,
            200,
        );
        p.raw_text = "# 원문".into();
        p.edit("# 편집".into());
        p.status = "done".into();
        s.project.pages.push(p);
        s.project.pages.push(Page::new(
            "다음.png".into(),
            "sources/b.png".into(),
            1,
            "pages/b.jpg".into(),
            "pages/b-thumb.jpg".into(),
            100,
            200,
        ));
        let v: serde_json::Value =
            serde_json::from_str(&render(&s.project, "json").unwrap()).unwrap();
        assert_eq!(v["pages"][0]["rawText"], "# 원문");
        assert_eq!(v["pages"][0]["blocks"][0]["text"], "편집");
        assert_eq!(v["pages"][1]["status"], "queued");
        assert_eq!(v["coordinatesAvailable"], false);
    }
}
