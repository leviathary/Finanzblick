use super::{
    chat_runtime::{self, Process, Runtime},
    Storage,
};
use serde::Serialize;
use serde_json::json;
use sha2::{Digest, Sha256};
use std::{
    sync::{
        atomic::{AtomicBool, AtomicU64, Ordering},
        Mutex,
    },
    time::{Duration, Instant},
};
use tauri::{Manager, State};
use tauri_plugin_opener::OpenerExt;

const STALE: &str = "Die Freigabe ist abgelaufen. Bitte die Datenvorschau erneut erstellen.";
#[derive(Default)]
pub struct ChatState {
    runtime: Mutex<Option<Runtime>>,
    process: Mutex<Option<(Process, (String, u64))>>,
    pending: Mutex<Option<Pending>>,
    context: Mutex<Option<Context>>,
    epoch: AtomicU64,
    sequence: AtomicU64,
    sending: AtomicBool,
}
struct Context {
    fingerprint: String,
    session: (String, u64),
    epoch: u64,
}
struct Pending {
    id: u64,
    session: (String, u64),
    epoch: u64,
    created: Instant,
    payload: String,
    instructions: &'static str,
    fingerprint: String,
    follow_up: bool,
}

impl ChatState {
    pub fn stop(&self) {
        self.epoch.fetch_add(1, Ordering::SeqCst);
        if let Ok(mut context) = self.context.lock() {
            *context = None;
        }
        if let Ok(mut pending) = self.pending.lock() {
            *pending = None;
        }
        if let Ok(mut control) = self.process.lock() {
            if let Some((process, _)) = control.take() {
                if let Ok(mut child) = process.lock() {
                    let _ = child.kill();
                }
            }
        }
        if let Ok(mut runtime) = self.runtime.try_lock() {
            *runtime = None;
        }
    }

    pub fn expire(&self, storage: &Storage) {
        let stale = self
            .process
            .lock()
            .ok()
            .and_then(|p| {
                p.as_ref()
                    .map(|(_, session)| storage.chat_session().as_ref() != Ok(session))
            })
            .unwrap_or(false);
        if stale {
            self.stop();
        }
    }

    pub(super) fn epoch(&self) -> u64 {
        self.epoch.load(Ordering::SeqCst)
    }

    pub(super) fn preview(
        &self,
        epoch: u64,
        session: (String, u64),
        payload: String,
        instructions: &'static str,
        fingerprint: String,
    ) -> Result<(u64, bool), String> {
        if self.sending.load(Ordering::SeqCst) {
            return Err("Bitte die laufende Antwort abwarten oder abbrechen.".into());
        }
        let id = self.sequence.fetch_add(1, Ordering::SeqCst) + 1;
        let mut pending = self.pending.lock().map_err(|_| STALE)?;
        if epoch != self.epoch() {
            return Err(STALE.into());
        }
        let follow_up = self
            .context
            .lock()
            .map_err(|_| STALE)?
            .as_ref()
            .is_some_and(|c| {
                c.fingerprint == fingerprint && c.session == session && c.epoch == epoch
            });
        let payload = if follow_up {
            let data: serde_json::Value = serde_json::from_str(&payload).map_err(|_| STALE)?;
            serde_json::to_string(&json!({"question":data["question"]})).map_err(|_| STALE)?
        } else {
            payload
        };
        *pending = Some(Pending {
            id,
            session,
            epoch,
            created: Instant::now(),
            payload,
            instructions,
            fingerprint,
            follow_up,
        });
        Ok((id, follow_up))
    }
}

fn binary(_app: &tauri::AppHandle) -> Result<std::path::PathBuf, String> {
    let manifest: serde_json::Value =
        serde_json::from_str(include_str!("../../chat-runtime/bin/manifest.json"))
            .map_err(|_| "ChatGPT-Komponente nicht verfügbar.")?;
    let name = if cfg!(windows) { "codex.exe" } else { "codex" };
    #[cfg(debug_assertions)]
    let path = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("chat-runtime/bin")
        .join(name);
    #[cfg(not(debug_assertions))]
    let path = _app
        .path()
        .resource_dir()
        .map_err(|_| "ChatGPT-Komponente nicht verfügbar.")?
        .join("chat-runtime")
        .join(name);
    let bytes = std::fs::read(&path)
        .map_err(|_| "Die ChatGPT-Komponente fehlt. Bitte Finanzblick neu installieren.")?;
    if manifest["version"] != "0.154.0"
        || manifest["sha256"] != format!("{:x}", Sha256::digest(&bytes))
    {
        return Err("Die ChatGPT-Komponente stimmt nicht mit dieser App-Version überein. Bitte Finanzblick neu installieren.".into());
    }
    Ok(path)
}

// A stable, app-owned namespace per profile lets Codex use the OS credential store.
// It never points at another application's Codex home and never falls back to auth.json.
fn credentials_home(app: &tauri::AppHandle, profile: &str) -> Result<std::path::PathBuf, String> {
    Ok(app
        .path()
        .app_local_data_dir()
        .map_err(|_| STALE)?
        .join("chatgpt-accounts")
        .join(format!("{:x}", Sha256::digest(profile.as_bytes()))))
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct AccountStatus {
    connected: bool,
    login_pending: bool,
    model: &'static str,
}

#[tauri::command]
pub async fn chatgpt_status(app: tauri::AppHandle) -> Result<AccountStatus, String> {
    tauri::async_runtime::spawn_blocking(move || {
        let storage = app.state::<Storage>();
        let state = app.state::<ChatState>();
        let session = storage.chat_session()?;
        let mut guard = state.runtime.lock().map_err(|_| STALE)?;
        if guard.as_ref().is_some_and(|r| r.session != session) {
            *guard = None;
        }
        if guard.is_none() {
            *state.context.lock().map_err(|_| STALE)? = None;
            let runtime = Runtime::start(
                &binary(&app)?,
                session.clone(),
                Some(&credentials_home(&app, &session.0)?),
            )?;
            *state.process.lock().map_err(|_| STALE)? =
                Some((runtime.process.clone(), session.clone()));
            *guard = Some(runtime);
        }
        if storage.chat_session()? != session {
            *guard = None;
            return Err(STALE.into());
        }
        let (connected, login_pending) = if let Some(runtime) = guard.as_mut() {
            match runtime.connected() {
                Ok(connected) => (connected, runtime.login_id.is_some()),
                Err(_) => {
                    *guard = None;
                    *state.context.lock().map_err(|_| STALE)? = None;
                    (false, false)
                }
            }
        } else {
            (false, false)
        };
        Ok(AccountStatus {
            connected,
            login_pending,
            model: chat_runtime::MODEL,
        })
    })
    .await
    .map_err(|_| STALE.to_string())?
}

#[tauri::command]
pub async fn chatgpt_login_start(app: tauri::AppHandle) -> Result<(), String> {
    tauri::async_runtime::spawn_blocking(move || {
        let storage = app.state::<Storage>();
        let state = app.state::<ChatState>();
        let session = storage.chat_session()?;
        let mut guard = state.runtime.lock().map_err(|_| STALE)?;
        if guard.as_ref().is_some_and(|r| r.session != session) {
            *guard = None;
        }
        if guard.is_none() {
            *state.context.lock().map_err(|_| STALE)? = None;
            let runtime = Runtime::start(
                &binary(&app)?,
                session.clone(),
                Some(&credentials_home(&app, &session.0)?),
            )?;
            *state.process.lock().map_err(|_| STALE)? =
                Some((runtime.process.clone(), session.clone()));
            *guard = Some(runtime);
        }
        if storage.chat_session()? != session {
            *guard = None;
            return Err(STALE.into());
        }
        let runtime = guard.as_mut().ok_or(STALE)?;
        if runtime.connected()? {
            return Ok(());
        }
        if let Some(id) = runtime.login_id.take() {
            runtime.rpc("account/login/cancel", json!({"loginId":id}))?;
        }
        let login = runtime.rpc("account/login/start", json!({"type":"chatgpt"}))?;
        let url = chat_runtime::login_url(login["authUrl"].as_str().ok_or(STALE)?)?;
        runtime.login_id = Some(login["loginId"].as_str().ok_or(STALE)?.to_string());
        app.opener().open_url(url, None::<&str>).map_err(|_| {
            "Der Browser konnte nicht geöffnet werden. Bitte die Anmeldung erneut starten."
        })?;
        Ok(())
    })
    .await
    .map_err(|_| STALE.to_string())?
}

#[tauri::command]
pub async fn chatgpt_disconnect(app: tauri::AppHandle) -> Result<(), String> {
    tauri::async_runtime::spawn_blocking(move || {
        let storage = app.state::<Storage>();
        let state = app.state::<ChatState>();
        let session = storage.chat_session()?;
        state.stop();
        let mut guard = state.runtime.lock().map_err(|_| STALE)?;
        *guard = None;
        let mut runtime = Runtime::start(
            &binary(&app)?,
            session.clone(),
            Some(&credentials_home(&app, &session.0)?),
        )?;
        let _lease = storage.require_chat_session(&session)?;
        runtime.rpc("account/logout", json!({}))?;
        Ok(())
    })
    .await
    .map_err(|_| STALE.to_string())?
}

#[tauri::command]
pub fn cancel_finance_chat(state: State<'_, ChatState>) {
    // End the ephemeral conversation on New Chat, leaving the view or cancellation.
    // The next send can restore the independent saved ChatGPT login.
    state.stop();
}

#[tauri::command]
pub async fn send_finance_chat(app: tauri::AppHandle, preview_id: u64) -> Result<String, String> {
    tauri::async_runtime::spawn_blocking(move || {
        let storage = app.state::<Storage>();
        let state = app.state::<ChatState>();
        if state.sending.swap(true, Ordering::SeqCst) {
            return Err("Bitte die laufende Antwort abwarten oder abbrechen.".into());
        }
        let result = (|| {
            let pending = state
                .pending
                .lock()
                .map_err(|_| STALE)?
                .take()
                .ok_or(STALE)?;
            if pending.id != preview_id
                || pending.created.elapsed() > Duration::from_secs(600)
                || pending.epoch != state.epoch.load(Ordering::SeqCst)
                || storage.chat_session()? != pending.session
            {
                return Err(STALE.into());
            }
            let mut guard = state.runtime.lock().map_err(|_| STALE)?;
            if guard.is_none() && !pending.follow_up {
                let runtime = Runtime::start(
                    &binary(&app)?,
                    pending.session.clone(),
                    Some(&credentials_home(&app, &pending.session.0)?),
                )?;
                *state.process.lock().map_err(|_| STALE)? =
                    Some((runtime.process.clone(), pending.session.clone()));
                *guard = Some(runtime);
            }
            let runtime = guard
                .as_mut()
                .filter(|r| r.session == pending.session)
                .ok_or("Bitte zuerst mit ChatGPT anmelden.")?;
            let answer = runtime.answer(
                &pending.payload,
                pending.instructions,
                pending.follow_up,
                || {
                    if state.epoch.load(Ordering::SeqCst) != pending.epoch
                        || storage.chat_session()? != pending.session
                    {
                        return Err(STALE.to_string());
                    }
                    storage.require_chat_session(&pending.session)
                },
            );
            if answer.is_err() {
                *guard = None;
                *state.context.lock().map_err(|_| STALE)? = None;
            }
            if storage.chat_session()? != pending.session
                || state.epoch.load(Ordering::SeqCst) != pending.epoch
            {
                return Err(STALE.into());
            }
            if answer.is_ok() {
                *state.context.lock().map_err(|_| STALE)? = Some(Context {
                    fingerprint: pending.fingerprint,
                    session: pending.session,
                    epoch: pending.epoch,
                });
            }
            answer
        })();
        state.sending.store(false, Ordering::SeqCst);
        result
    })
    .await
    .map_err(|_| STALE.to_string())?
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn follow_up_sends_only_question_and_requires_same_authorized_context() {
        let state = ChatState::default();
        let session = ("demo".into(), 3);
        let payload = json!({"data":{"amount":123}, "question":"Und warum?"}).to_string();
        let prepare = |fingerprint: &str, session| {
            state
                .preview(
                    state.epoch(),
                    session,
                    payload.clone(),
                    "instructions",
                    fingerprint.into(),
                )
                .unwrap()
                .1
        };
        assert!(!prepare("snapshot", session.clone()));
        *state.context.lock().unwrap() = Some(Context {
            fingerprint: "snapshot".into(),
            session: session.clone(),
            epoch: state.epoch(),
        });
        assert!(prepare("snapshot", session.clone()));
        assert_eq!(
            serde_json::from_str::<serde_json::Value>(
                &state.pending.lock().unwrap().as_ref().unwrap().payload
            )
            .unwrap(),
            json!({"question":"Und warum?"})
        );
        assert!(!prepare("changed-data", session.clone()));
        assert!(!prepare("snapshot", ("other-profile".into(), 3)));
        assert!(!prepare("snapshot", ("demo".into(), 4)));
        state.stop();
        assert!(!prepare("snapshot", session));
    }

    #[test]
    fn preview_is_bound_to_session_and_invalidated_on_disconnect() {
        let state = ChatState::default();
        let id = state
            .preview(
                0,
                ("demo".into(), 3),
                "synthetic".into(),
                "instructions",
                "snapshot".into(),
            )
            .unwrap();
        {
            let guard = state.pending.lock().unwrap();
            let pending = guard.as_ref().unwrap();
            assert_eq!(pending.id, id.0);
            assert_eq!(pending.session, ("demo".into(), 3));
        }
        state.stop();
        assert!(state.pending.lock().unwrap().is_none());
        assert_eq!(state.epoch.load(Ordering::SeqCst), 1);
        assert!(state
            .preview(
                0,
                ("demo".into(), 3),
                "late preview".into(),
                "instructions",
                "snapshot".into()
            )
            .is_err());
    }
}
