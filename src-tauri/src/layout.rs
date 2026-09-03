use crate::store::{err, id, Result};
use image::imageops::FilterType;
use ort::{session::Session, value::Tensor};
use serde::{Deserialize, Serialize};
use std::{collections::HashSet, fs, path::Path};

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Region {
    pub id: String,
    pub order: usize,
    pub label: String,
    /// [left, top, right, bottom], in the normalized page image's pixels.
    pub bbox: [f32; 4],
    pub confidence: f32,
    pub raw_text: String,
    pub markdown: String,
    pub ocr_mode: String,
    pub status: String,
    pub warning: Option<String>,
}
pub struct Detector {
    session: Session,
    labels: std::collections::HashMap<String, String>,
}
impl Detector {
    pub fn load(resources: &Path, models: &Path) -> Result<Self> {
        ort::init_from(resources.join("runtime/onnxruntime/onnxruntime.dll"))
            .map_err(err)?
            .commit();
        // The weight-free graph refers to the official safetensors file by relative byte offsets.
        let graph = models.join("layout.onnx");
        fs::copy(resources.join("layout/layout.onnx"), &graph).map_err(err)?;
        let session = Session::builder()
            .map_err(err)?
            .with_intra_threads(4)
            .map_err(err)?
            .commit_from_file(graph)
            .map_err(err)?;
        let labels =
            serde_json::from_slice(&fs::read(resources.join("layout/labels.json")).map_err(err)?)
                .map_err(err)?;
        Ok(Self { session, labels })
    }
    pub fn detect(&mut self, image: &image::DynamicImage) -> Result<Vec<Region>> {
        let resized = image
            .resize_exact(800, 800, FilterType::CatmullRom)
            .to_rgb8();
        let mut values = vec![0f32; 3 * 800 * 800];
        for (i, pixel) in resized.pixels().enumerate() {
            for c in 0..3 {
                values[c * 800 * 800 + i] = pixel[c] as f32 / 255.0;
            }
        }
        let input = Tensor::from_array(([1usize, 3, 800, 800], values)).map_err(err)?;
        let outputs = self
            .session
            .run(ort::inputs!["pixel_values"=>input])
            .map_err(err)?;
        let (shape, logits) = outputs["logits"].try_extract_tensor::<f32>().map_err(err)?;
        let (_, boxes) = outputs["pred_boxes"]
            .try_extract_tensor::<f32>()
            .map_err(err)?;
        let (_, order) = outputs["order_logits"]
            .try_extract_tensor::<f32>()
            .map_err(err)?;
        if shape.len() != 3 || shape[0] != 1 {
            return Err(crate::i18n::text("layoutShape").into());
        }
        let queries = shape[1] as usize;
        let classes = shape[2] as usize;
        postprocess(
            logits,
            boxes,
            order,
            queries,
            classes,
            image.width(),
            image.height(),
            &self.labels,
        )
    }
}
fn sigmoid(x: f32) -> f32 {
    1.0 / (1.0 + (-x).exp())
}
fn postprocess(
    logits: &[f32],
    boxes: &[f32],
    order: &[f32],
    queries: usize,
    classes: usize,
    width: u32,
    height: u32,
    labels: &std::collections::HashMap<String, String>,
) -> Result<Vec<Region>> {
    if queries == 0
        || classes == 0
        || logits.len() != queries * classes
        || boxes.len() != queries * 4
        || order.len() != queries * queries
    {
        return Err(crate::i18n::text("layoutTensor").into());
    }
    // Official PP-DocLayoutV3 pairwise reading-order vote: triu + complement(transpose).tril.
    let votes = (0..queries)
        .map(|i| {
            (0..queries)
                .filter(|&j| j != i)
                .map(|j| {
                    if j < i {
                        sigmoid(order[j * queries + i])
                    } else {
                        1.0 - sigmoid(order[i * queries + j])
                    }
                })
                .sum::<f32>()
        })
        .collect::<Vec<_>>();
    let mut candidates = logits
        .iter()
        .enumerate()
        .map(|(index, &v)| (index, sigmoid(v)))
        .filter(|(_, s)| s.is_finite())
        .collect::<Vec<_>>();
    candidates.sort_by(|a, b| b.1.total_cmp(&a.1));
    candidates.truncate(queries);
    let mut seen = HashSet::new();
    let mut regions = Vec::new();
    for (index, score) in candidates {
        if score < 0.5 {
            continue;
        }
        let query = index / classes;
        if !seen.insert(query) {
            continue;
        }
        let b = &boxes[query * 4..query * 4 + 4];
        if !b.iter().all(|v| v.is_finite()) {
            continue;
        }
        let bbox = [
            ((b[0] - b[2] / 2.0) * width as f32).clamp(0.0, width as f32),
            ((b[1] - b[3] / 2.0) * height as f32).clamp(0.0, height as f32),
            ((b[0] + b[2] / 2.0) * width as f32).clamp(0.0, width as f32),
            ((b[1] + b[3] / 2.0) * height as f32).clamp(0.0, height as f32),
        ];
        if bbox[2] - bbox[0] < 2.0 || bbox[3] - bbox[1] < 2.0 {
            continue;
        }
        regions.push((
            votes[query],
            Region {
                id: id(),
                order: 0,
                label: labels
                    .get(&(index % classes).to_string())
                    .cloned()
                    .unwrap_or("text".into()),
                bbox,
                confidence: score,
                raw_text: String::new(),
                markdown: String::new(),
                ocr_mode: String::new(),
                status: "queued".into(),
                warning: None,
            },
        ));
    }
    regions.sort_by(|a, b| a.0.total_cmp(&b.0));
    Ok(regions
        .into_iter()
        .enumerate()
        .map(|(i, (_, mut r))| {
            r.order = i + 1;
            r
        })
        .collect())
}
pub fn crop(image: &image::DynamicImage, bbox: [f32; 4]) -> image::DynamicImage {
    let x = (bbox[0].floor().max(0.0) as u32)
        .saturating_sub(2)
        .min(image.width() - 1);
    let y = (bbox[1].floor().max(0.0) as u32)
        .saturating_sub(2)
        .min(image.height() - 1);
    let right = (bbox[2].ceil() as u32).saturating_add(2).min(image.width());
    let bottom = (bbox[3].ceil() as u32)
        .saturating_add(2)
        .min(image.height());
    image.crop_imm(
        x,
        y,
        right.saturating_sub(x).max(1),
        bottom.saturating_sub(y).max(1),
    )
}
pub fn contains(outer: &Region, inner: &Region) -> bool {
    outer.id != inner.id
        && inner.bbox[0] >= outer.bbox[0]
        && inner.bbox[1] >= outer.bbox[1]
        && inner.bbox[2] <= outer.bbox[2]
        && inner.bbox[3] <= outer.bbox[3]
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn coordinates_and_order_follow_detector_not_text_order() {
        let labels = std::collections::HashMap::from([("0".into(), "text".into())]);
        let r = postprocess(
            &[5.0, 5.0],
            &[0.25, 0.25, 0.4, 0.4, 0.75, 0.75, 0.4, 0.4],
            &[0.0, 5.0, 0.0, 0.0],
            2,
            1,
            100,
            200,
            &labels,
        )
        .unwrap();
        assert_eq!(r.len(), 2);
        assert!(r[0].bbox[0] < 10.0);
        assert!(r[1].bbox[0] > 50.0);
        assert!(r[0].confidence > 0.99);
        assert_eq!(r[1].order, 2);
        assert!(r[0].raw_text.is_empty());
    }
}
