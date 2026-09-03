use super::*;
pub struct Paddle;
impl ModelAdapter for Paddle {
    fn descriptor(&self) -> Descriptor {
        Descriptor {
            id: DEFAULT_MODEL,
            name: "PaddleOCR-VL 1.6",
            description_key: "paddleDescription",
            modes: vec!["document", "text", "table", "formula", "comic"],
            devices: vec!["auto", "cpu", "vulkan", "cuda"],
            supports_layout: true,
            fields: vec![OptionField {
                key: "maxTokens",
                label_key: "maxTokens",
                kind: "select",
                choices: vec![4096.into(), 8192.into(), 16384.into()],
                default: 8192.into(),
                unit_key: Some("tokens"),
                min: None,
                max: None,
                step: None,
            }],
        }
    }
    fn manifest(&self) -> Manifest {
        serde_json::from_str(include_str!("paddle/manifest.json")).expect("Invalid Paddle manifest")
    }
    fn runtime(&self, resources: &Path, models: &Path) -> RuntimeModel {
        RuntimeModel {
            weights: models.join("PaddleOCR-VL-1.6-GGUF.gguf"),
            projector: models.join("PaddleOCR-VL-1.6-GGUF-mmproj.gguf"),
            template: resources.join("chat-template.jinja"),
            alias: "ocr",
            context: 24576,
        }
    }
    fn prompt(&self, settings: &Settings) -> String {
        let prompts: serde_json::Value =
            serde_json::from_str(include_str!("paddle/prompts.json")).unwrap();
        let base = prompts[&settings.mode].as_str().unwrap_or("OCR:");
        if settings.instructions.trim().is_empty() {
            base.into()
        } else {
            format!("{base}\n{}", settings.instructions.trim())
        }
    }
    fn max_tokens(&self, settings: &Settings) -> u32 {
        settings
            .model_options
            .get(DEFAULT_MODEL)
            .and_then(|v| v["maxTokens"].as_u64())
            .map(|v| v.min(u32::MAX as u64) as u32)
            .unwrap_or(settings.max_tokens)
    }
    fn validate(&self, settings: &Settings) -> Result<()> {
        let d = self.descriptor();
        if !d.modes.contains(&settings.mode.as_str()) {
            return Err(crate::i18n::text("unsupportedMode"));
        }
        if !d.devices.contains(&settings.device.as_str()) {
            return Err(crate::i18n::text("unsupportedDevice"));
        }
        if !(512..=16384).contains(&self.max_tokens(settings)) {
            return Err(crate::i18n::text("invalidTokens"));
        }
        if settings.instructions.chars().count() > 4000 {
            return Err(crate::i18n::text("longInstructions"));
        }
        Ok(())
    }
    fn normalize(&self, raw: &str, mode: &str) -> (String, Option<String>) {
        crate::table::normalize(raw, mode)
    }
    fn region_mode(&self, selected: &str, label: &str) -> String {
        if selected == "document" {
            match label {
                "table" => "table",
                "formula" => "formula",
                _ => "text",
            }
            .into()
        } else {
            selected.into()
        }
    }
}
