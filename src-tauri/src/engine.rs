use crate::store::{err, id, Result, Settings};
use base64::Engine as _;
use reqwest::blocking::Client;
use serde_json::{json, Value};
use std::{
    fs,
    net::TcpListener,
    path::{Path, PathBuf},
    process::{Child, Command, Stdio},
    sync::{
        atomic::{AtomicBool, Ordering},
        Mutex,
    },
    thread,
    time::{Duration, Instant},
};

#[cfg(windows)]
struct Job(windows_sys::Win32::Foundation::HANDLE);
#[cfg(windows)]
unsafe impl Send for Job {}
#[cfg(windows)]
impl Job {
    fn assign(child: &Child) -> Result<Self> {
        use std::os::windows::io::AsRawHandle;
        use windows_sys::Win32::{Foundation::CloseHandle, System::JobObjects::*};
        unsafe {
            let handle = CreateJobObjectW(std::ptr::null(), std::ptr::null());
            if handle.is_null() {
                return Err(std::io::Error::last_os_error().to_string());
            }
            let mut info: JOBOBJECT_EXTENDED_LIMIT_INFORMATION = std::mem::zeroed();
            info.BasicLimitInformation.LimitFlags = JOB_OBJECT_LIMIT_KILL_ON_JOB_CLOSE;
            if SetInformationJobObject(
                handle,
                JobObjectExtendedLimitInformation,
                &info as *const _ as _,
                std::mem::size_of_val(&info) as u32,
            ) == 0
                || AssignProcessToJobObject(handle, child.as_raw_handle()) == 0
            {
                CloseHandle(handle);
                return Err(std::io::Error::last_os_error().to_string());
            }
            Ok(Self(handle))
        }
    }
}
#[cfg(windows)]
impl Drop for Job {
    fn drop(&mut self) {
        unsafe {
            windows_sys::Win32::Foundation::CloseHandle(self.0);
        }
    }
}

pub struct Server {
    child: Child,
    pub port: u16,
    pub key: String,
    pub device: String,
    #[cfg(windows)]
    _job: Job,
}
impl Drop for Server {
    fn drop(&mut self) {
        let _ = self.child.kill();
        let _ = self.child.wait();
    }
}
pub struct Engine {
    pub server: Mutex<Option<Server>>,
    pub cancel: AtomicBool,
}
impl Default for Engine {
    fn default() -> Self {
        Self {
            server: Mutex::new(None),
            cancel: AtomicBool::new(false),
        }
    }
}
impl Engine {
    pub fn stop(&self) {
        if let Ok(mut server) = self.server.lock() {
            *server = None;
        }
    }
    pub fn cancel(&self) {
        self.cancel.store(true, Ordering::SeqCst);
        self.stop();
    }
    pub fn cancelled(&self) -> bool {
        self.cancel.load(Ordering::SeqCst)
    }
    pub fn prepare(
        &self,
        resources: &Path,
        models: &Path,
        log_dir: &Path,
        device: &str,
    ) -> Result<String> {
        self.stop();
        let variants = if device == "auto" {
            vec!["vulkan", "cpu"]
        } else {
            vec![device]
        };
        let mut failures = Vec::new();
        for variant in variants {
            if self.cancelled() {
                return Err("취소됨".into());
            }
            match self.launch(resources, models, log_dir, variant) {
                Ok(()) => {
                    return Ok(if failures.is_empty() {
                        variant.into()
                    } else {
                        "cpu (GPU 시작 실패 후 전환)".into()
                    })
                }
                Err(e) => {
                    self.stop();
                    failures.push(format!("{variant}: {e}"));
                }
            }
        }
        Err(format!(
            "OCR 엔진을 시작하지 못했습니다. {}\n로그: {}",
            failures.join("\n"),
            log_dir.display()
        ))
    }
    fn launch(&self, resources: &Path, models: &Path, log_dir: &Path, device: &str) -> Result<()> {
        let binary = resources.join(format!("runtime/{device}/llama-server.exe"));
        let model = models.join("PaddleOCR-VL-1.6-GGUF.gguf");
        let projector = models.join("PaddleOCR-VL-1.6-GGUF-mmproj.gguf");
        for path in [&binary, &model, &projector] {
            if !path.is_file() {
                return Err(format!("동봉 파일을 찾을 수 없습니다: {}", path.display()));
            }
        }
        let listener = TcpListener::bind(("127.0.0.1", 0)).map_err(err)?;
        let port = listener.local_addr().map_err(err)?.port();
        let key = id();
        fs::create_dir_all(log_dir).map_err(err)?;
        let log = fs::File::create(log_dir.join(format!("llama-{device}.log"))).map_err(err)?;
        let mut cmd = Command::new(&binary);
        cmd.current_dir(binary.parent().unwrap())
            .arg("--model")
            .arg(model)
            .arg("--mmproj")
            .arg(projector)
            .arg("--host")
            .arg("127.0.0.1")
            .arg("--port")
            .arg(port.to_string())
            .arg("--api-key")
            .arg(&key)
            .arg("--alias")
            .arg("paddleocr")
            .arg("--ctx-size")
            .arg("24576")
            .arg("--parallel")
            .arg("1")
            .arg("--n-gpu-layers")
            .arg(if device == "cpu" { "0" } else { "99" })
            .arg("--jinja")
            .arg("--chat-template-file")
            .arg(resources.join("chat-template.jinja"))
            .arg("--temp")
            .arg("0")
            .arg("--no-webui")
            .stdin(Stdio::null())
            .stdout(Stdio::from(log.try_clone().map_err(err)?))
            .stderr(Stdio::from(log));
        if device == "cpu" {
            cmd.arg("--no-mmproj-offload");
        }
        #[cfg(windows)]
        {
            use std::os::windows::process::CommandExt;
            cmd.creation_flags(0x08000000);
        }
        drop(listener);
        {
            let mut slot = self.server.lock().map_err(err)?;
            if self.cancelled() {
                return Err("취소됨".into());
            }
            let mut child = cmd.spawn().map_err(err)?;
            #[cfg(windows)]
            let job = match Job::assign(&child) {
                Ok(j) => j,
                Err(e) => {
                    let _ = child.kill();
                    let _ = child.wait();
                    return Err(format!("프로세스 수명 관리 실패: {e}"));
                }
            };
            *slot = Some(Server {
                child,
                port,
                key: key.clone(),
                device: device.into(),
                #[cfg(windows)]
                _job: job,
            });
        }
        let client = Client::builder()
            .no_proxy()
            .timeout(Duration::from_secs(2))
            .build()
            .map_err(err)?;
        let deadline = Instant::now() + Duration::from_secs(180);
        loop {
            if self.cancelled() {
                return Err("취소됨".into());
            }
            {
                let mut slot = self.server.lock().map_err(err)?;
                let server = slot.as_mut().ok_or("엔진이 종료되었습니다.")?;
                if let Some(code) = server.child.try_wait().map_err(err)? {
                    return Err(format!("프로세스 종료 ({code}). 로그 파일을 확인하세요."));
                }
            }
            if let Ok(response) = client
                .get(format!("http://127.0.0.1:{port}/health"))
                .bearer_auth(&key)
                .send()
            {
                if response.status().is_success() {
                    return Ok(());
                }
            }
            if Instant::now() > deadline {
                return Err("모델 로딩 제한 시간(180초)을 초과했습니다.".into());
            }
            thread::sleep(Duration::from_millis(250));
        }
    }
    pub fn recognize(
        &self,
        image: PathBuf,
        settings: &Settings,
    ) -> Result<(String, Option<String>)> {
        if self.cancelled() {
            return Err("취소됨".into());
        }
        let (port, key) = {
            let slot = self.server.lock().map_err(err)?;
            let server = slot.as_ref().ok_or("OCR 엔진이 실행 중이 아닙니다.")?;
            (server.port, server.key.clone())
        };
        let data = base64::engine::general_purpose::STANDARD.encode(fs::read(image).map_err(err)?);
        let payload = json!({
            "model": "paddleocr", "temperature": 0, "max_tokens": settings.max_tokens,
            "stream": false, "messages": [{"role":"user","content":[
                {"type":"image_url","image_url":{"url":format!("data:image/jpeg;base64,{data}")}},
                {"type":"text","text":settings.prompt()}
            ]}]
        });
        let client = Client::builder()
            .no_proxy()
            .connect_timeout(Duration::from_secs(5))
            .timeout(Duration::from_secs(900))
            .build()
            .map_err(err)?;
        let response = client
            .post(format!("http://127.0.0.1:{port}/v1/chat/completions"))
            .bearer_auth(key)
            .json(&payload)
            .send()
            .map_err(err)?;
        let status = response.status();
        let body: Value = response.json().map_err(err)?;
        if !status.is_success() {
            return Err(format!(
                "OCR 오류 ({status}): {}",
                body["error"]["message"]
                    .as_str()
                    .unwrap_or("런타임 응답 오류")
            ));
        }
        let choice = &body["choices"][0];
        let text = choice["message"]["content"]
            .as_str()
            .ok_or("OCR 응답에 텍스트가 없습니다.")?
            .to_string();
        let reason = choice["finish_reason"].as_str().unwrap_or("");
        let warning = if reason == "length" {
            Some("출력 한도에 도달했습니다. 일부 내용이 누락됐을 수 있습니다. 토큰 한도를 늘려 다시 스캔하세요.".into())
        } else if text.trim().is_empty() {
            Some("인식된 문자가 없습니다. 원본과 인식 모드를 확인하세요.".into())
        } else {
            None
        };
        Ok((text, warning))
    }
}
