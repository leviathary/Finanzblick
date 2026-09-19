// Verwaltet den Import und die manuelle Pflege jährlicher Steuerwerte in der Oberfläche.

import { invoke, isTauri } from "@tauri-apps/api/core";
import { getCurrentWebview } from "@tauri-apps/api/webview";
import { open } from "@tauri-apps/plugin-dialog";
import { useEffect, useRef, useState } from "react";

import { locale, t, tr } from "../../i18n";

interface TaxStatementPreview {
  sourceName: string;
  taxYear: number;
  valuationDate: string;
  grossAssetsMinor: number;
  liabilitiesMinor: number;
  taxableWealthMinor: number;
  securitiesAndCashMinor: number;
  realEstateMinor: number;
  otherAssetsMinor: number;
  cantonTaxableWealthMinor: number | null;
  currency: string;
  confidence: number;
  warnings: string[];
  existingYear: boolean;
}

interface TaxSnapshot extends Omit<TaxStatementPreview, "warnings" | "existingYear"> {
  id: number;
  importedAt: string;
}

interface EditableTaxValues {
  grossAssets: string;
  liabilities: string;
  taxableWealth: string;
  securitiesAndCash: string;
  realEstate: string;
  otherAssets: string;
}

interface EditablePreview extends TaxStatementPreview, EditableTaxValues {
  sourcePath: string;
}

interface ManualTaxEntry extends EditableTaxValues {
  taxYear: string;
}

interface SaveResult {
  snapshot: TaxSnapshot;
  replacedExistingYear: boolean;
}

interface EditableSnapshot extends EditableTaxValues {
  id: number;
  taxYear: number;
  cantonTaxableWealthMinor: number | null;
}

type TaxSeriesKey =
  | "securitiesAndCashMinor"
  | "realEstateMinor"
  | "otherAssetsMinor"
  | "grossAssetsMinor"
  | "liabilitiesMinor"
  | "taxableWealthMinor";

const TAX_CHART_SERIES: { key: TaxSeriesKey; label: string; color: string }[] = [
  { key: "securitiesAndCashMinor", label: "Wertschriften & Guthaben", color: "#059669" },
  { key: "realEstateMinor", label: "Liegenschaften", color: "#315c8a" },
  { key: "otherAssetsMinor", label: "Übrige / Korrekturen", color: "#b7791f" },
  { key: "grossAssetsMinor", label: "Total Vermögenswerte", color: "#64748b" },
  { key: "liabilitiesMinor", label: "Schulden", color: "#c2413b" },
  { key: "taxableWealthMinor", label: "Steuerbares Vermögen gesamt", color: "#7c3aed" },
];

type TaxHistoryView = "history" | "import" | "management";

export function TaxHistory({ view = "history", active = true }: { view?: TaxHistoryView; active?: boolean }) {
  const [snapshots, setSnapshots] = useState<TaxSnapshot[]>([]);
  const [snapshotsLoaded, setSnapshotsLoaded] = useState(false);
  const [previews, setPreviews] = useState<EditablePreview[]>([]);
  const [busy, setBusy] = useState(false);
  const [error, setError] = useState<string | null>(null);
  const [message, setMessage] = useState<string | null>(null);
  const [dragging, setDragging] = useState(false);
  const [editing, setEditing] = useState<EditableSnapshot | null>(null);
  const [manualOpen, setManualOpen] = useState(false);
  const [manual, setManual] = useState<ManualTaxEntry>(emptyManualEntry);
  const busyRef = useRef(false);
  const editorRef = useRef<HTMLDivElement>(null);
  busyRef.current = busy;

  useEffect(() => {
    editorRef.current?.scrollIntoView({ behavior: "smooth", block: "center" });
  }, [editing?.id]);

  const load = () =>
    invoke<TaxSnapshot[]>("list_tax_snapshots")
      .then(setSnapshots)
      .catch((reason) => setError(String(reason)))
      .finally(() => setSnapshotsLoaded(true));

  useEffect(() => {
    if (!isTauri()) {
      setError(t("Die Steuerhistorie ist in der Desktop-App verfügbar."));
      setSnapshotsLoaded(true);
      return;
    }
    void load();
  }, []);

  useEffect(() => {
    if (!isTauri()) return;
    let disposed = false;
    let unlisten: (() => void) | undefined;
    void getCurrentWebview().onDragDropEvent((event) => {
      if (!active || view !== "import" || window.location.hash !== "#imports/tax") return;
      if (event.payload.type === "enter" || event.payload.type === "over") {
        if (!busyRef.current) setDragging(true);
      } else if (event.payload.type === "drop") {
        setDragging(false);
        if (!busyRef.current) void processPaths(event.payload.paths);
      } else {
        setDragging(false);
      }
    }).then((dispose) => {
      if (disposed) dispose();
      else unlisten = dispose;
    }).catch((reason) => setError(String(reason)));
    return () => {
      disposed = true;
      unlisten?.();
    };
  }, [active, view]);

  async function chooseFiles() {
    if (!isTauri() || busyRef.current) return;
    busyRef.current = true;
    setBusy(true);
    setError(null);
    setMessage(null);
    try {
      const selection = await open({
        multiple: true,
        directory: false,
        title: t("Steuererklärungen auswählen"),
        filters: [{ name: t("Steuererklärungen"), extensions: ["pdf"] }],
      });
      if (!selection) return;
      await processSelectedPaths(Array.isArray(selection) ? selection : [selection]);
    } finally {
      setBusy(false);
      busyRef.current = false;
    }
  }

  async function processPaths(paths: string[]) {
    if (busyRef.current) return;
    busyRef.current = true;
    setBusy(true);
    setError(null);
    setMessage(null);
    try {
      await processSelectedPaths(paths);
    } finally {
      setBusy(false);
      busyRef.current = false;
    }
  }

  async function processSelectedPaths(paths: string[]) {
    const pdfPaths = paths.filter((path) => path.toLowerCase().endsWith(".pdf"));
    const next: EditablePreview[] = [];
    const failures = paths
      .filter((path) => !pdfPaths.includes(path))
      .map((path) => `${fileName(path)}: ${t("Bitte eine PDF-Datei ablegen.")}`);
    for (const sourcePath of pdfPaths) {
      try {
        const preview = await invoke<TaxStatementPreview>("preview_tax_statement", {
          path: sourcePath,
        });
        next.push(toEditable(preview, sourcePath));
      } catch (reason) {
        failures.push(`${fileName(sourcePath)}: ${String(reason)}`);
      }
    }
    setPreviews((current) => [
      ...current.filter((item) => !pdfPaths.includes(item.sourcePath)),
      ...next,
    ]);
    if (failures.length) {
      setError(failures.join("\n"));
      setManualOpen(true);
    }
  }

  function update(path: string, change: Partial<EditablePreview>) {
    setPreviews((current) =>
      current.map((item) => (item.sourcePath === path ? { ...item, ...change } : item)),
    );
  }

  async function save(item: EditablePreview) {
    const values = previewValues(item);
    if (!values || busy) return;
    setBusy(true);
    setError(null);
    setMessage(null);
    try {
      const result = await invoke<SaveResult>("save_tax_statement", {
        request: {
          sourcePath: item.sourcePath,
          grossAssetsMinor: values.grossAssetsMinor,
          liabilitiesMinor: values.liabilitiesMinor,
          taxableWealthMinor: values.taxableWealthMinor,
          securitiesAndCashMinor: values.securitiesAndCashMinor,
          realEstateMinor: values.realEstateMinor,
          otherAssetsMinor: values.otherAssetsMinor,
          cantonTaxableWealthMinor: item.cantonTaxableWealthMinor,
        },
      });
      setPreviews((current) => current.filter((row) => row.sourcePath !== item.sourcePath));
      setMessage(
        result.replacedExistingYear
          ? tr`Das Steuerjahr ${item.taxYear} wurde aktualisiert.`
          : tr`Das Steuerjahr ${item.taxYear} wurde gespeichert.`,
      );
      await load();
    } catch (reason) {
      setError(String(reason));
    } finally {
      setBusy(false);
    }
  }

  async function saveManual() {
    const values = editableValues(manual);
    const taxYear = Number(manual.taxYear);
    if (!values || !Number.isInteger(taxYear) || taxYear < 1990 || taxYear > 2100 || busy) return;
    setBusy(true);
    setError(null);
    setMessage(null);
    try {
      const result = await invoke<SaveResult>("save_manual_tax_snapshot", {
        request: {
          taxYear,
          ...values,
          cantonTaxableWealthMinor: null,
        },
      });
      setMessage(
        result.replacedExistingYear
          ? tr`Das Steuerjahr ${taxYear} wurde aktualisiert.`
          : tr`Das Steuerjahr ${taxYear} wurde gespeichert.`,
      );
      setManual(emptyManualEntry());
      setManualOpen(false);
      await load();
    } catch (reason) {
      setError(String(reason));
    } finally {
      setBusy(false);
    }
  }

  async function remove(snapshot: TaxSnapshot) {
    if (busy || !window.confirm(tr`Steuerwert ${snapshot.taxYear} wirklich löschen?`)) return;
    setBusy(true);
    setError(null);
    try {
      await invoke("delete_tax_snapshot", { id: snapshot.id });
      setSnapshots((current) => current.filter((item) => item.id !== snapshot.id));
      if (editing?.id === snapshot.id) setEditing(null);
    } catch (reason) {
      setError(String(reason));
    } finally {
      setBusy(false);
    }
  }

  function startEditing(snapshot: TaxSnapshot) {
    setError(null);
    setMessage(null);
    setEditing({
      id: snapshot.id,
      taxYear: snapshot.taxYear,
      grossAssets: String(snapshot.grossAssetsMinor / 100),
      liabilities: String(snapshot.liabilitiesMinor / 100),
      taxableWealth: String(snapshot.taxableWealthMinor / 100),
      securitiesAndCash: String(snapshot.securitiesAndCashMinor / 100),
      realEstate: String(snapshot.realEstateMinor / 100),
      otherAssets: String(snapshot.otherAssetsMinor / 100),
      cantonTaxableWealthMinor: snapshot.cantonTaxableWealthMinor,
    });
  }

  async function saveEditing() {
    if (!editing || busy) return;
    const values = editableValues(editing);
    if (!values || !netValuesAreConsistent(values)) return;
    setBusy(true);
    setError(null);
    setMessage(null);
    try {
      const updated = await invoke<TaxSnapshot>("update_tax_snapshot", {
        request: {
          id: editing.id,
          ...values,
          cantonTaxableWealthMinor: editing.cantonTaxableWealthMinor,
        },
      });
      setSnapshots((current) => current.map((item) => item.id === updated.id ? updated : item));
      setEditing(null);
      setMessage(tr`Das Steuerjahr ${editing.taxYear} wurde aktualisiert.`);
    } catch (reason) {
      setError(String(reason));
    } finally {
      setBusy(false);
    }
  }

  const first = snapshots[0];
  const latest = snapshots[snapshots.length - 1];
  const change = first && latest ? latest.taxableWealthMinor - first.taxableWealthMinor : null;
  const manualValues = editableValues(manual);
  const manualTaxYear = Number(manual.taxYear);
  const manualStatus = taxValueStatus(manualValues);
  const manualYearValid = Number.isInteger(manualTaxYear) && manualTaxYear >= 1990 && manualTaxYear <= 2100;
  const manualCanSave = Boolean(manualYearValid && manualValues && netValuesAreConsistent(manualValues));
  const manualYearExists = manualYearValid && snapshots.some((snapshot) => snapshot.taxYear === manualTaxYear);

  return (
    <section className="tax-history-page">
      {view === "history" && <div className="overview-heading">
        <div>
          <p className="eyebrow">{t("Langzeitverlauf")}</p>
          <h1>{t("Vermögen nach Steuerjahren")}</h1>
          <p className="intro">
            {t("Jährliche Vermögenswerte aus deinen Steuererklärungen, unabhängig von Bankimporten.")}
          </p>
        </div>
      </div>}

      {view === "import" && <div className="overview-heading">
        <div>
          <p className="eyebrow">{t("Import")}</p>
          <h1>{t("Steuererklärungen importieren")}</h1>
          <p className="intro">{t("Steuererklärungen als PDF auslesen oder Jahreswerte ohne Datei manuell erfassen.")}</p>
        </div>
      </div>}

      {view === "management" && <div className="overview-heading">
        <div>
          <p className="eyebrow">{t("Import")}</p>
          <h1>{t("Importierte Steuererklärungen")}</h1>
          <p className="intro">{t("Prüfe die gespeicherten Jahreswerte und passe sie bei Bedarf an.")}</p>
        </div>
        <a className="primary-button" href="#imports/tax">{t("Steuererklärung importieren")}</a>
      </div>}

      {view === "import" && <><div className="privacy-note">
        <strong>{t("Lokale Verarbeitung")}</strong>
        <span>{t("Die PDFs werden nur ausgelesen. Gespeichert werden das Steuerjahr, die geprüften Summen und ein Dateifingerabdruck - nicht das PDF selbst.")}</span>
      </div>

      <div
        className={`drop-zone tax-drop-zone${dragging ? " dragging" : ""}`}
        onDragOver={(event) => event.preventDefault()}
        onDrop={(event) => {
          event.preventDefault();
          if (!isTauri()) setError(t("Drag-and-drop ist in der Desktop-App verfügbar."));
        }}
      >
        <div className="file-icon" aria-hidden="true">PDF</div>
        <h2>{t("Steuererklärungen hierher ziehen")}</h2>
        <p>{t("Eine oder mehrere PDF-Dateien ablegen und die erkannten Jahreswerte anschliessend prüfen.")}</p>
        <button className="primary-button" type="button" disabled={busy} onClick={() => void chooseFiles()}>
          {busy ? t("PDFs werden verarbeitet…") : t("PDFs auswählen")}
        </button>
        <small>{t("Unterstützt werden elektronisch erzeugte Steuererklärungen im PDF-Format.")}</small>
      </div>

      <div className="tax-manual-prompt">
        <div>
          <strong>{t("Kein lesbares PDF vorhanden?")}</strong>
          <small>{t("Erfasse die Jahreswerte direkt von Hand.")}</small>
        </div>
        <button className="secondary-button" type="button" disabled={busy} aria-expanded={manualOpen} onClick={() => setManualOpen((current) => !current)}>
          {manualOpen ? t("Manuelle Eingabe schliessen") : t("Steuerjahr manuell erfassen")}
        </button>
      </div>

      {message && <p className="import-result" role="status">{message}</p>}
      {error && <p className="error-message preserve-lines" role="alert">{error}</p>}

      {manualOpen && <form className="tax-preview-section tax-manual-entry" aria-labelledby="tax-manual-title" onSubmit={(event) => { event.preventDefault(); void saveManual(); }}>
        <div className="card-heading">
          <div>
            <p className="eyebrow">{t("Manuelle Eingabe")}</p>
            <h2 id="tax-manual-title">{t("Steuerjahr ohne PDF erfassen")}</h2>
          </div>
        </div>
        <label className="tax-year-field">{t("Steuerjahr")}
          <input type="number" min="1990" max="2100" step="1" inputMode="numeric" value={manual.taxYear} onChange={(event) => setManual({ ...manual, taxYear: event.target.value })} />
        </label>
        {manualYearExists && <p className="warning-message">{t("Für dieses Jahr besteht bereits ein Wert. Beim Speichern wird er ersetzt.")}</p>}
        <TaxValueEditor item={manual} onChange={(change) => setManual({ ...manual, ...change })} />
        <div className={`tax-validation ${manualStatus}`} role="status">
          {manualStatus === "valid"
            ? t("Plausibel: Vermögensaufteilung und Nettowert sind rechnerisch konsistent.")
            : manualStatus === "notice"
              ? t("Hinweis: Die Vermögensaufteilung entspricht nicht den gesamten Vermögenswerten. Das Steuerjahr kann trotzdem gespeichert werden.")
              : t("Bitte Werte korrigieren: Vermögenswerte minus Schulden müssen dem steuerbaren Vermögen entsprechen.")}
        </div>
        {!manualYearValid && <p className="warning-message">{t("Bitte ein Steuerjahr zwischen 1990 und 2100 eingeben.")}</p>}
        <div className="tax-preview-actions">
          <button className="text-button" type="button" disabled={busy} onClick={() => { setManual(emptyManualEntry()); setManualOpen(false); }}>{t("Abbrechen")}</button>
          <button className="primary-button" type="submit" disabled={busy || !manualCanSave}>{manualYearExists ? t("Jahr aktualisieren") : t("Jahr speichern")}</button>
        </div>
      </form>}

      {previews.length > 0 && (
        <section className="tax-preview-section" aria-labelledby="tax-preview-title">
          <div className="card-heading">
            <div>
              <p className="eyebrow">{t("Importvorschau")}</p>
              <h2 id="tax-preview-title">{t("Erkannte Steuerwerte prüfen")}</h2>
            </div>
            <span>{previews.length} {previews.length === 1 ? t("PDF") : t("PDFs")}</span>
          </div>
          <div className="tax-preview-list">
            {previews.map((item) => {
              const values = previewValues(item);
              const status = taxValueStatus(values);
              const canSave = Boolean(values && netValuesAreConsistent(values));
              return (
                <article className="tax-preview-card" key={item.sourcePath}>
                  <div className="tax-preview-heading">
                    <div>
                      <strong>{t("Steuerjahr")} {item.taxYear}</strong>
                      <small>{item.sourceName}</small>
                    </div>
                    <span className={item.confidence >= 0.95 ? "confidence high" : "confidence medium"}>
                      {Math.round(item.confidence * 100)}% {t("Erkennung")}
                    </span>
                  </div>
                  {item.existingYear && <p className="warning-message">{t("Für dieses Jahr besteht bereits ein Wert. Beim Speichern wird er ersetzt.")}</p>}
                  {item.warnings.map((warning) => <p className="warning-message" key={warning}>{warning}</p>)}
                  <TaxValueEditor item={item} onChange={(change) => update(item.sourcePath, change)} />
                  <div className={`tax-validation ${status}`} role="status">
                    {status === "valid"
                      ? t("Plausibel: Vermögensaufteilung und Nettowert sind rechnerisch konsistent.")
                      : status === "notice"
                        ? t("Hinweis: Die Vermögensaufteilung entspricht nicht den gesamten Vermögenswerten. Das Steuerjahr kann trotzdem gespeichert werden.")
                        : t("Bitte Werte korrigieren: Vermögenswerte minus Schulden müssen dem steuerbaren Vermögen entsprechen.")}
                  </div>
                  <div className="tax-preview-actions">
                    <button className="text-button" type="button" disabled={busy} onClick={() => setPreviews((current) => current.filter((row) => row.sourcePath !== item.sourcePath))}>{t("Entfernen")}</button>
                    <button className="primary-button" type="button" disabled={busy || !canSave} onClick={() => void save(item)}>{item.existingYear ? t("Jahr aktualisieren") : t("Jahr speichern")}</button>
                  </div>
                </article>
              );
            })}
          </div>
        </section>
      )}</>}

      {view === "history" && snapshots.length > 0 ? (
        <>
          <div className="summary-grid tax-summary-grid">
            <article className="summary-card">
              <span>{t("Letzter Steuerwert")}</span>
              <strong>{money(latest?.taxableWealthMinor ?? 0)}</strong>
              <small>{t("Steuerjahr")} {latest?.taxYear}</small>
            </article>
            <article className="summary-card">
              <span>{t("Veränderung seit erstem Jahr")}</span>
              <strong className={change !== null && change < 0 ? "negative" : "positive"}>{change === null ? "–" : signedMoney(change)}</strong>
              <small>{first?.taxYear} - {latest?.taxYear}</small>
            </article>
            <article className="summary-card">
              <span>{t("Datenbasis")}</span>
              <strong>{snapshots.length}</strong>
              <small>{snapshots.length === 1 ? t("Steuerjahr") : t("Steuerjahre")}</small>
            </article>
          </div>

          <article className="dashboard-card tax-chart-card">
            <div className="card-heading">
              <div>
                <p className="eyebrow">{t("Zeitverlauf")}</p>
                <h2>{t("Vermögensentwicklung nach Steuerjahren")}</h2>
              </div>
              <span>CHF · {first?.taxYear} - {latest?.taxYear}</span>
            </div>
            <AnnualTaxChart snapshots={snapshots} />
          </article>

        </>
      ) : null}

      {view === "history" && snapshotsLoaded && snapshots.length === 0 && !error ? <article className="dashboard-card tax-empty-state">
        <p className="eyebrow">{t("Steuerhistorie")}</p>
        <h2>{t("Noch keine Jahreswerte")}</h2>
        <p>{t("Wähle eine oder mehrere Zürcher Steuererklärungen als PDF. Die erkannten Summen werden vor dem Speichern angezeigt.")}</p>
        <a className="primary-button" href="#imports/tax">{t("Steuererklärung importieren")}</a>
      </article> : null}

      {view === "management" && snapshots.length > 0 ? (
          <article className="dashboard-card tax-table-card">
            <div className="card-heading">
              <div>
                <p className="eyebrow">{t("Jahreswerte")}</p>
                <h2>{t("Importierte Steuererklärungen")}</h2>
              </div>
            </div>
            {editing && (() => {
              const values = editableValues(editing);
              const status = taxValueStatus(values);
              const canSave = Boolean(values && netValuesAreConsistent(values));
              return <div className="tax-inline-editor" ref={editorRef} aria-labelledby="tax-edit-title">
                <div className="tax-preview-heading">
                  <div>
                    <p className="eyebrow">{t("Bearbeiten")}</p>
                    <h3 id="tax-edit-title">{t("Steuerjahr bearbeiten")} {editing.taxYear}</h3>
                  </div>
                </div>
                <TaxValueEditor item={editing} onChange={(change) => setEditing({ ...editing, ...change })} />
                <div className={`tax-validation ${status}`} role="status">
                  {status === "valid"
                    ? t("Plausibel: Vermögensaufteilung und Nettowert sind rechnerisch konsistent.")
                    : status === "notice"
                      ? t("Hinweis: Die Vermögensaufteilung entspricht nicht den gesamten Vermögenswerten. Das Steuerjahr kann trotzdem gespeichert werden.")
                      : t("Bitte Werte korrigieren: Vermögenswerte minus Schulden müssen dem steuerbaren Vermögen entsprechen.")}
                </div>
                <div className="tax-preview-actions">
                  <button className="text-button" type="button" disabled={busy} onClick={() => setEditing(null)}>{t("Abbrechen")}</button>
                  <button className="primary-button" type="button" disabled={busy || !canSave} onClick={() => void saveEditing()}>{t("Änderungen speichern")}</button>
                </div>
              </div>;
            })()}
            <div className="tax-table-scroll">
              <table>
                <thead><tr><th>{t("Jahr")}</th><th>{t("Wertschriften & Guthaben")}</th><th>{t("Liegenschaften")}</th><th>{t("Übrige / Korrekturen")}</th><th>{t("Vermögenswerte")}</th><th>{t("Schulden")}</th><th>{t("Steuerbares Vermögen")}</th><th>{t("Aktion")}</th></tr></thead>
                <tbody>{[...snapshots].reverse().map((snapshot) => (
                  <tr key={snapshot.id}>
                    <td><strong>{snapshot.taxYear}</strong><small>{formatDate(snapshot.valuationDate)}</small></td>
                    <td>{money(snapshot.securitiesAndCashMinor)}</td>
                    <td>{money(snapshot.realEstateMinor)}</td>
                    <td>{signedMoney(snapshot.otherAssetsMinor)}</td>
                    <td>{money(snapshot.grossAssetsMinor)}</td>
                    <td>{money(snapshot.liabilitiesMinor)}</td>
                    <td><strong>{money(snapshot.taxableWealthMinor)}</strong></td>
                    <td><div className="tax-row-actions"><button className="text-button" type="button" disabled={busy} onClick={() => startEditing(snapshot)}>{t("Bearbeiten")}</button><button className="text-button danger-text" type="button" disabled={busy} onClick={() => void remove(snapshot)}>{t("Löschen")}</button></div></td>
                  </tr>
                ))}</tbody>
              </table>
            </div>
          </article>
      ) : null}

      {view === "management" && snapshotsLoaded && snapshots.length === 0 && !error ? <article className="dashboard-card tax-empty-state">
        <h2>{t("Noch keine Jahreswerte")}</h2>
        <p>{t("Importiere zuerst eine Steuererklärung, um ihre Jahreswerte hier verwalten zu können.")}</p>
        <a className="primary-button" href="#imports/tax">{t("Steuererklärung importieren")}</a>
      </article> : null}
    </section>
  );
}

function MoneyField({ label, value, onChange }: { label: string; value: string; onChange: (value: string) => void }) {
  return <label>{label}<span className="money-input"><span>CHF</span><input type="number" step="1" inputMode="numeric" value={value} onChange={(event) => onChange(event.target.value)} /></span></label>;
}

function TaxValueEditor({ item, onChange }: {
  item: EditableTaxValues;
  onChange: (change: Partial<EditableTaxValues>) => void;
}) {
  return <div className="tax-editor-groups">
    <section>
      <h4>{t("Vermögensaufteilung")}</h4>
      <div className="tax-value-fields">
        <MoneyField label={t("Wertschriften & Guthaben")} value={item.securitiesAndCash} onChange={(securitiesAndCash) => onChange({ securitiesAndCash })} />
        <MoneyField label={t("Liegenschaften")} value={item.realEstate} onChange={(realEstate) => onChange({ realEstate })} />
        <MoneyField label={t("Übrige Vermögenswerte / Bewertungskorrekturen")} value={item.otherAssets} onChange={(otherAssets) => onChange({ otherAssets })} />
      </div>
    </section>
    <section>
      <h4>{t("Gesamtrechnung")}</h4>
      <div className="tax-value-fields">
        <MoneyField label={t("Total Vermögenswerte")} value={item.grossAssets} onChange={(grossAssets) => onChange({ grossAssets })} />
        <MoneyField label={t("Schulden")} value={item.liabilities} onChange={(liabilities) => onChange({ liabilities })} />
        <MoneyField label={t("Steuerbares Vermögen gesamt")} value={item.taxableWealth} onChange={(taxableWealth) => onChange({ taxableWealth })} />
      </div>
    </section>
  </div>;
}

function AnnualTaxChart({ snapshots }: { snapshots: TaxSnapshot[] }) {
  const ref = useRef<HTMLDivElement>(null);
  const [width, setWidth] = useState(900);
  const [selectedSeries, setSelectedSeries] = useState<TaxSeriesKey[]>([
    "securitiesAndCashMinor",
    "realEstateMinor",
  ]);
  useEffect(() => {
    if (!ref.current) return;
    const element = ref.current;
    const update = () => setWidth(Math.max(520, Math.round(element.clientWidth)));
    update();
    const observer = new ResizeObserver(update);
    observer.observe(element);
    return () => observer.disconnect();
  }, []);
  const height = 290, left = 100, right = 28, top = 24, bottom = 48;
  const visibleSeries = TAX_CHART_SERIES.filter((series) => selectedSeries.includes(series.key));
  const values = visibleSeries.flatMap((series) => snapshots.map((item) => item[series.key]));
  const minValue = Math.min(...values);
  const maxValue = Math.max(...values);
  const padding = Math.max((maxValue - minValue) * 0.12, 1_000_000);
  const minimum = minValue < 0 ? minValue - padding : Math.max(0, minValue - padding);
  const maximum = maxValue + padding;
  const range = Math.max(maximum - minimum, 1);
  const plotWidth = width - left - right;
  const lines = visibleSeries.map((series) => ({
    ...series,
    points: snapshots.map((item, index) => ({
      id: item.id,
      taxYear: item.taxYear,
      value: item[series.key],
      x: snapshots.length === 1 ? left + plotWidth / 2 : left + (index / (snapshots.length - 1)) * plotWidth,
      y: top + ((maximum - item[series.key]) / range) * (height - top - bottom),
    })),
  }));
  const ticks = Array.from({ length: 5 }, (_, index) => minimum + ((maximum - minimum) * index) / 4).reverse();
  return <div className="annual-tax-chart" ref={ref}>
    <div className="tax-chart-series" aria-label={t("Angezeigte Werte")}>
      {TAX_CHART_SERIES.map((series) => {
        const selected = selectedSeries.includes(series.key);
        return <button
          key={series.key}
          type="button"
          aria-pressed={selected}
          className={selected ? "selected" : ""}
          onClick={() => setSelectedSeries((current) => {
            if (current.includes(series.key)) {
              return current.length === 1 ? current : current.filter((key) => key !== series.key);
            }
            return [...current, series.key];
          })}
        >
          <span style={{ backgroundColor: series.color }} aria-hidden="true" />
          {t(series.label)}
        </button>;
      })}
    </div>
    <svg viewBox={`0 0 ${width} ${height}`} role="img" aria-label={t("Jährlicher Verlauf der ausgewählten Vermögenswerte")}>
      {ticks.map((tick) => {
        const y = top + ((maximum - tick) / range) * (height - top - bottom);
        return <g key={tick}><line x1={left} y1={y} x2={width - right} y2={y} className="chart-grid" /><text x={left - 12} y={y + 4} textAnchor="end" className="chart-axis-label">{compactMoney(tick)}</text></g>;
      })}
      {lines.map((line) => {
        const path = line.points.map((point, index) => `${index ? "L" : "M"} ${point.x} ${point.y}`).join(" ");
        return <g key={line.key}>
          {line.points.length > 1 && <path d={path} className="tax-chart-line" style={{ stroke: line.color }} />}
          {line.points.map((point) => <circle key={point.id} cx={point.x} cy={point.y} r="4.5" className="tax-chart-dot" style={{ stroke: line.color }}><title>{t(line.label)} · {point.taxYear}: {money(point.value)}</title></circle>)}
        </g>;
      })}
      {snapshots.map((snapshot, index) => {
        const x = snapshots.length === 1 ? left + plotWidth / 2 : left + (index / (snapshots.length - 1)) * plotWidth;
        return <text key={snapshot.id} x={x} y={height - bottom + 22} textAnchor="middle" className="chart-axis-label">{snapshot.taxYear}</text>;
      })}
    </svg>
  </div>;
}

function toEditable(preview: TaxStatementPreview, sourcePath: string): EditablePreview {
  return {
    ...preview,
    sourcePath,
    grossAssets: String(preview.grossAssetsMinor / 100),
    liabilities: String(preview.liabilitiesMinor / 100),
    taxableWealth: String(preview.taxableWealthMinor / 100),
    securitiesAndCash: String(preview.securitiesAndCashMinor / 100),
    realEstate: String(preview.realEstateMinor / 100),
    otherAssets: String(preview.otherAssetsMinor / 100),
  };
}

function emptyManualEntry(): ManualTaxEntry {
  return {
    taxYear: String(new Date().getFullYear() - 1),
    grossAssets: "",
    liabilities: "0",
    taxableWealth: "",
    securitiesAndCash: "",
    realEstate: "0",
    otherAssets: "0",
  };
}

function previewValues(item: EditablePreview) {
  return editableValues(item);
}

function editableValues(item: EditableTaxValues) {
  const inputs = [item.grossAssets, item.liabilities, item.taxableWealth, item.securitiesAndCash, item.realEstate, item.otherAssets];
  if (inputs.some((value) => value.trim() === "")) return null;
  const values = inputs.map(Number);
  if (values.some((value) => !Number.isFinite(value) || !Number.isInteger(value)) || values.slice(0, 5).some((value) => value < 0)) return null;
  return {
    grossAssetsMinor: values[0] * 100,
    liabilitiesMinor: values[1] * 100,
    taxableWealthMinor: values[2] * 100,
    securitiesAndCashMinor: values[3] * 100,
    realEstateMinor: values[4] * 100,
    otherAssetsMinor: values[5] * 100,
  };
}

function breakdownValuesAreConsistent(values: NonNullable<ReturnType<typeof editableValues>>) {
  return values.securitiesAndCashMinor + values.realEstateMinor + values.otherAssetsMinor === values.grossAssetsMinor;
}

function netValuesAreConsistent(values: NonNullable<ReturnType<typeof editableValues>>) {
  return values.grossAssetsMinor - values.liabilitiesMinor === values.taxableWealthMinor;
}

function taxValueStatus(values: ReturnType<typeof editableValues>): "valid" | "notice" | "invalid" {
  if (!values || !netValuesAreConsistent(values)) return "invalid";
  return breakdownValuesAreConsistent(values) ? "valid" : "notice";
}

function money(value: number) {
  return new Intl.NumberFormat(locale(), { style: "currency", currency: "CHF", maximumFractionDigits: 0 }).format(value / 100);
}

function signedMoney(value: number) {
  return `${value >= 0 ? "+" : "−"}${money(Math.abs(value))}`;
}

function compactMoney(value: number) {
  return new Intl.NumberFormat(locale(), { style: "currency", currency: "CHF", notation: "compact", maximumFractionDigits: 1 }).format(value / 100);
}

function formatDate(value: string) {
  const [year, month, day] = value.split("-");
  return `${day}.${month}.${year}`;
}

function fileName(path: string) {
  const parts = path.split(/[\\/]/);
  return parts[parts.length - 1] ?? path;
}
