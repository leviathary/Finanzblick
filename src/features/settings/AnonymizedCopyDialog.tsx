// Bietet normale und anonymisierte Profilkopien in einem Dialog mit geschütztem Original und expliziter Variantenwahl an.
import { useId, useRef, useState, type FormEvent } from "react";
import { createPortal } from "react-dom";
import { invoke } from "@tauri-apps/api/core";
import { t } from "../../i18n";
import "./anonymizedCopy.css";

export function AnonymizedCopyDialog({ disabled, onCreated }: { disabled: boolean; onCreated: () => void }) {
  const dialog = useRef<HTMLDialogElement>(null);
  const trigger = useRef<HTMLButtonElement>(null);
  const form = useRef<HTMLFormElement>(null);
  const name = useRef<HTMLInputElement>(null);
  const title = useId();
  const [method, setMethod] = useState("random");
  const [busy, setBusy] = useState(false);
  const [error, setError] = useState("");
  const [descriptions, setDescriptions] = useState(true);
  const [kind, setKind] = useState("plain");

  async function submit(event: FormEvent<HTMLFormElement>) {
    event.preventDefault();
    if (busy) return;
    const data = new FormData(event.currentTarget);
    if (kind === "plain") {
      setBusy(true); setError("");
      try {
        await invoke("copy_database", { name: String(data.get("name")).trim() });
        window.location.reload();
      } catch (reason) { setError(t(String(reason))); setBusy(false); }
      return;
    }
    const password = String(data.get("password") ?? "");
    const factor = method === "factor" ? Number(String(data.get("factor")).trim().replace(",", ".")) : null;
    if ([...password].length < 7) { setError(t("Bitte mindestens 7 Zeichen verwenden.")); return; }
    if (password !== data.get("confirmation")) { setError(t("Die Passwörter stimmen nicht überein.")); return; }
    if (factor !== null && (!Number.isFinite(factor) || factor < 0.01 || factor > 100 || Math.abs(factor - 1) < 1e-9)) {
      setError(t("Bitte einen Faktor zwischen 0,01 und 100 eingeben, der nicht 1 ist.")); return;
    }
    setBusy(true); setError("");
    try {
      await invoke("create_anonymized_copy", { name: String(data.get("name")).trim(), password, factor, anonymizeDescriptions: descriptions });
      dialog.current?.close();
      onCreated();
    } catch (reason) { setError(t(String(reason))); }
    finally { setBusy(false); }
  }

  return <>
    <button ref={trigger} type="button" className="secondary-button" disabled={disabled} aria-haspopup="dialog" onClick={() => {
      form.current?.reset(); setKind("plain"); setMethod("random"); setDescriptions(true); setError(""); dialog.current?.showModal(); name.current?.focus();
    }}>{t("Kopie erstellen …")}</button>
    {createPortal(<dialog ref={dialog} className="settlement-rule-dialog anonymized-copy-dialog" aria-labelledby={title}
      onCancel={event => { if (busy) event.preventDefault(); }}
      onClose={() => { form.current?.reset(); trigger.current?.focus(); }}>
      <h2 id={title}>{t("Kopie erstellen")}</h2>
      <form ref={form} onSubmit={submit} autoComplete="off" aria-busy={busy}>
        <label>{t("Art der Kopie")}<select value={kind} disabled={busy} onChange={event => { setKind(event.target.value); setError(""); }}>
          <option value="plain">{t("Unveränderte Kopie")}</option>
          <option value="anonymous">{t("Anonymisierte Kopie")}</option>
        </select></label>
        <p className="settings-hint">{kind === "plain"
          ? t("Die Kopie enthält den vollständigen Datenbestand und verwendet zunächst dasselbe Passwort wie das aktuelle Finanzprofil.")
          : t("Die Kopie erhält einen eigenen Namen und ein eigenes Passwort. Dein Originalprofil bleibt unverändert und geöffnet.")}</p>
        <label>{t("Name der Kopie")}<input ref={name} name="name" required maxLength={80} disabled={busy} /></label>
        {kind === "anonymous" && <>
        <div className="anonymized-copy-passwords">
          <label>{t("Passwort")}<input name="password" type="password" required maxLength={1024} autoComplete="new-password" disabled={busy} /></label>
          <label>{t("Passwort wiederholen")}<input name="confirmation" type="password" required maxLength={1024} autoComplete="new-password" disabled={busy} /></label>
        </div>
        <label>{t("Methode")}<select aria-label={t("Methode")} value={method} disabled={busy} onChange={event => { setMethod(event.target.value); setError(""); }}>
          <option value="random">{t("Zufällige Beträge")}</option><option value="factor">{t("Fester Faktor")}</option>
        </select></label>
        <p className="settings-hint">{method === "random"
          ? t("Jede Buchung und jedes Steuerjahr erhält einen unabhängigen Zufallsfaktor zwischen 10 und 300 % des ursprünglichen Werts.")
          : t("Alle Buchungs-, Salden- und Steuerbeträge werden mit demselben Faktor multipliziert; ihre Verhältnisse bleiben erhalten.")}</p>
        {method === "factor" && <label>{t("Faktor")}<input name="factor" inputMode="decimal" defaultValue="0.5" required disabled={busy} /></label>}
        <label className="anonymized-copy-checkbox"><input type="checkbox" checked={descriptions} disabled={busy} onChange={event => setDescriptions(event.target.checked)} />{t("Buchungstexte ebenfalls anonymisieren")}</label>
        <p className="database-anonymization-warning">{t("Veränderte Beträge garantieren keine vollständige Anonymität. Ein fester Faktor erhält erkennbare Verhältnisse. Kontonamen, eigene Kategorien und weitere Angaben können weiterhin persönlich sein. Prüfe die Kopie vor einer Weitergabe.")}</p>
        {!descriptions && <p className="settings-hint">{t("Buchungstexte bleiben erhalten und können persönliche Angaben enthalten.")}</p>}
        </>}
        {error && <p role="alert" className="error-message">{error}</p>}
        <div className="anonymized-copy-footer">
          <button type="button" className="secondary-button" disabled={busy} onClick={() => dialog.current?.close()}>{t("Abbrechen")}</button>
          <button type="submit" className="primary-button" disabled={busy}>{busy ? t("Kopie wird erstellt …") : kind === "plain" ? t("Finanzprofil kopieren und öffnen") : t("Kopie erstellen")}</button>
        </div>
        {busy && <p role="status" className="settings-hint">{t("Kopie wird erstellt …")}</p>}
      </form>
    </dialog>, document.body)}
  </>;
}
