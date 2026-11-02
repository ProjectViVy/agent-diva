use crate::{
    process::OwnedChild,
    storage::{SendRecord, Store},
};
use serde_json::{Value, json};
use std::{
    collections::BTreeMap,
    fs,
    io::{BufRead, BufReader, Read, Write},
    path::PathBuf,
    process::{Command, Stdio},
    sync::{Arc, Mutex, mpsc},
    time::{Duration, Instant},
};

struct Inner {
    store: Option<Store>,
    child: Option<OwnedChild>,
    sender: Option<mpsc::SyncSender<Value>>,
    pending: BTreeMap<String, mpsc::Sender<Result<Value, String>>>,
    instance: Option<String>,
    sequence: u64,
    epoch: u64,
    closing: bool,
    error: Option<String>,
}

#[cfg(test)]
mod tests {
    use super::*;
    fn host(root: &std::path::Path) -> Arc<Bridge> {
        Bridge::new(root.into(), root.join("laputa"), |_, _| Ok(()))
    }
    fn until(mut predicate: impl FnMut() -> bool) {
        let end = Instant::now() + Duration::from_secs(10);
        while Instant::now() < end {
            if predicate() {
                return;
            }
            std::thread::sleep(Duration::from_millis(20));
        }
        panic!("native bridge condition timed out");
    }
    #[test]
    fn rejects_bad_requests_and_sequence_gaps() {
        let temp = tempfile::tempdir().unwrap();
        let h = host(temp.path());
        assert!(h.request(json!("snapshot")).unwrap_err().contains("object"));
        assert!(
            h.receive(0, json!({"v":1,"instance":"one","seq":2,"type":"ready"}))
                .unwrap_err()
                .contains("sequence gap")
        );
        h.receive(0, json!({"v":1,"instance":"one","seq":1,"type":"ready"}))
            .unwrap();
        assert!(
            h.receive(0, json!({"v":1,"instance":"two","seq":2,"type":"changed"}))
                .unwrap_err()
                .contains("instance")
        );
    }
    #[test]
    #[ignore = "requires explicit DIVA_TEST_CORE artifact"]
    fn native_workspace_resources_are_trusted_without_a_question() {
        let binary = PathBuf::from(std::env::var_os("DIVA_TEST_CORE").unwrap());
        let temp = tempfile::tempdir().unwrap();
        fs::create_dir_all(temp.path().join("workspace/.rig")).unwrap();
        fs::create_dir_all(temp.path().join("core-home")).unwrap();
        fs::write(
            temp.path().join("core-home/settings.toml"),
            "[trust]\ndefault='never'\n",
        )
        .unwrap();
        fs::write(
            temp.path().join("workspace/.rig/ASSISTANT.md"),
            "DIVA_TRUST_FIXTURE",
        )
        .unwrap();
        for setting in ["never", "ask"] {
            let events = Arc::new(Mutex::new(Vec::<Value>::new()));
            let captured = events.clone();
            let h = Bridge::new(temp.path().into(), temp.path().join("laputa"), move |_, v| {
                captured.lock().unwrap().push(v);
                Ok(())
            });
            fs::write(
                temp.path().join("core-home/settings.toml"),
                format!("[trust]\ndefault=\"{setting}\"\n"),
            )
            .unwrap();
            h.start_binary(binary.clone()).unwrap();
            let response = h.request(json!({"method":"prompts_read"})).unwrap();
            let operation = response["data"]["operation"].clone();
            until(|| {
                events.lock().unwrap().iter().any(|e| {
                    e["type"] == "management_completed" && e["data"]["operation"] == operation
                })
            });
            let result = events
                .lock()
                .unwrap()
                .iter()
                .find(|e| {
                    e["type"] == "management_completed" && e["data"]["operation"] == operation
                })
                .unwrap()
                .clone();
            assert!(
                result.to_string().contains("DIVA_TRUST_FIXTURE"),
                "{result}"
            );
            let snapshot = h.request(json!({"method":"snapshot"})).unwrap();
            assert!(
                snapshot["data"]["questions"].as_array().unwrap().is_empty(),
                "{snapshot}"
            );
            h.shutdown();
            events.lock().unwrap().clear();
        }
    }
    #[test]
    #[ignore = "requires explicit DIVA_TEST_CORE artifact"]
    fn native_outbox_failure_crash_reconcile_and_reopen() {
        let binary =
            PathBuf::from(std::env::var_os("DIVA_TEST_CORE").expect("explicit core artifact"));
        let temp = tempfile::tempdir().unwrap();
        let h = host(temp.path());
        assert_eq!(h.start_binary(binary.clone()).unwrap()["connected"], true);
        let saved_path = h.inner.lock().unwrap().store.as_ref().unwrap().path.clone();
        h.inner.lock().unwrap().store.as_mut().unwrap().path = temp.path().to_owned();
        let record = SendRecord {
            request_id: "bridge-stable".into(),
            text: "Synthetic bridge admission".into(),
        };
        assert!(h.submit(record.clone()).unwrap_err().contains("not sent"));
        assert_eq!(
            h.request(json!({"method":"admission","params":record}))
                .unwrap()["data"]["phase"],
            "missing"
        );
        h.inner.lock().unwrap().store.as_mut().unwrap().path = saved_path;
        let providers = h.request(json!({"method":"providers_read"})).unwrap();
        let config = json!({"default_model":"fixture/exact-model","instances":{"fixture":{"protocol":"openai","base_url":"http://127.0.0.1:9/v1","credential":{"source":"none"},"models":[{"id":"exact-model","context_window":32000,"max_output_tokens":1024}]}}});
        let saved=h.request(json!({"method":"providers_save","params":{"generation":providers["data"]["generation"],"config":config}})).unwrap();
        assert_eq!(saved["type"], "reply", "{saved}");
        until(|| {
            h.request(json!({"method":"snapshot"})).unwrap()["data"]["model"]["configured"] == true
        });
        let admitted = h.submit(record.clone()).unwrap();
        assert_eq!(admitted["type"], "reply", "{admitted}");
        until(|| h.status()["outbox"].as_array().unwrap().is_empty());
        let session = h
            .inner
            .lock()
            .unwrap()
            .store
            .as_ref()
            .unwrap()
            .value
            .session
            .clone();
        h.inner.lock().unwrap().child.as_ref().unwrap().kill();
        until(|| h.status()["connected"] == false);
        h.start_binary(binary).unwrap();
        assert_eq!(
            h.inner
                .lock()
                .unwrap()
                .store
                .as_ref()
                .unwrap()
                .value
                .session,
            session
        );
        let read = h
            .request(json!({"method":"admission","params":record}))
            .unwrap();
        assert_eq!(read["data"]["phase"], "committed");
        let duplicate = h.submit(record).unwrap();
        assert_eq!(duplicate["data"]["task"], read["data"]["task"]);
        assert!(h.status()["outbox"].as_array().unwrap().is_empty());
        h.shutdown();
    }
    #[test]
    #[ignore = "requires explicit DIVA_TEST_CORE artifact"]
    fn native_invalid_configuration_remains_manageable_without_replacement() {
        let binary =
            PathBuf::from(std::env::var_os("DIVA_TEST_CORE").expect("explicit core artifact"));
        let temp = tempfile::tempdir().unwrap();
        fs::create_dir_all(temp.path().join("core-home")).unwrap();
        let path = temp.path().join("core-home/settings.toml");
        fs::write(&path, "[broken\n").unwrap();
        let h = host(temp.path());
        h.start_binary(binary).unwrap();
        let view = h.request(json!({"method":"providers_read"})).unwrap();
        assert!(
            view["data"]["issue"]
                .as_str()
                .is_some_and(|s| s.contains("TOML") || s.contains("settings")),
            "{view}"
        );
        assert_eq!(fs::read_to_string(path).unwrap(), "[broken\n");
        h.shutdown();
    }
    #[test]
    #[ignore = "requires explicit DIVA_TEST_CORE artifact"]
    fn native_second_writer_is_rejected_with_session_cause() {
        let binary = PathBuf::from(std::env::var_os("DIVA_TEST_CORE").unwrap());
        let temp = tempfile::tempdir().unwrap();
        let first = host(temp.path());
        let second = host(temp.path());
        first.start_binary(binary.clone()).unwrap();
        let error = second.start_binary(binary).unwrap_err();
        assert!(error.to_ascii_lowercase().contains("session"), "{error}");
        assert_eq!(first.status()["connected"], true);
        second.shutdown();
        first.shutdown();
    }
}
pub struct Bridge {
    events: Box<dyn Fn(&str, Value) -> Result<(), String> + Send + Sync>,
    root: PathBuf,
    laputa_root: PathBuf,
    inner: Mutex<Inner>,
    redactor: crate::diagnostics::Redactor,
    log_lock: Mutex<()>,
}
impl Bridge {
    pub fn new(
        root: PathBuf,
        laputa_root: PathBuf,
        events: impl Fn(&str, Value) -> Result<(), String> + Send + Sync + 'static,
    ) -> Arc<Self> {
        Arc::new(Self {
            events: Box::new(events),
            root,
            laputa_root,
            redactor: crate::diagnostics::Redactor::environment(),
            log_lock: Mutex::new(()),
            inner: Mutex::new(Inner {
                store: None,
                child: None,
                sender: None,
                pending: BTreeMap::new(),
                instance: None,
                sequence: 0,
                epoch: 0,
                closing: false,
                error: None,
            }),
        })
    }
    pub fn persona_directory(&self) -> Result<PathBuf, String> {
        let directory = self.laputa_root.join("personas").join("main");
        fs::create_dir_all(&directory).map_err(|e| format!("create persona directory: {e}"))?;
        Ok(directory)
    }
    pub fn status(&self) -> Value {
        let inner = self.inner.lock().unwrap();
        json!({"connected":inner.instance.is_some(),"error":inner.error,"data_directory":self.root,
            "outbox":inner.store.as_ref().map(|s| s.value.outbox.values().cloned().collect::<Vec<_>>()).unwrap_or_default()})
    }
    pub fn diagnostics(&self) -> Result<String, String> {
        use std::io::{Seek, SeekFrom};
        let _lock = self.log_lock.lock().unwrap();
        let mut file = match fs::File::open(self.root.join("logs/desktop.log")) {
            Ok(f) => f,
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => return Ok(String::new()),
            Err(e) => return Err(e.to_string()),
        };
        let length = file.metadata().map_err(|e| e.to_string())?.len();
        file.seek(SeekFrom::Start(length.saturating_sub(16384)))
            .map_err(|e| e.to_string())?;
        let mut bytes = Vec::new();
        file.take(16384)
            .read_to_end(&mut bytes)
            .map_err(|e| e.to_string())?;
        Ok(self.redactor.text(&String::from_utf8_lossy(&bytes)))
    }
    pub fn start(self: &Arc<Self>) -> Result<Value, String> {
        let binary = std::env::current_exe()
            .map_err(|e| e.to_string())?
            .with_file_name("diva-core.exe");
        self.start_binary(binary)
    }
    // Only start() is exposed to IPC; tests provide their explicit build artifact.
    fn start_binary(self: &Arc<Self>, binary: PathBuf) -> Result<Value, String> {
        let mut inner = self.inner.lock().unwrap();
        if inner.closing {
            return Err("desktop is shutting down".into());
        }
        if inner.instance.is_some() {
            drop(inner);
            return Ok(self.status());
        }
        if inner.sender.is_some() {
            return Err("core is still starting".into());
        }
        inner.child.take();
        for name in [
            "core-home",
            "workspace",
            "desktop-state",
            "webview2",
            "logs",
        ] {
            fs::create_dir_all(self.root.join(name)).map_err(|e| e.to_string())?;
        }
        let store = Store::open(&self.root.join("desktop-state"))
            .map_err(|e| format!("desktop state: {e}"))?;
        self.redactor.stored_keys(&self.root.join("core-home"));
        if !binary.is_file() {
            return Err(format!("bundled core is missing: {}", binary.display()));
        }
        let mut command = Command::new(binary);
        command
            .arg("--diva")
            // Diva owns a fixed workspace; project resources are approved at launch.
            .arg("--approve")
            .current_dir(self.root.join("workspace"))
            .env("RIG_HOME", self.root.join("core-home"))
            .env("RIG_SESSION", &store.value.session)
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped());
        for (name, _) in std::env::vars_os() {
            let key = name.to_string_lossy().to_ascii_uppercase();
            if key.starts_with("VIVY_BOOTSTRAP_")
                || key.starts_with("VIVY_LAUNCHER_")
                || key.starts_with("RIG_") && !key.starts_with("RIG_SECRET_")
            {
                command.env_remove(name);
            }
        }
        command
            .env("RIG_HOME", self.root.join("core-home"))
            .env("RIG_SESSION", &store.value.session)
            .env("LAPUTA_HOME", &self.laputa_root);
        let mut owned =
            OwnedChild::spawn(&mut command).map_err(|e| format!("start bundled core: {e}"))?;
        let mut stdin = owned.child.stdin.take().unwrap();
        let stdout = owned.child.stdout.take().unwrap();
        let stderr = owned.child.stderr.take().unwrap();
        let (sender, receiver) = mpsc::sync_channel::<Value>(64);
        inner.epoch += 1;
        let epoch = inner.epoch;
        inner.store = Some(store);
        inner.child = Some(owned);
        inner.sender = Some(sender);
        inner.error = None;
        inner.sequence = 0;
        drop(inner);
        let this = self.clone();
        std::thread::spawn(move || {
            for frame in receiver {
                if let Err(e) = serde_json::to_writer(&mut stdin, &frame)
                    .map_err(std::io::Error::other)
                    .and_then(|_| stdin.write_all(b"\n"))
                    .and_then(|_| stdin.flush())
                {
                    this.fail(epoch, format!("core input pipe: {e}"));
                    break;
                }
            }
        });
        let tail = Arc::new(Mutex::new(String::new()));
        let captured = tail.clone();
        let redact = self.redactor.clone();
        let logger = self.clone();
        let (done_tx, done_rx) = mpsc::channel();
        std::thread::spawn(move || {
            let mut reader = BufReader::new(stderr);
            let mut bytes = Vec::new();
            loop {
                bytes.clear();
                match reader.by_ref().take(8192).read_until(b'\n', &mut bytes) {
                    Ok(0) | Err(_) => break,
                    _ => {}
                }
                let safe = redact.text(&String::from_utf8_lossy(&bytes));
                crate::diagnostics::append_tail(&mut captured.lock().unwrap(), &safe);
                logger.log(&safe);
            }
            let _ = done_tx.send(());
        });
        let this = self.clone();
        std::thread::spawn(move || {
            let mut reader = BufReader::new(stdout);
            loop {
                let mut bytes = Vec::new();
                match reader
                    .by_ref()
                    .take(4 * 1024 * 1024 + 1)
                    .read_until(b'\n', &mut bytes)
                {
                    Ok(0) => {
                        let _ = done_rx.recv_timeout(Duration::from_millis(250));
                        this.fail(
                            epoch,
                            format!(
                                "core disconnected; unconfirmed sends retained\n{}",
                                tail.lock().unwrap()
                            ),
                        );
                        break;
                    }
                    Ok(_) if bytes.len() <= 4 * 1024 * 1024 && bytes.ends_with(b"\n") => {}
                    Ok(_) => {
                        this.fail(epoch, "oversized or incomplete core frame".into());
                        break;
                    }
                    Err(e) => {
                        this.fail(epoch, format!("core output pipe: {e}"));
                        break;
                    }
                }
                match serde_json::from_slice::<Value>(&bytes) {
                    Ok(frame) => {
                        if let Err(e) = this.receive(epoch, frame) {
                            this.fail(epoch, e);
                            break;
                        }
                    }
                    Err(_) => {
                        this.fail(epoch, "invalid core protocol JSON".into());
                        break;
                    }
                }
            }
        });
        let deadline = Instant::now() + Duration::from_secs(20);
        while Instant::now() < deadline {
            {
                let inner = self.inner.lock().unwrap();
                if inner.instance.is_some() {
                    drop(inner);
                    self.reconcile()?;
                    return Ok(self.status());
                }
                if let Some(e) = &inner.error {
                    return Err(e.clone());
                }
            }
            std::thread::sleep(Duration::from_millis(20));
        }
        self.fail(epoch, "core handshake timed out".into());
        Err("core handshake timed out".into())
    }
    pub fn request(&self, mut request: Value) -> Result<Value, String> {
        if !request.is_object() || !request["method"].is_string() {
            return Err("core request must be an object with a method".into());
        }
        if request["method"] == "provider_key" {
            if let Some(key) = request["params"]["key"].as_str() {
                self.redactor.register(key);
            }
        }
        let id = uuid::Uuid::new_v4().to_string();
        request["id"] = json!(id);
        request["v"] = json!(1);
        if request["method"] == "submit" {
            return Err("use the durable desktop submit command".into());
        }
        self.send_and_wait(request)
    }
    pub fn submit(&self, record: SendRecord) -> Result<Value, String> {
        {
            let mut inner = self.inner.lock().unwrap();
            if inner.closing {
                return Err("desktop is shutting down".into());
            }
            if inner.instance.is_none() {
                return Err("core is disconnected".into());
            }
            inner
                .store
                .as_mut()
                .ok_or("desktop state is not open")?
                .admit(record.clone())
                .map_err(|e| format!("outbox write failed; request was not sent: {e}"))?;
        }
        self.send_and_wait(
            json!({"v":1,"id":uuid::Uuid::new_v4().to_string(),"method":"submit","params":record}),
        )
    }
    fn send_and_wait(&self, request: Value) -> Result<Value, String> {
        let id = request["id"]
            .as_str()
            .ok_or("request ID missing")?
            .to_owned();
        let (tx, rx) = mpsc::channel();
        {
            let mut inner = self.inner.lock().unwrap();
            if inner.closing && request["method"] != "shutdown" {
                return Err("desktop is shutting down".into());
            }
            if inner.instance.is_none() {
                return Err("core is disconnected".into());
            }
            if inner.pending.len() >= 64 {
                return Err("too many pending core requests".into());
            }
            let sender = inner
                .sender
                .as_ref()
                .ok_or("core input pipe disconnected")?
                .clone();
            inner.pending.insert(id.clone(), tx);
            if sender.try_send(request).is_err() {
                inner.pending.remove(&id);
                return Err(
                    "core request queue is full or disconnected; reconcile before retrying".into(),
                );
            }
        }
        let result = rx
            .recv_timeout(Duration::from_secs(30))
            .map_err(|_| "core response uncertain; reconcile before retrying".to_owned());
        self.inner.lock().unwrap().pending.remove(&id);
        result?
    }
    fn receive(&self, epoch: u64, frame: Value) -> Result<(), String> {
        let mut inner = self.inner.lock().unwrap();
        if epoch != inner.epoch {
            return Ok(());
        }
        if frame["v"] != 1 {
            return Err("incompatible core protocol".into());
        }
        let seq = frame["seq"].as_u64().ok_or("missing sequence")?;
        let instance = frame["instance"].as_str().ok_or("missing core instance")?;
        if seq != inner.sequence + 1 {
            return Err("core event sequence gap; reconnect and resynchronize".into());
        }
        if frame["type"] == "ready" && inner.instance.is_none() {
            inner.instance = Some(instance.into());
        }
        if inner.instance.as_deref() != Some(instance) {
            return Err("unexpected core instance".into());
        }
        inner.sequence = seq;
        let data = &frame["data"];
        if (frame["type"] == "admission_committed" || frame["type"] == "reply")
            && data["phase"] == "committed"
            && data["task"].is_string()
        {
            if let Some(key) = data["request_id"].as_str() {
                inner
                    .store
                    .as_mut()
                    .ok_or("missing outbox store")?
                    .acknowledge(key)
                    .map_err(|e| {
                        format!("acknowledgement could not be saved; retained outbox: {e}")
                    })?;
            }
        }
        if let Some(id) = frame["id"].as_str() {
            if let Some(reply) = inner.pending.remove(id) {
                let _ = reply.send(Ok(frame.clone()));
            }
        }
        drop(inner);
        (self.events)("core-frame", frame)?;
        Ok(())
    }
    fn reconcile(&self) -> Result<(), String> {
        let records = self
            .inner
            .lock()
            .unwrap()
            .store
            .as_ref()
            .unwrap()
            .value
            .outbox
            .values()
            .cloned()
            .collect::<Vec<_>>();
        for record in records {
            let found = self.request(json!({"method":"admission","params":record}))?;
            if found["type"] == "error" {
                return Err(format!(
                    "outbox reconciliation: {}",
                    found["data"]["message"].as_str().unwrap_or("failed")
                ));
            }
            if found["data"]["phase"] == "missing" {
                self.submit(record)?;
            }
        }
        Ok(())
    }
    fn fail(&self, epoch: u64, error: String) {
        let error = self.redactor.text(&error);
        let mut inner = self.inner.lock().unwrap();
        if epoch != inner.epoch {
            return;
        }
        inner.sender = None;
        inner.instance = None;
        if !inner.closing {
            inner.error = Some(error.clone());
        }
        for (_, sender) in std::mem::take(&mut inner.pending) {
            let _ = sender.send(Err(error.clone()));
        }
        if let Some(child) = inner.child.as_ref() {
            child.kill();
        }
        drop(inner);
        self.log(&error);
        let _ = (self.events)("core-disconnected", self.status());
    }
    pub fn shutdown(&self) {
        {
            let mut inner = self.inner.lock().unwrap();
            inner.closing = true;
            if let Some(sender) = &inner.sender {
                let _ = sender.try_send(json!({"v":1,"id":"desktop-shutdown","method":"shutdown"}));
            }
        }
        let deadline = Instant::now() + Duration::from_secs(10);
        while Instant::now() < deadline {
            let ended = {
                let mut inner = self.inner.lock().unwrap();
                inner
                    .child
                    .as_mut()
                    .is_none_or(|p| p.child.try_wait().ok().flatten().is_some())
            };
            if ended {
                self.inner.lock().unwrap().child.take();
                return;
            }
            std::thread::sleep(Duration::from_millis(25));
        }
        self.log("Core shutdown exceeded 10 seconds; terminated the owned process tree.");
        self.inner.lock().unwrap().child.take();
    }
    fn log(&self, message: &str) {
        let _lock = self.log_lock.lock().unwrap();
        let message = self.redactor.text(message);
        let path = self.root.join("logs/desktop.log");
        if fs::metadata(&path).is_ok_and(|m| m.len() > 256 * 1024) {
            let previous = self.root.join("logs/desktop.previous.log");
            let _ = fs::remove_file(&previous);
            let _ = fs::rename(&path, previous);
        }
        if let Ok(mut file) = fs::OpenOptions::new().create(true).append(true).open(path) {
            let _ = writeln!(file, "{message}");
        }
    }
}
