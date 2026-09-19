// Steuert Export und Wiederherstellung verschlüsselter Sicherungen über Dateidialoge.

import { useState, type FormEvent } from "react";
import { invoke } from "@tauri-apps/api/core";
import { open, save } from "@tauri-apps/plugin-dialog";
import { t } from "../../i18n";

export function BackupPanel({ onRestored, restoreOnly = false, hideHeading = false, onCancel }: {
  onRestored: () => void; restoreOnly?: boolean; hideHeading?: boolean; onCancel?: () => void;
}) {
  const [busy, setBusy] = useState(false);
  const [source, setSource] = useState("");
  const [error, setError] = useState("");
  const [notice, setNotice] = useState("");
  const filters = [{ name: "Finanzblick Backup", extensions: ["finanzblick-backup"] }];

  async function createBackup() {
    setBusy(true);
    setError("");
    setNotice("");
    try {
      const path = await save({ title: t("Backup erstellen"), defaultPath: `Finanzblick-${new Date().toISOString().replace(/[:.]/g, "-")}.finanzblick-backup`, filters });
      if (!path) return;
      await invoke("create_backup", { path });
      setNotice(t("Backup erfolgreich gespeichert.") + " " + path);
    } catch (reason) { setError(String(reason)); }
    finally { setBusy(false); }
  }

  async function chooseBackup() {
    setBusy(true);
    setError("");
    setNotice("");
    try {
      const path = await open({ title: t("Backup wiederherstellen"), multiple: false, directory: false, filters });
      if (typeof path === "string") setSource(path);
    } catch (reason) { setError(String(reason)); }
    finally { setBusy(false); }
  }

  async function restoreBackup(event: FormEvent<HTMLFormElement>) {
    event.preventDefault();
    const form = event.currentTarget;
    const data = new FormData(form);
    setBusy(true);
    setError("");
    setNotice("");
    try {
      await invoke("restore_backup", { path: source, name: String(data.get("backup-name") ?? "").trim(), password: String(data.get("backup-password") ?? "") });
      form.reset();
      setSource("");
      setNotice(t("Backup importiert. Das zusätzliche Finanzprofil kann unter Finanzprofil ausgewählt werden."));
      onRestored();
    } catch (reason) { setError(String(reason)); }
    finally { setBusy(false); }
  }

  return <section id="data-backup" className={restoreOnly ? "backup-panel" : "dashboard-card settings-card backup-panel"}>
    {!hideHeading && <h2>{t(restoreOnly ? "Backup wiederherstellen" : "Backup")}</h2>}
    {!restoreOnly && <p>{t("Gesichert wird das aktuell ausgewählte Finanzprofil mit allen Daten und Einstellungen. Weitere Finanzprofile bitte einzeln sichern.")}</p>}
    <p className="settings-hint">{t("Backups bleiben verschlüsselt. Zum Wiederherstellen brauchst du das Passwort zum Zeitpunkt der Sicherung.")}</p>
    <div className="settings-actions">
      {!restoreOnly && <button className="primary-button" type="button" disabled={busy} onClick={() => void createBackup()}>{t("Backup erstellen")}</button>}
      <button className={restoreOnly ? "primary-button vault-submit" : "secondary-button"} type="button" disabled={busy} onClick={() => void chooseBackup()}>{restoreOnly ? t("Sicherungsdatei auswählen …") : t("Backup wiederherstellen")}</button>
    </div>
    {source && <form onSubmit={restoreBackup}>
      <p className="backup-path">{source}</p>
      <p>{t("Das Backup wird als zusätzliches Finanzprofil importiert. Bestehende Finanzprofile bleiben erhalten.")}</p>
      <label>{t("Name des wiederhergestellten Finanzprofils")}<input name="backup-name" required maxLength={80} disabled={busy} /></label>
      <label>{t("Passwort des Backups")}<input name="backup-password" type="password" autoComplete="current-password" required maxLength={1024} disabled={busy} /></label>
      <div className="settings-actions">
        <button className="primary-button" type="submit" disabled={busy}>{t("Backup importieren")}</button>
        <button className="secondary-button" type="button" disabled={busy} onClick={() => setSource("")}>{t("Abbrechen")}</button>
      </div>
    </form>}
    {busy && <p role="status">{t("Bitte warten …")}</p>}
    {error && <p role="alert" className="error-message">{t(error)}</p>}
    {notice && <p role="status" className="settings-success backup-path">{notice}</p>}
    {onCancel && <div className="vault-backup"><button className="vault-restore-link" type="button" disabled={busy} onClick={onCancel}>
      <span aria-hidden="true">←</span> {t("Zurück zur Anmeldung")}
    </button></div>}
  </section>;
}
