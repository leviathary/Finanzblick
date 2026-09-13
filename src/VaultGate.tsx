import { DatabasePicker } from "./features/settings/DatabasePicker";
import { t, setLanguage, setRegion } from "./i18n";
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

import { SettingsContext, defaultSettings, type AppSettings } from "./settings";

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
  const [error, setError] = useState("");
  const revision = useRef(0);

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
            "Der lokale Datentresor ist nicht erreichbar. Bitte Finanzblick als Desktop-App öffnen.",
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
    setError("");
    revision.current++;
    setPassword("");
    setConfirmation("");
    try {
      await invoke("unlock_vault", { password: secret, setup });
      await refresh();
    } catch (reason) {
      setError(String(reason));
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
        <div className="vault-key" aria-hidden="true">
          <svg
            width="30"
            height="30"
            viewBox="0 0 24 24"
            fill="none"
            stroke="currentColor"
            strokeWidth="1.7"
            strokeLinecap="round"
            strokeLinejoin="round"
            focusable="false"
          >
            <circle cx="8" cy="8" r="5" />
            <path d="m11.5 11.5 9 9M17 17l3-3M14 14l2-2" />
            <circle
              cx="6.5"
              cy="6.5"
              r=".75"
              fill="currentColor"
              stroke="none"
            />
          </svg>
        </div>
        <span className="vault-eyebrow">
          {t("FINANZBLICK · NUR AUF DIESEM GERÄT")}
        </span>
        <h1 id="vault-heading">
          {status?.initialized
            ? t("Finanzblick entsperren")
            : t("Deine Finanzen sicher verwahren")}
        </h1>
        {!status ? (
          <>
            <p>{t("Lokalen Datentresor prüfen …")}</p>
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
            <p>
              {status.initialized
                ? t("Gib dein Passwort ein, um deine Finanzen zu öffnen.")
                : t(
                    "Schütze deine Finanzen mit einem Passwort aus mindestens 7 Zeichen.",
                  )}
            </p>
            <DatabasePicker />
            <form onSubmit={submit} autoComplete="on">
              <label htmlFor="vault-password">{t("Passwort")}</label>
              <input
                id="vault-password"
                name="password"
                type="password"
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
                onChange={(e) => setPassword(e.target.value)}
              />
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
              <button type="submit" disabled={busy}>
                {busy
                  ? t("Bitte warten …")
                  : status.initialized
                    ? t("Entsperren")
                    : t("Lokalen Schutz einrichten")}
              </button>
            </form>
          </>
        )}
      </section>
    </main>
  );
}
