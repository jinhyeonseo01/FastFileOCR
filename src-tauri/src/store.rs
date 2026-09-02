use crate::document::{blocks, Block};
use serde::{Deserialize, Serialize};
use std::{
    fs,
    io::Write,
    path::{Component, Path, PathBuf},
    time::{SystemTime, UNIX_EPOCH},
};
pub type Result<T> = std::result::Result<T, String>;
pub fn err(e: impl std::fmt::Display) -> String {
    e.to_string()
}
pub fn now() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_millis() as u64
}
pub fn id() -> String {
    uuid::Uuid::new_v4().to_string()
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Settings {
    pub mode: String,
    pub instructions: String,
    pub device: String,
    pub max_tokens: u32,
    #[serde(default)]
    pub use_layout: bool,
}
impl Default for Settings {
    fn default() -> Self {
        Self {
            mode: "document".into(),
            instructions: String::new(),
            device: "auto".into(),
            max_tokens: 8192,
            use_layout: false,
        }
    }
}
impl Settings {
    pub fn validate(&self) -> Result<()> {
        if !["text", "document", "table", "formula", "comic"].contains(&self.mode.as_str()) {
            return Err("지원하지 않는 인식 모드입니다.".into());
        }
        if !["auto", "cpu", "vulkan"].contains(&self.device.as_str()) {
            return Err("지원하지 않는 장치입니다.".into());
        }
        if !(512..=16384).contains(&self.max_tokens) {
            return Err("출력 토큰은 512~16384여야 합니다.".into());
        }
        if self.instructions.chars().count() > 4000 {
            return Err("지침은 4,000자 이내로 입력하세요.".into());
        }
        Ok(())
    }
    pub fn prompt(&self) -> String {
        let base = match self.mode.as_str() {
            "comic" => "OCR:\nRead this comic page. Transcribe all visible speech balloons, narration boxes, captions and sound effects in panel reading order. Separate panels and speakers when visually clear. Preserve original language and punctuation. Do not describe the artwork, translate, invent names, or add dialogue. Follow any user-specified reading direction. Output Markdown.",
            "table" => "Table Recognition:",
            "formula" => "Formula Recognition:",
            "document" => "OCR:\nPreserve the document reading order, headings, paragraphs, lists and tables. Output Markdown. Transcribe visible text without translating or inventing content.",
            _ => "OCR:",
        };
        if self.instructions.trim().is_empty() {
            base.into()
        } else {
            format!("{}\n{}", base, self.instructions.trim())
        }
    }
}
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Page {
    pub id: String,
    pub name: String,
    pub source: String,
    pub source_page: u32,
    pub image: String,
    pub thumbnail: String,
    pub width: u32,
    pub height: u32,
    pub status: String,
    pub raw_text: String,
    pub markdown: String,
    pub blocks: Vec<Block>,
    pub error: Option<String>,
    pub warning: Option<String>,
    pub elapsed_ms: u64,
    pub recognized_with: Option<Settings>,
    #[serde(default)]
    pub regions: Vec<crate::layout::Region>,
}
impl Page {
    pub fn new(
        name: String,
        source: String,
        source_page: u32,
        image: String,
        thumbnail: String,
        width: u32,
        height: u32,
    ) -> Self {
        Self {
            id: id(),
            name,
            source,
            source_page,
            image,
            thumbnail,
            width,
            height,
            status: "queued".into(),
            raw_text: String::new(),
            markdown: String::new(),
            blocks: vec![],
            error: None,
            warning: None,
            elapsed_ms: 0,
            recognized_with: None,
            regions: vec![],
        }
    }
    pub fn edit(&mut self, markdown: String) {
        self.blocks = blocks(&markdown);
        self.markdown = markdown;
    }
}
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Project {
    pub schema_version: u32,
    pub id: String,
    pub name: String,
    pub created_at: u64,
    pub updated_at: u64,
    pub settings: Settings,
    pub pages: Vec<Page>,
}
pub struct Store {
    pub root: PathBuf,
    pub project: Project,
}
pub fn atomic_write(path: &Path, data: &[u8]) -> Result<()> {
    let parent = path.parent().ok_or("잘못된 저장 경로입니다.")?;
    fs::create_dir_all(parent).map_err(err)?;
    let mut temp = tempfile::NamedTempFile::new_in(parent).map_err(err)?;
    temp.write_all(data).map_err(err)?;
    temp.as_file().sync_all().map_err(err)?;
    temp.persist(path).map_err(err)?;
    Ok(())
}
pub fn inside(root: &Path, relative: &str) -> Result<PathBuf> {
    let rel = Path::new(relative);
    if relative.is_empty() || rel.components().any(|c| !matches!(c, Component::Normal(_))) {
        return Err("프로젝트 내부 경로가 올바르지 않습니다.".into());
    }
    let target = root.join(rel);
    if target.exists()
        && !target
            .canonicalize()
            .map_err(err)?
            .starts_with(root.canonicalize().map_err(err)?)
    {
        return Err("프로젝트 밖의 파일에는 접근할 수 없습니다.".into());
    }
    Ok(target)
}
impl Store {
    pub fn create(parent: &Path, name: String) -> Result<Self> {
        let project = Project {
            schema_version: 2,
            id: id(),
            name: if name.trim().is_empty() {
                "새 문서".into()
            } else {
                name.trim().into()
            },
            created_at: now(),
            updated_at: now(),
            settings: Settings::default(),
            pages: vec![],
        };
        let root = parent.join(format!("Glyph-{}", &project.id[..8]));
        fs::create_dir_all(root.join("pages")).map_err(err)?;
        fs::create_dir_all(root.join("sources")).map_err(err)?;
        let mut store = Self { root, project };
        store.save()?;
        Ok(store)
    }
    pub fn open(root: PathBuf) -> Result<Self> {
        let read = |name: &str| -> Result<Project> {
            serde_json::from_slice(&fs::read(root.join(name)).map_err(err)?).map_err(err)
        };
        let mut project = read("project.json").or_else(|_| read("project.json.bak"))?;
        if project.schema_version != 2 {
            return Err("Glyph 2 작업 폴더를 선택하세요. 기존 v1 프로젝트는 원본 이미지를 새로 추가할 수 있습니다.".into());
        }
        project.settings.validate()?;
        for page in &mut project.pages {
            inside(&root, &page.image)?;
            inside(&root, &page.thumbnail)?;
            inside(&root, &page.source)?;
            if page.status == "processing" {
                page.status = "queued".into();
                page.error = None;
                page.warning = Some("중단된 페이지입니다. 스캔을 다시 시작하세요.".into());
            }
        }
        let mut store = Self { root, project };
        store.save()?;
        Ok(store)
    }
    pub fn save(&mut self) -> Result<()> {
        self.project.updated_at = now();
        let data = serde_json::to_vec_pretty(&self.project).map_err(err)?;
        let path = self.root.join("project.json");
        if let Ok(previous) = fs::read(&path) {
            if serde_json::from_slice::<Project>(&previous).is_ok() {
                atomic_write(&self.root.join("project.json.bak"), &previous)?;
            }
        }
        atomic_write(&path, &data)
    }
    pub fn page(&self, page_id: &str) -> Result<&Page> {
        self.project
            .pages
            .iter()
            .find(|p| p.id == page_id)
            .ok_or_else(|| "페이지를 찾지 못했습니다.".into())
    }
    pub fn page_mut(&mut self, page_id: &str) -> Result<&mut Page> {
        self.project
            .pages
            .iter_mut()
            .find(|p| p.id == page_id)
            .ok_or_else(|| "페이지를 찾지 못했습니다.".into())
    }
    pub fn save_result(&mut self, page_id: &str) -> Result<()> {
        let p = self.page(page_id)?;
        atomic_write(
            &self.root.join("results").join(format!("{}.md", p.id)),
            p.markdown.as_bytes(),
        )?;
        atomic_write(
            &self.root.join("results").join(format!("{}.json", p.id)),
            &serde_json::to_vec_pretty(p).map_err(err)?,
        )?;
        self.save()
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn restores_interrupted_queue_without_losing_completed_text() {
        let dir = tempfile::tempdir().unwrap();
        let mut s = Store::create(dir.path(), "복구".into()).unwrap();
        let mut p = Page::new(
            "a".into(),
            "sources/a.png".into(),
            1,
            "pages/a.jpg".into(),
            "pages/a-thumb.jpg".into(),
            100,
            100,
        );
        p.status = "processing".into();
        p.raw_text = "preserve".into();
        s.project.pages.push(p);
        s.save().unwrap();
        let reopened = Store::open(s.root).unwrap();
        assert_eq!(reopened.project.pages[0].status, "queued");
        assert_eq!(reopened.project.pages[0].raw_text, "preserve");
    }
    #[test]
    fn rejects_traversal_and_recovers_backup() {
        let dir = tempfile::tempdir().unwrap();
        let mut s = Store::create(dir.path(), "one".into()).unwrap();
        assert!(inside(&s.root, "../secret").is_err());
        s.project.name = "two".into();
        s.save().unwrap();
        fs::write(s.root.join("project.json"), "broken").unwrap();
        let recovered = Store::open(s.root).unwrap();
        assert_eq!(recovered.project.name, "one");
    }
    #[test]
    fn replaces_existing_file_atomically() {
        let dir = tempfile::tempdir().unwrap();
        let p = dir.path().join("한글.txt");
        atomic_write(&p, b"one").unwrap();
        atomic_write(&p, b"two").unwrap();
        assert_eq!(fs::read(p).unwrap(), b"two");
    }
}
