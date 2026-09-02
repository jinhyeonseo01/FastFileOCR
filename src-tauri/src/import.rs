use crate::store::{err, id, Page, Result, Store};
use image::{codecs::jpeg::JpegEncoder, DynamicImage, GenericImageView, ImageDecoder, ImageReader};
use pdfium_render::prelude::*;
use std::{fs, path::Path};
fn save_jpeg(image: &DynamicImage, path: &Path) -> Result<()> {
    let mut rgb = image.to_rgb8();
    if image.color().has_alpha() {
        for (pixel, rgba) in rgb.pixels_mut().zip(image.to_rgba8().pixels()) {
            let a = rgba[3] as u16;
            for c in 0..3 {
                pixel[c] = ((rgba[c] as u16 * a + 255 * (255 - a)) / 255) as u8;
            }
        }
    }
    JpegEncoder::new_with_quality(fs::File::create(path).map_err(err)?, 95)
        .encode_image(&rgb)
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
    let image_path = format!("pages/{uid}.jpg");
    let thumb_path = format!("pages/{uid}-thumb.jpg");
    let (width, height) = image.dimensions();
    // Keep the entire page and its aspect ratio; never crop or tile.
    let scan = if width.max(height) > 4000 {
        image.resize(4000, 4000, image::imageops::FilterType::Lanczos3)
    } else {
        image
    };
    save_jpeg(&scan, &store.root.join(&image_path))?;
    save_jpeg(&scan.thumbnail(420, 420), &store.root.join(&thumb_path))?;
    store.project.pages.push(Page::new(
        name, source, number, image_path, thumb_path, width, height,
    ));
    store.save()
}
pub fn import_file(store: &mut Store, path: &Path, resources: &Path) -> Result<usize> {
    if store.project.pages.len() >= 1000 {
        return Err("작업당 최대 1,000페이지입니다. 새 작업을 만드세요.".into());
    }
    let extension = path
        .extension()
        .and_then(|s| s.to_str())
        .unwrap_or("")
        .to_lowercase();
    if !["pdf", "png", "jpg", "jpeg", "webp", "bmp"].contains(&extension.as_str()) {
        return Err("PDF, PNG, JPG, WEBP, BMP 파일을 추가하세요.".into());
    }
    let metadata = fs::metadata(path).map_err(err)?;
    let limit = if extension == "pdf" {
        1024 * 1024 * 1024
    } else {
        100 * 1024 * 1024
    };
    if !metadata.is_file() || metadata.len() > limit {
        return Err("파일이 너무 큽니다. 이미지 최대 100MB, PDF 최대 1GB입니다.".into());
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
        let pdfium = Pdfium::new(
            Pdfium::bind_to_library(resources.join("runtime/pdfium/pdfium.dll")).map_err(err)?,
        );
        let doc = pdfium
            .load_pdf_from_file(&store.root.join(&source), None)
            .map_err(|e| {
                format!("PDF를 열지 못했습니다. 암호화된 PDF는 암호를 해제한 뒤 추가하세요: {e}")
            })?;
        if count + doc.pages().len() as usize > 1000 {
            return Err("작업당 최대 1,000페이지입니다.".into());
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
        return Err("작업당 최대 1,000페이지입니다.".into());
    }
    let mut clipboard = arboard::Clipboard::new().map_err(err)?;
    let image = clipboard
        .get_image()
        .map_err(|_| "클립보드에 이미지가 없습니다. 캡처를 복사한 뒤 다시 시도하세요.")?;
    let buffer = image::RgbaImage::from_raw(
        image.width as u32,
        image.height as u32,
        image.bytes.into_owned(),
    )
    .ok_or("클립보드 이미지를 읽지 못했습니다.")?;
    let source = format!("sources/{}.png", id());
    let image = DynamicImage::ImageRgba8(buffer);
    image.save(store.root.join(&source)).map_err(err)?;
    add_image(store, image, "클립보드 캡처".into(), source, 1)
}
