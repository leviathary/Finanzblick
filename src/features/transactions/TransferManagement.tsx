// Verwaltet neutrale Buchungen mit Konto-, Datums- und Textfiltern sowie atomarer Sammelwiederherstellung.
import { TransactionTabs } from "./TransactionTabs";
import { useCallback, useEffect, useRef, useState } from "react";
import { invoke } from "@tauri-apps/api/core";
import { locale, t, tr } from "../../i18n";
import type { TransferType } from "./TransactionActions";
import { TransferRules } from "./TransferRules";
import { nextTransferSort, sortTransfers, type TransferSort, type TransferSortKey } from "./transferSorting";

interface Row { id: number; bookingDate: string; accountName: string; description: string; amountMinor: number; currency: string; transferType: TransferType; counterpartyName: string | null; remittanceInformation: string | null; provider: string }
interface Account { id: number; name: string; currency: string }

export function TransferManagement() {
  const [rows, setRows] = useState<Row[]>([]);
  const [accounts, setAccounts] = useState<Account[]>([]);
  const [account, setAccount] = useState("");
  const [from, setFrom] = useState("");
  const [to, setTo] = useState("");
  const [search, setSearch] = useState("");
  const [selected, setSelected] = useState<number[]>([]);
  const [sort, setSort] = useState<TransferSort>({ key: "bookingDate", descending: true });
  const [busy, setBusy] = useState(false);
  const [loading, setLoading] = useState(false);
  const [error, setError] = useState("");
  const [message, setMessage] = useState("");
  const revision = useRef(0);
  const load = useCallback(async () => {
    const request = ++revision.current;
    setLoading(true); setError(""); setSelected([]);
    try {
      const result = await invoke<Row[]>("list_transaction_transfers", { accountId: account ? Number(account) : null, from: from || null, to: to || null, search });
      if (request === revision.current) setRows(result);
    } catch (reason) { if (request === revision.current) { setRows([]); setError(String(reason)); } }
    finally { if (request === revision.current) setLoading(false); }
  }, [account, from, to, search]);
  useEffect(() => { void load(); return () => { revision.current++; }; }, [load]);
  useEffect(() => { void invoke<Account[]>("list_accounts").then(setAccounts).catch(reason => setError(String(reason))); }, []);
  async function restore(ids: number[]) {
    setBusy(true); setError(""); setMessage("");
    try {
      await invoke("set_transaction_transfers", { transactionIds: ids, transferType: "NONE" });
      setMessage(tr`${ids.length} Buchungen wiederhergestellt.`);
      await load();
    } catch (reason) { setError(String(reason)); }
    finally { setBusy(false); }
  }
  const disabled = busy || loading;
  const restoreHelp = t("Wiederherstellen hebt die Markierung als Umbuchung oder Kartenausgleich auf. Die betroffenen Buchungen zählen wieder in Einnahmen-, Ausgaben- und Budgetauswertungen. Kontostände bleiben unverändert.");
  const typeLabel = (type: string) => t(type === "CREDIT_CARD_SETTLEMENT" ? "Kartenausgleich" : "Umbuchung");
  const sortedRows = sortTransfers(rows, sort, locale(), typeLabel);
  const columns: [TransferSortKey, string][] = [["bookingDate", "Datum"], ["accountName", "Konto"], ["description", "Beschreibung"], ["amountMinor", "Betrag"], ["transferType", "Typ"]];
  return <section className="transactions-page">
    <h1>{t("Umbuchungen & Ausgleiche")}</h1>
    <TransactionTabs active="transfers" />
    <aside className="transfer-help">
      <h2>{t("Neutrale Geldbewegungen")}</h2>
      <p>{t("Überträge zwischen eigenen Konten sowie Kreditkartenabrechnungen sind weder Einnahmen noch Konsumausgaben. Sie verändern deine Kontensaldi, werden aber in Einnahmen-, Ausgaben- und Budgetauswertungen nicht berücksichtigt.")}</p>
      <p>{t("Markiere die Abbuchung und die Gutschrift jeweils über das Drei-Punkte-Menü in der Transaktionsliste. Kartenkäufe und Händlererstattungen bleiben reguläre Buchungen. Es ist keine Verknüpfung der beiden Seiten nötig.")}</p>
    </aside>
    <TransferRules onChanged={load} />
    {error && <p className="error-message" role="alert">{t(error)}</p>}
    {message && <p role="status">{message}</p>}
    <article className="dashboard-card">
      <div className="transfer-filters">
        <label>{t("Konto")}<select value={account} disabled={busy} onChange={e => setAccount(e.target.value)}><option value="">{t("Alle Konten")}</option>{accounts.map(a => <option key={a.id} value={a.id}>{a.name} ({a.currency})</option>)}</select></label>
        <label>{t("Von")}<input type="date" value={from} max={to || undefined} disabled={busy} onChange={e => setFrom(e.target.value)} /></label>
        <label>{t("Bis")}<input type="date" value={to} min={from || undefined} disabled={busy} onChange={e => setTo(e.target.value)} /></label>
        <label>{t("Buchungen durchsuchen")}<input type="search" value={search} disabled={busy} onChange={e => setSearch(e.target.value)} placeholder={t("Beschreibung, Konto oder Bank")} /></label>
      </div>
      <div className="transfer-toolbar">
        <span>{tr`${selected.length} Buchungen ausgewählt`}</span>
        <button type="button" className="secondary-button" disabled={disabled || !selected.length || selected.length > 5000}
          title={restoreHelp} aria-describedby="transfer-restore-help"
          onClick={() => void restore(selected)}>{t("Ausgewählte Buchungen wiederherstellen")}</button>
      </div>
      <p id="transfer-restore-help" className="drilldown-result">{restoreHelp}</p>
      <div className="transfer-table-scroll" aria-busy={loading}><table className="transfer-table">
        <thead><tr><th><input type="checkbox" aria-label={t("Alle angezeigten Buchungen auswählen")} disabled={disabled || !rows.length}
          checked={rows.length > 0 && selected.length === rows.length} onChange={e => setSelected(e.target.checked ? rows.map(r => r.id) : [])} /></th>
          {columns.map(([key, label]) => <th key={key} scope="col" aria-sort={sort.key === key ? sort.descending ? "descending" : "ascending" : undefined}>
            <button type="button" className="expense-sort" disabled={disabled} aria-pressed={sort.key === key}
              aria-label={tr`${t(label)}: ${nextTransferSort(sort, key).descending ? t("absteigend") : t("aufsteigend")} sortieren`}
              onClick={() => setSort(current => nextTransferSort(current, key))}>
              {t(label)} <span aria-hidden="true">{sort.key === key ? sort.descending ? "↓" : "↑" : "↕"}</span>
            </button>
          </th>)}<th scope="col">{t("Aktionen")}</th></tr></thead>
        <tbody>{!loading && sortedRows.map(row => <tr key={row.id}>
          <td><input type="checkbox" disabled={busy} aria-label={tr`Buchung auswählen: ${row.description}`} checked={selected.includes(row.id)}
            onChange={e => setSelected(current => e.target.checked ? [...current, row.id] : current.filter(id => id !== row.id))} /></td>
          <td>{new Intl.DateTimeFormat(locale()).format(new Date(row.bookingDate + "T12:00:00"))}</td><td>{row.accountName}<small>{row.provider}</small></td><td>{row.description}{row.counterpartyName && <small>{row.counterpartyName}</small>}{row.remittanceInformation && <small>{row.remittanceInformation}</small>}</td>
          <td className="transfer-amount">{new Intl.NumberFormat(locale(), { style: "currency", currency: row.currency }).format(row.amountMinor / 100)}</td>
          <td><span className="transfer-badge">{typeLabel(row.transferType)}</span></td>
          <td><button type="button" className="secondary-button" disabled={busy} title={restoreHelp} aria-describedby="transfer-restore-help" onClick={() => void restore([row.id])}>{t("Wiederherstellen")}</button></td>
        </tr>)}</tbody>
      </table></div>
      {loading ? <p role="status">{t("Buchungen werden geladen…")}</p> : !rows.length && <p>{t("Keine neutralisierten Buchungen für diese Auswahl.")}</p>}
    </article>
  </section>;
}
