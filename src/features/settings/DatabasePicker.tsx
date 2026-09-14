import { useEffect, useState, type FormEvent } from "react";
import { invoke } from "@tauri-apps/api/core";
import { t } from "../../i18n";

type Choice = { id: string; name: string; active: boolean; demo: boolean };
type DatabaseTab = "manage" | "create" | "copy";

export function DatabasePicker({ allowCreate = false, onCreatingChange }: { allowCreate?: boolean; onCreatingChange?: (creating: boolean) => void }) {
  const [choices, setChoices] = useState<Choice[]>([]);
  const [selected, setSelected] = useState("");
  const [busy, setBusy] = useState(false);
  const [error, setError] = useState("");
  const [notice, setNotice] = useState("");
  const [tab, setTab] = useState<DatabaseTab>("manage");
  const [randomizeDescriptions, setRandomizeDescriptions] = useState(true);
  const [scaleDescriptions, setScaleDescriptions] = useState(true);
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

  async function anonymizeDatabase() {
    const confirmation = randomizeDescriptions
      ? t("Buchungstexte und Beträge dieses Finanzprofils werden unwiderruflich verändert. Finanzprofil jetzt anonymisieren?")
      : t("Die Beträge dieses Finanzprofils werden unwiderruflich zufällig verändert. Die Buchungstexte bleiben erhalten. Jetzt fortfahren?");
    if (!window.confirm(confirmation)) return;
    setBusy(true);
    setError("");
    setNotice("");
    try {
      await invoke("anonymize_database", { anonymizeDescriptions: randomizeDescriptions });
      setNotice(t("Finanzprofil wurde anonymisiert."));
    } catch (reason) {
      setError(String(reason));
    } finally {
      setBusy(false);
    }
  }

  async function anonymizeDatabaseWithFactor(event: FormEvent<HTMLFormElement>) {
    event.preventDefault();
    const rawFactor = String(new FormData(event.currentTarget).get("anonymization-factor") ?? "");
    const factor = Number(rawFactor.trim().replace(",", "."));
    if (!Number.isFinite(factor) || factor < 0.01 || factor > 100 || Math.abs(factor - 1) < 1e-9) {
      setError(t("Bitte einen Faktor zwischen 0,01 und 100 eingeben, der nicht 1 ist."));
      return;
    }
    const confirmation = scaleDescriptions
      ? t("Buchungstexte und Beträge werden unwiderruflich mit dem angegebenen Faktor anonymisiert. Jetzt fortfahren?")
      : t("Die Beträge werden unwiderruflich mit dem angegebenen Faktor anonymisiert. Die Buchungstexte bleiben erhalten. Jetzt fortfahren?");
    if (!window.confirm(confirmation)) return;
    setBusy(true);
    setError("");
    setNotice("");
    try {
      await invoke("anonymize_database_with_factor", { factor, anonymizeDescriptions: scaleDescriptions });
      setNotice(t("Finanzprofil wurde mit festem Faktor anonymisiert."));
    } catch (reason) {
      setError(String(reason));
    } finally {
      setBusy(false);
    }
  }

  async function copyDatabase(event: FormEvent<HTMLFormElement>) {
    event.preventDefault();
    const name = String(new FormData(event.currentTarget).get("database-copy-name") ?? "").trim();
    setBusy(true);
    setError("");
    setNotice("");
    try {
      await invoke("copy_database", { name });
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
    <label>{t("Finanzprofil")}
      <select value={selected} disabled={busy} onChange={event => void selectDatabase(event.target.value)}>
        <option value="action:new">{t("Neues Finanzprofil")}</option>
        <option value={demoChoice?.id ?? "action:demo"}>{t("Demo-Daten")}</option>
        {choices.filter(choice => choice.id !== demoChoice?.id).map(choice => <option key={choice.id} value={choice.id}>{choice.id === "original" ? t("Meine Daten") : choice.name}</option>)}
      </select>
    </label>
    {allowCreate && <>
      <p className="settings-hint">{t("Das gewählte Finanzprofil wird beim nächsten Start wieder geöffnet. Jedes Finanzprofil hat sein eigenes Passwort und seinen eigenen Datenbestand.")}</p>
      <div className="database-tabs" role="tablist" aria-label={t("Profilaktionen")}>
        <button type="button" role="tab" id="database-tab-manage" aria-selected={tab === "manage"} aria-controls="database-panel-manage" disabled={busy} onClick={() => { setSelected(choices.find(choice => choice.active)?.id ?? ""); setTab("manage"); }}>{t("Verwalten")}</button>
        <button type="button" role="tab" id="database-tab-create" aria-selected={tab === "create"} aria-controls="database-panel-create" disabled={busy} onClick={() => void selectDatabase("action:new")}>{t("Neu anlegen")}</button>
        <button type="button" role="tab" id="database-tab-copy" aria-selected={tab === "copy"} aria-controls="database-panel-copy" disabled={busy} onClick={() => { setSelected(choices.find(choice => choice.active)?.id ?? ""); setTab("copy"); }}>{t("Kopieren")}</button>
      </div>

      {tab === "manage" && <section className="database-tab-panel" role="tabpanel" id="database-panel-manage" aria-labelledby="database-tab-manage">
        <h3>{t("Aktuelles Finanzprofil verwalten")}</h3>
        <p className="database-anonymization-warning">{t("Die gewählte Änderung kann nicht rückgängig gemacht werden.")}</p>
        <div className="database-anonymization-options">
          <section>
            <h4>{t("Zufällige Beträge")}</h4>
            <p className="settings-hint">{t("Jede Buchung und jedes Steuerjahr erhält einen unabhängigen Zufallsfaktor zwischen 10 und 300 % des ursprünglichen Werts.")}</p>
            <div className="database-anonymization-card-actions">
              <label className="database-anonymize-descriptions">
                <input type="checkbox" checked={randomizeDescriptions} disabled={busy} onChange={event => setRandomizeDescriptions(event.target.checked)} />
                <span>{t("Buchungstexte ebenfalls anonymisieren")}</span>
              </label>
              <button type="button" className="secondary-button" disabled={busy} onClick={() => void anonymizeDatabase()}>{t("Zufällig anonymisieren")}</button>
            </div>
          </section>
          <section>
            <h4>{t("Fester Faktor")}</h4>
            <p className="settings-hint">{t("Alle Buchungs-, Salden- und Steuerbeträge werden mit demselben Faktor multipliziert; ihre Verhältnisse bleiben erhalten.")}</p>
            <form className="database-factor-form" onSubmit={anonymizeDatabaseWithFactor}>
              <label>{t("Faktor")}
                <input name="anonymization-factor" type="text" inputMode="decimal" defaultValue="0.5" disabled={busy} required />
              </label>
              <small>{t("Beispiele: 0,5 halbiert; 0,333333 drittelt; 2 verdoppelt.")}</small>
              <label className="database-anonymize-descriptions">
                <input type="checkbox" checked={scaleDescriptions} disabled={busy} onChange={event => setScaleDescriptions(event.target.checked)} />
                <span>{t("Buchungstexte ebenfalls anonymisieren")}</span>
              </label>
              <button type="submit" className="secondary-button" disabled={busy}>{t("Mit Faktor anonymisieren")}</button>
            </form>
          </section>
        </div>
        <div className="database-action-buttons database-delete-actions">
          <button type="button" className="danger-button" disabled={busy || selected === "original"} onClick={() => void deleteDatabase()}>{t("Finanzprofil löschen")}</button>
        </div>
        {selected === "original" && <p className="settings-hint database-delete-hint">{t("Das Hauptprofil kann nicht gelöscht werden.")}</p>}
      </section>}

    </>}
      {tab === "create" && <section className="database-tab-panel" id="database-panel-create">
        <form className="database-create-form" onSubmit={createDatabase} autoComplete="off">
          <h3>{t("Neues Finanzprofil erstellen")}</h3>
          <label>{t("Name")}<input name="database-name" required maxLength={80} disabled={busy} /></label>
          <label>{t("Passwort")}<input name="database-password" type="password" required maxLength={1024} autoComplete="new-password" disabled={busy} /></label>
          <label>{t("Passwort wiederholen")}<input name="database-password-confirmation" type="password" required maxLength={1024} autoComplete="new-password" disabled={busy} /></label>
          <button type="submit" className="primary-button" disabled={busy}>{busy ? t("Bitte warten …") : t("Finanzprofil erstellen")}</button>
        </form>
      </section>}

      {allowCreate && tab === "copy" && <section className="database-tab-panel" role="tabpanel" id="database-panel-copy" aria-labelledby="database-tab-copy">
        <h3>{t("Finanzprofil kopieren")}</h3>
        <p className="settings-hint">{t("Die Kopie enthält den vollständigen Datenbestand und verwendet zunächst dasselbe Passwort wie das aktuelle Finanzprofil.")}</p>
        <form className="database-copy-form" onSubmit={copyDatabase}>
          <label>{t("Name der Kopie")}<input name="database-copy-name" required maxLength={80} disabled={busy} /></label>
          <button type="submit" className="primary-button" disabled={busy}>{t("Finanzprofil kopieren und öffnen")}</button>
        </form>
      </section>}
    {busy && selected === "action:demo" && <p role="status">{t("Demo wird geöffnet …")}</p>}
    {notice && <p role="status" className="settings-success">{notice}</p>}
    {error && <p role="alert" className="error-message">{t(error)}</p>}
  </div>;
}
