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
        model_id: &str,
        runtimes: &crate::runtimes::Runtimes,
        notify: impl Fn(crate::download::Progress),
    ) -> Result<String> {
        self.stop();
        let hardware = if device == "cpu" {
            crate::runtimes::hardware::Hardware::default()
        } else {
            crate::runtimes::hardware::Hardware::detect()
        };
        let variants = hardware.candidates(device);
        let mut failures = Vec::new();
        for variant in variants {
            if self.cancelled() {
                return Err(crate::i18n::text("cancelledOperation").into());
            }
            let result = (|| {
                if (variant == "cuda" && !hardware.cuda)
                    || (variant == "vulkan" && !hardware.vulkan)
                {
                    return Err(crate::i18n::f("runtimeUnavailable", &[variant.into()]));
                }
                let directory = runtimes.ensure(resources, variant, &notify)?;
                if self.cancelled() {
                    return Err(crate::i18n::text("cancelledOperation"));
                }
                self.launch(resources, models, log_dir, variant, model_id, &directory)
            })();
            match result {
                Ok(()) => return Ok(variant.into()),
                Err(e) => {
                    self.stop();
                    failures.push(format!("{variant}: {e}"));
                }
            }
        }
        Err(crate::i18n::f(
            "engineStartError",
            &[
                (failures.join("\n")).to_string(),
                (log_dir.display()).to_string(),
            ],
        ))
    }
    fn launch(
        &self,
        resources: &Path,
        models: &Path,
        log_dir: &Path,
        device: &str,
        model_id: &str,
        directory: &Path,
    ) -> Result<()> {
        let binary = directory.join("llama-server.exe");
        let runtime = crate::models::get(model_id)?.runtime(resources, models);
        let model = runtime.weights;
        let projector = runtime.projector;
        for path in [&binary, &model, &projector] {
            if !path.is_file() {
                return Err(crate::i18n::f(
                    "missingResource",
                    &[(path.display()).to_string()],
                ));
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
            .arg(runtime.alias)
            .arg("--ctx-size")
            .arg(runtime.context.to_string())
            .arg("--parallel")
            .arg("1")
            .arg("--n-gpu-layers")
            .arg(if device == "cpu" { "0" } else { "99" })
            .arg("--jinja")
            .arg("--chat-template-file")
            .arg(runtime.template)
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
                return Err(crate::i18n::text("cancelledOperation").into());
            }
            let mut child = cmd.spawn().map_err(err)?;
            #[cfg(windows)]
            let job = match Job::assign(&child) {
                Ok(j) => j,
                Err(e) => {
                    let _ = child.kill();
                    let _ = child.wait();
                    return Err(crate::i18n::f("processLifetime", &[(e).to_string()]));
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
                return Err(crate::i18n::text("cancelledOperation").into());
            }
            {
                let mut slot = self.server.lock().map_err(err)?;
                let server = slot.as_mut().ok_or(crate::i18n::text("engineStopped"))?;
                if let Some(code) = server.child.try_wait().map_err(err)? {
                    return Err(crate::i18n::f("processExit", &[(code).to_string()]));
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
                return Err(crate::i18n::text("loadTimeout").into());
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
            return Err(crate::i18n::text("cancelledOperation").into());
        }
        let (port, key) = {
            let slot = self.server.lock().map_err(err)?;
            let server = slot.as_ref().ok_or(crate::i18n::text("engineNotRunning"))?;
            (server.port, server.key.clone())
        };
        let data = base64::engine::general_purpose::STANDARD.encode(fs::read(image).map_err(err)?);
        let payload = json!({
            "model": "ocr", "temperature": 0, "max_tokens": crate::models::get(&settings.model_id)?.max_tokens(settings),
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
            return Err(crate::i18n::f(
                "ocrError",
                &[
                    (status).to_string(),
                    (body["error"]["message"].as_str().unwrap_or("runtime_error")).to_string(),
                ],
            ));
        }
        let choice = &body["choices"][0];
        let text = choice["message"]["content"]
            .as_str()
            .ok_or(crate::i18n::text("emptyResponse"))?
            .to_string();
        let reason = choice["finish_reason"].as_str().unwrap_or("");
        let warning = if reason == "length" {
            Some(crate::i18n::text("outputTruncated").into())
        } else if text.trim().is_empty() {
            Some(crate::i18n::text("emptyRecognition").into())
        } else {
            None
        };
        Ok((text, warning))
    }
}
