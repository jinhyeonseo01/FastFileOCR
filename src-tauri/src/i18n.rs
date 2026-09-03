use base64::Engine;
use std::sync::atomic::{AtomicU8, Ordering};
static LANGUAGE: AtomicU8 = AtomicU8::new(0);
pub fn set_language(language: &str) {
    LANGUAGE.store(
        match language {
            "ko" => 1,
            "ja" => 2,
            _ => 0,
        },
        Ordering::Relaxed,
    );
}
pub fn text(key: &str) -> String {
    format!("@i18n({key},)")
}
pub fn f(key: &str, args: &[String]) -> String {
    let encoded = base64::engine::general_purpose::URL_SAFE_NO_PAD
        .encode(serde_json::to_vec(args).unwrap_or_default());
    format!("@i18n({key},{encoded})")
}
pub fn translated(key: &str) -> String {
    let json = match LANGUAGE.load(Ordering::Relaxed) {
        1 => include_str!("../../locate/ko.json"),
        2 => include_str!("../../locate/ja.json"),
        _ => include_str!("../../locate/en.json"),
    };
    let values: serde_json::Value = serde_json::from_str(json).expect("Invalid locale");
    values[key].as_str().unwrap_or(key).to_string()
}
