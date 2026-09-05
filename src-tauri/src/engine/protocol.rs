//! llama.cpp multimodal boundary; model details come from the selected adapter.
use crate::{
    models::RuntimeModel,
    store::{Result, Settings},
};
use serde_json::{json, Value};
use std::{path::Path, process::Command};

pub(super) fn configure_model(cmd: &mut Command, model: &RuntimeModel, device: &str) {
    cmd.arg("--model")
        .arg(&model.weights)
        .arg("--mmproj")
        .arg(&model.projector)
        .arg("--alias")
        .arg(model.alias)
        .arg("--ctx-size")
        .arg(model.context.to_string())
        .arg("--n-gpu-layers")
        .arg(if device == "cpu" { "0" } else { "99" })
        .arg("--jinja")
        .arg("--chat-template-file")
        .arg(&model.template);
    if device == "cpu" {
        cmd.arg("--no-mmproj-offload");
    }
}
pub(super) fn request(path: &Path, settings: &Settings) -> Result<Value> {
    let image_url = crate::images::data_url(path)?;
    Ok(json!({
        "model": "ocr", "temperature": 0,
        "max_tokens": crate::models::get(&settings.model_id)?.max_tokens(settings),
        "stream": false, "messages": [{"role":"user","content":[
            {"type":"image_url","image_url":{"url":image_url}},
            {"type":"text","text":settings.prompt()}
        ]}]
    }))
}
#[cfg(test)]
mod tests {
    use super::*;
    use base64::Engine as _;
    use std::fs;
    #[test]
    fn every_device_loads_the_adapters_weights_and_projector_together() {
        for descriptor in crate::models::descriptors() {
            let adapter = crate::models::get(descriptor.id).unwrap();
            let model = adapter.runtime(Path::new("app resources"), Path::new("model cache"));
            let manifest = adapter.manifest();
            for path in [&model.weights, &model.projector] {
                assert!(manifest
                    .files
                    .iter()
                    .any(|f| Some(f.name.as_str()) == path.file_name().and_then(|n| n.to_str())));
            }
            for device in ["cpu", "vulkan", "cuda"] {
                let mut cmd = Command::new("llama-server");
                configure_model(&mut cmd, &model, device);
                let args: Vec<_> = cmd.get_args().collect();
                for (flag, path) in [("--model", &model.weights), ("--mmproj", &model.projector)] {
                    let at = args.iter().position(|arg| *arg == flag).unwrap();
                    assert_eq!(args[at + 1], path.as_os_str());
                }
                assert_eq!(
                    args.contains(&std::ffi::OsStr::new("--no-mmproj-offload")),
                    device == "cpu"
                );
            }
        }
    }
    #[test]
    fn multimodal_payload_preserves_image_bytes_and_uses_matching_mime() {
        let dir = tempfile::tempdir().unwrap();
        for (format, mime) in [
            (image::ImageFormat::Png, "image/png"),
            (image::ImageFormat::Jpeg, "image/jpeg"),
        ] {
            let path = dir.path().join("image.bin");
            image::RgbImage::new(8, 8)
                .save_with_format(&path, format)
                .unwrap();
            let settings = Settings {
                mode: "text".into(),
                instructions: "Keep line breaks".into(),
                ..Settings::default()
            };
            let payload = request(&path, &settings).unwrap();
            let content = &payload["messages"][0]["content"];
            let url = content[0]["image_url"]["url"].as_str().unwrap();
            let encoded = url.strip_prefix(&format!("data:{mime};base64,")).unwrap();
            assert_eq!(
                base64::engine::general_purpose::STANDARD
                    .decode(encoded)
                    .unwrap(),
                fs::read(path).unwrap()
            );
            assert_eq!(content[1]["text"], settings.prompt());
        }
    }
}
