// Erzeugt synthetische Excel-Bankbelege als Import-Testdaten.

import fs from "node:fs/promises";
import path from "node:path";
import { SpreadsheetFile, Workbook } from "@oai/artifact-tool";

const root = process.cwd();
const outDir = path.join(root, "fixtures", "bank-statements", "xlsx");
const previewDir = path.join(root, "tmp", "xlsx-previews");

const samples = [
  {
    file: "ubs_kontoauszug_2026-08.xlsx",
    sheet: "Kontobewegungen",
    title: "UBS Kontoauszug - Testdaten",
    subtitle: "Privatkonto CHF | 01.08.2026 bis 31.08.2026 | IBAN CH00 0000 0000 0000 1000 1",
    headers: ["Buchungsdatum", "Valutadatum", "Beschreibung", "Belastung", "Gutschrift", "Saldo", "Währung"],
    widths: [16, 16, 44, 16, 16, 16, 12],
    rows: [
      [new Date("2026-08-01"), new Date("2026-08-01"), "Anfangssaldo", null, null, 7840.25, "CHF"],
      [new Date("2026-08-03"), new Date("2026-08-03"), "Muster Immobilien AG - Miete", 1850.00, null, 5990.25, "CHF"],
      [new Date("2026-08-05"), new Date("2026-08-05"), "Coop Supermarkt Zürich", 126.45, null, 5863.80, "CHF"],
      [new Date("2026-08-07"), new Date("2026-08-07"), "Arbeitgeber Beispiel AG - Lohn", null, 6850.00, 12713.80, "CHF"],
      [new Date("2026-08-09"), new Date("2026-08-09"), "SBB Mobile", 68.00, null, 12645.80, "CHF"],
      [new Date("2026-08-12"), new Date("2026-08-12"), "Swisscom Rechnung", 89.90, null, 12555.90, "CHF"],
      [new Date("2026-08-16"), new Date("2026-08-16"), "Übertrag an Sparkonto", 1200.00, null, 11355.90, "CHF"],
      [new Date("2026-08-20"), new Date("2026-08-20"), "Restaurant Seeblick", 142.80, null, 11213.10, "CHF"],
      [new Date("2026-08-25"), new Date("2026-08-25"), "Krankenkasse Muster", 428.60, null, 10784.50, "CHF"],
      [new Date("2026-08-31"), new Date("2026-08-31"), "Schlusssaldo", null, null, 10784.50, "CHF"],
    ],
  },
  {
    file: "migros_bank_konto_2026-08.xlsx",
    sheet: "Buchungen",
    title: "Migros Bank Kontobewegungen - Testdaten",
    subtitle: "Sparkonto | August 2026 | Konto MB-TEST-2001",
    headers: ["Datum", "Text", "Soll CHF", "Haben CHF", "Saldo CHF", "Referenz"],
    widths: [16, 46, 16, 16, 16, 24],
    rows: [
      [new Date("2026-08-01"), "Saldovortrag", null, null, 28420.10, "SALDO"],
      [new Date("2026-08-04"), "Kartenzahlung Migros Markt", 94.70, null, 28325.40, "TX-MB-001"],
      [new Date("2026-08-08"), "Eingang Dauerauftrag", null, 1200.00, 29525.40, "TX-MB-002"],
      [new Date("2026-08-14"), "Apotheke am Bahnhof", 47.30, null, 29478.10, "TX-MB-003"],
      [new Date("2026-08-18"), "Online-Shop Beispiel", 219.00, null, 29259.10, "TX-MB-004"],
      [new Date("2026-08-22"), "Zinsgutschrift", null, 18.40, 29277.50, "TX-MB-005"],
      [new Date("2026-08-31"), "Schlusssaldo", null, null, 29277.50, "SALDO"],
    ],
  },
  {
    file: "raiffeisen_transaktionen_2026-08.xlsx",
    sheet: "Transaktionen",
    title: "Raiffeisen Transaktionsliste - Testdaten",
    subtitle: "Privatkonto | 01.08.2026 - 31.08.2026 | CH00 0000 0000 0000 3000 3",
    headers: ["Buchungstag", "Valuta", "Buchungstext", "Betrag", "Saldo", "Währung", "Mitteilung"],
    widths: [16, 16, 38, 16, 16, 12, 30],
    rows: [
      [new Date("2026-08-01"), new Date("2026-08-01"), "Anfangssaldo", 0, 15640.00, "CHF", ""],
      [new Date("2026-08-02"), new Date("2026-08-02"), "TWINT Zahlung", -34.50, 15605.50, "CHF", "Bäckerei Muster"],
      [new Date("2026-08-06"), new Date("2026-08-06"), "Gutschrift", 240.00, 15845.50, "CHF", "Spesenrückerstattung"],
      [new Date("2026-08-11"), new Date("2026-08-11"), "Lastschrift", -165.20, 15680.30, "CHF", "Elektrizitätswerk"],
      [new Date("2026-08-15"), new Date("2026-08-15"), "Kartenzahlung", -76.80, 15603.50, "CHF", "Tankstelle Beispiel"],
      [new Date("2026-08-21"), new Date("2026-08-21"), "Dauerauftrag", -500.00, 15103.50, "CHF", "Säule 3a Einzahlung"],
      [new Date("2026-08-27"), new Date("2026-08-27"), "TWINT Zahlung", -58.40, 15045.10, "CHF", "Freizeit"],
      [new Date("2026-08-31"), new Date("2026-08-31"), "Schlusssaldo", 0, 15045.10, "CHF", ""],
    ],
  },
  {
    file: "generali_vorsorge_2026.xlsx",
    sheet: "Vertragsbewegungen",
    title: "Generali Vorsorgeübersicht - Testdaten",
    subtitle: "Police GE-TEST-4001 | Gebundene Vorsorge 3a | Stand 31.08.2026",
    headers: ["Datum", "Vorgang", "Belastung", "Gutschrift", "Vertragswert", "Währung", "Hinweis"],
    widths: [16, 34, 16, 16, 18, 12, 40],
    rows: [
      [new Date("2026-01-01"), "Anfangswert", null, null, 36580.00, "CHF", "Synthetischer Anfangswert"],
      [new Date("2026-01-31"), "Prämienzahlung", null, 500.00, 37080.00, "CHF", "Monatliche Vorsorgeprämie"],
      [new Date("2026-02-28"), "Prämienzahlung", null, 500.00, 37580.00, "CHF", "Monatliche Vorsorgeprämie"],
      [new Date("2026-03-31"), "Risikoprämie", 42.00, null, 38038.00, "CHF", "Versicherungskosten"],
      [new Date("2026-04-30"), "Prämienzahlung", null, 500.00, 38538.00, "CHF", "Monatliche Vorsorgeprämie"],
      [new Date("2026-05-31"), "Prämienzahlung", null, 500.00, 39038.00, "CHF", "Monatliche Vorsorgeprämie"],
      [new Date("2026-06-30"), "Wertentwicklung", null, 614.20, 39652.20, "CHF", "Periodische Bewertung"],
      [new Date("2026-07-31"), "Prämienzahlung", null, 500.00, 40152.20, "CHF", "Monatliche Vorsorgeprämie"],
      [new Date("2026-08-31"), "Vertragswert", null, null, 40152.20, "CHF", "Stand per Monatsende"],
    ],
  },
];

await fs.mkdir(outDir, { recursive: true });
await fs.mkdir(previewDir, { recursive: true });

for (const sample of samples) {
  const workbook = Workbook.create();
  const sheet = workbook.worksheets.add(sample.sheet);
  sheet.showGridLines = false;
  sheet.getRange("A1:G1").merge();
  sheet.getRange("A1").values = [[sample.title]];
  sheet.getRange("A2:G2").merge();
  sheet.getRange("A2").values = [[sample.subtitle]];
  const lastCol = String.fromCharCode(64 + sample.headers.length);
  sheet.getRange(`A4:${lastCol}4`).values = [sample.headers];
  sheet.getRange(`A5:${lastCol}${4 + sample.rows.length}`).values = sample.rows;
  sheet.getRange(`A4:${lastCol}${4 + sample.rows.length}`).format.borders = { preset: "all", style: "thin", color: "#DDE3E8" };
  sheet.getRange("A1:G1").format = { fill: "#10223F", font: { bold: true, color: "#FFFFFF", size: 18 }, rowHeight: 32 };
  sheet.getRange("A2:G2").format = { fill: "#EAF1FB", font: { color: "#465870", italic: true }, rowHeight: 26 };
  sheet.getRange(`A4:${lastCol}4`).format = { fill: "#176B57", font: { bold: true, color: "#FFFFFF" }, rowHeight: 25 };
  sheet.getRange(`A5:B${4 + sample.rows.length}`).setNumberFormat("yyyy-mm-dd");
  sheet.getRange(`C5:${lastCol}${4 + sample.rows.length}`).format.rowHeight = 22;
  sample.widths.forEach((width, idx) => { sheet.getRangeByIndexes(0, idx, 4 + sample.rows.length, 1).format.columnWidth = width; });
  sheet.freezePanes.freezeRows(4);
  const table = sheet.tables.add(`A4:${lastCol}${4 + sample.rows.length}`, true, `${sample.sheet.replace(/[^A-Za-z]/g, "")}Table`);
  table.style = "TableStyleMedium4";
  const numericStart = sample.file.includes("raiffeisen") ? 3 : 2;
  sheet.getRangeByIndexes(4, numericStart, sample.rows.length, sample.headers.length - numericStart).setNumberFormat("#,##0.00;[Red]-#,##0.00;-");
  const xlsx = await SpreadsheetFile.exportXlsx(workbook);
  await xlsx.save(path.join(outDir, sample.file));
  const preview = await workbook.render({ sheetName: sample.sheet, autoCrop: "all", scale: 1.25, format: "png" });
  await fs.writeFile(path.join(previewDir, sample.file.replace(".xlsx", ".png")), new Uint8Array(await preview.arrayBuffer()));
  const inspection = await workbook.inspect({ kind: "table", range: `${sample.sheet}!A1:${lastCol}${4 + sample.rows.length}`, include: "values,formulas", tableMaxRows: 20, tableMaxCols: 8 });
  console.log(sample.file, inspection.ndjson);
  const errors = await workbook.inspect({ kind: "match", searchTerm: "#REF!|#DIV/0!|#VALUE!|#NAME\\?|#N/A|#NUM!|#NULL!|#SPILL!|#CALC!", options: { useRegex: true, maxResults: 50 }, summary: "formula error scan" });
  console.log(errors.ndjson);
}
