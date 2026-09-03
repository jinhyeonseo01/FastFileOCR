use crate::store::{err, id, Page, Result, Store};
use image::{codecs::jpeg::JpegEncoder, DynamicImage, GenericImageView, ImageDecoder, ImageReader};
use pdfium_render::prelude::*;
use std::{fs, path::Path};
fn opaque_rgb(image: &DynamicImage) -> image::RgbImage {
    let mut rgb = image.to_rgb8();
    if image.color().has_alpha() {
        for (pixel, rgba) in rgb.pixels_mut().zip(image.to_rgba8().pixels()) {
            let a = rgba[3] as u16;
            for c in 0..3 {
                pixel[c] = ((rgba[c] as u16 * a + 255 * (255 - a)) / 255) as u8;
            }
        }
    }
    rgb
}
fn save_jpeg(image: &DynamicImage, path: &Path) -> Result<()> {
    JpegEncoder::new_with_quality(fs::File::create(path).map_err(err)?, 95)
        .encode_image(&opaque_rgb(image))
        .map_err(err)
}
fn add_image(
    store: &mut Store,
    image: DynamicImage,
    name: String,
    source: String,
    number: u32,
) -> Result<()> {
    let uid = id();
    let image_path = format!("pages/{uid}.png");
    let thumb_path = format!("pages/{uid}-thumb.jpg");
    let (width, height) = image.dimensions();
    // Keep the entire page and its aspect ratio; never crop or tile.
    let scan = if width.max(height) > 4000 {
        image.resize(4000, 4000, image::imageops::FilterType::Lanczos3)
    } else {
        image
    };
    // Preserve text edges in OCR input; only the UI thumbnail is lossy.
    opaque_rgb(&scan)
        .save_with_format(store.root.join(&image_path), image::ImageFormat::Png)
        .map_err(err)?;
    save_jpeg(&scan.thumbnail(420, 420), &store.root.join(&thumb_path))?;
    let (width, height) = scan.dimensions();
    store.project.pages.push(Page::new(
        name, source, number, image_path, thumb_path, width, height,
    ));
    store.save()
}
pub fn import_file(store: &mut Store, path: &Path, resources: &Path) -> Result<usize> {
    if store.project.pages.len() >= 1000 {
        return Err(crate::i18n::text("pageLimit").into());
    }
    let extension = path
        .extension()
        .and_then(|s| s.to_str())
        .unwrap_or("")
        .to_lowercase();
    if !["pdf", "png", "jpg", "jpeg", "webp", "bmp"].contains(&extension.as_str()) {
        return Err(crate::i18n::text("unsupportedFile").into());
    }
    let metadata = fs::metadata(path).map_err(err)?;
    let limit = if extension == "pdf" {
        1024 * 1024 * 1024
    } else {
        100 * 1024 * 1024
    };
    if !metadata.is_file() || metadata.len() > limit {
        return Err(crate::i18n::text("fileTooLarge").into());
    }
    let name = path
        .file_name()
        .unwrap_or_default()
        .to_string_lossy()
        .to_string();
    let source = format!("sources/{}.{}", id(), extension);
    fs::copy(path, store.root.join(&source)).map_err(err)?;
    let count = store.project.pages.len();
    if extension == "pdf" {
        static PDF_BINDING_LOCK: std::sync::Mutex<()> = std::sync::Mutex::new(());
        let _guard = PDF_BINDING_LOCK.lock().map_err(err)?;
        let pdfium = match Pdfium::bind_to_library(resources.join("runtime/pdfium/pdfium.dll")) {
            Ok(bindings) => Pdfium::new(bindings),
            Err(PdfiumError::PdfiumLibraryBindingsAlreadyInitialized) => Pdfium::default(),
            Err(error) => return Err(err(error)),
        };
        let doc = pdfium
            .load_pdf_from_file(&store.root.join(&source), None)
            .map_err(|e| crate::i18n::f("pdfOpenError", &[(e).to_string()]))?;
        if count + doc.pages().len() as usize > 1000 {
            return Err(crate::i18n::text("pageLimitShort").into());
        }
        for (index, page) in doc.pages().iter().enumerate() {
            let image = page
                .render_with_config(
                    &PdfRenderConfig::new()
                        .set_target_width(2400)
                        .set_maximum_height(4000),
                )
                .map_err(err)?
                .as_image()
                .map_err(err)?;
            add_image(
                store,
                image,
                format!("{name} · {}", index + 1),
                source.clone(),
                index as u32 + 1,
            )?;
        }
    } else {
        let mut reader = ImageReader::open(path)
            .map_err(err)?
            .with_guessed_format()
            .map_err(err)?;
        let mut limits = image::Limits::default();
        limits.max_alloc = Some(512 * 1024 * 1024);
        limits.max_image_width = Some(40000);
        limits.max_image_height = Some(40000);
        reader.limits(limits);
        let mut decoder = reader.into_decoder().map_err(err)?;
        let orientation = decoder.orientation().map_err(err)?;
        let mut image = DynamicImage::from_decoder(decoder).map_err(err)?;
        image.apply_orientation(orientation);
        add_image(store, image, name, source, 1)?;
    }
    Ok(store.project.pages.len() - count)
}
pub fn import_clipboard(store: &mut Store) -> Result<()> {
    if store.project.pages.len() >= 1000 {
        return Err(crate::i18n::text("pageLimitShort").into());
    }
    let mut clipboard = arboard::Clipboard::new().map_err(err)?;
    let image = clipboard
        .get_image()
        .map_err(|_| crate::i18n::text("clipboardEmpty"))?;
    let buffer = image::RgbaImage::from_raw(
        image.width as u32,
        image.height as u32,
        image.bytes.into_owned(),
    )
    .ok_or(crate::i18n::text("clipboardInvalid"))?;
    let source = format!("sources/{}.png", id());
    let image = DynamicImage::ImageRgba8(buffer);
    image.save(store.root.join(&source)).map_err(err)?;
    add_image(
        store,
        image,
        crate::i18n::translated("clipboardName"),
        source,
        1,
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn imported_scan_preserves_pixels_and_flattens_alpha_on_white() {
        let temporary = tempfile::tempdir().unwrap();
        let mut store = Store::create(temporary.path(), "Image import".into()).unwrap();
        let input = temporary.path().join("capture.png");
        let image = image::RgbaImage::from_fn(39, 23, |x, y| {
            image::Rgba([
                (x * 6) as u8,
                (y * 11) as u8,
                29,
                if x < 3 { 0 } else { 255 },
            ])
        });
        image.save(&input).unwrap();
        import_file(&mut store, &input, Path::new("no-resources")).unwrap();
        let page = &store.project.pages[0];
        let scan = image::open(store.root.join(&page.image)).unwrap().to_rgb8();
        assert_eq!(scan.dimensions(), image.dimensions());
        for (x, y, pixel) in scan.enumerate_pixels() {
            let original = image.get_pixel(x, y);
            let expected = if x < 3 {
                [255; 3]
            } else {
                [original[0], original[1], original[2]]
            };
            assert_eq!(pixel.0, expected);
        }
        assert_eq!(
            fs::read(store.root.join(&page.source)).unwrap(),
            fs::read(input).unwrap()
        );
    }

    #[test]
    #[cfg(windows)]
    #[ignore = "requires prepared PDFium; run npm run test:resources"]
    fn imports_more_than_one_pdf_in_the_same_process() {
        let root = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"));
        let resources = root.join("resources");
        let fixture = root.join("../docs/assets/sample-invoice.pdf");
        let temporary = tempfile::tempdir().unwrap();
        let mut store = Store::create(temporary.path(), "PDF import".into()).unwrap();
        assert_eq!(import_file(&mut store, &fixture, &resources).unwrap(), 2);
        assert_eq!(import_file(&mut store, &fixture, &resources).unwrap(), 2);
        assert_eq!(store.project.pages.len(), 4);
    }
}
