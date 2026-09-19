// Ermöglicht Auswahl, Anlage und Verwaltung lokaler Datenbankprofile.

import { useEffect, useState, type FormEvent } from "react";
import { invoke } from "@tauri-apps/api/core";
import { t } from "../../i18n";
import { AnonymizedCopyDialog } from "./AnonymizedCopyDialog";

type Choice = { id: string; name: string; active: boolean; demo: boolean };
type DatabaseTab = "manage" | "create";

export function DatabasePicker({ allowCreate = false, compact = false, disabled = false, onCreatingChange }: { allowCreate?: boolean; compact?: boolean; disabled?: boolean; onCreatingChange?: (creating: boolean) => void }) {
  const [expanded, setExpanded] = useState(false);
  const [choices, setChoices] = useState<Choice[]>([]);
  const [selected, setSelected] = useState("");
  const [busy, setBusy] = useState(false);
  const [error, setError] = useState("");
  const [notice, setNotice] = useState("");
  const [tab, setTab] = useState<DatabaseTab>("manage");

  const demoChoice = choices.find(choice => choice.demo && choice.active)
    ?? choices.filter(choice => choice.demo).sort((first, second) => first.id.localeCompare(second.id, undefined, { numeric: true }))[0];

  useEffect(() => {
    void invoke<Choice[]>("list_databases").then(items => {
      setChoices(items);
      setSelected(items.find(item => item.active)?.id ?? items[0]?.id ?? "action:new");
      if (!items.length) { setTab("create"); onCreatingChange?.(true); }
    }).catch(() => setError("Finanzprofile konnten nicht gelesen werden."));
  }, []);

  async function selectDatabase(id: string) {
    setSelected(id);
    setError("");
    setNotice("");
    onCreatingChange?.(id === "action:new");
    if (id === "action:new") { setTab("create"); return; }
    setTab("manage");
    if (!id || choices.some(choice => choice.id === id && choice.active)) return;
    setBusy(true);
    setError("");
    try {
      if (id === "action:demo" || id === demoChoice?.id) {
        await invoke("create_demo_database");
        window.location.hash = "overview";
      } else {
        await invoke("switch_database", { id });
      }
      window.location.reload();
    } catch (reason) {
      setError(String(reason));
      setSelected(choices.find(choice => choice.active)?.id ?? "action:new");
      if (!choices.length) { setTab("create"); onCreatingChange?.(true); }
      setBusy(false);
    }
  }

  async function createDatabase(event: FormEvent<HTMLFormElement>) {
    event.preventDefault();
    const data = new FormData(event.currentTarget);
    const name = String(data.get("database-name") ?? "").trim();
    const password = String(data.get("database-password") ?? "");
    if ([...password].length < 7) {
      setError(t("Bitte mindestens 7 Zeichen verwenden."));
      return;
    }
    if (password !== data.get("database-password-confirmation")) {
      setError(t("Die Passwörter stimmen nicht überein."));
      return;
    }
    setBusy(true);
    setError("");
    try {
      await invoke("create_database", { name, password });
      window.location.reload();
    } catch (reason) {
      setError(String(reason));
      setBusy(false);
    }
  }

  async function deleteDatabase() {
    if (!window.confirm(t("Dieses Finanzprofil und alle darin enthaltenen Daten werden dauerhaft gelöscht. Finanzprofil jetzt löschen?"))) return;
    setBusy(true);
    setError("");
    setNotice("");
    try {
      await invoke("delete_database");
      window.location.reload();
    } catch (reason) {
      setError(String(reason));
      setBusy(false);
    }
  }

  return <div className="database-picker">
    {allowCreate && <h2>{t("Finanzprofile")}</h2>}
    {compact && choices.length === 1 && !expanded ? <div className="vault-profile">
      <span className="vault-profile-avatar" aria-hidden="true">{choices[0].name.slice(0, 1).toLocaleUpperCase()}</span>
      <div><small>{t("Finanzprofil")}</small><strong>{choices[0].id === "original" ? t("Meine Daten") : choices[0].name}</strong></div>
      <button type="button" disabled={busy || disabled} onClick={() => setExpanded(true)}>{t("Wechseln")}</button>
    </div> : <label>{t("Finanzprofil")}
      <select autoFocus={compact && expanded} value={selected} disabled={busy || disabled} onChange={event => void selectDatabase(event.target.value)}>
        <option value="action:new">{t("Neues Finanzprofil")}</option>
        <option value={demoChoice?.id ?? "action:demo"}>{t("Demo-Daten")}</option>
        {choices.filter(choice => choice.id !== demoChoice?.id).map(choice => <option key={choice.id} value={choice.id}>{choice.id === "original" ? t("Meine Daten") : choice.name}</option>)}
      </select>
    </label>}
    {allowCreate && <>
      <p className="settings-hint">{t("Das gewählte Finanzprofil wird beim nächsten Start wieder geöffnet. Jedes Finanzprofil hat sein eigenes Passwort und seinen eigenen Datenbestand.")}</p>
      <div className="database-action-buttons">
        <button type="button" className="secondary-button" disabled={busy || disabled} onClick={() => { setTab("create"); onCreatingChange?.(true); }}>{t("Neues Profil")}</button>
        <AnonymizedCopyDialog disabled={busy || disabled} onCreated={() => {
          void invoke<Choice[]>("list_databases").then(setChoices).catch(() => setError("Finanzprofile konnten nicht gelesen werden."));
          setNotice(t("Anonymisierte Kopie wurde erstellt. Dein Originalprofil bleibt geöffnet."));
        }} />
      </div>

    </>}
      {tab === "create" && <section className="database-tab-panel" id="database-panel-create">
        <form className="database-create-form" onSubmit={createDatabase} autoComplete="off">
          <h3>{t("Neues Finanzprofil erstellen")}</h3>
          <label>{t("Name")}<input name="database-name" required maxLength={80} disabled={busy} /></label>
          <label>{t("Passwort")}<input name="database-password" type="password" required maxLength={1024} autoComplete="new-password" disabled={busy} /></label>
          <label>{t("Passwort wiederholen")}<input name="database-password-confirmation" type="password" required maxLength={1024} autoComplete="new-password" disabled={busy} /></label>
          <button type="submit" className="primary-button" disabled={busy}>{busy ? t("Bitte warten …") : t("Finanzprofil erstellen")}</button>
          {allowCreate && <button type="button" className="secondary-button" disabled={busy} onClick={() => { setTab("manage"); setSelected(choices.find(choice => choice.active)?.id ?? ""); onCreatingChange?.(false); }}>{t("Abbrechen")}</button>}
        </form>
      </section>}

      {allowCreate && tab === "manage" && <div className="database-delete-actions">
        {choices.find(choice => choice.active)?.id === "original"
          ? <p className="settings-hint">{t("Das Hauptprofil kann nicht gelöscht werden.")}</p>
          : <button type="button" className="danger-button" disabled={busy || disabled || !choices.some(choice => choice.active)} onClick={() => void deleteDatabase()}>{t("Finanzprofil löschen")}</button>}
      </div>}
    {busy && selected === "action:demo" && <p role="status">{t("Demo wird geöffnet …")}</p>}
    {notice && <p role="status" className="settings-success">{notice}</p>}
    {error && <p role="alert" className="error-message">{t(error)}</p>}
  </div>;
}
