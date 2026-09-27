// Ermöglicht eine tabellenähnliche Zuordnung erkannter PDF-Felder und speichert wiederverwendbare Layoutprofile.

import { useEffect, useMemo, useRef, useState } from "react";
import { t } from "../../i18n";
import type { PdfAmountSign, PdfImportMappingProfile, PdfInspection, PdfInspectionRow, PdfMapping, PdfTextSource } from "./importTypes";

interface Props {
  inspection: PdfInspection;
  profiles: PdfImportMappingProfile[];
  initial?: PdfMapping;
  onClose: () => void;
  onApply: (mapping: PdfMapping, profileName?: string) => Promise<void>;
}

type Target = "ignore" | "date" | "valueDate" | "description" | "amount" | "balance";
type Column = { key: string; label: string; kind: "text" | "date" | "money"; index?: number };

export function PdfMappingDialog({ inspection, profiles, initial, onClose, onApply }: Props) {
  const dialog = useRef<HTMLDialogElement>(null);
  const columns = useMemo(() => pdfColumns(inspection), [inspection]);
  const initialMapping = initial ?? suggestPdfMapping(inspection);
  const [mapping, setMapping] = useState(initialMapping);
  const [targets, setTargets] = useState<Record<string, Target>>(() => targetsFromMapping(initialMapping, columns));
  const [ignoredText, setIgnoredText] = useState(initialMapping.ignoredDescriptions.join(", "));
  const [saveProfile, setSaveProfile] = useState(false);
  const [profileName, setProfileName] = useState("");
  const [saving, setSaving] = useState(false);
  const [error, setError] = useState<string | null>(null);
  const matchingProfiles = profiles.filter(profile => profile.layoutFingerprint === inspection.layoutFingerprint);
  const missing = [
    !Object.values(targets).includes("date") && t("Buchungsdatum"),
    !Object.values(targets).includes("description") && t("Beschreibung"),
    !Object.values(targets).includes("amount") && t("Betrag"),
    mapping.amountSign === "infer-from-balance" && !Object.values(targets).includes("balance") && t("Saldo"),
  ].filter((value): value is string => Boolean(value));
  const valid = missing.length === 0 && /^[A-Za-z]{3}$/.test(mapping.fixedCurrency);

  useEffect(() => {
    dialog.current?.showModal();
    const frame = requestAnimationFrame(() => dialog.current?.querySelector<HTMLSelectElement>("select")?.focus());
    return () => { cancelAnimationFrame(frame); dialog.current?.close(); };
  }, []);

  function assignColumn(key: string, target: Target) {
    setTargets(current => {
      const next = { ...current };
      if (target !== "ignore" && target !== "description") {
        for (const column of Object.keys(next)) if (next[column] === target) next[column] = "ignore";
      }
      next[key] = target;
      return next;
    });
    setError(null);
  }

  function applyProfile(id: string) {
    const profile = profiles.find(item => item.id === Number(id));
    if (!profile) return;
    setMapping(profile.mapping);
    setTargets(targetsFromMapping(profile.mapping, columns));
    setIgnoredText(profile.mapping.ignoredDescriptions.join(", "));
    setSaveProfile(false);
    setProfileName(profile.name);
  }

  async function apply() {
    if (!valid) return;
    setSaving(true);
    setError(null);
    const finalMapping = mappingFromTargets(mapping, targets, columns, ignoredText);
    try {
      await onApply(finalMapping, saveProfile ? profileName.trim() || undefined : undefined);
    } catch (reason) {
      setError(String(reason));
      setSaving(false);
    }
  }

  return <dialog ref={dialog} className="mapping-dialog pdf-mapping-dialog" aria-label={t("PDF-Felder zuordnen")} onCancel={event => { event.preventDefault(); onClose(); }}>
    <header><div><p className="eyebrow">{t("Generischer PDF-Import")}</p><h2>{t("PDF-Felder zuordnen")}</h2><p className="intro">{t("Ordne die erkannten PDF-Felder den Finanzdaten zu. Die Quelldatei bleibt lokal.")}</p></div><button className="text-button" onClick={onClose}>{t("Schliessen")}</button></header>
    <div className="mapping-topbar pdf-mapping-topbar">
      {profiles.length > 0 && <label>{t("Gespeichertes Profil")}<select defaultValue="" onChange={event => applyProfile(event.target.value)}><option value="">{matchingProfiles.length ? t("Passendes Profil wählen") : t("Profil wählen")}</option>{profiles.map(profile => <option key={profile.id} value={profile.id}>{profile.name}{profile.layoutFingerprint === inspection.layoutFingerprint ? ` · ${t("passend")}` : ""}</option>)}</select></label>}
      <label>{t("Vorzeichenlogik")}<select value={mapping.amountSign} onChange={event => setMapping(current => ({ ...current, amountSign: event.target.value as PdfAmountSign }))}><option value="infer-from-balance">{t("Aus laufendem Saldo ableiten")}</option><option value="signed">{t("Wie im PDF angegeben")}</option><option value="debit">{t("Immer Belastung")}</option><option value="credit">{t("Immer Gutschrift")}</option></select></label>
      <label>{t("Feste Währung")}<input value={mapping.fixedCurrency} maxLength={3} onChange={event => setMapping(current => ({ ...current, fixedCurrency: event.target.value.toUpperCase() }))} /></label>
      <label>{t("Datumsformat")}<select value={mapping.dateFormat} onChange={event => setMapping(current => ({ ...current, dateFormat: event.target.value as PdfMapping["dateFormat"] }))}><option value="auto">{t("Automatisch")}</option><option value="dmy">{t("Tag–Monat–Jahr")}</option><option value="mdy">{t("Monat–Tag–Jahr")}</option><option value="ymd">{t("Jahr–Monat–Tag")}</option></select></label>
      <label>{t("Zahlenformat")}<select value={mapping.numberFormat} onChange={event => setMapping(current => ({ ...current, numberFormat: event.target.value as PdfMapping["numberFormat"] }))}><option value="auto">{t("Automatisch")}</option><option value="decimal-comma">{t("Dezimalkomma")}</option><option value="decimal-point">{t("Dezimalpunkt")}</option></select></label>
      <label className="pdf-ignore-field">{t("Buchungszeilen ignorieren, wenn der Text enthält")}<input value={ignoredText} maxLength={500} placeholder={t("Kommagetrennt, z. B. Anfangssaldo, Opening balance")} onChange={event => setIgnoredText(event.target.value)} /></label>
    </div>
    <div className="mapping-table-wrap">
      <table className="mapping-table pdf-mapping-table">
        <thead><tr>{columns.map(column => <th key={column.key}><small>{column.kind === "text" ? t("Textfeld") : column.kind === "date" ? t("Datumswert") : t("Betragswert")}</small><strong>{t(column.label)}</strong><select className={targets[column.key] !== "ignore" ? "mapped" : ""} value={targets[column.key] ?? "ignore"} aria-label={`${t(column.label)} · ${t("Zuordnung")}`} onChange={event => assignColumn(column.key, event.target.value as Target)}><option value="ignore">{t("-- Nicht importieren --")}</option>{column.kind === "date" && <><option value="date">{t("Buchungsdatum")}</option><option value="valueDate">{t("Valutadatum")}</option></>}{column.kind === "text" && <option value="description">{t("Beschreibung / Verwendungszweck")}</option>}{column.kind === "money" && <><option value="amount">{t("Betrag")}</option><option value="balance">{t("Saldo")}</option></>}</select></th>)}</tr></thead>
        <tbody>{inspection.preview.map(row => <tr key={row.sourceRow}>{columns.map(column => <td className={column.kind === "money" ? "numeric" : ""} key={column.key} title={cellValue(row, column)}>{cellValue(row, column) || "–"}</td>)}</tr>)}</tbody>
      </table>
    </div>
    {error && <p className="error-message" role="alert">{error}</p>}
    <footer className="mapping-footer">
      <div className="mapping-profile-save"><label><input type="checkbox" checked={saveProfile} onChange={event => setSaveProfile(event.target.checked)} /> {t("Als Profil speichern")}</label>{saveProfile && <input value={profileName} maxLength={80} placeholder={t("Profilname, z. B. Bank PDF")} onChange={event => setProfileName(event.target.value)} />}</div>
      <div className="mapping-summary"><strong>{inspection.rowCount} {t("PDF-Zeilen erkannt")} · {Object.values(targets).filter(target => target !== "ignore").length} {t("Felder gemappt")}</strong>{missing.length > 0 && <small role="status">{t("Noch zuordnen:")} {missing.join(", ")}</small>}</div>
      <div className="batch-buttons"><button className="secondary-button" disabled={saving} onClick={onClose}>{t("Abbrechen")}</button><button className="primary-button" disabled={saving || !valid} onClick={() => void apply()}>{saving ? t("Wird geprüft…") : t("Zuordnung übernehmen")}</button></div>
    </footer>
  </dialog>;
}

function pdfColumns(inspection: PdfInspection): Column[] {
  return [
    { key: "text-before", label: "Text vor der Buchungszeile", kind: "text" },
    { key: "text-inline", label: "Text in der Buchungszeile", kind: "text" },
    { key: "text-after", label: "Text nach der Buchungszeile", kind: "text" },
    ...Array.from({ length: Math.min(inspection.maxDateCount, 6) }, (_, index): Column => ({ key: `date-${index}`, label: `${t("Datumswert")} ${index + 1}`, kind: "date", index })),
    ...Array.from({ length: Math.min(inspection.maxMoneyCount, 6) }, (_, index): Column => ({ key: `money-${index}`, label: `${t("Betragswert")} ${index + 1}`, kind: "money", index })),
  ];
}

function cellValue(row: PdfInspectionRow, column: Column): string {
  if (column.key === "text-before") return row.textBefore;
  if (column.key === "text-inline") return row.textInline;
  if (column.key === "text-after") return row.textAfter;
  return column.kind === "date" ? row.dates[column.index ?? 0] ?? "" : row.amounts[column.index ?? 0] ?? "";
}

function targetsFromMapping(mapping: PdfMapping, columns: Column[]): Record<string, Target> {
  const targets = Object.fromEntries(columns.map(column => [column.key, "ignore" as Target]));
  targets[`date-${mapping.dateIndex}`] = "date";
  if (mapping.valueDateIndex !== null) targets[`date-${mapping.valueDateIndex}`] = "valueDate";
  targets[`text-${mapping.descriptionSource}`] = "description";
  for (const source of mapping.additionalDescriptionSources ?? []) targets[`text-${source}`] = "description";
  targets[`money-${mapping.amountIndex}`] = "amount";
  if (mapping.balanceIndex !== null) targets[`money-${mapping.balanceIndex}`] = "balance";
  return targets;
}

function mappingFromTargets(mapping: PdfMapping, targets: Record<string, Target>, columns: Column[], ignoredText: string): PdfMapping {
  const column = (target: Target) => columns.find(item => targets[item.key] === target);
  const descriptionSources = (["before", "inline", "after"] as PdfTextSource[])
    .filter(source => targets[`text-${source}`] === "description");
  return {
    ...mapping,
    dateIndex: column("date")?.index ?? 0,
    valueDateIndex: column("valueDate")?.index ?? null,
    descriptionSource: descriptionSources[0] ?? "inline",
    additionalDescriptionSources: descriptionSources.slice(1),
    amountIndex: column("amount")?.index ?? 0,
    balanceIndex: column("balance")?.index ?? null,
    ignoredDescriptions: ignoredText.split(",").map(value => value.trim()).filter(Boolean).slice(0, 20),
  };
}

function suggestPdfMapping(inspection: PdfInspection): PdfMapping {
  const rowCount = Math.max(inspection.preview.length, 1);
  const beforeCount = inspection.preview.filter(row => row.textBefore).length;
  const inlineCount = inspection.preview.filter(row => row.textInline).length;
  const afterCount = inspection.preview.filter(row => row.textAfter).length;
  const inlineLayout = inlineCount >= Math.ceil(rowCount / 2);
  const descriptionSource: PdfTextSource = inlineLayout ? "inline" : beforeCount > 0 ? "before" : afterCount > 0 ? "after" : "inline";
  const additionalDescriptionSources: PdfTextSource[] = inlineLayout && afterCount > 0
    ? ["after"]
    : descriptionSource === "before" && inlineCount > 0 ? ["inline"] : [];
  const pair = bestMoneyPair(inspection.preview, inspection.maxMoneyCount);
  return { dateIndex: 0, valueDateIndex: inspection.maxDateCount > 1 ? 1 : null, descriptionSource, additionalDescriptionSources, amountIndex: pair?.amount ?? 0, balanceIndex: pair?.balance ?? null, fixedCurrency: "CHF", amountSign: pair ? "infer-from-balance" : "signed", dateFormat: "auto", numberFormat: "auto", ignoredDescriptions: [] };
}

function bestMoneyPair(rows: PdfInspectionRow[], count: number): { amount: number; balance: number } | null {
  let best: { amount: number; balance: number; score: number } | null = null;
  for (let balance = 0; balance < count; balance++) for (let amount = 0; amount < count; amount++) {
    if (amount === balance) continue;
    let previous: number | null = null;
    let score = 0;
    for (const row of rows) {
      const nextBalance = parseNumber(row.amounts[balance]);
      const nextAmount = Math.abs(parseNumber(row.amounts[amount]) ?? Number.NaN);
      if (previous !== null && nextBalance !== null && Number.isFinite(nextAmount) && Math.abs(Math.abs(nextBalance - previous) - nextAmount) < .005) score++;
      if (nextBalance !== null) previous = nextBalance;
    }
    if (!best || score > best.score) best = { amount, balance, score };
  }
  return best && best.score > 0 ? best : null;
}

function parseNumber(value?: string): number | null {
  if (!value) return null;
  const cleaned = value.replace(/[ '\u2019]/g, "");
  const decimal = Math.max(cleaned.lastIndexOf("."), cleaned.lastIndexOf(","));
  const normalized = [...cleaned].filter((character, index) => /\d|-/.test(character) || index === decimal).join("").replace(",", ".");
  const number = Number(normalized);
  return Number.isFinite(number) ? number : null;
}
