// Prüft bestehende Buchungen auf mögliche Dubletten und führt durch reversible Einzelentscheidungen.
import { useState } from "react";
import { invoke, isTauri } from "@tauri-apps/api/core";
import { locale, t } from "../../i18n";
import { DuplicateRemovalDialog } from "../transactions/DuplicateRemovalDialog";

interface AuditTransaction {
  id: number; bookingDate: string; valueDate: string | null; description: string;
  amountMinor: number; currency: string; accountName: string; provider: string;
  sourceName: string; importedAt: string; sourceRow: number;
}

interface AuditGroup {
  matchKind: "exact" | "possible";
  transactions: AuditTransaction[];
}

export function DuplicateAudit({ onChanged }: { onChanged: () => void }) {
  const [groups, setGroups] = useState<AuditGroup[] | null>(null);
  const [running, setRunning] = useState(false);
  const [acting, setActing] = useState(false);
  const [error, setError] = useState<string | null>(null);
  const [message, setMessage] = useState<string | null>(null);
  const [removal, setRemoval] = useState<AuditTransaction | null>(null);
  const [removalError, setRemovalError] = useState<string | null>(null);

  async function runAudit(successMessage?: string) {
    setRunning(true); setError(null); setMessage(null);
    try {
      if (!isTauri()) throw new Error(t("Die Bestandsprüfung ist in der Desktop-App verfügbar."));
      const result = await invoke<AuditGroup[]>("audit_duplicate_transactions");
      setGroups(result);
      setMessage(successMessage ?? (result.length === 0 ? t("Keine ungeklärten Duplikate gefunden.") : null));
    } catch (reason) { setError(String(reason)); }
    finally { setRunning(false); }
  }

  async function dismiss(group: AuditGroup) {
    setActing(true); setError(null); setMessage(null);
    try {
      await invoke("dismiss_duplicate_candidate_group", { transactionIds: group.transactions.map(row => row.id) });
      await runAudit(t("Die Gruppe wurde als echte Mehrfachzahlung gespeichert und wird nicht erneut angezeigt."));
    } catch (reason) { setError(String(reason)); }
    finally { setActing(false); }
  }

  async function removeDuplicate() {
    if (!removal) return;
    setActing(true); setRemovalError(null);
    try {
      await invoke("ignore_duplicate_transaction", { transactionId: removal.id });
      setRemoval(null); onChanged();
      await runAudit(t("Die doppelte Buchung wurde entfernt. Du kannst sie unter „Importierte Dateien“ wiederherstellen."));
    } catch (reason) { setRemovalError(String(reason)); }
    finally { setActing(false); }
  }

  const transactionCount = groups?.reduce((sum, group) => sum + group.transactions.length, 0) ?? 0;
  return <section className="dashboard-card duplicate-audit" aria-labelledby="duplicate-audit-title">
    <div className="duplicate-audit-heading">
      <div><p className="eyebrow">{t("Datenqualität")}</p><h2 id="duplicate-audit-title">{t("Bestand auf Duplikate prüfen")}</h2><p>{t("Findet mögliche Dubletten aus früheren Importen, ohne Buchungen automatisch zu verändern.")}</p></div>
      <button type="button" className="secondary-button" disabled={running || acting} onClick={() => void runAudit()}>{running ? t("Bestand wird geprüft …") : groups === null ? t("Prüfung starten") : t("Erneut prüfen")}</button>
    </div>
    {error && <p className="error-message" role="alert">{t(error)}</p>}
    {message && <p className="mapping-ready-notice" role="status">{t(message)}</p>}
    {groups && groups.length > 0 && <>
      <div className="duplicate-audit-summary" role="status"><strong>{groups.length} {groups.length === 1 ? t("Verdachtsgruppe") : t("Verdachtsgruppen")}</strong><span>{transactionCount} {t("beteiligte Buchungen")}</span></div>
      <div className="duplicate-audit-groups">{groups.map(group => {
        const first = group.transactions[0];
        const ids = group.transactions.map(row => row.id).sort((left, right) => left - right).join("-");
        return <details className={`duplicate-audit-group ${group.matchKind}`} key={ids} open>
          <summary><span><strong>{group.matchKind === "exact" ? t("Eindeutiges Duplikat") : t("Mögliches Duplikat")}</strong><small>{first.provider} · {first.accountName}</small></span><span>{money(first.amountMinor, first.currency)} · {group.transactions.length} {t("Buchungen")}</span></summary>
          <div className="duplicate-audit-table"><table><thead><tr><th>{t("Datum")}</th><th>{t("Beschreibung")}</th><th>{t("Importquelle")}</th><th>{t("Betrag")}</th><th>{t("Aktion")}</th></tr></thead>
            <tbody>{group.transactions.map(row => <tr key={row.id}><td>{date(row.bookingDate)}</td><td><strong>{row.description}</strong><small>{row.provider} · {row.accountName}</small></td><td>{row.sourceName}<small>{t("Importiert am")} {dateTime(row.importedAt)} · {t("Zeile")} {row.sourceRow}</small></td><td className="amount-cell">{money(row.amountMinor, row.currency)}</td><td><button type="button" className="danger-button" disabled={acting || running} onClick={() => { setRemovalError(null); setRemoval(row); }}>{t("Als Duplikat entfernen …")}</button></td></tr>)}</tbody>
          </table></div>
          <div className="duplicate-audit-footer"><p>{group.matchKind === "exact" ? t("Datum, Betrag und Buchungstext stimmen überein.") : t("Datum und Betrag stimmen überein oder der Buchungstext ist innerhalb von zwei Tagen ähnlich.")}</p><button type="button" className="secondary-button" disabled={acting || running} onClick={() => void dismiss(group)}>{t("Kein Duplikat – nicht mehr anzeigen")}</button></div>
        </details>;
      })}</div>
    </>}
    {removal && <DuplicateRemovalDialog description={removal.description} date={date(removal.bookingDate)} account={`${removal.provider} · ${removal.accountName}`} amount={money(removal.amountMinor, removal.currency)} busy={acting} error={removalError} onCancel={() => { setRemoval(null); setRemovalError(null); }} onConfirm={() => void removeDuplicate()} />}
  </section>;
}

function money(value: number, currency: string) { return new Intl.NumberFormat(locale(), { style: "currency", currency }).format(value / 100); }
function date(value: string) { return new Date(`${value}T12:00:00`).toLocaleDateString(locale()); }
function dateTime(value: string) { const parsed = new Date(value); return Number.isNaN(parsed.getTime()) ? value : parsed.toLocaleString(locale()); }
