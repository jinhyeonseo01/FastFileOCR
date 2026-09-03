//! Model contracts. UI consumes descriptors; only adapters know prompts, weights and normalization.
mod paddle;
use crate::{
    download::Manifest,
    store::{Result, Settings},
};
use serde::Serialize;
use std::path::{Path, PathBuf};
pub const DEFAULT_MODEL: &str = "paddleocr-vl-1.6";
pub fn default_model() -> String {
    DEFAULT_MODEL.into()
}
#[derive(Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct OptionField {
    pub key: &'static str,
    pub label_key: &'static str,
    pub kind: &'static str,
    pub choices: Vec<serde_json::Value>,
    pub default: serde_json::Value,
    pub unit_key: Option<&'static str>,
    pub min: Option<f64>,
    pub max: Option<f64>,
    pub step: Option<f64>,
}
#[derive(Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Descriptor {
    pub id: &'static str,
    pub name: &'static str,
    pub description_key: &'static str,
    pub modes: Vec<&'static str>,
    pub devices: Vec<&'static str>,
    pub supports_layout: bool,
    pub fields: Vec<OptionField>,
}
pub struct RuntimeModel {
    pub weights: PathBuf,
    pub projector: PathBuf,
    pub template: PathBuf,
    pub alias: &'static str,
    pub context: u32,
}
pub trait ModelAdapter: Sync {
    fn descriptor(&self) -> Descriptor;
    fn manifest(&self) -> Manifest;
    fn runtime(&self, resources: &Path, models: &Path) -> RuntimeModel;
    fn prompt(&self, settings: &Settings) -> String;
    fn max_tokens(&self, settings: &Settings) -> u32;
    fn validate(&self, settings: &Settings) -> Result<()>;
    fn normalize(&self, raw: &str, mode: &str) -> (String, Option<String>);
    fn region_mode(&self, selected: &str, label: &str) -> String;
}
static PADDLE: paddle::Paddle = paddle::Paddle;
pub fn get(id: &str) -> Result<&'static dyn ModelAdapter> {
    match id {
        DEFAULT_MODEL => Ok(&PADDLE),
        _ => Err(crate::i18n::text("modelUnknown")),
    }
}
pub fn descriptors() -> Vec<Descriptor> {
    vec![PADDLE.descriptor()]
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn registry_settings_are_adapter_owned() {
        let mut settings = Settings::default();
        let adapter = get(&settings.model_id).unwrap();
        assert!(adapter.descriptor().devices.contains(&"cuda"));
        settings.model_options.insert(
            settings.model_id.clone(),
            serde_json::json!({"maxTokens":4096}),
        );
        assert_eq!(adapter.max_tokens(&settings), 4096);
        assert!(adapter.validate(&settings).is_ok());
        settings.model_options.insert(
            settings.model_id.clone(),
            serde_json::json!({"maxTokens":1}),
        );
        assert!(adapter.validate(&settings).is_err());
        assert!(get("missing").is_err());
    }
}
