use crate::store::{err, Result};
use reqwest::{blocking::Client, header, StatusCode};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::{
    fs::{self, File, OpenOptions},
    io::{Read, Write},
    path::{Path, PathBuf},
    sync::{
        atomic::{AtomicU8, Ordering},
        Mutex,
    },
    thread,
    time::{Duration, Instant},
};

#[derive(Clone, Deserialize)]
pub struct ModelFile {
    pub name: String,
    pub bytes: u64,
    pub sha256: String,
    #[serde(default)]
    pub layout: bool,
    pub repository: Option<String>,
    pub revision: Option<String>,
    pub url: Option<String>,
}
#[derive(Clone, Deserialize)]
pub struct Manifest {
    pub repository: String,
    pub revision: String,
    pub files: Vec<ModelFile>,
}
pub fn manifest() -> Manifest {
    crate::models::get(crate::models::DEFAULT_MODEL)
        .unwrap()
        .manifest()
}
#[derive(Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Progress {
    pub kind: String,
    pub status: String,
    pub file: String,
    pub downloaded: u64,
    pub total: u64,
    pub bytes_per_second: u64,
    pub error: Option<String>,
}
pub struct Downloads {
    root: PathBuf,
    spec: Manifest,
    state: Mutex<Progress>,
    // 0: running, 1: pause requested, 2: cancel requested. Partial files survive all three.
    control: AtomicU8,
    paused_from: Mutex<String>,
}
impl Downloads {
    pub fn new(base: &Path, include_layout: bool) -> Self {
        Self::for_model(base, crate::models::DEFAULT_MODEL, include_layout)
            .expect("Default model exists")
    }
    pub fn for_model(base: &Path, model_id: &str, include_layout: bool) -> Result<Self> {
        let spec = crate::models::get(model_id)?.manifest();
        let root = base.join(&spec.revision);
        Ok(Self::from_spec(root, spec, include_layout, "model"))
    }
    pub(crate) fn for_files(root: PathBuf, files: Vec<ModelFile>) -> Self {
        Self::from_spec(
            root,
            Manifest {
                repository: String::new(),
                revision: String::new(),
                files,
            },
            true,
            "runtime",
        )
    }
    fn from_spec(root: PathBuf, full_spec: Manifest, include_layout: bool, kind: &str) -> Self {
        let mut spec = full_spec.clone();
        spec.files.retain(|f| !f.layout || include_layout);
        let total = spec.files.iter().map(|f| f.bytes).sum();
        let downloaded = spec
            .files
            .iter()
            .map(|f| {
                size(&root.join(&f.name))
                    .unwrap_or_else(|| size(&root.join(format!("{}.part", f.name))).unwrap_or(0))
                    .min(f.bytes)
            })
            .sum();
        let status = if spec
            .files
            .iter()
            .all(|f| size(&root.join(&f.name)) == Some(f.bytes))
        {
            "ready"
        } else if downloaded > 0 {
            "interrupted"
        } else {
            "idle"
        };
        Self {
            root,
            spec: full_spec,
            state: Mutex::new(Progress {
                kind: kind.into(),
                status: status.into(),
                file: String::new(),
                downloaded,
                total,
                bytes_per_second: 0,
                error: None,
            }),
            control: AtomicU8::new(0),
            paused_from: Mutex::new("downloading".into()),
        }
    }
    pub fn directory(&self) -> &Path {
        &self.root
    }
    pub fn snapshot(&self) -> Progress {
        self.state.lock().unwrap_or_else(|e| e.into_inner()).clone()
    }
    pub fn reset(&self) {
        self.control.store(0, Ordering::SeqCst);
    }
    pub fn pause(&self) {
        let status = self.snapshot().status;
        if ["downloading", "checking", "extracting"].contains(&status.as_str()) {
            *self.paused_from.lock().unwrap_or_else(|e| e.into_inner()) = status;
            if self
                .control
                .compare_exchange(0, 1, Ordering::SeqCst, Ordering::SeqCst)
                .is_ok()
            {
                self.update("pausing", None, None, 0);
            }
        }
    }
    pub fn resume(&self) {
        self.control
            .compare_exchange(1, 0, Ordering::SeqCst, Ordering::SeqCst)
            .ok();
    }
    pub fn cancel(&self) {
        self.control.store(2, Ordering::SeqCst);
    }
    fn update(&self, status: &str, file: Option<&str>, downloaded: Option<u64>, speed: u64) {
        let mut state = self.state.lock().unwrap_or_else(|e| e.into_inner());
        state.status = status.into();
        if let Some(f) = file {
            state.file = f.into();
        }
        if let Some(n) = downloaded {
            state.downloaded = n.min(state.total);
        }
        state.bytes_per_second = speed;
        state.error = None;
    }
    pub(crate) fn checkpoint(&self, notify: &impl Fn(Progress)) -> Result<()> {
        if self.control.load(Ordering::SeqCst) == 1 {
            self.update("paused", None, None, 0);
            notify(self.snapshot());
            while self.control.load(Ordering::SeqCst) == 1 {
                thread::sleep(Duration::from_millis(100));
            }
            if self.control.load(Ordering::SeqCst) == 0 {
                let stage = self
                    .paused_from
                    .lock()
                    .unwrap_or_else(|e| e.into_inner())
                    .clone();
                self.update(&stage, None, None, 0);
                notify(self.snapshot());
            }
        }
        if self.control.load(Ordering::SeqCst) == 2 {
            return Err(crate::i18n::text("downloadCancelled").into());
        }
        Ok(())
    }
    fn valid(&self, path: &Path, model: &ModelFile, notify: &impl Fn(Progress)) -> Result<bool> {
        if size(path) != Some(model.bytes) {
            return Ok(false);
        }
        let mut file = File::open(path).map_err(err)?;
        let mut hash = Sha256::new();
        let mut buffer = vec![0; 1024 * 1024];
        loop {
            self.checkpoint(notify)?;
            let n = file.read(&mut buffer).map_err(err)?;
            if n == 0 {
                break;
            }
            hash.update(&buffer[..n]);
        }
        Ok(hash
            .finalize()
            .iter()
            .map(|byte| format!("{byte:02x}"))
            .collect::<String>()
            == model.sha256)
    }
    pub(crate) fn stage(&self, status: &str, notify: &impl Fn(Progress)) {
        self.update(
            status,
            None,
            (status == "ready").then(|| self.snapshot().total),
            0,
        );
        notify(self.snapshot());
    }
    pub(crate) fn report_error(&self, result: &Result<()>, notify: &impl Fn(Progress)) {
        if let Err(e) = &result {
            let mut state = self.state.lock().unwrap_or_else(|e| e.into_inner());
            state.status = if self.control.load(Ordering::SeqCst) == 2 {
                "cancelled"
            } else {
                "error"
            }
            .into();
            state.error = Some(e.clone());
            state.bytes_per_second = 0;
            drop(state);
            notify(self.snapshot());
        }
    }
    pub fn ensure(&self, include_layout: bool, notify: impl Fn(Progress)) -> Result<()> {
        let result = self.ensure_inner(include_layout, &notify);
        self.report_error(&result, &notify);
        result
    }
    fn ensure_inner(&self, include_layout: bool, notify: &impl Fn(Progress)) -> Result<()> {
        fs::create_dir_all(&self.root).map_err(err)?;
        let mut spec = self.spec.clone();
        spec.files.retain(|f| !f.layout || include_layout);
        {
            let mut state = self.state.lock().map_err(err)?;
            state.total = spec.files.iter().map(|f| f.bytes).sum();
            state.downloaded = spec
                .files
                .iter()
                .map(|f| {
                    size(&self.root.join(&f.name))
                        .unwrap_or_else(|| {
                            size(&self.root.join(format!("{}.part", f.name))).unwrap_or(0)
                        })
                        .min(f.bytes)
                })
                .sum();
        }
        // Requests contain only pinned public model/runtime URLs, never document contents.
        let client = Client::builder()
            .connect_timeout(Duration::from_secs(15))
            .timeout(Duration::from_secs(30))
            .user_agent(concat!("FastFileOCR/", env!("CARGO_PKG_VERSION")))
            .build()
            .map_err(err)?;
        let mut complete = 0;
        for model in &spec.files {
            self.checkpoint(notify)?;
            self.update("checking", Some(&model.name), None, 0);
            notify(self.snapshot());
            let target = self.root.join(&model.name);
            if self.valid(&target, model, notify)? {
                complete += model.bytes;
                continue;
            }
            if target.exists() {
                fs::remove_file(&target).map_err(err)?;
            }
            let partial = self.root.join(format!("{}.part", &model.name));
            if size(&partial).is_some_and(|n| n > model.bytes) {
                fs::remove_file(&partial).map_err(err)?;
            }
            let mut offset = size(&partial).unwrap_or(0);
            let url = model.url.clone().unwrap_or_else(|| {
                format!(
                    "https://huggingface.co/{}/resolve/{}/{}",
                    model.repository.as_ref().unwrap_or(&spec.repository),
                    model.revision.as_ref().unwrap_or(&spec.revision),
                    model.name
                )
            });
            let mut failures = 0;
            while offset < model.bytes {
                self.checkpoint(notify)?;
                self.update(
                    "downloading",
                    Some(&model.name),
                    Some(complete + offset),
                    self.snapshot().bytes_per_second,
                );
                let previous_offset = offset;
                notify(self.snapshot());
                let attempt = self.transfer(
                    &client,
                    &url,
                    &partial,
                    model.bytes,
                    offset,
                    complete,
                    notify,
                );
                offset = size(&partial).unwrap_or(0);
                match attempt {
                    Ok(()) => failures = 0,
                    Err(e) => {
                        self.checkpoint(notify)?;
                        failures = if offset > previous_offset {
                            0
                        } else {
                            failures + 1
                        };
                        if failures >= 3 {
                            return Err(crate::i18n::f("downloadFailed", &[(e).to_string()]));
                        }
                        // Bounded backoff remains responsive to pause and cancel.
                        for _ in 0..10 * failures {
                            self.checkpoint(notify)?;
                            thread::sleep(Duration::from_millis(100));
                        }
                    }
                }
            }
            self.update(
                "checking",
                Some(&model.name),
                Some(complete + model.bytes),
                0,
            );
            notify(self.snapshot());
            if !self.valid(&partial, model, notify)? {
                fs::remove_file(&partial).map_err(err)?;
                self.update("error", None, Some(complete), 0);
                return Err(crate::i18n::text("modelHashMismatch").into());
            }
            fs::rename(&partial, &target).map_err(err)?;
            complete += model.bytes;
        }
        self.update("ready", Some(""), Some(complete), 0);
        notify(self.snapshot());
        Ok(())
    }
    fn transfer(
        &self,
        client: &Client,
        url: &str,
        path: &Path,
        total: u64,
        offset: u64,
        complete: u64,
        notify: &impl Fn(Progress),
    ) -> Result<()> {
        let start = Instant::now();
        let mut request = client.get(url).header(header::ACCEPT_ENCODING, "identity");
        let end = (offset + 32 * 1024 * 1024 - 1).min(total - 1);
        request = request.header(header::RANGE, format!("bytes={offset}-{end}"));
        let mut response = request.send().map_err(err)?;
        let resume = resume_offset(
            response.status(),
            response
                .headers()
                .get(header::CONTENT_RANGE)
                .and_then(|v| v.to_str().ok()),
            offset,
            total,
        )?;
        let mut file = OpenOptions::new()
            .create(true)
            .write(true)
            .truncate(resume == 0)
            .append(resume > 0)
            .open(path)
            .map_err(err)?;
        let expected = if response.status() == StatusCode::PARTIAL_CONTENT {
            response
                .headers()
                .get(header::CONTENT_RANGE)
                .and_then(|v| v.to_str().ok())
                .and_then(|v| v.split_once('/'))
                .and_then(|(v, _)| v.split_once('-'))
                .and_then(|(_, v)| v.parse::<u64>().ok())
                .ok_or("Invalid Content-Range")?
                + 1
        } else {
            total
        };
        let mut received = resume;
        let mut last = Instant::now();
        let mut buffer = vec![0; 256 * 1024];
        loop {
            if self.control.load(Ordering::SeqCst) != 0 {
                file.sync_all().map_err(err)?;
            }
            self.checkpoint(notify)?;
            let n = response.read(&mut buffer).map_err(err)?;
            if n == 0 {
                break;
            }
            if received + n as u64 > total {
                return Err(crate::i18n::text("modelTooLarge").into());
            }
            file.write_all(&buffer[..n]).map_err(err)?;
            received += n as u64;
            if last.elapsed() > Duration::from_millis(150) {
                let speed =
                    ((received - resume) as f64 / start.elapsed().as_secs_f64().max(0.1)) as u64;
                self.update("downloading", None, Some(complete + received), speed);
                notify(self.snapshot());
                last = Instant::now();
            }
        }
        file.sync_all().map_err(err)?;
        self.update(
            "downloading",
            None,
            Some(complete + received),
            ((received - resume) as f64 / start.elapsed().as_secs_f64().max(0.1)) as u64,
        );
        notify(self.snapshot());
        if received != expected {
            return Err(crate::i18n::text("downloadInterrupted").into());
        }
        Ok(())
    }
}
fn size(path: &Path) -> Option<u64> {
    fs::metadata(path)
        .ok()
        .filter(|m| m.is_file())
        .map(|m| m.len())
}
fn resume_offset(status: StatusCode, range: Option<&str>, offset: u64, total: u64) -> Result<u64> {
    if status == StatusCode::OK {
        return Ok(0);
    } // Server ignores Range: replace, never append a full response.
    if status != StatusCode::PARTIAL_CONTENT {
        return Err(crate::i18n::f("downloadStatus", &[(status).to_string()]));
    }
    let range = range
        .and_then(|s| s.strip_prefix("bytes "))
        .ok_or(crate::i18n::text("missingRange"))?;
    let (span, length) = range
        .split_once('/')
        .ok_or(crate::i18n::text("invalidRange"))?;
    let (start, end) = span
        .split_once('-')
        .ok_or(crate::i18n::text("invalidRangeSpan"))?;
    let start = start.parse::<u64>().map_err(err)?;
    let end = end.parse::<u64>().map_err(err)?;
    if start != offset
        || end >= total
        || end < start
        || length.parse::<u64>().map_err(err)? != total
    {
        return Err(crate::i18n::text("rangeMismatch").into());
    }
    Ok(offset)
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn range_resume_never_appends_a_full_response_or_wrong_range() {
        assert_eq!(
            resume_offset(StatusCode::PARTIAL_CONTENT, Some("bytes 5-9/10"), 5, 10).unwrap(),
            5
        );
        assert_eq!(resume_offset(StatusCode::OK, None, 5, 10).unwrap(), 0);
        assert!(resume_offset(StatusCode::PARTIAL_CONTENT, Some("bytes 0-9/10"), 5, 10).is_err());
        assert!(resume_offset(StatusCode::PARTIAL_CONTENT, Some("bytes 5-10/11"), 5, 10).is_err());
        assert!(resume_offset(StatusCode::FORBIDDEN, None, 0, 10).is_err());
    }
    #[test]
    fn restart_discovers_partial_files_without_treating_them_as_ready() {
        let dir = tempfile::tempdir().unwrap();
        let dl = Downloads::new(dir.path(), false);
        fs::create_dir_all(dl.directory()).unwrap();
        fs::write(
            dl.directory()
                .join(format!("{}.part", manifest().files[0].name)),
            b"partial",
        )
        .unwrap();
        let restored = Downloads::new(dir.path(), false);
        assert_eq!(restored.snapshot().status, "interrupted");
        assert_eq!(restored.snapshot().downloaded, 7);
        restored.cancel();
        assert!(restored.checkpoint(&|_| {}).is_err());
        assert_eq!(fs::read_dir(restored.directory()).unwrap().count(), 1);
    }

    #[test]
    fn runtime_download_resumes_a_partial_archive_over_http() {
        use std::net::TcpListener;
        let dir = tempfile::tempdir().unwrap();
        let data = b"verified GPU archive contents";
        let listener = TcpListener::bind("127.0.0.1:0").unwrap();
        let url = format!("http://{}/engine.zip", listener.local_addr().unwrap());
        let server = thread::spawn(move || {
            let (mut stream, _) = listener.accept().unwrap();
            stream
                .set_read_timeout(Some(Duration::from_secs(5)))
                .unwrap();
            let mut request = Vec::new();
            let mut byte = [0];
            while !request.ends_with(b"\r\n\r\n") {
                stream.read_exact(&mut byte).unwrap();
                request.push(byte[0]);
            }
            assert!(String::from_utf8_lossy(&request)
                .to_lowercase()
                .contains("range: bytes=7-"));
            write!(stream, "HTTP/1.1 206 Partial Content\r\nContent-Range: bytes 7-{}/{}\r\nContent-Length: {}\r\nConnection: close\r\n\r\n", data.len()-1, data.len(), data.len()-7).unwrap();
            stream.write_all(&data[7..]).unwrap();
        });
        fs::write(dir.path().join("engine.zip.part"), &data[..7]).unwrap();
        let spec = ModelFile {
            name: "engine.zip".into(),
            bytes: data.len() as u64,
            sha256: Sha256::digest(data)
                .iter()
                .map(|byte| format!("{byte:02x}"))
                .collect::<String>(),
            url: Some(url),
            layout: false,
            repository: None,
            revision: None,
        };
        let dl = Downloads::for_files(dir.path().into(), vec![spec]);
        assert_eq!(dl.snapshot().kind, "runtime");
        assert_eq!(dl.snapshot().status, "interrupted");
        dl.ensure(true, |_| {}).unwrap();
        server.join().unwrap();
        assert_eq!(fs::read(dir.path().join("engine.zip")).unwrap(), data);
        assert!(!dir.path().join("engine.zip.part").exists());
        assert_eq!(dl.snapshot().status, "ready");
    }
    #[test]
    fn pause_resumes_installation_stage_and_cancel_preserves_partials() {
        let dir = tempfile::tempdir().unwrap();
        let dl = Downloads::for_files(dir.path().into(), vec![]);
        fs::write(dir.path().join("engine.part"), b"keep").unwrap();
        dl.stage("extracting", &|_| {});
        dl.pause();
        dl.checkpoint(&|p| {
            if p.status == "paused" {
                dl.resume();
            }
        })
        .unwrap();
        assert_eq!(dl.snapshot().status, "extracting");
        dl.pause();
        assert!(dl
            .checkpoint(&|p| {
                if p.status == "paused" {
                    dl.cancel();
                }
            })
            .is_err());
        assert_eq!(fs::read(dir.path().join("engine.part")).unwrap(), b"keep");
        dl.stage("extracting", &|_| {});
        dl.pause();
        assert_eq!(dl.control.load(Ordering::SeqCst), 2);
        assert!(dl.checkpoint(&|_| {}).is_err());
    }
}
