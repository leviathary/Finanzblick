// Bündelt Editor, Diagramm und reine Präsentationshelfer für die Steuerhistorie.

import { useEffect, useRef, useState } from "react";

import { locale, t } from "../../i18n";
import type { EditablePreview, EditableTaxValues, ManualTaxEntry, TaxSnapshot, TaxStatementPreview } from "./TaxHistory";

type TaxSeriesKey =
  | "securitiesAndCashMinor"
  | "realEstateMinor"
  | "otherAssetsMinor"
  | "grossAssetsMinor"
  | "liabilitiesMinor"
  | "taxableWealthMinor";

const TAX_CHART_SERIES: { key: TaxSeriesKey; label: string; color: string }[] = [
  { key: "securitiesAndCashMinor", label: "Wertschriften & Guthaben", color: "var(--accent-positive-strong)" },
  { key: "realEstateMinor", label: "Liegenschaften", color: "var(--chart-blue)" },
  { key: "otherAssetsMinor", label: "Übrige / Korrekturen", color: "var(--chart-amber)" },
  { key: "grossAssetsMinor", label: "Total Vermögenswerte", color: "var(--text-secondary)" },
  { key: "liabilitiesMinor", label: "Schulden", color: "var(--accent-negative)" },
  { key: "taxableWealthMinor", label: "Steuerbares Vermögen gesamt", color: "var(--chart-purple)" },
];

function MoneyField({ label, value, onChange }: { label: string; value: string; onChange: (value: string) => void }) {
  return <label>{label}<span className="money-input"><span>CHF</span><input type="number" step="1" inputMode="numeric" value={value} onChange={(event) => onChange(event.target.value)} /></span></label>;
}

export function TaxValueEditor({ item, onChange }: {
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

export function AnnualTaxChart({ snapshots }: { snapshots: TaxSnapshot[] }) {
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

export function toEditable(preview: TaxStatementPreview, sourcePath: string): EditablePreview {
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

export function emptyManualEntry(): ManualTaxEntry {
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

export function previewValues(item: EditablePreview) {
  return editableValues(item);
}

export function editableValues(item: EditableTaxValues) {
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

export function netValuesAreConsistent(values: NonNullable<ReturnType<typeof editableValues>>) {
  return values.grossAssetsMinor - values.liabilitiesMinor === values.taxableWealthMinor;
}

export function taxValueStatus(values: ReturnType<typeof editableValues>): "valid" | "notice" | "invalid" {
  if (!values || !netValuesAreConsistent(values)) return "invalid";
  return breakdownValuesAreConsistent(values) ? "valid" : "notice";
}

export function money(value: number) {
  return new Intl.NumberFormat(locale(), { style: "currency", currency: "CHF", maximumFractionDigits: 0 }).format(value / 100);
}

export function signedMoney(value: number) {
  return `${value >= 0 ? "+" : "−"}${money(Math.abs(value))}`;
}

function compactMoney(value: number) {
  return new Intl.NumberFormat(locale(), { style: "currency", currency: "CHF", notation: "compact", maximumFractionDigits: 1 }).format(value / 100);
}

export function formatDate(value: string) {
  const [year, month, day] = value.split("-");
  return `${day}.${month}.${year}`;
}

export function fileName(path: string) {
  const parts = path.split(/[\\/]/);
  return parts[parts.length - 1] ?? path;
}
