//! Private stdio transport. No arbitrary RPC, executable, path or model from the WebView.
use serde_json::{json, Value};
use std::{
    collections::VecDeque,
    io::{BufRead, BufReader, Read, Write},
    path::Path,
    process::{Child, ChildStdin, Command, Stdio},
    sync::{mpsc, Arc, Mutex},
    time::{Duration, Instant},
};
use tempfile::TempDir;

pub const MODEL: &str = "gpt-5.6-sol";
pub const CONFIG: &str = include_str!("../../chat-runtime/config.toml");
const MODELS: &str = include_str!("../../chat-runtime/models.json");
const FAILED: &str = "Die Verbindung zu ChatGPT wurde unterbrochen. Bitte erneut anmelden.";
pub type Process = Arc<Mutex<Child>>;

pub struct Runtime {
    pub process: Process,
    input: ChildStdin,
    output: mpsc::Receiver<Value>,
    events: VecDeque<Value>,
    next: u64,
    pub session: (String, u64),
    pub login_id: Option<String>,
    home: TempDir,
    thread_id: Option<String>,
}

impl Runtime {
    pub fn start(
        binary: &Path,
        session: (String, u64),
        credentials_home: Option<&Path>,
    ) -> Result<Self, String> {
        let home = tempfile::Builder::new()
            .prefix("finanzblick-chat-")
            .tempdir()
            .map_err(|_| FAILED)?;
        let codex_home = credentials_home.unwrap_or(home.path());
        std::fs::create_dir_all(codex_home).map_err(|_| FAILED)?;
        let auth_store = if credentials_home.is_some() {
            "keyring"
        } else {
            "ephemeral"
        };
        let models = home.path().join("models.json");
        std::fs::write(&models, MODELS).map_err(|_| FAILED)?;
        let config = format!(
            "model_catalog_json = {}\n{}",
            serde_json::to_string(&models).map_err(|_| FAILED)?,
            CONFIG.replace(
                r#"cli_auth_credentials_store = "ephemeral""#,
                &format!(r#"cli_auth_credentials_store = "{auth_store}""#)
            )
        );
        std::fs::write(codex_home.join("config.toml"), config).map_err(|_| FAILED)?;
        let mut command = Command::new(binary);
        command
            .args(["app-server", "--listen", "stdio://"])
            .current_dir(home.path())
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::null());
        // Keep only OS essentials. Never inherit API keys, proxies or agent configuration.
        let essentials: Vec<_> = std::env::vars_os()
            .filter(|(key, _)| {
                matches!(
                    key.to_string_lossy().to_ascii_uppercase().as_str(),
                    "SYSTEMROOT"
                        | "WINDIR"
                        | "HOME"
                        | "USERPROFILE"
                        | "PATH"
                        | "PATHEXT"
                        | "TEMP"
                        | "TMP"
                        | "TMPDIR"
                        | "LANG"
                        | "LC_ALL"
                )
            })
            .collect();
        command
            .env_clear()
            .envs(essentials)
            // The OS credential store needs the real user home (macOS otherwise
            // cannot find its default keychain). CODEX_HOME and the working
            // directory remain app-owned; host skill discovery is disabled.
            .env("CODEX_HOME", codex_home);
        #[cfg(windows)]
        {
            use std::os::windows::process::CommandExt;
            command.creation_flags(0x08000000); // No console window for the embedded runtime.
        }
        let mut child = command.spawn().map_err(|_| "Die ChatGPT-Komponente konnte nicht gestartet werden. Bitte Finanzblick neu installieren.")?;
        let input = child.stdin.take().ok_or(FAILED)?;
        let output = child.stdout.take().ok_or(FAILED)?;
        let process = Arc::new(Mutex::new(child));
        let (tx, rx) = mpsc::sync_channel(256);
        std::thread::spawn(move || {
            let mut reader = BufReader::new(output);
            loop {
                let mut line = Vec::new();
                if reader
                    .by_ref()
                    .take(2_000_001)
                    .read_until(b'\n', &mut line)
                    .unwrap_or(0)
                    == 0
                    || line.len() > 2_000_000
                {
                    break;
                }
                let Ok(value) = serde_json::from_slice(&line) else {
                    break;
                };
                if tx.send(value).is_err() {
                    break;
                }
            }
        });
        let mut runtime = Self {
            process,
            input,
            output: rx,
            events: VecDeque::new(),
            next: 0,
            session,
            login_id: None,
            home,
            thread_id: None,
        };
        runtime.rpc("initialize", json!({"clientInfo":{"name":"finanzblick","title":"Finanzblick","version":env!("CARGO_PKG_VERSION")},"capabilities":{"experimentalApi":false}}))?;
        runtime.write(json!({"method":"initialized"}))?;
        let effective = runtime.rpc("config/read", json!({"includeLayers":false}))?;
        validate_config(&effective["config"], &models, auth_store)?;
        Ok(runtime)
    }

    fn write(&mut self, message: Value) -> Result<(), String> {
        serde_json::to_writer(&mut self.input, &message).map_err(|_| FAILED)?;
        self.input
            .write_all(b"\n")
            .and_then(|_| self.input.flush())
            .map_err(|_| FAILED.into())
    }

    fn receive(&mut self, deadline: Instant) -> Result<Value, String> {
        let remaining = deadline
            .checked_duration_since(Instant::now())
            .ok_or("ChatGPT antwortet nicht rechtzeitig. Bitte später erneut versuchen.")?;
        let message = self.output.recv_timeout(remaining).map_err(|_| FAILED)?;
        if message.get("id").is_some() && message.get("method").is_some() {
            self.write(json!({"id":message["id"],"error":{"code":-32601,"message":"Tools are unavailable in Finanzblick"}}))?;
            return Err("ChatGPT hat eine nicht erlaubte Aktion angefordert. Die Anfrage wurde abgebrochen.".into());
        }
        Ok(message)
    }

    fn request(&mut self, method: &str, params: Value) -> Result<u64, String> {
        self.next += 1;
        let id = self.next;
        self.write(json!({"id":id,"method":method,"params":params}))?;
        Ok(id)
    }

    pub fn rpc(&mut self, method: &str, params: Value) -> Result<Value, String> {
        let id = self.request(method, params)?;
        self.response(id)
    }

    fn response(&mut self, id: u64) -> Result<Value, String> {
        let deadline = Instant::now() + Duration::from_secs(20);
        loop {
            let message = self.receive(deadline)?;
            if message["id"] == id {
                if let Some(error) = message.get("error") {
                    return Err(provider_error(error));
                }
                return message.get("result").cloned().ok_or_else(|| FAILED.into());
            }
            if message.get("method").is_some() {
                if self.events.len() >= 512 {
                    return Err(FAILED.into());
                }
                self.events.push_back(message);
            }
        }
    }

    pub fn connected(&mut self) -> Result<bool, String> {
        let account = self.rpc("account/read", json!({"refreshToken":false}))?;
        let connected = account["account"]["type"] == "chatgpt";
        if connected {
            self.login_id = None;
        }
        self.events.clear();
        Ok(connected)
    }

    pub fn answer<L: Drop>(
        &mut self,
        payload: &str,
        instructions: &str,
        reuse_context: bool,
        authorize: impl FnOnce() -> Result<L, String>,
    ) -> Result<String, String> {
        if !self.connected()? {
            return Err("Bitte zuerst mit ChatGPT anmelden.".into());
        }
        let thread_id = if reuse_context {
            self.thread_id
                .clone()
                .ok_or("Der Chatkontext ist abgelaufen. Bitte die Daten erneut freigeben.")?
        } else {
            if let Some(id) = self.thread_id.take() {
                self.rpc("thread/unsubscribe", json!({"threadId":id}))?;
            }
            let thread = self.rpc("thread/start", json!({"ephemeral":true,"model":MODEL,"modelProvider":"openai","cwd":self.home.path(),"sandbox":"read-only","approvalPolicy":"never","baseInstructions":instructions}))?;
            if thread["thread"]["ephemeral"] != true {
                return Err(FAILED.into());
            }
            let id = thread["thread"]["id"].as_str().ok_or(FAILED)?.to_string();
            self.thread_id = Some(id.clone());
            id
        };
        self.events.clear();
        let request_id = {
            let _lease = authorize()?;
            self.request("turn/start", json!({"threadId":thread_id,"input":[{"type":"text","text":payload,"text_elements":[]}]}))?
        };
        let turn = self.response(request_id)?;
        let turn_id = turn["turn"]["id"].as_str().ok_or(FAILED)?.to_string();
        let deadline = Instant::now() + Duration::from_secs(180);
        let mut answer = String::new();
        loop {
            let event = match self.events.pop_front() {
                Some(event) => event,
                None => self.receive(deadline)?,
            };
            let params = &event["params"];
            if params["threadId"] != thread_id {
                continue;
            }
            match event["method"].as_str() {
                Some("item/completed")
                    if params["turnId"] == turn_id && params["item"]["type"] == "agentMessage" =>
                {
                    if let Some(text) = params["item"]["text"].as_str() {
                        if !answer.is_empty() {
                            answer.push_str("\n\n");
                        }
                        answer.push_str(text);
                        if answer.len() > 20_000 {
                            return Err(
                                "Die Antwort ist zu lang. Bitte die Frage eingrenzen.".into()
                            );
                        }
                    }
                }
                Some("turn/completed") if params["turn"]["id"] == turn_id => {
                    if params["turn"]["status"] != "completed" {
                        return Err(provider_error(&params["turn"]["error"]));
                    }
                    if answer.trim().is_empty() {
                        return Err(
                            "ChatGPT hat keine Antwort geliefert. Bitte erneut versuchen.".into(),
                        );
                    }
                    return Ok(answer);
                }
                _ => {}
            }
        }
    }
}

impl Drop for Runtime {
    fn drop(&mut self) {
        if let Ok(mut process) = self.process.lock() {
            let _ = process.kill();
            let _ = process.wait();
        }
        // The ephemeral chat ends with this process; the separate OS login store remains.
    }
}

pub fn provider_error(error: &Value) -> String {
    let code = error.to_string().to_ascii_lowercase();
    if code.contains("usagelimit")
        || code.contains("usage_limit")
        || code.contains("usage limit")
        || code.contains("rate_limit")
        || code.contains("ratelimit")
        || code.contains("429")
    {
        "Das Nutzungslimit deines ChatGPT-Kontos ist erreicht. Bitte später erneut versuchen. API-Guthaben ist hierfür nicht nötig.".into()
    } else if code.contains("unauthorized") || code.contains("401") || code.contains("auth") {
        "Bitte erneut mit ChatGPT anmelden.".into()
    } else {
        "Die Anfrage an ChatGPT ist fehlgeschlagen. Bitte Verbindung und Kontozugang prüfen.".into()
    }
}

pub fn login_url(value: &str) -> Result<String, String> {
    let url = reqwest::Url::parse(value).map_err(|_| FAILED)?;
    if url.scheme() != "https"
        || !matches!(url.host_str(), Some("auth.openai.com" | "chatgpt.com"))
        || !url.username().is_empty()
        || url.password().is_some()
        || url.port().is_some()
    {
        return Err(FAILED.into());
    }
    Ok(url.to_string())
}

// Managed machine settings can override a private CODEX_HOME. Fail closed if they
// would introduce tools, an alternate endpoint, instructions or credential storage.
fn validate_config(config: &Value, models: &Path, auth_store: &str) -> Result<(), String> {
    let policy_error =
        "Die ChatGPT-Konfiguration erlaubt keinen geschützten Finanzchat auf diesem Gerät.";
    let features = config["features"].as_object().ok_or(policy_error)?;
    for line in CONFIG
        .split("[features]")
        .nth(1)
        .ok_or(policy_error)?
        .lines()
    {
        let Some((key, value)) = line.split_once('=') else {
            continue;
        };
        if features.get(key.trim()) != Some(&Value::Bool(value.trim() == "true")) {
            return Err(policy_error.into());
        }
    }
    for (name, value) in features {
        if !matches!(
            name.as_str(),
            "skip_host_skill_discovery" | "auth_elicitation" | "mentions_v2"
        ) && value != &Value::Bool(false)
            && !value.is_null()
        {
            return Err(policy_error.into());
        }
    }
    if !config["mcp_servers"]
        .as_object()
        .is_some_and(|v| v.is_empty())
        || config["cli_auth_credentials_store"] != auth_store
        || config["chatgpt_base_url"] != "https://chatgpt.com/backend-api/"
        || config["analytics"]["enabled"] != false
        || config["model_catalog_json"] != models.to_string_lossy().as_ref()
        || [
            "notify",
            "instructions",
            "developer_instructions",
            "model_instructions_file",
            "openai_base_url",
        ]
        .iter()
        .any(|key| !config[*key].is_null())
    {
        return Err(policy_error.into());
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn keyring_survives_restart_is_isolated_and_logout_removes_credentials() {
        let binary = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("chat-runtime/bin")
            .join(if cfg!(windows) { "codex.exe" } else { "codex" });
        let namespace = tempfile::tempdir().unwrap();
        let profile = namespace.path().join("profile-one");
        let other = namespace.path().join("profile-two");
        let session = ("synthetic-keyring-test".into(), 1);
        let mut first = Runtime::start(&binary, session.clone(), Some(&profile)).unwrap();
        // The API-key variant exercises the same credential store without real OAuth or network.
        first
            .rpc(
                "account/login/start",
                json!({"type":"apiKey","apiKey":"synthetic-finanzblick-test-not-a-real-key"}),
            )
            .unwrap();
        drop(first);
        let mut second = Runtime::start(&binary, session.clone(), Some(&profile)).unwrap();
        let remembered = second
            .rpc("account/read", json!({"refreshToken":false}))
            .unwrap();
        let mut isolated = Runtime::start(&binary, session.clone(), Some(&other)).unwrap();
        let other_account = isolated
            .rpc("account/read", json!({"refreshToken":false}))
            .unwrap();
        drop(isolated);
        // Delete the test credential before asserting, including on a failed assertion.
        second.rpc("account/logout", json!({})).unwrap();
        drop(second);
        assert_eq!(remembered["account"]["type"], "apiKey");
        assert!(other_account["account"].is_null());
        assert!(!profile.join("auth.json").exists());
        for home in [&profile, &other] {
            let mut runtime = Runtime::start(&binary, session.clone(), Some(home)).unwrap();
            assert!(runtime
                .rpc("account/read", json!({"refreshToken":false}))
                .unwrap()["account"]
                .is_null());
        }
    }
    #[test]
    fn bundled_runtime_accepts_private_policy_and_rejects_unsafe_overrides() {
        let binary = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("chat-runtime/bin")
            .join(if cfg!(windows) { "codex.exe" } else { "codex" });
        let mut runtime = Runtime::start(&binary, ("synthetic".into(), 1), None).unwrap();
        assert!(!runtime.connected().unwrap());
        assert!(!runtime.home.path().join("auth.json").exists());
        let config = runtime
            .rpc("config/read", json!({"includeLayers":false}))
            .unwrap()["config"]
            .clone();
        let models = runtime.home.path().join("models.json");
        for (key, value) in [
            ("mcp_servers", json!({"external":{"command":"untrusted"}})),
            ("chatgpt_base_url", json!("https://example.invalid/")),
            ("cli_auth_credentials_store", json!("file")),
        ] {
            let mut changed = config.clone();
            changed[key] = value;
            assert!(validate_config(&changed, &models, "ephemeral").is_err());
        }
        let mut changed = config;
        changed["features"]["shell_tool"] = json!(true);
        assert!(validate_config(&changed, &models, "ephemeral").is_err());
    }

    #[test]
    fn login_accepts_only_official_https_hosts() {
        assert!(login_url("https://auth.openai.com/oauth/authorize?state=test").is_ok());
        for url in [
            "http://auth.openai.com",
            "https://auth.openai.com.evil.test",
            "https://user@auth.openai.com",
            "file:///tmp/test",
            "https://chatgpt.com:8443/",
        ] {
            assert!(login_url(url).is_err());
        }
    }
    #[test]
    fn error_messages_never_echo_provider_content() {
        assert!(!provider_error(&json!({"message":"private financial data"})).contains("private"));
        assert!(
            provider_error(&json!({"codexErrorInfo":"usageLimitExceeded"}))
                .contains("Nutzungslimit")
        );
        assert!(provider_error(&json!({"code":"rate_limit_exceeded"}))
            .contains("API-Guthaben ist hierfür nicht nötig"));
    }
}
