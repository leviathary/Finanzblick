// Bearbeitet explizite Umbuchungsregeln mit einer unverbindlichen Vorschau vor dem Speichern.
import { useEffect, useRef, useState } from "react";
import { invoke } from "@tauri-apps/api/core";
import { locale, t, tr } from "../../i18n";

export interface TransferRule {
  id: number; name: string; accountId: number; accountName: string; currency: string;
  direction: number; prefix: string; transferType: "CREDIT_CARD_SETTLEMENT" | "INTERNAL_TRANSFER";
}
export interface RuleAccount { id: number; name: string; currency: string }
type Draft = Omit<TransferRule, "id" | "accountName"> & { id: number | null };
interface Preview { matches: { id: number; bookingDate: string; description: string; amountMinor: number }[]; protectedCount: number }

export function TransferRuleEditor({ rule, accounts, onClose, onSaved }: {
  rule: TransferRule | null; accounts: RuleAccount[]; onClose: () => void; onSaved: () => void;
}) {
  const dialog = useRef<HTMLDialogElement>(null);
  const request = useRef(0);
  const [draft, setDraft] = useState<Draft>(rule ? { ...rule, name: rule.name || rule.accountName } : {
    id: null, name: "", accountId: accounts[0]?.id ?? 0, currency: accounts[0]?.currency ?? "CHF",
    direction: -1, prefix: "", transferType: "INTERNAL_TRANSFER",
  });
  const [preview, setPreview] = useState<Preview | null>(null);
  const [past, setPast] = useState(false);
  const [future, setFuture] = useState(true);
  const [loading, setLoading] = useState(false);
  const [busy, setBusy] = useState(false);
  const [error, setError] = useState("");
  const [sort, setSort] = useState<{ key: "bookingDate" | "amountMinor"; descending: boolean }>({ key: "bookingDate", descending: true });
  useEffect(() => {
    const opener = document.activeElement;
    const element = dialog.current;
    element?.showModal();
    return () => { request.current++; element?.close(); if (opener instanceof HTMLElement) opener.focus(); };
  }, []);
  function update(change: Partial<Draft>) {
    request.current++; setLoading(false); setPreview(null); setError(""); setDraft(current => ({ ...current, ...change }));
  }
  async function loadPreview() {
    const revision = ++request.current;
    setLoading(true); setPreview(null); setError("");
    try {
      const result = await invoke<Preview>("preview_transfer_rule", { input: draft });
      if (revision === request.current) setPreview(result);
    } catch (reason) { if (revision === request.current) setError(String(reason)); }
    finally { if (revision === request.current) setLoading(false); }
  }
  async function save() {
    if (!preview) return;
    setBusy(true); setError("");
    try {
      await invoke("save_transfer_rule", { input: draft, expectedIds: preview.matches.map(row => row.id), past, future });
      onSaved();
    } catch (reason) { setError(String(reason)); setPreview(null); }
    finally { setBusy(false); }
  }
  const rows = [...(preview?.matches ?? [])].sort((a, b) => {
    const comparison = sort.key === "amountMinor" ? Math.abs(a.amountMinor) - Math.abs(b.amountMinor) : a.bookingDate.localeCompare(b.bookingDate);
    return (sort.descending ? -comparison : comparison) || b.id - a.id;
  });
  return <dialog ref={dialog} className="settlement-rule-dialog transfer-rule-editor" aria-labelledby="transfer-rule-title"
    onCancel={e => { e.preventDefault(); if (!busy) onClose(); }}>
    <h2 id="transfer-rule-title">{t(rule ? "Regel bearbeiten" : "Neue Umbuchungsregel")}</h2>
    <p>{t("Die Regel gilt nur für das gewählte Konto. Für das Gegenkonto eine eigene Regel anlegen. Kontostände bleiben unverändert.")}</p>
    <div className="form-grid">
      <label>{t("Regelname")}<input autoFocus value={draft.name} maxLength={120} disabled={busy} onChange={e => update({ name: e.target.value })} /></label>
      <label>{t("Konto")}<select value={draft.accountId} disabled={busy} onChange={e => {
        const account = accounts.find(a => a.id === Number(e.target.value));
        if (account) update({ accountId: account.id, currency: account.currency });
      }}>{accounts.map(a => <option key={a.id} value={a.id}>{a.name} ({a.currency})</option>)}</select></label>
      <label>{t("Währung")}<input value={draft.currency} maxLength={3} disabled={busy} onChange={e => update({ currency: e.target.value.toUpperCase() })} /></label>
      <label>{t("Zahlungsrichtung")}<select value={draft.direction} disabled={busy} onChange={e => update({ direction: Number(e.target.value) })}>
        <option value={-1}>{t("Belastung")}</option><option value={1}>{t("Gutschrift")}</option>
      </select></label>
      <label>{t("Regeltyp")}<select value={draft.transferType} disabled={busy} onChange={e => update({ transferType: e.target.value as Draft["transferType"] })}>
        <option value="INTERNAL_TRANSFER">{t("Eigene Umbuchung")}</option><option value="CREDIT_CARD_SETTLEMENT">{t("Kartenausgleich")}</option>
      </select></label>
      <label>{t("Buchungstext beginnt mit")}<input value={draft.prefix} disabled={busy} onChange={e => update({ prefix: e.target.value })} /></label>
    </div>
    <p className="intro">{t("Groß-/Kleinschreibung und mehrfache Leerzeichen werden ignoriert. Betrag und Datum dürfen abweichen. Dies ist keine Suche nach beliebigen Textteilen.")}</p>
    <button type="button" className="secondary-button" disabled={busy || loading || !draft.name.trim() || draft.prefix.trim().length < 8 || !draft.accountId} onClick={() => void loadPreview()}>{t("Treffer prüfen")}</button>
    {loading && <p role="status">{t("Buchungen werden geladen…")}</p>}
    {error && <p className="error-message" role="alert">{t(error)}</p>}
    {preview && <section aria-label={t("Vorschau")}>
      <p role="status">{tr`${preview.matches.length} passende Buchungen in der gesamten Historie. ${preview.protectedCount} manuelle Entscheidungen werden nicht verändert.`}</p>
      <div className="settlement-preview"><table className="transfer-table"><thead><tr>
        {([['bookingDate', 'Datum'], ['description', 'Beschreibung'], ['amountMinor', 'Betrag']] as const).map(([key, label]) => <th key={key} scope="col" aria-sort={sort.key === key ? sort.descending ? "descending" : "ascending" : undefined}>
          {key === "description" ? t(label) : <button type="button" className="expense-sort" aria-pressed={sort.key === key}
            aria-label={tr`${t(label)}: ${sort.key !== key || !sort.descending ? t("absteigend") : t("aufsteigend")} sortieren`}
            onClick={() => setSort(current => ({ key, descending: current.key === key ? !current.descending : true }))}>
            {t(label)} <span aria-hidden="true">{sort.key === key ? sort.descending ? "↓" : "↑" : "↕"}</span></button>}
        </th>)}
      </tr></thead><tbody>{rows.slice(0, 200).map(row => <tr key={row.id}>
        <td>{new Intl.DateTimeFormat(locale()).format(new Date(row.bookingDate + "T12:00:00"))}</td><td>{row.description}</td>
        <td>{new Intl.NumberFormat(locale(), { style: "currency", currency: draft.currency }).format(row.amountMinor / 100)}</td>
      </tr>)}</tbody></table></div>
      {rows.length > 200 && <p>{t("Die Vorschau zeigt die ersten 200 Treffer. Die Anwendung betrifft alle bestätigten Treffer.")}</p>}
    </section>}
    <label className="settlement-future"><input type="checkbox" checked={future} disabled={busy} onChange={e => setFuture(e.target.checked)} />{t("Regel auch auf zukünftige Importe anwenden")}</label>
    <label className="settlement-future"><input type="checkbox" checked={past} disabled={busy} onChange={e => setPast(e.target.checked)} />{t("Zusätzlich auf bestehende Buchungen anwenden")}</label>
    {rule && !future && <p role="status">{t("Die gespeicherte Regel wird für zukünftige Importe deaktiviert.")}</p>}
    <p className="intro">{t("Manuelle Entscheidungen bleiben geschützt. Bisherige Markierungen außerhalb der Treffer bleiben unverändert.")}</p>
    <div className="settlement-dialog-actions">
      <button type="button" className="secondary-button" disabled={busy} onClick={onClose}>{t("Abbrechen")}</button>
      <button type="button" className="primary-button" disabled={busy || loading || !preview || (!past && !future) || (past && (!preview.matches.length || preview.matches.length > 5000))} onClick={() => void save()}>{t("Bestätigen & Anwenden")}</button>
    </div>
  </dialog>;
}
