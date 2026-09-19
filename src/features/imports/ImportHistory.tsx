// Zeigt importierte Belege mit Such-, Filter-, Gruppierungs- und Löschfunktionen.

import { t, tr, locale } from "../../i18n";
import { useEffect, useMemo, useState } from "react";
import { invoke, isTauri } from "@tauri-apps/api/core";
import { filenameFilter } from "./filenameFilter";

interface ImportRun {
  id: number; sourceName: string; sourceFormat: string; provider: string; accounts: string;
  importedAt: string; transactionCount: number; firstDate: string | null; lastDate: string | null;
}

interface WealthDataBasis {
  history: { date: string; totalMinor: number }[];
  excludedAccountCount: number;
}

export function ImportHistory() {
  const [grouping, setGrouping] = useState("month");
  const [expanded, setExpanded] = useState<string[]>([]);
  const [imports, setImports] = useState<ImportRun[]>([]);
  const [dataBasis, setDataBasis] = useState<WealthDataBasis | null>(null);
  const [selected, setSelected] = useState<number[]>([]);
  const [pending, setPending] = useState<ImportRun[]>([]);
  const [loading, setLoading] = useState(true);
  const [deleting, setDeleting] = useState(false);
  const [error, setError] = useState<string | null>(null);
  const [message, setMessage] = useState<string | null>(null);
  const [search, setSearch] = useState("");
  const [regex, setRegex] = useState(false);
  const [fileType, setFileType] = useState("");
  const filter = useMemo(() => filenameFilter(search, regex), [search, regex]);
  const visibleImports = imports.filter(row => filter.matches(row.sourceName) && (!fileType || (fileType === "excel" ? ["XLS", "XLSX"].includes(row.sourceFormat.toUpperCase()) : row.sourceFormat.toUpperCase() === fileType)));
  const visibleSelected = visibleImports.filter(row => selected.includes(row.id));
  const storedTransactions = imports.reduce((sum, row) => sum + row.transactionCount, 0);
  const recentImport = imports[0];

  async function refresh() {
    setLoading(true); setError(null);
    try {
      if (!isTauri()) throw new Error(t("Die Importverwaltung ist in der Desktop-App verfügbar."));
      const [rows, wealth] = await Promise.all([
        invoke<ImportRun[]>("list_imports"),
        invoke<WealthDataBasis>("wealth_data", { accountIds: null }),
      ]);
      setImports(rows); setDataBasis(wealth); setSelected(current => current.filter(id => rows.some(row => row.id === id)));
    } catch (reason) { setError(String(reason)); }
    finally { setLoading(false); }
  }
  useEffect(() => { void refresh(); }, []);

  async function remove() {
    setDeleting(true); setError(null);
    const ids = pending.map(row => row.id);
    try {
      const count = await invoke<number>("delete_imports", { ids });
      setPending([]); setSelected(current => current.filter(id => !ids.includes(id)));
      setMessage(tr`${count} ${count === 1 ? t("Import gelöscht") : t("Importe gelöscht")}. Die Dateien können erneut importiert werden.`);
      window.dispatchEvent(new CustomEvent("imports-deleted", { detail: ids }));
      await refresh();
    } catch (reason) { setError(String(reason)); }
    finally { setDeleting(false); }
  }

  const locked = loading || deleting || pending.length > 0;
  const groups = new Map<string, ImportRun[]>();
  for (const row of visibleImports) {
    const period = row.lastDate ?? row.importedAt.slice(0, 10);
    const key = grouping === "provider" ? row.provider : grouping === "year" ? period.slice(0, 4) : grouping === "month" ? period.slice(0, 7) : t("Alle Importe");
    groups.set(key, [...(groups.get(key) ?? []), row]);
  }
  const orderedGroups = [...groups.entries()].sort(([a], [b]) => grouping === "provider" ? a.localeCompare(b, locale()) : b.localeCompare(a));
  useEffect(() => {
    if (search || fileType) setExpanded([...groups.keys()]);
  }, [search, regex, fileType, grouping, imports]);
  function groupLabel(key: string) {
    if (grouping !== "month") return key;
    return new Date(`${key}-01T12:00:00`).toLocaleDateString(locale(), { month: "long", year: "numeric" });
  }
  function selectGroup(rows: ImportRun[], checked: boolean) {
    const ids = rows.map(row => row.id);
    setSelected(current => checked ? [...new Set([...current, ...ids])] : current.filter(id => !ids.includes(id)));
  }
  function table(rows: ImportRun[]) {
    return <div className="history-table"><table>
      <thead><tr><th><input type="checkbox" aria-label={t("Alle Importe in dieser Gruppe auswählen")} disabled={locked} checked={rows.every(row => selected.includes(row.id))} ref={element => { if (element) element.indeterminate = rows.some(row => selected.includes(row.id)) && !rows.every(row => selected.includes(row.id)); }} onChange={event => selectGroup(rows, event.target.checked)} /></th><th>{t("Datei / Anbieter")}</th><th>{t("Konten")}</th><th>{t("Buchungszeitraum")}</th><th>{t("Importiert am")}</th><th>{t("Buchungen")}</th><th>{t("Aktion")}</th></tr></thead>
      <tbody>{rows.map(row => <tr key={row.id}><td><input type="checkbox" aria-label={tr`Import ${row.id}: ${row.sourceName} auswählen`} checked={selected.includes(row.id)} disabled={locked} onChange={event => selectGroup([row], event.target.checked)} /></td><td><strong>{row.sourceName}</strong><small>#{row.id} · {row.provider} · {row.sourceFormat}</small></td><td>{row.accounts}</td><td>{date(row.firstDate)} – {date(row.lastDate)}</td><td>{new Date(row.importedAt).toLocaleString(locale())}</td><td>{row.transactionCount}</td><td><button className="danger-button" disabled={locked} onClick={() => { setError(null); setPending([row]); }}>{t("Löschen")}</button></td></tr>)}</tbody>
    </table></div>;
  }
  return <section className="import-history">
    <div className="overview-heading"><div><p className="eyebrow">{t("Importverwaltung")}</p><h1>{t("Importierte Dateien")}</h1><p className="intro">{t("Prüfe deine Importe und entferne bei Bedarf einzelne oder mehrere Auszüge.")}</p></div><a className="primary-button" href="#imports">{t("Datei importieren")}</a></div>
    {message && <p className="import-result" role="status">{t(message)}</p>}
    {error && <p className="error-message" role="alert">{t(error)}</p>}
    {!loading && recentImport && <div className="import-management-overview">
      <article className="summary-card">
        <span>{t("Gespeicherte Buchungen")}</span>
        <strong>{storedTransactions.toLocaleString(locale())}</strong>
        <small>{t("aus allen bisherigen Importen")}</small>
      </article>
      <article className="summary-card">
        <span>{t("Letzter Import")}</span>
        <strong>{formatDateTime(recentImport.importedAt)}</strong>
        <small>{tr`${recentImport.provider} · ${recentImport.transactionCount} Buchungen`}</small>
      </article>
      {dataBasis && <article className="summary-card">
        <span>{t("Datenbasis")}</span>
        <strong>{dataBasis.history.length.toLocaleString(locale())}</strong>
        <small>{t("Stichtage ·")} {dataBasis.excludedAccountCount} {t("ausgeschlossen")}</small>
      </article>}
      <article className="dashboard-card recent-import-card">
        <p className="eyebrow">{t("Aktivität")}</p>
        <h2>{t("Letzter Import")}</h2>
        <strong>{recentImport.sourceName}</strong>
        <span>{recentImport.provider} · {recentImport.accounts}</span>
        <small>{recentImport.transactionCount} {t("Buchungen, importiert am")} {formatDateTime(recentImport.importedAt)}</small>
      </article>
    </div>}
    <div className="history-search">
      <label className="history-search-field" htmlFor="import-filename-search">{t("Dateiname durchsuchen")}<input id="import-filename-search" type="search" value={search} disabled={locked} placeholder={regex ? "UBS|Kontoauszug" : t("z. B. UBS oder *Kontoauszug*")} aria-describedby={filter.error ? "import-search-help import-search-error" : "import-search-help"} aria-invalid={Boolean(filter.error)} onChange={event => { setSearch(event.target.value); setSelected([]); }} />
      </label>
      <label className="history-type-filter">{t("Dateityp")}<select value={fileType} disabled={locked} onChange={event => { setFileType(event.target.value); setSelected([]); }}><option value="">{t("Alle Dateitypen")}</option><option value="CAMT053">camt.053 (XML)</option><option value="MT940">MT940 (.mt940 / .sta)</option><option value="PDF">PDF</option><option value="CSV">CSV</option><option value="excel">Excel (XLS / XLSX)</option><option value="XLS">XLS</option><option value="XLSX">XLSX</option></select></label>
      <label className="history-search-mode"><input type="checkbox" checked={regex} disabled={locked} onChange={event => { setRegex(event.target.checked); setSelected([]); }} />  {t("Regulärer Ausdruck (Regex)")}</label>
      {(search || fileType) && <button className="secondary-button" disabled={locked} onClick={() => { setSearch(""); setFileType(""); setSelected([]); }}>{t("Filter zurücksetzen")}</button>}
    </div>
    <p className="intro" id="import-search-help">{regex ? "Regex ohne /…/ eingeben, z. B. UBS|Kontoauszug." : t("Sucht im Dateinamen. * steht für beliebig viele Zeichen, ? für ein Zeichen.")}  {t("Gross- und Kleinschreibung wird ignoriert.")}</p>
    {filter.error && <p className="error-message" id="import-search-error" role="alert">{filter.error}</p>}
    <div className="history-group-controls"><label>{t("Gruppieren nach")} <select value={grouping} onChange={event => { setGrouping(event.target.value); setExpanded([]); }}><option value="month">{t("Monat und Jahr")}</option><option value="year">{t("Jahr")}</option><option value="provider">{t("Anbieter")}</option><option value="none">{t("Keine Gruppierung")}</option></select></label>{grouping !== "none" && <><button className="secondary-button" onClick={() => setExpanded([...groups.keys()])}>{t("Alle aufklappen")}</button><button className="secondary-button" onClick={() => setExpanded([])}>{t("Alle zuklappen")}</button></>}</div>
    {(grouping === "month" || grouping === "year") && <p className="intro">{t("Zuordnung nach dem letzten Buchungsdatum des Auszugs; ohne Buchungen nach dem Importdatum.")}</p>}
    <div className="history-actions"><span role="status">{search || fileType ? tr`${visibleImports.length} von ${imports.length} Importen` : tr`${imports.length} Importe`} · {visibleSelected.length}  {t("ausgewählt")}</span><button className="secondary-button" disabled={locked} onClick={() => void refresh()}>{t("Aktualisieren")}</button><button className="danger-button" disabled={locked || visibleSelected.length === 0} onClick={() => { setError(null); setPending(visibleSelected); }}>{t("Auswahl löschen")}</button></div>
    {!loading && imports.length > 0 && visibleImports.length === 0 && !filter.error && <p role="status">{t("Keine Dateien gefunden. Passe den Suchbegriff oder Dateityp an oder setze die Filter zurück.")}</p>}
    {pending.length > 0 && <div className="history-confirm" role="region" aria-label={t("Löschung bestätigen")}>
      <h2>{pending.length === 1 ? t("Diesen Import löschen?") : tr`${pending.length} Importe löschen?`}</h2>
      <ul>{pending.map(row => <li key={row.id}>#{row.id} · {row.sourceName} · {row.transactionCount}  {t("Buchungen")}</li>)}</ul>
      <p>{t("Es werden")} {pending.reduce((sum, row) => sum + row.transactionCount, 0)}  {t("Buchungen und alle zugehörigen Salden entfernt. Konten und Originaldateien bleiben erhalten. Die Übersicht verwendet danach die verbleibenden Importe.")}</p>
      <div className="history-actions"><button className="secondary-button" disabled={deleting} onClick={() => setPending([])}>{t("Abbrechen")}</button><button className="danger-button" disabled={deleting} onClick={() => void remove()}>{deleting ? t("Wird gelöscht…") : t("Importe endgültig löschen")}</button></div>
    </div>}
    {loading ? <p role="status">{t("Importe werden geladen…")}</p> : imports.length === 0 && !error ? <div className="wizard-card"><h2>{t("Noch keine Importe")}</h2><p>{t("Importiere einen Auszug, um ihn hier verwalten zu können.")}</p></div> : <div className="history-groups">{orderedGroups.map(([key, rows]) => grouping === "none" ? <div key={key}>{table(rows)}</div> : <section className="history-group" key={key}>
      <div className="history-group-heading">
        <input type="checkbox" aria-label={tr`${groupLabel(key)} auswählen`} disabled={locked} checked={rows.every(row => selected.includes(row.id))} ref={element => { if (element) element.indeterminate = rows.some(row => selected.includes(row.id)) && !rows.every(row => selected.includes(row.id)); }} onChange={event => selectGroup(rows, event.target.checked)} />
        <button type="button" aria-expanded={expanded.includes(key)} onClick={() => setExpanded(current => current.includes(key) ? current.filter(value => value !== key) : [...current, key])}><span>{expanded.includes(key) ? "▾" : "▸"} {groupLabel(key)}</span><small>{rows.length}  {t("Importe ·")} {rows.reduce((sum, row) => sum + row.transactionCount, 0)}  {t("Buchungen")}{rows.some(row => selected.includes(row.id)) ? tr` · ${rows.filter(row => selected.includes(row.id)).length} ausgewählt` : ""}</small></button>
      </div>
      {expanded.includes(key) && table(rows)}
    </section>)}</div>}
  </section>;
}

function date(value: string | null) { return value ? value.split("-").reverse().join(".") : "–"; }

function formatDateTime(value: string): string {
  const parsed = new Date(value);
  return Number.isNaN(parsed.getTime())
    ? value
    : new Intl.DateTimeFormat(locale(), { dateStyle: "medium", timeStyle: "short" }).format(parsed);
}
