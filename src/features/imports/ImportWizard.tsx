import { t, tr, locale } from "../../i18n";
import { useEffect, useRef, useState } from "react";
import { invoke, isTauri } from "@tauri-apps/api/core";
import { getCurrentWebview } from "@tauri-apps/api/webview";
import { open } from "@tauri-apps/plugin-dialog";
import { detectProvider, formatFileSize, getSupportedExtension, providers, type ProviderId } from "./fileDetection";
import type { ImportAccount, ParsedStatement, SaveImportResult } from "./importTypes";
import { displayedProvider, currencies, hasAccounts, matchingAccounts, readyToSave, saveBatch, suggestAccounts, type BatchItem } from "./importBatch";

interface FileSelection {
  files: Array<{ path: string; name: string; size: number }>;
  warnings: string[];
}

export function ImportWizard({ enabled = true }: { enabled?: boolean }) {
  const [items, setItems] = useState<BatchItem[]>([]);
  const [accounts, setAccounts] = useState<ImportAccount[]>([]);
  const [accountsLoaded, setAccountsLoaded] = useState(false);
  const [active, setActive] = useState<string | null>(null);
  const [recursive, setRecursive] = useState(false);
  const [busy, setBusy] = useState(false);
  const [progress, setProgress] = useState("");
  const [error, setError] = useState<string | null>(null);
  const [warnings, setWarnings] = useState<string[]>([]);
  const [completed, setCompleted] = useState<{ imported: number; duplicates: number } | null>(null);
  const [dragging, setDragging] = useState(false);
  const busyRef = useRef(false);
  const stopRef = useRef(false);
  const previewRef = useRef<HTMLDivElement>(null);
  const recursiveRef = useRef(recursive);
  const itemsRef = useRef(items);
  const accountsRef = useRef(accounts);
  itemsRef.current = items;
  accountsRef.current = accounts;
  recursiveRef.current = recursive;
  useEffect(() => { previewRef.current?.scrollIntoView({ block: "start" }); }, [active]);

  function update(path: string, changes: Partial<BatchItem>) {
    setItems(current => current.map(item => item.file.path === path ? { ...item, ...changes } : item));
  }

  function begin() {
    if (busyRef.current) return false;
    busyRef.current = true;
    stopRef.current = false;
    setBusy(true);
    setError(null);
    return true;
  }

  function finish() {
    busyRef.current = false;
    setBusy(false);
    setProgress("");
  }

  async function loadAccounts() {
    if (!isTauri()) return;
    try {
      setAccounts(await invoke<ImportAccount[]>("list_accounts"));
      setAccountsLoaded(true);
    }
    catch (reason) { setError(tr`Konten konnten nicht geladen werden: ${String(reason)}`); }
  }

  useEffect(() => {
    void loadAccounts();
    const refresh = () => { if (window.location.hash === "#imports") void loadAccounts(); };
    const deleted = (event: Event) => {
      const ids = (event as CustomEvent<number[]>).detail;
      setItems(current => current.map(item => item.result && ids.includes(item.result.importId) ? { ...item, result: undefined, reviewed: false } : item));
    };
    window.addEventListener("focus", refresh);
    window.addEventListener("hashchange", refresh);
    window.addEventListener("imports-deleted", deleted);
    return () => {
      window.removeEventListener("focus", refresh);
      window.removeEventListener("hashchange", refresh);
      window.removeEventListener("imports-deleted", deleted);
    };
  }, []);

  async function addPaths(paths: string[]) {
    setCompleted(null);
    setProgress(t("Dateien werden gesammelt…"));
    const selection = await invoke<FileSelection>("collect_import_files", { paths, recursive: recursiveRef.current });
    setWarnings(selection.warnings);
    const known = new Set(itemsRef.current.map(item => item.file.path));
    const added: BatchItem[] = selection.files.filter(file => !known.has(file.path)).flatMap(file => {
      const extension = getSupportedExtension(file.name);
      return extension ? [{ file: { ...file, extension, provider: detectProvider(file.name) }, accountIds: {}, reviewed: false }] : [];
    });
    setItems(current => {
      const known = new Set(current.map(item => item.file.path));
      return [...current, ...added.filter(item => !known.has(item.file.path))];
    });
    if (!selection.files.length) setError(t("Keine unterstützten Dateien gefunden. Unterstützt werden XLSX, XLS, CSV, PDF und MT940 bis 25 MB pro Datei."));
    await analyzeFiles(added);
  }

  async function choose(directory: boolean) {
    if (!isTauri()) { setError(t("Bitte Dateien oder Ordner in der Desktop-App auswählen.")); return; }
    if (!begin()) return;
    try {
      const selected = await open({ directory, multiple: true, title: directory ? t("Ordner mit Bankauszügen auswählen") : t("Bankauszüge auswählen"), ...(directory ? {} : { filters: [{ name: "Bankauszüge", extensions: ["xlsx", "xls", "csv", "pdf", "mt940", "sta"] }] }) });
      if (selected) await addPaths(Array.isArray(selected) ? selected : [selected]);
    } catch (reason) { setError(String(reason)); }
    finally { finish(); }
  }

  useEffect(() => {
    if (!isTauri()) return;
    let disposed = false;
    let unlisten: (() => void) | undefined;
    void getCurrentWebview().onDragDropEvent(event => {
      if (!enabled || window.location.hash !== "#imports" || busyRef.current) return;
      if (event.payload.type === "enter" || event.payload.type === "over") setDragging(true);
      else if (event.payload.type === "drop") {
        setDragging(false);
        if (!begin()) return;
        void addPaths(event.payload.paths).catch(reason => setError(String(reason))).finally(finish);
      } else setDragging(false);
    }).then(dispose => { if (disposed) dispose(); else unlisten = dispose; }).catch(reason => setError(String(reason)));
    return () => { disposed = true; unlisten?.(); };
  }, [enabled]);

  async function analyze(targets: BatchItem[]) {
    if (!begin()) return;
    try { await analyzeFiles(targets); } finally { finish(); }
  }

  async function analyzeFiles(targets: BatchItem[]) {
      for (let index = 0; index < targets.length; index++) {
        if (stopRef.current) break;
        const item = targets[index];
        setProgress(tr`Analyse ${index + 1} von ${targets.length}: ${item.file.name}`);
        update(item.file.path, { error: undefined });
        try {
          if (await invoke<boolean>("is_file_imported", { path: item.file.path })) {
            update(item.file.path, { alreadyImported: true, duplicateNotice: "Datei bereits vorhanden. Diese Datei wurde schon importiert.", reviewed: false });
            continue;
          }
          const parsed = await invoke<ParsedStatement>("parse_statement", { path: item.file.path, selectedProvider: item.file.provider === "unknown" ? null : item.file.provider });
          const next = { ...item, parsed, file: { ...item.file, provider: parsed.provider }, accountIds: suggestAccounts(accountsRef.current, parsed), reviewed: false };
          update(item.file.path, { ...next, ...await checkDuplicates(next) });
        } catch (reason) { update(item.file.path, { error: String(reason), parsed: undefined, reviewed: false }); }
      }
  }

  async function save() {
    const targets = items.filter(item => readyToSave(item, accounts));
    if (!targets.length || !begin()) return;
    let count = 0;
    const finishedPaths = new Set<string>();
    const summary = { imported: 0, duplicates: 0 };
    try {
      await saveBatch(targets, async item => {
        setProgress(tr`Import ${++count} von ${targets.length}: ${item.file.name}`);
        const duplicate = await checkDuplicates(item);
        if (duplicate.alreadyImported) { update(item.file.path, duplicate); throw new Error(duplicate.duplicateNotice); }
        const account = accounts.find(account => Object.values(item.accountIds).includes(account.id));
        const statement = item.parsed?.provider === "unknown" ? { ...item.parsed, provider: account?.providerKey } : item.parsed;
        return invoke<SaveImportResult>("save_import", { request: { sourcePath: item.file.path, accountName: account?.name ?? "", accountIds: item.accountIds, statement } });
      }, (path, change) => {
        update(path, change);
        if (change.result) {
          finishedPaths.add(path);
          if (change.result.duplicate) summary.duplicates++; else summary.imported++;
        }
      }, () => stopRef.current);
    } finally {
      setItems(current => current.filter(item => !finishedPaths.has(item.file.path)));
      setActive(current => current && finishedPaths.has(current) ? null : current);
      if (finishedPaths.size) { setCompleted(summary); setWarnings([]); }
      finish();
    }
  }

  async function checkDuplicates(item: BatchItem) {
    if (!item.parsed) return { alreadyImported: false, duplicateNotice: undefined };
    const check = await invoke<{ exactFile: boolean; matchingTransactions: number; totalTransactions: number }>("check_import_duplicates", { request: { sourcePath: item.file.path, accountName: "", accountIds: item.accountIds, statement: item.parsed } });
    const all = check.totalTransactions > 0 && check.matchingTransactions === check.totalTransactions;
    return { alreadyImported: check.exactFile || all, duplicateNotice: check.exactFile ? t("Datei bereits vorhanden. Diese Datei wurde schon importiert.") : all ? t("Datei schon importiert: Alle Transaktionen sind auf dem gewählten Konto bereits vorhanden.") : check.matchingTransactions > 0 ? tr`${check.matchingTransactions} von ${check.totalTransactions} Transaktionen sind auf dem gewählten Konto bereits vorhanden und werden beim Import übersprungen.` : undefined };
  }

  async function changeAccounts(item: BatchItem, accountIds: Record<string, number>) {
    if (!begin()) return;
    const next = { ...item, accountIds, reviewed: false };
    update(item.file.path, next);
    try { update(item.file.path, { ...await checkDuplicates(next), error: undefined }); }
    catch (reason) { update(item.file.path, { error: String(reason) }); }
    finally { finish(); }
  }

  const current = items.find(item => item.file.path === active);
  const currentIndex = items.findIndex(item => item.file.path === active);
  const nextOpenPreview = current
    ? items
        .slice(currentIndex + 1)
        .find(item => item.parsed && !item.result && !item.alreadyImported)
    : undefined;
  const pending = items.filter(item => !item.parsed && !item.result && !item.alreadyImported);
  const ready = items.filter(item => readyToSave(item, accounts));
  const noAccounts = accountsLoaded && accounts.length === 0;
  const releaseWarnings = new Map<string, number>();
  const releaseAccounts = new Map<string, number>();
  for (const item of ready) {
    for (const warning of new Set(item.parsed?.warnings ?? [])) releaseWarnings.set(warning, (releaseWarnings.get(warning) ?? 0) + 1);
    if (item.duplicateNotice) releaseWarnings.set(item.duplicateNotice, (releaseWarnings.get(item.duplicateNotice) ?? 0) + 1);
    const names = [...new Set(Object.values(item.accountIds))].map(id => {
      const account = accounts.find(account => account.id === id)!;
      return `${account.provider} · ${account.name} (${account.currency})`;
    }).join(", ");
    releaseAccounts.set(names, (releaseAccounts.get(names) ?? 0) + 1);
  }
  const failed = items.filter(item => item.error).length;

  function status(item: BatchItem) {
    if (item.alreadyImported) return t("Datei bereits vorhanden");
    if (item.result) return item.result.duplicate ? t("Bereits importiert") : t("Importiert");
    if (item.error) return t("Fehler – erneut versuchen");
    if (!item.parsed) return t("Noch nicht analysiert");
    if (!hasAccounts(item, accounts)) return t("Kontozuordnung fehlt");
    return t("Bereit zum Import");
  }

  return <section className="import-wizard" aria-labelledby="import-title">
    <div className="wizard-heading"><div><p className="eyebrow">{t("Import")}</p><h1 id="import-title">{t("Bankauszüge importieren")}</h1><p className="intro">{t("Mehrere Dateien oder ganze Ordner auswählen, Kontozuordnungen prüfen und direkt importieren. Alle Dateien bleiben auf diesem Computer.")}</p></div><span className="privacy-chip">{t("✓ Lokale Verarbeitung")}</span></div>
    <div className={`drop-zone batch-drop${dragging ? " dragging" : ""}`} onDragOver={event => event.preventDefault()} onDrop={event => { event.preventDefault(); if (!isTauri()) setError(t("Drag-and-drop ist in der Desktop-App verfügbar.")); }}>
      <h2>{t("Dateien oder Ordner hierher ziehen")}</h2>
      <div className="batch-buttons"><button className="primary-button" disabled={busy} onClick={() => void choose(false)}>{t("Dateien auswählen")}</button><button className="secondary-button" disabled={busy} onClick={() => void choose(true)}>{t("Ordner auswählen")}</button></div>
      <label className="batch-check"><input type="checkbox" checked={recursive} disabled={busy} onChange={event => setRecursive(event.target.checked)} />  {t("Unterordner einbeziehen")}</label>
      <small>{t("XLSX, XLS, CSV, PDF und MT940 · maximal 25 MB pro Datei")}</small>
    </div>
    {error && <p className="error-message" role="alert">{t(error)}</p>}
    {completed && <div className="batch-complete" role="status"><div><h2>{t("Import abgeschlossen")}</h2><p>{completed.imported}  {t("Dateien importiert")}{completed.duplicates > 0 ? tr` · ${completed.duplicates} bereits vorhanden` : ""}.</p><p>{items.length ? t("Offene oder fehlgeschlagene Dateien stehen weiterhin unten in der Liste.") : t("Die Importliste ist leer. Du kannst jetzt weitere Dateien oder Ordner hinzufügen.")}</p></div><a className="secondary-button" href="#import-history">{t("Importe ansehen")}</a></div>}
    {warnings.length > 0 && <details className="warning-message"><summary>{warnings.length}  {t("Hinweise zur Dateiauswahl")}</summary><ul>{warnings.map((warning, index) => <li key={index}>{warning}</li>)}</ul></details>}
    {busy && <div className="info-panel" role="status"><p>{progress || t("Dateiauswahl geöffnet…")}</p>{(progress.startsWith("Analyse ") || progress.startsWith("Import ")) && <button className="secondary-button" disabled={stopRef.current} onClick={() => { stopRef.current = true; setProgress(value => tr`${value} · Stopp angefordert`); }}>{t("Nach aktueller Datei stoppen")}</button>}</div>}
    {items.length > 0 && <>
      <div className="batch-toolbar"><div><h2>{t("Deine Importliste")}</h2><p role="status">{items.length}  {t("Dateien")}{failed > 0 ? tr` · ${failed} Fehler` : ""}{ready.length > 0 ? tr` · ${ready.length} bereit zum Import` : ""}</p></div><div className="batch-buttons">{pending.length > 0 && <button className="secondary-button" disabled={busy} onClick={() => void analyze(pending)}>{t("Offene Dateien analysieren (")}{pending.length})</button>}<button className="text-button" disabled={busy} onClick={() => { setItems([]); setActive(null); setWarnings([]); setError(null); }}>{t("Liste leeren")}</button></div></div>
      {ready.length > 0 && <div className="batch-step-card" aria-label={t("Importübersicht")}>
        <p className="eyebrow">{t("Importübersicht")}</p><h2>{ready.length === 1 ? t("1 Datei ist bereit zum Import") : tr`${ready.length} Dateien sind bereit zum Import`}</h2>
        <p>{ready.reduce((sum, item) => sum + (item.parsed?.transactions.length ?? 0), 0)}  {t("Buchungen · Kontozuordnung:")}</p>
        <ul>{[...releaseAccounts].map(([name, count]) => <li key={name}>{name} · {count}  {t("Dateien")}</li>)}</ul>
        {releaseWarnings.size > 0 && <details className="batch-notices"><summary>{releaseWarnings.size}  {t("Hinweise zu diesen Dateien ansehen")}</summary><ul>{[...releaseWarnings].map(([warning, count]) => <li key={warning}>{warning} ({count}  {t("Dateien)")}</li>)}</ul></details>}
        <p>{t("Mit einem Klick werden die Dateien mit diesen Kontozuordnungen importiert. Einzelvorschauen findest du unten.")}</p>
      </div>}
      <div className="history-table batch-table"><table><thead><tr><th>{t("Datei")}</th><th>{t("Anbieter")}</th><th>Status</th><th>{t("Aktion")}</th></tr></thead><tbody>{items.map(item => <tr key={item.file.path} className={active === item.file.path ? "batch-active" : ""}>
        <td><strong>{item.file.name}</strong><small className="batch-path">{item.file.path}</small><small>{formatFileSize(item.file.size)}{item.parsed ? tr` · ${item.parsed.transactions.length} Buchungen` : ""}</small></td>
        <td><label><span className="visually-hidden">{t("Anbieter für")} {item.file.name}</span><select disabled={busy || Boolean(item.result)} value={displayedProvider(item, accounts)} onChange={event => {
          const next = { ...item, file: { ...item.file, provider: event.target.value as ProviderId }, parsed: undefined, accountIds: {}, reviewed: false, error: undefined };
          update(item.file.path, next);
          void analyze([next]);
        }}>{providers.map(provider => <option key={provider.id} value={provider.id}>{provider.id === "unknown" ? t("Automatisch erkennen") : provider.label}</option>)}{!providers.some(provider => provider.id === displayedProvider(item, accounts)) && <option value={displayedProvider(item, accounts)}>{accounts.find(account => account.providerKey === displayedProvider(item, accounts))?.provider}</option>}</select></label>{item.file.provider === "unknown" && hasAccounts(item, accounts) && <small>{t("Aus Kontozuordnung übernommen")}</small>}<small>{[...new Set(Object.values(item.accountIds))].map(id => accounts.find(account => account.id === id)?.name).filter(Boolean).join(", ")}</small></td>
        <td><strong>{status(item)}</strong>{item.duplicateNotice && <small className="batch-duplicate" role="status">{item.duplicateNotice}</small>}{item.error && <small className="batch-error" role="alert">{item.error}</small>}{item.parsed && item.parsed.warnings.length > 0 && <small>{item.parsed.warnings.length}  {t("Warnungen in der Vorschau")}</small>}</td>
        <td><div className="batch-row-actions">{item.alreadyImported && !item.parsed ? <a href="#import-history">{t("Importe ansehen")}</a> : item.parsed ? <button className="secondary-button" disabled={busy} aria-expanded={active === item.file.path} onClick={() => setActive(active === item.file.path ? null : item.file.path)}>{t("Vorschau")}</button> : <button className="secondary-button" disabled={busy} onClick={() => void analyze([item])}>{t("Analysieren")}</button>}<button className="text-button" disabled={busy} onClick={() => { setItems(value => value.filter(row => row.file.path !== item.file.path)); if (active === item.file.path) setActive(null); }} aria-label={tr`${item.file.name} aus der Liste entfernen`}>{t("Entfernen")}</button></div></td>
      </tr>)}</tbody></table></div>
      <div className="batch-import-action">
        <div>
          <strong>{noAccounts ? t("Noch kein Konto vorhanden.") : ready.length === 1 ? t("1 Datei ist bereit zum Import") : tr`${ready.length} Dateien sind bereit zum Import`}</strong>
          {noAccounts
            ? <small>{t("Lege zuerst unter Banken & Konten ein Konto an. Deine Importliste bleibt dabei erhalten.")}</small>
            : ready.length === 0 && <small>{t("Ordne zuerst für jede Datei alle benötigten Konten zu.")}</small>}
        </div>
        <div className="batch-buttons">
          {noAccounts && <a className="secondary-button" href="#banks">{t("Konto anlegen")}</a>}
          <button className="primary-button" disabled={busy || ready.length === 0} onClick={() => void save()}>
            {t("Importieren")} ({ready.length})
          </button>
        </div>
      </div>
      {current?.parsed && <div className="wizard-card batch-preview" key={current.file.path} ref={previewRef}>
        <div className="review-header"><div><p className="eyebrow">{t("Importvorschau")}</p><h2>{current.file.name}</h2></div><span className="success-chip">{current.parsed.transactions.length}  {t("Buchungen")}</span></div>
        <p className="intro">{current.parsed.provider === "unknown" ? t("Anbieter aus dem ausgewählten Konto") : providers.find(provider => provider.id === current.parsed?.provider)?.label} · {current.file.extension.toUpperCase()}</p>
        {current.parsed.format === "MT940" && <p className="intro">{t("Kontokennung im Auszug:")} <strong>{current.parsed.accountName}</strong>  {t("· Vorauswahl anhand der hinterlegten IBAN / Kontoreferenz.")}</p>}<fieldset disabled={busy || Boolean(current.result)} className="batch-account-fields"><div className="form-grid">{currencies(current.parsed).map(currency => {
          const options = matchingAccounts(accounts, current.parsed!, currency);
          return <label key={currency}>{t("Konto oder Vertrag ·")} {currency}<select value={options.some(account => account.id === current.accountIds[currency]) ? current.accountIds[currency] : ""} onChange={event => void changeAccounts(current, { ...current.accountIds, [currency]: Number(event.target.value) })}><option value="">{options.length ? t("Konto auswählen") : t("Kein passendes Konto vorhanden")}</option>{options.map(account => <option key={account.id} value={account.id}>{account.name} · {account.currency} · {account.provider}</option>)}</select></label>;
        })}</div></fieldset>
        {!current.result && <p className="intro"><a href="#banks">{t("Banken &amp; Konten verwalten")}</a> · <button className="text-button" disabled={busy} onClick={() => void loadAccounts()}>{t("Konten aktualisieren")}</button></p>}
        <dl className="review-list">{current.parsed.currencyBalances.length ? current.parsed.currencyBalances.flatMap(balance => [balance.openingDate && <div key={`${balance.currency}-opening`}><dt>{t("Anfangssaldo")} {balance.currency}</dt><dd>{money(balance.openingBalanceMinor, balance.currency)} · {date(balance.openingDate)}</dd></div>, <div key={`${balance.currency}-closing`}><dt>{t("Schlusssaldo")} {balance.currency}</dt><dd>{money(balance.closingBalanceMinor, balance.currency)} · {date(balance.closingDate)}</dd></div>]) : <div><dt>{t("Schlusssaldo")}</dt><dd>{money(current.parsed.closingBalanceMinor, current.parsed.transactions[0]?.currency ?? "CHF")}</dd></div>}</dl>
        {current.duplicateNotice && <p className="warning-message">{current.duplicateNotice}</p>}{current.parsed.warnings.map((warning, index) => <p className="warning-message" key={index}>{warning}</p>)}
        <div className="transaction-preview"><table><thead><tr><th>{t("Datum")}</th><th>{t("Beschreibung")}</th><th>{t("Branche")}</th><th>{t("Betrag")}</th><th>{t("Saldo")}</th><th>{t("Erkennung")}</th></tr></thead><tbody>{current.parsed.transactions.map((row, index) => <tr key={index}><td>{date(row.bookingDate)}</td><td>{row.description}</td><td>{row.industry ?? "–"}</td><td className={row.amountMinor < 0 ? "negative" : "positive"}>{money(row.amountMinor, row.currency)}</td><td>{money(row.balanceMinor, row.currency)}</td><td><span className={row.confidence >= .95 ? "confidence high" : "confidence review"}>{Math.round(row.confidence * 100)}%</span></td></tr>)}</tbody></table></div>
        {!current.result && !hasAccounts(current, accounts) && <p className="warning-message">{t("Bitte für jede Währung ein passendes aktives Konto auswählen.")}</p>}
        {nextOpenPreview && <div className="batch-buttons"><button className="secondary-button" disabled={busy} onClick={() => setActive(nextOpenPreview.file.path)}>{t("Nächste offene Vorschau")}</button></div>}
      </div>}
      <div className="wizard-actions"><a href="#import-history">{t("Importe verwalten")}</a></div>
    </>}
  </section>;
}

function money(value: number | null, currency: string) { return value === null ? "–" : new Intl.NumberFormat(locale(), { style: "currency", currency }).format(value / 100); }
function date(value: string) { return value.split("-").reverse().join("."); }
