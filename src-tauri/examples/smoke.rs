use fastfileocr_core::{
    engine::Engine,
    export,
    import::import_file,
    store::{Settings, Store},
};
use std::{path::PathBuf, time::Instant};
fn main() {
    if let Err(e) = execute() {
        eprintln!("{e}");
        std::process::exit(1);
    }
}
fn execute() -> Result<(), String> {
    let args = std::env::args().skip(1).collect::<Vec<_>>();
    if args.first().is_some_and(|s| s == "--check-updates") {
        let repository = args
            .get(1)
            .map(String::as_str)
            .unwrap_or(fastfileocr_core::updates::default_repository());
        let updater = fastfileocr_core::updates::Updater::default();
        updater.check(repository)?;
        println!(
            "{}",
            serde_json::to_string_pretty(&updater.snapshot()).map_err(|e| e.to_string())?
        );
        return Ok(());
    }
    if args.first().is_some_and(|s| s == "--copy-image") {
        let image = image::open(args.get(1).ok_or("image path missing")?)
            .map_err(|e| e.to_string())?
            .to_rgba8();
        arboard::Clipboard::new()
            .map_err(|e| e.to_string())?
            .set_image(arboard::ImageData {
                width: image.width() as usize,
                height: image.height() as usize,
                bytes: std::borrow::Cow::Owned(image.into_raw()),
            })
            .map_err(|e| e.to_string())?;
        println!("Fixture image copied to clipboard.");
        return Ok(());
    }
    let input = args
        .first()
        .ok_or("usage: smoke INPUT [auto|cpu|vulkan|cuda] [text|document|table]")?;
    let root = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let resources = root.join("resources");
    let mut store = Store::create(&root.join("../outputs/smoke"), "OCR validation".into())?;
    let added = import_file(&mut store, &PathBuf::from(input), &resources)?;
    println!("Imported {added} page(s).");
    let settings = Settings {
        device: args.get(1).cloned().unwrap_or("auto".into()),
        mode: args.get(2).cloned().unwrap_or("text".into()),
        max_tokens: 4096,
        use_layout: false, // This example validates whole-page OCR.
        ..Settings::default()
    };
    let downloads = fastfileocr_core::download::Downloads::new(
        &root.join("../.cache/smoke-models"),
        settings.use_layout,
    );
    downloads.ensure(settings.use_layout, |p| {
        println!("Models: {} {} / {}", p.status, p.downloaded, p.total)
    })?;
    let engine = Engine::default();
    let start = Instant::now();
    println!(
        "Engine: {}",
        engine.prepare(
            &resources,
            downloads.directory(),
            &store.root.join("logs"),
            &settings.device,
            &settings.model_id
        )?
    );
    for index in 0..store.project.pages.len() {
        let page = store.project.pages[index].clone();
        let (text, warning) = engine.recognize(store.root.join(&page.image), &settings)?;
        println!("PAGE {}: {}\nWARNING: {:?}", index + 1, text, warning);
        let p = store.page_mut(&page.id)?;
        p.raw_text = text.clone();
        let (normalized, w) = fastfileocr_core::table::normalize(&text, &settings.mode);
        p.edit(normalized);
        p.warning = warning.or(w);
        p.status = "done".into();
        p.recognized_with = Some(settings.clone());
        store.save_result(&page.id)?;
    }
    engine.stop();
    for format in ["md", "txt", "json", "html"] {
        export::save(
            &store.project,
            &store.root.join(format!("document.{format}")),
            format,
        )?;
    }
    println!(
        "Finished in {:.1}s. Results: {}",
        start.elapsed().as_secs_f32(),
        store.root.display()
    );
    Ok(())
}
