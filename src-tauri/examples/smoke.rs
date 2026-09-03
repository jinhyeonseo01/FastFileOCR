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
fn resources() -> PathBuf {
    std::env::var_os("FASTFILEOCR_SMOKE_RESOURCES")
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("resources"))
}
fn execute() -> Result<(), String> {
    let args = std::env::args().skip(1).collect::<Vec<_>>();
    if args.first().is_some_and(|s| s == "--prepare-runtime") {
        let device = args.get(1).ok_or(
            "usage: smoke --prepare-runtime cpu|vulkan|cuda [DATA_DIR] [CANCEL_AFTER_BYTES]",
        )?;
        let base = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
        let data = args
            .get(2)
            .map(PathBuf::from)
            .unwrap_or_else(|| base.join("../.cache/runtime-smoke"));
        let runtime = fastfileocr_core::runtimes::Runtimes::new(&data);
        let cancel_after = args.get(3).and_then(|v| v.parse::<u64>().ok());
        println!(
            "Hardware: {:?}",
            fastfileocr_core::runtimes::hardware::Hardware::detect()
        );
        let directory = runtime.ensure(&resources(), device, |p| {
            println!("{} {} {}/{}", p.kind, p.status, p.downloaded, p.total);
            if p.status == "downloading" && cancel_after.is_some_and(|n| p.downloaded >= n) {
                runtime.cancel();
            }
        })?;
        println!("Ready: {}", directory.display());
        let mut command = std::process::Command::new(directory.join("llama-server.exe"));
        command.arg("--version").current_dir(&directory);
        #[cfg(windows)]
        {
            use std::os::windows::process::CommandExt;
            command.creation_flags(0x08000000);
        }
        let output = command.output().map_err(|e| e.to_string())?;
        println!(
            "{}{}",
            String::from_utf8_lossy(&output.stdout),
            String::from_utf8_lossy(&output.stderr)
        );
        if !output.status.success() {
            return Err("Runtime executable failed".into());
        }
        return Ok(());
    }
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
        .ok_or("usage: smoke INPUT [auto|cpu|vulkan|cuda] [text|document|table] [--layout]")?;
    let root = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let resources = resources();
    let mut store = Store::create(&root.join("../outputs/smoke"), "OCR validation".into())?;
    let added = import_file(&mut store, &PathBuf::from(input), &resources)?;
    println!("Imported {added} page(s).");
    let settings = Settings {
        device: args.get(1).cloned().unwrap_or("auto".into()),
        mode: args.get(2).cloned().unwrap_or("text".into()),
        max_tokens: 4096,
        use_layout: args.iter().any(|arg| arg == "--layout"),
        ..Settings::default()
    };
    let downloads = fastfileocr_core::download::Downloads::new(
        &std::env::var_os("FASTFILEOCR_SMOKE_MODELS")
            .map(PathBuf::from)
            .unwrap_or_else(|| root.join("../.cache/smoke-models")),
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
            &settings.model_id,
            &fastfileocr_core::runtimes::Runtimes::new(&root.join("../.cache/runtime-smoke")),
            |p| println!("Runtime: {} {} / {}", p.status, p.downloaded, p.total),
        )?
    );
    let mut detector = if settings.use_layout {
        Some(fastfileocr_core::layout::Detector::load(
            &resources,
            downloads.directory(),
        )?)
    } else {
        None
    };
    for index in 0..store.project.pages.len() {
        let page = store.project.pages[index].clone();
        let result = fastfileocr_core::recognition::recognize_page(
            &engine,
            &store.root.join(&page.image),
            &settings,
            detector.as_mut(),
            |_| {},
        )?;
        println!(
            "PAGE {}: {}\nREGIONS: {}\nWARNING: {:?}",
            index + 1,
            result.raw,
            result.regions.len(),
            result.warning
        );
        let p = store.page_mut(&page.id)?;
        p.raw_text = result.raw;
        p.edit(result.markdown);
        p.regions = result.regions;
        p.warning = result.warning;
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
