//! Shared image transport for previews and OCR; preserve encoded bytes.
use crate::store::{err, Result};
use base64::Engine as _;
use std::{fs, path::Path};
pub(crate) fn data_url(path: &Path) -> Result<String> {
    let bytes = fs::read(path).map_err(err)?;
    let mime = image::guess_format(&bytes).map_err(err)?.to_mime_type();
    let data = base64::engine::general_purpose::STANDARD.encode(bytes);
    Ok(format!("data:{mime};base64,{data}"))
}
