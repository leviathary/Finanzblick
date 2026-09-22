// Führt durch Dateiauswahl, Importvorschau, Kontozuordnung und Stapelimport.

import { t, tr, locale } from "../../i18n";
import { Fragment, useEffect, useRef, useState } from "react";
import { invoke, isTauri } from "@tauri-apps/api/core";
import { getCurrentWebview } from "@tauri-apps/api/webview";
import { open } from "@tauri-apps/plugin-dialog";
import { CUSTOM_EXCEL_PROVIDER, detectProvider, formatFileSize, getSupportedExtension, providers, type ProviderId } from "./fileDetection";
import type { DuplicateCheck, DuplicateResolutionAction, ImportAccount, ImportMappingProfile, ParsedStatement, SaveImportResult, SavePositionSnapshotResult, PositionSnapshotPreview, TabularInspection, TabularMapping } from "./importTypes";
import { displayedProvider, currencies, hasAccountReferenceMismatch, hasAccounts, matchingAccounts, orderedBatchItems, orderedPreviewTransactionIndices, readyPositionSnapshot, readyToSave, saveBatch, suggestAccounts, unresolvedDuplicateCount, type BatchItem } from "./importBatch";
import { ExcelMappingDialog, headerFingerprint } from "./ExcelMappingDialog";
import { PositionSnapshotReview, positionSnapshotAccounts } from "./PositionSnapshotImport";

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
  const [saving, setSaving] = useState(false);
  const [progress, setProgress] = useState("");
  const [error, setError] = useState<string | null>(null);
  const [warnings, setWarnings] = useState<string[]>([]);
  const [completed, setCompleted] = useState<{ imported: number; duplicates: number; updated: number; ids: number[]; positionSnapshots: number } | null>(null);
  const [dragging, setDragging] = useState(false);
  const [mappingEditor, setMappingEditor] = useState<{ item: BatchItem; inspection: TabularInspection; profiles: ImportMappingProfile[] } | null>(null);
  const busyRef = useRef(false);
  const stopRef = useRef(false);
  const previewRef = useRef<HTMLDivElement>(null);
  const completedRef = useRef<HTMLDivElement>(null);
  const recursiveRef = useRef(recursive);
  const itemsRef = useRef(items);
  const accountsRef = useRef(accounts);
  itemsRef.current = items;
  accountsRef.current = accounts;
  recursiveRef.current = recursive;
  useEffect(() => { previewRef.current?.scrollIntoView({ block: "start" }); }, [active]);
  useEffect(() => { if (completed) completedRef.current?.scrollIntoView({ behavior: "smooth", block: "center" }); }, [completed]);

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
    if (!selection.files.length) setError(t("Keine unterstützten Dateien gefunden. Unterstützt werden XLSX, XLS, CSV, PDF, MT940 sowie camt.053 und camt.054 (ISO 20022 XML) bis 25 MB pro Datei."));
    await analyzeFiles(added);
  }

  async function choose(directory: boolean) {
    if (!isTauri()) { setError(t("Bitte Dateien oder Ordner in der Desktop-App auswählen.")); return; }
    if (!begin()) return;
    try {
      const selected = await open({ directory, multiple: true, title: directory ? t("Ordner mit Finanzdateien auswählen") : t("Finanzdateien auswählen"), ...(directory ? {} : { filters: [{ name: t("Finanzdateien"), extensions: ["xlsx", "xls", "csv", "pdf", "mt940", "sta", "xml"] }] }) });
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

  async function openSource(item: BatchItem) {
    update(item.file.path, { sourceOpenError: undefined });
    if (!isTauri()) {
      update(item.file.path, { sourceOpenError: t("PDFs können nur in der Desktop-App angezeigt werden.") });
      return;
    }
    try {
      await invoke<void>("open_import_pdf", { path: item.file.path });
    } catch (reason) {
      update(item.file.path, { sourceOpenError: String(reason) });
    }
  }

  async function analyzeFiles(targets: BatchItem[]) {
      for (let index = 0; index < targets.length; index++) {
        if (stopRef.current) break;
        const item = targets[index];
        setProgress(tr`Analyse ${index + 1} von ${targets.length}: ${item.file.name}`);
        update(item.file.path, { error: undefined, sourceOpenError: undefined });
        try {
          if (await analyzePositionSnapshots(item)) continue;
          if (await invoke<boolean>("is_file_imported", { path: item.file.path })) {
            update(item.file.path, { alreadyImported: true, duplicateNotice: "Datei bereits vorhanden. Diese Datei wurde schon importiert.", reviewed: false });
            continue;
          }
          const parsed = await invoke<ParsedStatement>("parse_statement", { path: item.file.path, selectedProvider: selectedProviderForImport(item.file.provider), mapping: null });
          const next = { ...item, parsed, file: { ...item.file, provider: parsed.provider }, accountIds: suggestAccounts(accountsRef.current, parsed), reviewed: false, duplicateCheck: undefined, duplicateResolutions: {} };
          update(item.file.path, { ...next, ...await checkDuplicates(next) });
        } catch (reason) {
          const message = String(reason);
          const useExcelMapping = (item.file.extension === "xlsx" || item.file.extension === "xls")
            && item.file.provider === "unknown"
            && message.includes("Anbieter konnte nicht erkannt werden");
          update(item.file.path, useExcelMapping
            ? { file: { ...item.file, provider: CUSTOM_EXCEL_PROVIDER }, error: undefined, parsed: undefined, reviewed: false }
            : { error: message, parsed: undefined, reviewed: false });
        }
      }
  }

  async function analyzePositionSnapshots(item: BatchItem) {
    try {
      let preview = await invoke<PositionSnapshotPreview | null>("preview_position_snapshot", { path: item.file.path, accountId: null, snapshotDate: null });
      if (!preview) return false;
      const positionDateEditable = !preview.snapshotDate;
      const candidates = positionSnapshotAccounts(accountsRef.current, preview);
      const accountId = candidates.length === 1 ? candidates[0].id : null;
      if (accountId) preview = await invoke<PositionSnapshotPreview>("preview_position_snapshot", { path: item.file.path, accountId, snapshotDate: preview.snapshotDate });
      update(item.file.path, {
        file: { ...item.file, provider: preview.provider as ProviderId },
        parsed: undefined,
        accountIds: {},
        positionSnapshot: preview,
        positionDateEditable,
        positionAccountId: accountId,
        positionResult: undefined,
        alreadyImported: preview.alreadyImported,
        error: undefined,
        reviewed: false,
      });
      return true;
    } catch (reason) {
      const message = String(reason);
      update(item.file.path, { error: message, parsed: undefined, reviewed: false });
      return true;
    }
  }

  async function changePositionAccount(item: BatchItem, accountId: number | null) {
    if (!item.positionSnapshot || !begin()) return;
    update(item.file.path, { positionAccountId: accountId, error: undefined });
    try {
      const preview = await invoke<PositionSnapshotPreview>("preview_position_snapshot", { path: item.file.path, accountId, snapshotDate: item.positionSnapshot.snapshotDate });
      update(item.file.path, { positionSnapshot: preview, positionAccountId: accountId, alreadyImported: preview.alreadyImported });
    } catch (reason) { update(item.file.path, { error: String(reason) }); }
    finally { finish(); }
  }

  async function changePositionDate(item: BatchItem, snapshotDate: string) {
    if (!item.positionSnapshot || !begin()) return;
    const draft = { ...item.positionSnapshot, snapshotDate: snapshotDate || null, changes: [] };
    update(item.file.path, { positionSnapshot: draft, error: undefined });
    try {
      if (!snapshotDate) return;
      const preview = await invoke<PositionSnapshotPreview>("preview_position_snapshot", {
        path: item.file.path, accountId: item.positionAccountId ?? null, snapshotDate,
      });
      update(item.file.path, { positionSnapshot: preview, alreadyImported: preview.alreadyImported });
    } catch (reason) { update(item.file.path, { error: String(reason) }); }
    finally { finish(); }
  }

  async function openMapping(item: BatchItem) {
    if (!begin()) return;
    setProgress(t("Excel-Datei wird für die Zuordnung gelesen…"));
    try {
      const [inspection, profiles] = await Promise.all([
        invoke<TabularInspection>("inspect_tabular_file", { path: item.file.path }),
        invoke<ImportMappingProfile[]>("list_import_mapping_profiles"),
      ]);
      setMappingEditor({ item, inspection, profiles });
    } catch (reason) { setError(String(reason)); }
    finally { finish(); }
  }

  async function applyMapping(mapping: TabularMapping, profileName?: string) {
    if (!mappingEditor) return;
    const item = itemsRef.current.find(candidate => candidate.file.path === mappingEditor.item.file.path) ?? mappingEditor.item;
    const parsed = await invoke<ParsedStatement>("parse_statement", { path: item.file.path, selectedProvider: selectedProviderForImport(item.file.provider), mapping });
    if (profileName) {
      const sheet = mappingEditor.inspection.sheets.find(candidate => candidate.name === mapping.sheetName);
      const headers = sheet?.preview[mapping.headerRow - 1] ?? [];
      await invoke<number>("save_import_mapping_profile", { profile: { name: profileName, headerFingerprint: headerFingerprint(headers), mapping } });
    }
    const next = { ...item, parsed, mapping, file: { ...item.file, provider: item.file.provider === CUSTOM_EXCEL_PROVIDER ? CUSTOM_EXCEL_PROVIDER : parsed.provider }, accountIds: suggestAccounts(accountsRef.current, parsed), reviewed: false, error: undefined, duplicateCheck: undefined, duplicateResolutions: {} };
    update(item.file.path, { ...next, ...await checkDuplicates(next) });
    setMappingEditor(null);
    setActive(item.file.path);
  }

  async function save() {
    const targets = items.filter(item => readyToSave(item, accounts));
    const positionTargets = items.filter(item => readyPositionSnapshot(item, accounts));
    if ((!targets.length && !positionTargets.length) || !begin()) return;
    setSaving(true);
    let count = 0;
    const finishedPaths = new Set<string>();
    const total = targets.length + positionTargets.length;
    const summary = { imported: 0, duplicates: 0, updated: 0, ids: [] as number[], positionSnapshots: 0 };
    try {
      await saveBatch(targets, async item => {
        setProgress(tr`Import ${++count} von ${total}: ${item.file.name}`);
        const duplicate = await checkDuplicates(item);
        const checkedItem = { ...item, ...duplicate };
        update(item.file.path, duplicate);
        if (duplicate.alreadyImported) throw new Error(duplicate.duplicateNotice);
        if (unresolvedDuplicateCount(checkedItem) > 0) throw new Error(t("Mögliche Duplikate müssen vor dem Import vollständig geprüft werden."));
        const account = accounts.find(account => Object.values(item.accountIds).includes(account.id));
        const statement = item.parsed?.provider === "unknown" ? { ...item.parsed, provider: account?.providerKey } : item.parsed;
        const duplicateResolutions = Object.entries(checkedItem.duplicateResolutions ?? {}).map(([transactionIndex, action]) => ({ transactionIndex: Number(transactionIndex), action }));
        return invoke<SaveImportResult>("save_import", { request: { sourcePath: item.file.path, accountName: account?.name ?? "", accountIds: item.accountIds, statement, duplicateResolutions } });
      }, (path, change) => {
        update(path, change);
        if (change.result) {
          summary.ids.push(change.result.importId);
          summary.updated += change.result.updatedTransactions ?? 0;
          finishedPaths.add(path);
          if (change.result.duplicate) summary.duplicates++; else summary.imported++;
        }
      }, () => stopRef.current);
      for (const item of positionTargets) {
        if (stopRef.current) break;
        setProgress(tr`Import ${++count} von ${total}: ${item.file.name}`);
        try {
          const result = await invoke<SavePositionSnapshotResult>("save_position_snapshot", { path: item.file.path, accountId: item.positionAccountId, snapshotDate: item.positionSnapshot?.snapshotDate });
          update(item.file.path, { positionResult: result, error: undefined });
          finishedPaths.add(item.file.path);
          if (result.duplicate) summary.duplicates++;
          else { summary.imported++; summary.positionSnapshots++; }
          window.dispatchEvent(new Event("positions-updated"));
        } catch (reason) { update(item.file.path, { error: String(reason) }); }
      }
    } finally {
      setItems(current => current.filter(item => !finishedPaths.has(item.file.path)));
      setActive(current => current && finishedPaths.has(current) ? null : current);
      if (finishedPaths.size) { setCompleted(summary); setWarnings([]); }
      setSaving(false);
      finish();
    }
  }

  async function checkDuplicates(item: BatchItem) {
    if (!item.parsed) return { alreadyImported: false, duplicateNotice: undefined };
    const check = await invoke<DuplicateCheck>("check_import_duplicates", { request: { sourcePath: item.file.path, accountName: "", accountIds: item.accountIds, statement: item.parsed, duplicateResolutions: [] } });
    const all = check.totalTransactions > 0 && check.matchingTransactions === check.totalTransactions && check.updatableTransactions === 0;
    const valid = new Set(check.suspectedTransactions.map(match => match.transactionIndex));
    const duplicateResolutions = Object.fromEntries(Object.entries(item.duplicateResolutions ?? {}).filter(([index]) => valid.has(Number(index)))) as Record<number, DuplicateResolutionAction>;
    const unchanged = check.matchingTransactions - check.updatableTransactions;
    const existing = unchanged > 0 ? tr`${unchanged} von ${check.totalTransactions} Transaktionen sind auf dem gewählten Konto bereits vorhanden und werden beim Import übersprungen.` : undefined;
    const updates = check.updatableTransactions > 0 ? tr`${check.updatableTransactions} vorläufige Kreditkartenbuchungen werden mit den endgültigen Abrechnungsdaten aktualisiert.` : undefined;
    const suspected = check.suspectedTransactions.length > 0 ? tr`${check.suspectedTransactions.length} mögliche Duplikate müssen vor dem Import geprüft werden.` : undefined;
    return { alreadyImported: check.exactFile || all, duplicateCheck: check, duplicateResolutions, duplicateNotice: check.exactFile ? t("Datei bereits vorhanden. Diese Datei wurde schon importiert.") : all ? t("Datei schon importiert: Alle Transaktionen sind auf dem gewählten Konto bereits vorhanden.") : [updates, existing, suspected].filter(Boolean).join(" ") || undefined };
  }

  async function changeAccounts(item: BatchItem, accountIds: Record<string, number>) {
    if (!begin()) return;
    const next = { ...item, accountIds, reviewed: false, duplicateCheck: undefined, duplicateResolutions: {} };
    update(item.file.path, next);
    try { update(item.file.path, { ...await checkDuplicates(next), error: undefined }); }
    catch (reason) { update(item.file.path, { error: String(reason) }); }
    finally { finish(); }
  }

  const current = items.find(item => item.file.path === active);
  const currentDuplicateTotal = current?.duplicateCheck?.suspectedTransactions.length ?? 0;
  const currentUnresolvedDuplicates = current ? unresolvedDuplicateCount(current) : 0;
  const previewTransactionIndices = current ? orderedPreviewTransactionIndices(current) : [];
  const displayedItems = orderedBatchItems(items);
  const currentIndex = displayedItems.findIndex(item => item.file.path === active);
  const nextOpenPreview = current
    ? displayedItems
        .slice(currentIndex + 1)
        .find(item => item.parsed && !item.result && !item.alreadyImported)
    : undefined;
  const pending = items.filter(item => !item.parsed && !item.positionSnapshot && !item.result && !item.alreadyImported && item.file.provider !== CUSTOM_EXCEL_PROVIDER);
  const ready = items.filter(item => readyToSave(item, accounts));
  const readyPositions = items.filter(item => readyPositionSnapshot(item, accounts));
  const readyCount = ready.length + readyPositions.length;
  const awaitingDuplicateReview = items.reduce((sum, item) => sum + unresolvedDuplicateCount(item), 0);
  const noAccounts = accountsLoaded && accounts.length === 0;
  const releaseAccounts = new Map<string, number>();
  for (const item of ready) {
    const names = [...new Set(Object.values(item.accountIds))].map(id => {
      const account = accounts.find(account => account.id === id)!;
      return `${account.provider} · ${account.name} (${account.currency})`;
    }).join(", ");
    releaseAccounts.set(names, (releaseAccounts.get(names) ?? 0) + 1);
  }
  for (const item of readyPositions) {
    const account = accounts.find(candidate => candidate.id === item.positionAccountId);
    if (account) {
      const name = `${account.provider} · ${account.name} (${account.currency})`;
      releaseAccounts.set(name, (releaseAccounts.get(name) ?? 0) + 1);
    }
  }
  const failed = items.filter(item => item.error).length;

  function status(item: BatchItem) {
    if (item.alreadyImported) return t("Datei bereits vorhanden");
    if (item.positionResult) return item.positionResult.duplicate ? t("Bereits importiert") : t("Importiert");
    if (item.result) return item.result.duplicate ? t("Bereits importiert") : t("Importiert");
    if (item.error) return t("Fehler – erneut versuchen");
    if (item.positionSnapshot && !item.positionAccountId) return t("Kontozuordnung fehlt");
    if (item.positionSnapshot && !item.positionSnapshot.snapshotDate) return t("Stichtag fehlt");
    if (item.positionSnapshot) return t("Bereit zum Import");
    if (!item.parsed && item.file.provider === CUSTOM_EXCEL_PROVIDER) return t("Spaltenzuordnung fehlt");
    if (!item.parsed) return t("Noch nicht analysiert");
    if (!hasAccounts(item, accounts)) return t("Kontozuordnung fehlt");
    if (unresolvedDuplicateCount(item) > 0) return t("Duplikate prüfen");
    return t("Bereit zum Import");
  }

  function setDuplicateResolution(item: BatchItem, transactionIndex: number, action: DuplicateResolutionAction) {
    update(item.file.path, { duplicateResolutions: { ...(item.duplicateResolutions ?? {}), [transactionIndex]: action }, error: undefined });
  }

  return <section className="import-wizard" aria-labelledby="import-title" aria-busy={saving}>
    <div className="wizard-heading"><div><p className="eyebrow">{t("Import")}</p><h1 id="import-title">{t("Finanzdateien importieren")}</h1><p className="intro">{t("Bankauszüge und Depotbestände gemeinsam auswählen oder hierher ziehen. Der Dokumenttyp wird automatisch erkannt.")}</p></div><span className="privacy-chip">{t("✓ Lokale Verarbeitung")}</span></div>
    <div className={`drop-zone batch-drop${dragging ? " dragging" : ""}`} onDragOver={event => event.preventDefault()} onDrop={event => { event.preventDefault(); if (!isTauri()) setError(t("Drag-and-drop ist in der Desktop-App verfügbar.")); }}>
      <h2>{t("Dateien oder Ordner hierher ziehen")}</h2>
      <div className="batch-buttons"><button className="primary-button" disabled={busy} onClick={() => void choose(false)}>{t("Dateien auswählen")}</button><button className="secondary-button" disabled={busy} onClick={() => void choose(true)}>{t("Ordner auswählen")}</button></div>
      <label className="batch-check"><input type="checkbox" checked={recursive} disabled={busy} onChange={event => setRecursive(event.target.checked)} />  {t("Unterordner einbeziehen")}</label>
      <small>{t("Bankauszüge und Depotbestände: XLSX, XLS, CSV, PDF, MT940 sowie camt.053 und camt.054 (ISO 20022 XML) · maximal 25 MB pro Datei")}</small>
    </div>
    {error && <p className="error-message" role="alert">{t(error)}</p>}
    {completed && <div className="batch-complete" role="status" ref={completedRef}><span className="batch-complete-icon" aria-hidden="true">✓</span><div><h2>{t("Import erfolgreich abgeschlossen")}</h2><p>{completed.imported}  {t("Dateien importiert")}{completed.positionSnapshots > 0 ? tr` · ${completed.positionSnapshots} Depotbestände abgeglichen` : ""}{completed.updated > 0 ? tr` · ${completed.updated} vorläufige Kartenbuchungen aktualisiert` : ""}{completed.duplicates > 0 ? tr` · ${completed.duplicates} bereits vorhanden` : ""}.</p><p>{items.length ? t("Offene oder fehlgeschlagene Dateien stehen weiterhin unten in der Liste.") : completed.ids.length ? t("Die importierten Dateien findest du im Reiter „Importierte Dateien“.") : t("Die Positionen wurden im zugehörigen Depot aktualisiert.")}</p></div>{completed.ids.length > 0 && <a className="secondary-button" href={`#import-history?ids=${completed.ids.join(",")}`}>{t("Importierte Dateien ansehen")}</a>}</div>}
    {warnings.length > 0 && <details className="warning-message"><summary>{warnings.length}  {t("Hinweise zur Dateiauswahl")}</summary><ul>{warnings.map((warning, index) => <li key={index}>{warning}</li>)}</ul></details>}
    {busy && <div className="info-panel" role="status"><p>{progress || t("Dateiauswahl geöffnet…")}</p>{(progress.startsWith("Analyse ") || progress.startsWith("Import ")) && <button className="secondary-button" disabled={stopRef.current} onClick={() => { stopRef.current = true; setProgress(value => tr`${value} · Stopp angefordert`); }}>{t("Nach aktueller Datei stoppen")}</button>}</div>}
    {items.length > 0 && <section className="dashboard-card import-batch-card" aria-label={t("Deine Importliste")}>
      <div className="batch-toolbar"><div><h2>{t("Deine Importliste")}</h2><p role="status">{items.length}  {t("Dateien")}{failed > 0 ? tr` · ${failed} Fehler` : ""}{awaitingDuplicateReview > 0 ? tr` · ${awaitingDuplicateReview} mögliche Duplikate ungeklärt` : ""}{readyCount > 0 ? tr` · ${readyCount} bereit zum Import` : ""}</p></div><div className="batch-buttons">{pending.length > 0 && <button className="secondary-button" disabled={busy} onClick={() => void analyze(pending)}>{t("Offene Dateien analysieren (")}{pending.length})</button>}<button className="text-button" disabled={busy} onClick={() => { setItems([]); setActive(null); setWarnings([]); setError(null); }}>{t("Liste leeren")}</button></div></div>
      {readyCount > 0 && <div className="batch-summary" aria-label={t("Importübersicht")}>
        <p>{ready.reduce((sum, item) => sum + (item.parsed?.transactions.length ?? 0), 0)} {t("Buchungen")}{readyPositions.length > 0 ? tr` · ${readyPositions.reduce((sum, item) => sum + (item.positionSnapshot?.positions.length ?? 0), 0)} Positionen` : ""} {t("· Kontozuordnung:")}</p>
        <ul>{[...releaseAccounts].map(([name, count]) => <li key={name}>{name} · {count} {t("Dateien")}</li>)}</ul>
      </div>}
      <div className="history-table batch-table"><table><thead><tr><th>{t("Datei")}</th><th>{t("Anbieter")}</th><th>Status</th><th>{t("Aktion")}</th></tr></thead><tbody>{displayedItems.map(item => <Fragment key={item.file.path}><tr className={active === item.file.path ? "batch-active" : ""}>
        <td><strong>{item.file.name}</strong><small className="batch-path">{item.file.path}</small><small>{formatFileSize(item.file.size)}{item.parsed ? tr` · ${item.parsed.transactions.length} Buchungen` : item.positionSnapshot ? tr` · ${item.positionSnapshot.positions.length} Positionen` : ""}</small></td>
        <td>{item.positionSnapshot ? <><strong>{providers.find(provider => provider.id === item.positionSnapshot?.provider)?.label ?? item.positionSnapshot.provider}</strong><small>{t("Positionsbestand")}</small></> : <label><span className="visually-hidden">{t("Anbieter für")} {item.file.name}</span><select disabled={busy || Boolean(item.result)} value={displayedProvider(item, accounts)} onChange={event => {
            const next = { ...item, file: { ...item.file, provider: event.target.value as ProviderId }, parsed: undefined, mapping: undefined, accountIds: {}, reviewed: false, error: undefined, sourceOpenError: undefined };
          update(item.file.path, next);
          if (next.file.provider === CUSTOM_EXCEL_PROVIDER) void openMapping(next);
          else void analyze([next]);
        }}>{providers.filter(provider => provider.id !== CUSTOM_EXCEL_PROVIDER || item.file.extension === "xlsx" || item.file.extension === "xls").map(provider => <option key={provider.id} value={provider.id}>{provider.id === "unknown" ? t("Automatisch erkennen") : provider.id === CUSTOM_EXCEL_PROVIDER ? t("Eigene Excel-Datei") : provider.label}</option>)}{!providers.some(provider => provider.id === displayedProvider(item, accounts)) && <option value={displayedProvider(item, accounts)}>{accounts.find(account => account.providerKey === displayedProvider(item, accounts))?.provider}</option>}</select></label>}{!item.positionSnapshot && item.file.provider === "unknown" && hasAccounts(item, accounts) && <small>{t("Aus Kontozuordnung übernommen")}</small>}<small>{item.positionSnapshot ? accounts.find(account => account.id === item.positionAccountId)?.name : [...new Set(Object.values(item.accountIds))].map(id => accounts.find(account => account.id === id)?.name).filter(Boolean).join(", ")}</small></td>
        <td><strong>{status(item)}</strong>{item.duplicateNotice && <small className="batch-duplicate" role="status">{item.duplicateNotice}</small>}{item.error && <small className="batch-error" role="alert">{item.error}</small>}{item.sourceOpenError && <small className="batch-error" role="alert">{item.sourceOpenError}</small>}{item.parsed && item.parsed.warnings.length > 0 && <small className="batch-warning-status" title={item.parsed.warnings.join("\n")}>⚠ {item.parsed.warnings.length === 1 ? t("1 Warnhinweis") : tr`${item.parsed.warnings.length} Warnhinweise`}</small>}</td>
        <td><div className="batch-row-actions">{item.positionSnapshot ? <button className="secondary-button" disabled={busy} aria-expanded={active === item.file.path} onClick={() => setActive(active === item.file.path ? null : item.file.path)}>{item.positionAccountId ? t("Vorschau") : t("Konto auswählen")}</button> : item.alreadyImported && !item.parsed ? <a href="#import-history">{t("Importe ansehen")}</a> : item.parsed ? <button className="secondary-button" disabled={busy} aria-expanded={active === item.file.path} onClick={() => setActive(active === item.file.path ? null : item.file.path)}>{hasAccounts(item, accounts) ? t("Vorschau") : t("Konto auswählen")}</button> : item.file.provider === CUSTOM_EXCEL_PROVIDER ? <button className="secondary-button" disabled={busy} onClick={() => void openMapping(item)}>{t("Spalten zuordnen")}</button> : <button className="secondary-button" disabled={busy} onClick={() => void analyze([item])}>{t("Analysieren")}</button>}{item.error && item.file.extension === "pdf" && <button className="secondary-button" disabled={busy} onClick={() => void openSource(item)}>{t("PDF anzeigen")}</button>}{!item.positionSnapshot && (item.file.extension === "xlsx" || item.file.extension === "xls") && !item.alreadyImported && (item.file.provider !== CUSTOM_EXCEL_PROVIDER || Boolean(item.parsed)) && <button className="text-button" disabled={busy} onClick={() => void openMapping(item)}>{item.mapping ? t("Mapping ändern") : t("Spalten zuordnen")}</button>}<button className="text-button" disabled={busy} onClick={() => { setItems(value => value.filter(row => row.file.path !== item.file.path)); if (active === item.file.path) setActive(null); }} aria-label={tr`${item.file.name} aus der Liste entfernen`}>{t("Entfernen")}</button></div></td>
      </tr>
      {active === item.file.path && current?.positionSnapshot && <tr className="batch-preview-row"><td colSpan={4}><div className="batch-preview" key={current.file.path} ref={previewRef}>
        <div className="review-header"><div><p className="eyebrow">{t("Automatisch erkannt")}</p><h2>{current.file.name}</h2></div><span className="success-chip">{t("Positionsbestand")}</span></div>
        <PositionSnapshotReview preview={current.positionSnapshot} accountId={current.positionAccountId ?? null} accounts={accounts} busy={busy} result={current.positionResult} onAccountChange={accountId => void changePositionAccount(current, accountId)} dateEditable={current.positionDateEditable ?? false} onDateChange={value => void changePositionDate(current, value)} />
      </div></td></tr>}
      {active === item.file.path && current?.parsed && <tr className="batch-preview-row"><td colSpan={4}><div className="batch-preview" key={current.file.path} ref={previewRef}>
        <div className="review-header"><div><p className="eyebrow">{hasAccounts(current, accounts) ? t("Importvorschau") : t("Nächster Schritt")}</p><h2>{hasAccounts(current, accounts) ? current.file.name : t("Zielkonto auswählen")}</h2></div><div className="review-header-actions"><div className="review-status-chips"><span className="success-chip">{current.parsed.transactions.length}  {t("Buchungen")}</span>{current.parsed.warnings.length > 0 && <span className="warning-chip" role="status" title={current.parsed.warnings.join("\n")} aria-label={`${current.parsed.warnings.length === 1 ? t("1 Warnhinweis") : tr`${current.parsed.warnings.length} Warnhinweise`}: ${current.parsed.warnings.join(" ")}`}>⚠ {current.parsed.warnings.length === 1 ? t("1 Warnhinweis") : tr`${current.parsed.warnings.length} Warnhinweise`}</span>}</div>{current.file.extension === "pdf" && <button type="button" className="secondary-button" disabled={busy} onClick={() => void openSource(current)}>{t("PDF anzeigen")}</button>}</div></div>
        {!hasAccounts(current, accounts) && <div className="mapping-applied-notice" role="status"><strong>{t("Spaltenzuordnung übernommen – noch nicht importiert")}</strong><p>{t("Wähle jetzt das Zielkonto. Erst danach kann die Datei importiert werden.")}</p></div>}
        <p className="intro">{current.parsed.provider === "unknown" ? t("Anbieter aus dem ausgewählten Konto") : providers.find(provider => provider.id === current.parsed?.provider)?.label} · {current.file.extension.toUpperCase()}</p>
        {current.parsed.accountReference && <p className="intro">{t("Kontokennung im Auszug:")} <strong>{current.parsed.accountReference}</strong>  {t("· Vorauswahl anhand der hinterlegten IBAN / Kontoreferenz.")}</p>}<fieldset disabled={busy || Boolean(current.result)} className="batch-account-fields"><div className="form-grid">{currencies(current.parsed).map(currency => {
          const options = matchingAccounts(accounts, current.parsed!, currency);
          return <label key={currency}>{t("Konto oder Vertrag ·")} {currency}<select value={options.some(account => account.id === current.accountIds[currency]) ? current.accountIds[currency] : ""} onChange={event => void changeAccounts(current, { ...current.accountIds, [currency]: Number(event.target.value) })}><option value="">{options.length ? t("Konto auswählen") : t("Kein passendes Konto vorhanden")}</option>{options.map(account => <option key={account.id} value={account.id}>{account.name} · {account.currency} · {account.provider}</option>)}</select></label>;
        })}</div></fieldset>
        {!current.result && hasAccountReferenceMismatch(accounts, current.parsed) && <p className="warning-message" role="alert">{t("Für die Kontokennung im Auszug ist kein aktives Konto mit derselben hinterlegten IBAN oder Kontoreferenz vorhanden. Prüfe die Kontodaten unter Banken & Konten.")}</p>}
        {!current.result && <p className="intro"><a href="#banks">{t("Banken &amp; Konten verwalten")}</a> · <button className="text-button" disabled={busy} onClick={() => void loadAccounts()}>{t("Konten aktualisieren")}</button></p>}
        <dl className="review-list">{current.parsed.currencyBalances.length ? current.parsed.currencyBalances.flatMap(balance => [balance.openingDate && <div key={`${balance.currency}-opening`}><dt>{t("Anfangssaldo")} {balance.currency}</dt><dd>{money(balance.openingBalanceMinor, balance.currency)} · {date(balance.openingDate)}</dd></div>, <div key={`${balance.currency}-closing`}><dt>{t("Schlusssaldo")} {balance.currency}</dt><dd>{money(balance.closingBalanceMinor, balance.currency)} · {date(balance.closingDate)}</dd></div>]) : <div><dt>{t("Schlusssaldo")}</dt><dd>{money(current.parsed.closingBalanceMinor, current.parsed.transactions[0]?.currency ?? "CHF")}</dd></div>}</dl>
        {current.duplicateNotice && <p className="warning-message">{current.duplicateNotice}</p>}
        {currentDuplicateTotal ? <div className="duplicate-review-summary" role="status"><strong>{currentDuplicateTotal} {t("mögliche Duplikate")}</strong><span>{currentUnresolvedDuplicates > 0 ? tr`${currentUnresolvedDuplicates} von ${currentDuplicateTotal} ungeklärt` : t("Duplikatprüfung vollständig abgeschlossen.")}</span></div> : null}
        <div className="transaction-preview"><table><colgroup><col className="preview-date-column"/><col className="preview-description-column"/><col className="preview-industry-column"/><col className="preview-amount-column"/><col className="preview-balance-column"/><col className="preview-confidence-column"/><col className="preview-duplicate-column"/></colgroup><thead><tr><th>{t("Datum")}</th><th>{t("Beschreibung")}</th><th>{t("Branche")}</th><th>{t("Betrag")}</th><th>{t("Saldo")}</th><th>{t("Erkennung")}</th><th>{t("Duplikatprüfung")}</th></tr></thead><tbody>{previewTransactionIndices.map(index => {
          const row = current.parsed!.transactions[index];
          const match = current.duplicateCheck?.suspectedTransactions.find(candidate => candidate.transactionIndex === index);
          const resolution = current.duplicateResolutions?.[index];
          return <Fragment key={index}>
            <tr className={match ? `suspected-duplicate ${resolution ? "resolved" : "unresolved"}` : ""}>
              <td>{date(row.bookingDate)}</td><td>{row.description}</td><td>{row.industry ?? "–"}</td><td className={row.amountMinor < 0 ? "negative" : "positive"}>{money(row.amountMinor, row.currency)}</td><td>{money(row.balanceMinor, row.currency)}</td><td><span className={row.confidence >= .95 ? "confidence high" : "confidence review"}>{Math.round(row.confidence * 100)}%</span></td>
              <td className="duplicate-review-cell">{match ? <><strong>{t("Mögliches Duplikat")}</strong><div className="duplicate-review-actions" role="group" aria-label={`${t("Duplikatentscheidung")}: ${row.description}`}><button type="button" className="secondary-button" disabled={busy} aria-pressed={resolution === "keep"} onClick={() => setDuplicateResolution(current, index, "keep")}>{t(resolution === "keep" ? "✓ Kein Duplikat – wird importiert" : "Kein Duplikat – behalten")}</button><button type="button" className="secondary-button" disabled={busy} aria-pressed={resolution === "skip"} onClick={() => setDuplicateResolution(current, index, "skip")}>{t(resolution === "skip" ? "✓ Als Duplikat erkannt – wird übersprungen" : "Als Duplikat überspringen")}</button></div></> : <span className="duplicate-clear">{t("Kein verdächtiger Treffer")}</span>}</td>
            </tr>
            {match && <tr className="duplicate-comparison-row" aria-label={t("Vergleichsbuchung")}>
              <td>{date(match.bookingDate)}</td><td>{match.description}</td><td>–</td><td className={match.amountMinor < 0 ? "negative" : "positive"}>{money(match.amountMinor, match.currency)}</td><td>–</td><td>–</td><td><strong>{t("Vergleichsbuchung")}</strong><small>{match.matchSource === "stored" ? t("Bereits gespeichert") : t("Aus derselben Datei")}</small></td>
            </tr>}
          </Fragment>;
        })}</tbody></table></div>
        {!current.result && !hasAccounts(current, accounts) && <p className="warning-message">{t("Bitte für jede Währung ein passendes aktives Konto auswählen.")}</p>}
        {!current.result && hasAccounts(current, accounts) && unresolvedDuplicateCount(current) === 0 && <p className="mapping-ready-notice">{t("Kontozuordnung und Duplikatprüfung vollständig. Die Datei ist jetzt bereit zum Import.")}</p>}
        {nextOpenPreview && <div className="batch-buttons"><button className="secondary-button" disabled={busy} onClick={() => setActive(nextOpenPreview.file.path)}>{t("Nächste offene Vorschau")}</button></div>}
      </div></td></tr>}
      </Fragment>)}</tbody></table></div>
      <div className="batch-import-action">
        <div>
          {noAccounts && <strong>{t("Noch kein Konto vorhanden.")}</strong>}
          {noAccounts
            ? <small>{t("Lege zuerst unter Banken & Konten ein Konto an. Deine Importliste bleibt dabei erhalten.")}</small>
            : awaitingDuplicateReview > 0 ? <small>{t("Prüfe zuerst alle möglichen Duplikate in den Dateivorschauen.")}</small>
            : readyCount === 0 && <small>{t("Ordne zuerst für jede Datei alle benötigten Konten zu.")}</small>}
        </div>
        <div className="batch-buttons">
          {noAccounts && <a className="secondary-button" href="#banks">{t("Konto anlegen")}</a>}
          <button className="primary-button" disabled={busy || readyCount === 0} aria-busy={saving} onClick={() => void save()}>
            {saving ? t("Import läuft …") : <>{t("Importieren")} ({readyCount})</>}
          </button>
        </div>
      </div>
    </section>}
    {mappingEditor && <ExcelMappingDialog inspection={mappingEditor.inspection} profiles={mappingEditor.profiles} initial={mappingEditor.item.mapping} onClose={() => setMappingEditor(null)} onApply={applyMapping} />}
  </section>;
}

function money(value: number | null, currency: string) { return value === null ? "–" : new Intl.NumberFormat(locale(), { style: "currency", currency }).format(value / 100); }
function date(value: string) { return value.split("-").reverse().join("."); }
function selectedProviderForImport(provider: ProviderId) { return provider === "unknown" || provider === CUSTOM_EXCEL_PROVIDER ? null : provider; }
