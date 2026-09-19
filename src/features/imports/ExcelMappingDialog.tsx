// Ermöglicht die Auswahl von Tabellenblättern und die Zuordnung von Importspalten.

import { useEffect, useMemo, useRef, useState } from "react";
import { t } from "../../i18n";
import type { ImportMappingProfile, SheetInspection, TabularInspection, TabularMapping } from "./importTypes";

interface Props {
  inspection: TabularInspection;
  profiles: ImportMappingProfile[];
  initial?: TabularMapping;
  onClose: () => void;
  onApply: (mapping: TabularMapping, profileName?: string) => Promise<void>;
}

type MappingTarget = "ignore" | "date" | "description" | "counterparty" | "debit" | "credit" | "amount" | "currency" | "balance" | "category" | "valueDate" | "reference" | "transactionType" | "additionalDescription";

const commonTargetOptions: Array<{ value: MappingTarget; label: string }> = [
  { value: "ignore", label: "-- Nicht importieren --" },
  { value: "date", label: "Buchungsdatum" },
  { value: "description", label: "Beschreibung / Verwendungszweck" },
  { value: "counterparty", label: "Gegenpartei / Empfänger" },
  { value: "debit", label: "Betrag (Ausgabe / Abfluss)" },
  { value: "credit", label: "Betrag (Einnahme / Zufluss)" },
  { value: "amount", label: "Betrag (Kombiniert +/-)" },
  { value: "currency", label: "Währung" },
  { value: "balance", label: "Saldo" },
  { value: "category", label: "Kategorie" },
];

const additionalTargetOptions: Array<{ value: MappingTarget; label: string }> = [
  { value: "valueDate", label: "Valutadatum" },
  { value: "reference", label: "Referenz / Transaktions-ID" },
  { value: "transactionType", label: "Buchungsart / Transaktionstyp" },
  { value: "additionalDescription", label: "Zusatztext / weitere Informationen" },
];

export function ExcelMappingDialog({ inspection, profiles, initial, onClose, onApply }: Props) {
  const dialog = useRef<HTMLDialogElement>(null);
  const detectedSheet = inspection.sheets.reduce((best, item) => item.headerScore > best.headerScore ? item : best, inspection.sheets[0]);
  const first = inspection.sheets.find(item => item.name === initial?.sheetName) ?? (detectedSheet.headerDetected ? detectedSheet : inspection.sheets[0]);
  const initialMapping = initial ?? suggestMapping(first, first.suggestedHeaderRow);
  const [sheetName, setSheetName] = useState(initialMapping.sheetName);
  const sheet = inspection.sheets.find(item => item.name === sheetName) ?? first;
  const [headerRow, setHeaderRow] = useState(initialMapping.headerRow);
  const [mapping, setMapping] = useState<TabularMapping>(initialMapping);
  const [targets, setTargets] = useState<MappingTarget[]>(() => targetsFromMapping(initialMapping, first.columnCount));
  const [saveProfile, setSaveProfile] = useState(false);
  const [profileName, setProfileName] = useState("");
  const [saving, setSaving] = useState(false);
  const [error, setError] = useState<string | null>(null);
  const headers = useMemo(() => sheet.preview[headerRow - 1] ?? [], [sheet, headerRow]);
  const fingerprint = headerFingerprint(headers);
  const matchingProfiles = profiles.filter(profile => profile.headerFingerprint === fingerprint);
  const previewRows = sheet.preview.slice(Math.max(0, mapping.dataStartRow - 1), Math.min(sheet.preview.length, mapping.dataStartRow + 7));
  const mappedCount = targets.filter(target => target !== "ignore").length;
  const missing = [
    !targets.includes("date") && t("Buchungsdatum"),
    !targets.includes("description") && !targets.includes("counterparty") && t("Beschreibung"),
    !targets.some(target => target === "amount" || target === "debit" || target === "credit") && t("Betrag"),
  ].filter((value): value is string => Boolean(value));
  const valid = missing.length === 0;

  useEffect(() => { dialog.current?.showModal(); return () => dialog.current?.close(); }, []);

  function applySuggested(nextSheet: SheetInspection, row: number) {
    const suggested = suggestMapping(nextSheet, row);
    setHeaderRow(row);
    setMapping(suggested);
    setTargets(targetsFromMapping(suggested, nextSheet.columnCount));
    setError(null);
  }

  function changeSheet(name: string) {
    const next = inspection.sheets.find(item => item.name === name)!;
    setSheetName(name);
    applySuggested(next, next.suggestedHeaderRow);
  }

  function changeHeader(row: number) { applySuggested(sheet, row); }

  function assignColumn(column: number, target: MappingTarget) {
    setTargets(current => {
      const next = [...current];
      if (target !== "ignore") {
        for (let index = 0; index < next.length; index++) if (next[index] === target) next[index] = "ignore";
        if (target === "amount") {
          for (let index = 0; index < next.length; index++) if (next[index] === "debit" || next[index] === "credit") next[index] = "ignore";
        } else if (target === "debit" || target === "credit") {
          for (let index = 0; index < next.length; index++) if (next[index] === "amount") next[index] = "ignore";
        }
      }
      next[column] = target;
      return next;
    });
    setError(null);
  }

  function applyProfile(id: string) {
    const profile = profiles.find(item => item.id === Number(id));
    if (!profile) return;
    const next = { ...profile.mapping, sheetName, headerRow, dataStartRow: headerRow + 1 };
    setMapping(next);
    setTargets(targetsFromMapping(next, sheet.columnCount));
    setSaveProfile(false);
    setProfileName(profile.name);
  }

  async function apply() {
    if (!valid) return;
    setError(null);
    setSaving(true);
    const finalMapping = mappingFromTargets({ ...mapping, sheetName, headerRow, dataStartRow: Math.max(mapping.dataStartRow, headerRow + 1) }, targets);
    try { await onApply(finalMapping, saveProfile ? profileName.trim() || undefined : undefined); }
    catch (reason) { setError(String(reason)); setSaving(false); }
  }

  return <dialog ref={dialog} className="mapping-dialog" aria-label={t("Excel-Spalten zuordnen")} onCancel={event => { event.preventDefault(); onClose(); }}>
    <header><div><p className="eyebrow">{t("Generischer Excel-Import")}</p><h2>{t("Excel-Spalten zuordnen")}</h2><p className="intro">{t("Ordne die Excel-Spalten direkt den Finanzdaten zu.")}</p></div><button className="text-button" onClick={onClose}>{t("Schliessen")}</button></header>
    <div className="mapping-topbar">
      <label>{t("Tabellenblatt")}<select value={sheetName} onChange={event => changeSheet(event.target.value)}>{inspection.sheets.map(item => <option key={item.name}>{item.name}</option>)}</select></label>
      <label>{t("Kopfzeile")}<input type="number" min={1} max={Math.max(1, sheet.preview.length)} value={headerRow} onChange={event => changeHeader(Number(event.target.value))} /></label>
      <label>{t("Erste Datenzeile")}<input type="number" min={headerRow + 1} max={sheet.rowCount + 1} value={mapping.dataStartRow} onChange={event => setMapping(current => ({ ...current, dataStartRow: Number(event.target.value) }))} /></label>
      {profiles.length > 0 && <label>{t("Gespeichertes Profil")}<select defaultValue="" onChange={event => applyProfile(event.target.value)}><option value="">{matchingProfiles.length ? t("Passendes Profil wählen") : t("Profil wählen")}</option>{profiles.map(profile => <option key={profile.id} value={profile.id}>{profile.name}{profile.headerFingerprint === fingerprint ? ` · ${t("passend")}` : ""}</option>)}</select></label>}
    </div>
    <div className="mapping-table-wrap">
      <table className="mapping-table">
        <thead><tr>{Array.from({ length: sheet.columnCount }, (_, column) => <th key={column}>
          <small>{t("Spalte")} {excelColumn(column)}</small>
          <strong title={headers[column] || t("Ohne Überschrift")}>{headers[column] || t("Ohne Überschrift")}</strong>
          <select className={targets[column] !== "ignore" ? "mapped" : ""} value={targets[column] ?? "ignore"} aria-label={`${t("Spalte")} ${excelColumn(column)} · ${headers[column] || t("Ohne Überschrift")}`} onChange={event => assignColumn(column, event.target.value as MappingTarget)}>
            {commonTargetOptions.slice(0, 1).map(option => <option key={option.value} value={option.value}>{t(option.label)}</option>)}
            <optgroup label={t("Häufig verwendet")}>{commonTargetOptions.slice(1).map(option => <option key={option.value} value={option.value}>{t(option.label)}</option>)}</optgroup>
            <optgroup label={t("Weitere Felder")}>{additionalTargetOptions.map(option => <option key={option.value} value={option.value}>{t(option.label)}</option>)}</optgroup>
          </select>
        </th>)}</tr></thead>
        <tbody>{previewRows.map((row, rowIndex) => <tr key={rowIndex}>{Array.from({ length: sheet.columnCount }, (_, column) => <td className={isNumericTarget(targets[column]) ? "numeric" : ""} key={column}>{row[column] ?? ""}</td>)}</tr>)}</tbody>
      </table>
    </div>
    {error && <p className="error-message" role="alert">{error}</p>}
    <footer className="mapping-footer">
      <div className="mapping-profile-save"><label><input type="checkbox" checked={saveProfile} onChange={event => setSaveProfile(event.target.checked)} /> {t("Als Profil speichern")}</label>{saveProfile && <input value={profileName} maxLength={80} placeholder={t("Profilname, z. B. UBS Export")} onChange={event => setProfileName(event.target.value)} />}</div>
      <div className="mapping-summary"><strong>{sheet.rowCount} {t("Zeilen erkannt")} · {mappedCount} {t("Spalten gemappt")}</strong>{missing.length > 0 && <small role="status">{t("Noch zuordnen:")} {missing.join(", ")}</small>}</div>
      <div className="batch-buttons"><button className="secondary-button" disabled={saving} onClick={onClose}>{t("Abbrechen")}</button><button className="primary-button" disabled={saving || !valid} onClick={() => void apply()}>{saving ? t("Wird geprüft…") : t("Zuordnung übernehmen")}</button></div>
    </footer>
  </dialog>;
}

export function headerFingerprint(headers: string[]) { return headers.map(normalize).join("\u001f"); }
function normalize(value: string) { return value.trim().toLocaleLowerCase("de-CH").normalize("NFD").replace(/[\u0300-\u036f]/g, ""); }
function excelColumn(index: number) { let label = ""; for (let value = index + 1; value; value = Math.floor((value - 1) / 26)) label = String.fromCharCode(65 + ((value - 1) % 26)) + label; return label; }
function find(headers: string[], names: string[]) { const normalized = headers.map(normalize); const index = normalized.findIndex(header => names.includes(header)); return index < 0 ? null : index; }
function isNumericTarget(target?: MappingTarget) { return target === "debit" || target === "credit" || target === "amount" || target === "balance"; }

function targetsFromMapping(mapping: TabularMapping, columnCount: number): MappingTarget[] {
  const targets = Array<MappingTarget>(columnCount).fill("ignore");
  const assign = (column: number | null | undefined, target: MappingTarget) => { if (column !== null && column !== undefined && column >= 0 && column < columnCount) targets[column] = target; };
  assign(mapping.dateColumn, "date");
  assign(mapping.descriptionColumns[0], "description");
  assign(mapping.descriptionColumns[1], "counterparty");
  assign(mapping.amountColumn, "amount");
  assign(mapping.debitColumn, "debit");
  assign(mapping.creditColumn, "credit");
  assign(mapping.currencyColumn, "currency");
  assign(mapping.balanceColumn, "balance");
  assign(mapping.industryColumn, "category");
  assign(mapping.valueDateColumn, "valueDate");
  assign(mapping.descriptionColumns[2], "reference");
  assign(mapping.descriptionColumns[3], "transactionType");
  assign(mapping.descriptionColumns[4], "additionalDescription");
  return targets;
}

function mappingFromTargets(mapping: TabularMapping, targets: MappingTarget[]): TabularMapping {
  const column = (target: MappingTarget) => { const index = targets.indexOf(target); return index < 0 ? null : index; };
  return { ...mapping, dateColumn: column("date") ?? 0, descriptionColumns: [column("description"), column("counterparty"), column("reference"), column("transactionType"), column("additionalDescription")].filter((value): value is number => value !== null), amountColumn: column("amount"), debitColumn: column("debit"), creditColumn: column("credit"), valueDateColumn: column("valueDate"), balanceColumn: column("balance"), currencyColumn: column("currency"), industryColumn: column("category"), fixedCurrency: "CHF", invertAmount: false, dateFormat: "auto", numberFormat: "auto" };
}

function suggestMapping(sheet: SheetInspection, headerRow: number): TabularMapping {
  const headers = sheet.preview[headerRow - 1] ?? [];
  const date = find(headers, ["datum", "buchungsdatum", "buchungstag", "buchung", "kaufdatum", "transaktionsdatum"]);
  const description = find(headers, ["beschreibung", "informationen", "text", "buchungstext", "vorgang", "notiz"]);
  const counterparty = find(headers, ["gegenpartei", "empfanger", "begunstigter", "auftraggeber", "merchant"]);
  const amount = find(headers, ["betrag", "betrag chf", "amount"]);
  const debit = find(headers, ["belastung", "soll", "soll chf", "debit", "abfluss"]);
  const credit = find(headers, ["gutschrift", "haben", "haben chf", "credit", "zufluss"]);
  const reference = find(headers, ["referenz", "transaktions-id", "transaktions id", "transaction id", "eigene id", "kundenreferenz", "bankreferenz"]);
  const transactionType = find(headers, ["buchungsart", "transaktionstyp", "transaction type", "vorgangsart"]);
  const extraDescription = find(headers, ["zusatztext", "details", "weitere informationen", "supplementary details"]);
  const primaryDescription = description ?? counterparty ?? Math.min(1, Math.max(0, headers.length - 1));
  const descriptionColumns = [primaryDescription, counterparty, reference, transactionType, extraDescription].filter((value, index, values): value is number => value !== null && values.indexOf(value) === index);
  return { sheetName: sheet.name, headerRow, dataStartRow: headerRow + 1, dateColumn: date ?? 0, descriptionColumns, amountColumn: amount, debitColumn: amount === null ? debit : null, creditColumn: amount === null ? credit : null, valueDateColumn: find(headers, ["valutadatum", "valuta", "wertstellung", "value date"]), balanceColumn: find(headers, ["saldo", "kontostand", "balance"]), currencyColumn: find(headers, ["wahrung", "waehrung", "currency", "iso-wahrung", "iso wahrung", "wahrungscode", "currency code"]), industryColumn: find(headers, ["branche", "industry", "kategorie", "kategoriehinweis"]), fixedCurrency: "CHF", invertAmount: false, dateFormat: "auto", numberFormat: "auto" };
}
