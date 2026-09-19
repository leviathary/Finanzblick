// Fragt vor einem Kartenausgleich nach dem Umfang und zeigt konto- und richtungsgebundene Regeltreffer.
import { useEffect, useRef, useState } from "react";
import { invoke } from "@tauri-apps/api/core";
import { t, tr, locale } from "../../i18n";

interface Preview { prefix: string; accountName: string; currency: string; direction: number; protectedCount: number; matches: { id: number; bookingDate: string; description: string; amountMinor: number }[] }

export function SettlementRuleDialog({ transaction, onClose, onSaved }: {
  transaction: { id: number; description: string }; onClose: () => void; onSaved: () => Promise<void>;
}) {
  const dialog = useRef<HTMLDialogElement>(null);
  const [prefix, setPrefix] = useState(transaction.description.split("·")[0].trim());
  const [preview, setPreview] = useState<Preview | null>(null);
  const [future, setFuture] = useState(false);
  const [loading, setLoading] = useState(true);
  const [busy, setBusy] = useState(false);
  const [error, setError] = useState("");
  const [revision, setRevision] = useState(0);
  useEffect(() => { dialog.current?.showModal(); }, []);
  useEffect(() => {
    let active = true;
    setLoading(true); setPreview(null); setError("");
    void invoke<Preview>("preview_settlement_rule", { transactionId: transaction.id, prefix })
      .then(result => { if (active) setPreview(result); })
      .catch(reason => { if (active) setError(String(reason)); })
      .finally(() => { if (active) setLoading(false); });
    return () => { active = false; };
  }, [transaction.id, prefix, revision]);
  async function save(all: boolean) {
    setBusy(true); setError("");
    try {
      if (all && preview) {
        await invoke("confirm_settlement_rule", { transactionId: transaction.id, prefix: preview.prefix, expectedIds: preview.matches.map(row => row.id), future });
      } else {
        await invoke("set_transaction_transfers", { transactionIds: [transaction.id], transferType: "CREDIT_CARD_SETTLEMENT" });
      }
      await onSaved(); onClose();
    } catch (reason) { setError(String(reason)); }
    finally { setBusy(false); }
  }
  return <dialog ref={dialog} className="settlement-rule-dialog" aria-labelledby="settlement-rule-title"
    onCancel={event => { event.preventDefault(); if (!busy) onClose(); }}>
    <h2 id="settlement-rule-title">{t("Auch ähnliche Buchungen als Kartenausgleich markieren?")}</h2>
    <p>{t("Es gelten nur Buchungen desselben Kontos, derselben Währung und Zahlungsrichtung. Beträge und Datum dürfen abweichen. Manuelle Entscheidungen bleiben geschützt.")}</p>
    <label>{t("Buchungstext beginnt mit")}<input autoFocus value={prefix} disabled={busy} onChange={event => setPrefix(event.target.value)} /></label>
    <p className="intro">{t("Prüfe den Textanfang und die Treffer. Eine zu allgemeine Regel kann normale Ausgaben fälschlich ausblenden. Die Regel gilt nicht für das Gegenkonto.")}</p>
    {error && <p className="error-message" role="alert">{t(error)} <button type="button" disabled={busy} onClick={() => setRevision(value => value + 1)}>{t("Vorschau neu laden")}</button></p>}
    {loading && <p role="status">{t("Buchungen werden geladen…")}</p>}
    {preview && <>
      <p><strong>{preview.accountName} · {preview.currency} · {t(preview.direction < 0 ? "Belastung" : "Gutschrift")}</strong></p>
      <p>{tr`${preview.matches.length} passende Buchungen in der gesamten Historie. ${preview.protectedCount} manuelle Entscheidungen werden nicht verändert.`}</p>
      <div className="settlement-preview"><table><thead><tr><th>{t("Datum")}</th><th>{t("Beschreibung")}</th><th>{t("Betrag")}</th></tr></thead><tbody>
        {preview.matches.map(row => <tr key={row.id}><td>{row.bookingDate}</td><td>{row.description}</td><td>{new Intl.NumberFormat(locale(), { style: "currency", currency: preview.currency }).format(row.amountMinor / 100)}</td></tr>)}
      </tbody></table></div>
    </>}
    <label className="settlement-future"><input type="checkbox" checked={future} disabled={busy} onChange={event => setFuture(event.target.checked)} />{t("Regel auch auf zukünftige Importe anwenden")}</label>
    <p className="intro">{t("Gespeicherte Regeln lassen sich unter Umbuchungen & Ausgleiche deaktivieren.")}</p>
    <div className="settlement-dialog-actions">
      <button type="button" className="secondary-button" disabled={busy} onClick={onClose}>{t("Abbrechen")}</button>
      <button type="button" className="secondary-button" disabled={busy} onClick={() => void save(false)}>{t("Nur diese Buchung")}</button>
      <button type="button" className="primary-button" disabled={busy || loading || !preview?.matches.length || preview.matches.length > 5000} onClick={() => void save(true)}>{t("Auf passende Buchungen anwenden")}</button>
    </div>
  </dialog>;
}
