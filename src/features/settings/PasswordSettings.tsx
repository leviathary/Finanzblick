import { useState, type FormEvent } from "react";
import { invoke } from "@tauri-apps/api/core";
import { t } from "../../i18n";

export function PasswordSettings() {
  const [changing, setChanging] = useState(false);
  const [passwordNotice, setPasswordNotice] = useState("");
  const [passwordError, setPasswordError] = useState("");
  async function changePassword(event: FormEvent<HTMLFormElement>) {
    event.preventDefault();
    const form = event.currentTarget;
    const data = new FormData(form);
    const currentPassword = String(data.get("current-password") ?? "");
    const newPassword = String(data.get("new-password") ?? "");
    setPasswordNotice("");
    setPasswordError("");
    if ([...newPassword].length < 7) {
      setPasswordError(t("Bitte mindestens 7 Zeichen verwenden."));
      return;
    }
    if (newPassword !== data.get("confirm-password")) {
      setPasswordError(t("Die Passwörter stimmen nicht überein."));
      return;
    }
    setChanging(true);
    try {
      await invoke("change_vault_password", { currentPassword, newPassword });
      form.reset();
      setPasswordNotice(t("Passwort geändert."));
    } catch (reason) {
      setPasswordError(String(reason));
    } finally {
      setChanging(false);
    }
  }

  return (
      <form
        id="settings-security"
        className="dashboard-card settings-card"
        onSubmit={changePassword}
        autoComplete="on"
      >
        <h2>{t("Passwort ändern")}</h2>
        <label>
          {t("Bisheriges Passwort")}
          <input
            name="current-password"
            type="password"
            required
            autoComplete="current-password"
            disabled={changing}
          />
        </label>
        <label>
          {t("Neues Passwort")}
          <input
            name="new-password"
            type="password"
            required
            autoComplete="new-password"
            maxLength={1024}
            disabled={changing}
          />
        </label>
        <label>
          {t("Neues Passwort wiederholen")}
          <input
            name="confirm-password"
            type="password"
            required
            autoComplete="new-password"
            maxLength={1024}
            disabled={changing}
          />
        </label>
        <p className="settings-hint">
          {t(
            "Mindestens 7 Zeichen. Bereits erstellte Sicherungen behalten ihr bisheriges Passwort.",
          )}
        </p>
        {passwordError && (
          <p role="alert" className="error-message">
            {t(passwordError)}
          </p>
        )}
        {passwordNotice && (
          <p role="status" className="settings-success">
            {t(passwordNotice)}
          </p>
        )}
        <button className="primary-button" disabled={changing}>
          {changing ? t("Bitte warten …") : t("Passwort ändern")}
        </button>
      </form>
  );
}
