// Steuert Profilwahl und Entsperrung, bevor die geschützte Anwendung angezeigt wird.

import { DatabasePicker } from "../settings/DatabasePicker";
import { BackupPanel } from "../settings/BackupPanel";
import { t, setLanguage, setRegion } from "../../i18n";
import {
  createContext,
  useCallback,
  useContext,
  useEffect,
  useRef,
  useState,
  type FormEvent,
  type ReactNode,
} from "react";
import { invoke } from "@tauri-apps/api/core";
import { listen } from "@tauri-apps/api/event";

import { SettingsContext, defaultSettings, type AppSettings } from "../../settings";

type Status = {
  initialized: boolean;
  unlocked: boolean;
  dataPath: string;
  settings: AppSettings;
  databaseId?: string;
};

const VaultLockContext = createContext<(() => Promise<void>) | null>(null);

export function useVaultLock() {
  const lock = useContext(VaultLockContext);
  if (!lock) throw new Error("useVaultLock must be used inside VaultGate");
  return lock;
}

export function VaultGate({ children }: { children: ReactNode }) {
  const [status, setStatus] = useState<Status | null>(null);
  const [password, setPassword] = useState("");
  const [confirmation, setConfirmation] = useState("");
  const [acknowledged, setAcknowledged] = useState(false);
  const [busy, setBusy] = useState(false);
  const [showPassword, setShowPassword] = useState(false);
  const [capsLock, setCapsLock] = useState(false);
  const [showRestore, setShowRestore] = useState(false);
  const [creatingDatabase, setCreatingDatabase] = useState(false);
  const [error, setError] = useState("");
  const passwordField = useRef<HTMLInputElement>(null);
  const heading = useRef<HTMLHeadingElement>(null);
  const [failedAttempts, setFailedAttempts] = useState(0);
  const revision = useRef(0);

  useEffect(() => {
    if (showRestore) heading.current?.focus();
    else if (status && !status.unlocked && !creatingDatabase) passwordField.current?.focus();
  }, [status?.unlocked, status?.databaseId, creatingDatabase, showRestore]);

  useEffect(() => {
    // Wait for React to re-enable the input before restoring keyboard focus.
    if (!busy && failedAttempts > 0) {
      passwordField.current?.focus();
      passwordField.current?.select();
    }
  }, [busy, failedAttempts]);

  const refresh = useCallback(async () => {
    const current = revision.current;
    try {
      const next = await invoke<Status>("vault_status");
      if (current === revision.current) {
        setLanguage(next.settings?.language ?? "de");
        setRegion(next.settings?.region ?? "CH");
        setStatus(next);
      }
    } catch {
      if (current === revision.current) {
        setStatus(null);
        setError(
          t(
            "Die lokalen Daten sind nicht erreichbar. Bitte Finanzblick als Desktop-App öffnen.",
          ),
        );
      }
    }
  }, []);

  const hideUnlockedContent = useCallback(() => {
    revision.current++;
    setStatus((previous) =>
      previous ? { ...previous, unlocked: false } : null,
    );
    setPassword("");
    setConfirmation("");
    setShowPassword(false);
    setCapsLock(false);
    setShowRestore(false);
  }, []);

  const lock = useCallback(async () => {
    hideUnlockedContent();
    try {
      await invoke("lock_vault");
    } catch {
      setError(
        t(
          "Sperren konnte nicht bestätigt werden. Bitte die Anwendung schliessen.",
        ),
      );
    }
  }, [hideUnlockedContent]);

  useEffect(() => {
    const unlisten = listen("vault-locked", hideUnlockedContent);
    return () => {
      void unlisten.then((dispose) => dispose());
    };
  }, [hideUnlockedContent]);

  useEffect(() => {
    void refresh();
    return () => {
      revision.current++;
    };
  }, [refresh]);

  useEffect(() => {
    if (!status?.unlocked) return;
    let lastActivity = 0;
    const activity = (event: Event) => {
      if (!event.isTrusted || Date.now() - lastActivity < 30_000) return;
      lastActivity = Date.now();
      void invoke("vault_activity").catch(() => {
        void lock();
      });
    };
    const visibility = () => {
      if (document.hidden) void lock();
    };
    const events = ["pointerdown", "keydown", "wheel", "pointermove"];
    events.forEach((name) =>
      window.addEventListener(name, activity, { passive: true }),
    );
    document.addEventListener("visibilitychange", visibility);
    const poll = window.setInterval(() => {
      void refresh();
    }, 1000);
    return () => {
      events.forEach((name) => window.removeEventListener(name, activity));
      document.removeEventListener("visibilitychange", visibility);
      window.clearInterval(poll);
    };
  }, [status?.unlocked, lock, refresh]);

  useEffect(() => {
    if (!status?.unlocked) return;
    const synchronize = () => {
      void invoke("refresh_market_data", { force: false })
        .then(() => window.dispatchEvent(new Event("market-data-refreshed")))
        .catch(() => undefined);
    };
    synchronize();
    const timer = window.setInterval(synchronize, 60 * 60 * 1000);
    return () => window.clearInterval(timer);
  }, [
    status?.unlocked,
    status?.settings?.marketstackApiKey,
    status?.settings?.alphaVantageApiKey,
  ]);

  async function submit(event: FormEvent<HTMLFormElement>) {
    event.preventDefault();
    if (!status || busy) return;
    const setup = !status.initialized;
    // Read the actual fields: password managers can fill inputs without firing
    // React's onChange event. Never trim or otherwise change the password.
    const fields = new FormData(event.currentTarget);
    const secret = String(fields.get("password") ?? "");
    if (setup && [...secret].length < 7) {
      setError(t("Bitte mindestens 7 Zeichen verwenden."));
      return;
    }
    if (setup && secret !== fields.get("password-confirmation")) {
      setError(t("Die Passwörter stimmen nicht überein."));
      return;
    }
    if (setup && !acknowledged) {
      setError(t("Bitte den Hinweis zur Wiederherstellung bestätigen."));
      return;
    }
    setBusy(true);
    setShowPassword(false);
    setError("");
    revision.current++;
    setPassword("");
    setConfirmation("");
    try {
      await invoke("unlock_vault", { password: secret, setup });
      await refresh();
    } catch (reason) {
      setError(String(reason));
      setFailedAttempts(attempts => attempts + 1);
    } finally {
      setBusy(false);
    }
  }

  async function saveSettings(settings: AppSettings) {
    await invoke("save_app_settings", { settings });
    await refresh();
  }

  if (status?.unlocked)
    return (
      <SettingsContext.Provider
        value={{ settings: status.settings ?? defaultSettings, saveSettings }}
      >
        <VaultLockContext.Provider value={lock}>
          {children}
        </VaultLockContext.Provider>
      </SettingsContext.Provider>
    );

  return (
    <main className="vault-screen">
      <section className="vault-card" aria-labelledby="vault-heading">
        <header className="vault-brand">
          <img className="vault-brand-icon" src="/finanzblick.svg" width="40" height="40" alt="" />
          <div><strong>Finanzblick</strong><span>{t("Persönliche Finanzen")}</span></div>
        </header>
        <h1 id="vault-heading" ref={heading} tabIndex={-1}>
          {showRestore ? t("Backup wiederherstellen") : status && !status.initialized ? t("Deine Finanzen sicher verwahren") : t("Anmelden")}
        </h1>
        {showRestore ? (
          <BackupPanel restoreOnly hideHeading onRestored={() => window.location.reload()}
            onCancel={() => setShowRestore(false)} />
        ) : !status ? (
          <>
            <p>{t("Lokale Daten prüfen …")}</p>
            {error && <p role="alert">{t(error)}</p>}
            <button
              onClick={() => {
                void refresh();
              }}
            >
              {t("Erneut versuchen")}
            </button>
          </>
        ) : (
          <>
            <p className="vault-subtitle">
              {status.initialized
                ? t("Gib dein Passwort ein, um deine Finanzen zu öffnen.")
                : t(
                    "Schütze deine Finanzen mit einem Passwort aus mindestens 7 Zeichen.",
                  )}
            </p>
            <DatabasePicker disabled={busy} onCreatingChange={setCreatingDatabase} />
            {!creatingDatabase && <form className="vault-login-form" onSubmit={submit} autoComplete="on" aria-busy={busy}>
              <label htmlFor="vault-password">{t("Passwort")}</label>
              <div className="vault-password-field">
              <input
                ref={passwordField}
                id="vault-password"
                name="password"
                type={showPassword ? "text" : "password"}
                autoFocus
                required
                maxLength={1024}
                autoComplete={
                  status.initialized ? "current-password" : "new-password"
                }
                autoCapitalize="none"
                spellCheck={false}
                value={password}
                disabled={busy}
                aria-describedby={capsLock ? "vault-caps-lock" : undefined}
                onKeyDown={event => setCapsLock(event.getModifierState("CapsLock"))}
                onKeyUp={event => setCapsLock(event.getModifierState("CapsLock"))}
                onBlur={() => setCapsLock(false)}
                onChange={(e) => setPassword(e.target.value)}
              />
              <button className="vault-eye" type="button" disabled={busy}
                aria-label={showPassword ? t("Passwort verbergen") : t("Passwort anzeigen")}
                aria-controls="vault-password" aria-pressed={showPassword}
                onClick={() => { setShowPassword(value => !value); passwordField.current?.focus(); }}>
                <svg width="20" height="20" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="1.7" aria-hidden="true">
                  <path d="M2 12s3.5-7 10-7 10 7 10 7-3.5 7-10 7S2 12 2 12Z" />
                  <circle cx="12" cy="12" r="3" />
                  {showPassword && <path d="m3 3 18 18" />}
                </svg>
              </button>
              </div>
              {capsLock && <p id="vault-caps-lock" className="vault-caps-lock" role="status">{t("Caps Lock ist aktiviert")}</p>}
              {!status.initialized && (
                <>
                  <label htmlFor="vault-confirmation">
                    {t("Passwort wiederholen")}
                  </label>
                  <input
                    id="vault-confirmation"
                    name="password-confirmation"
                    type="password"
                    required
                    autoComplete="new-password"
                    autoCapitalize="none"
                    spellCheck={false}
                    value={confirmation}
                    disabled={busy}
                    onChange={(e) => setConfirmation(e.target.value)}
                  />
                  <label className="vault-acknowledgement">
                    <input
                      type="checkbox"
                      checked={acknowledged}
                      onChange={(e) => setAcknowledged(e.target.checked)}
                      required
                    />
                    {t(
                      "Ich bewahre mein Passwort sicher auf. Ohne Passwort kann ich meine Daten nicht wiederherstellen.",
                    )}
                  </label>
                </>
              )}
              {error && (
                <p role="alert" className="vault-error">
                  {t(error)}
                </p>
              )}
              <button className="vault-submit" type="submit" disabled={busy}>
                {busy && <span className="vault-spinner" aria-hidden="true" />}
                {busy
                  ? t("Bitte warten …")
                  : status.initialized
                    ? t("Anmelden")
                    : t("Lokalen Schutz einrichten")}
              </button>
            </form>}
            <div className="vault-backup">
              <button className="vault-restore-link" type="button" disabled={busy}
                onClick={() => { setPassword(""); setConfirmation(""); setShowPassword(false); setCapsLock(false); setError(""); setShowRestore(true); }}>
                <span aria-hidden="true">↺</span> {t("Aus Backup wiederherstellen")}
              </button>
            </div>
          </>
        )}
      </section>
    </main>
  );
}
