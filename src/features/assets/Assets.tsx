import { t, tr, locale } from "../../i18n";
import { useEffect, useRef, useState } from "react";
import { invoke } from "@tauri-apps/api/core";
import { ProviderLogo } from "../accounts/ProviderLogo";

interface HistoryPoint {
  date: string;
  totalMinor: number;
}
interface Breakdown {
  key: string;
  label: string;
  amountMinor: number;
  accountCount: number;
}
interface AssetAccount {
  id: number;
  provider: string;
  providerKey: string;
  name: string;
  accountType: string;
  currency: string;
  balanceMinor: number | null;
  balanceDate: string | null;
  logoDataUrl: string | null;
}
interface WealthData {
  currency: string;
  currentTotalMinor: number;
  firstTotalMinor: number | null;
  changeMinor: number | null;
  history: HistoryPoint[];
  byType: Breakdown[];
  byProvider: Breakdown[];
  accounts: AssetAccount[];
}

export function Assets({
  onAccounts,
  onImport,
}: {
  onAccounts: () => void;
  onImport: () => void;
}) {
  const [data, setData] = useState<WealthData | null>(null);
  const [error, setError] = useState<string | null>(null);
  const [period, setPeriod] = useState<
    "all" | "currentYear" | "1y" | "2y" | "3y" | "5y" | "custom"
  >("all");
  const [customFrom, setCustomFrom] = useState("");
  const [customTo, setCustomTo] = useState("");
  const [selectedAccountIds, setSelectedAccountIds] = useState<number[]>([]);
  useEffect(() => {
    const load = () =>
      void invoke<WealthData>("wealth_data", {
        accountIds: selectedAccountIds.length ? selectedAccountIds : null,
      })
        .then(setData)
        .catch((reason) =>
          setError(
            typeof reason === "string"
              ? reason
              : t("Vermögensdaten konnten nicht geladen werden."),
          ),
        );
    load();
    window.addEventListener("market-data-refreshed", load);
    return () => window.removeEventListener("market-data-refreshed", load);
  }, [selectedAccountIds]);
  if (error)
    return (
      <section className="assets-page">
        <p className="error-message">{t(error)}</p>
      </section>
    );
  if (!data)
    return (
      <section className="assets-page">
        <p className="intro">{t("Vermögen wird geladen…")}</p>
      </section>
    );
  if (!data.accounts.length)
    return (
      <section className="assets-page empty-overview">
        <p className="eyebrow">{t("Vermögen")}</p>
        <h1>{t("Noch keine Vermögensdaten")}</h1>
        <p className="intro">
          {t(
            "Importiere einen Auszug oder nimm ein Konto in die Vermögensberechnung auf.",
          )}
        </p>
        <div className="empty-actions">
          <button className="primary-button" onClick={onImport}>
            {t("Auszug importieren")}
          </button>
          <button className="secondary-button" onClick={onAccounts}>
            {t("Konten verwalten")}
          </button>
        </div>
      </section>
    );

  const fullFrom = data.history[0]?.date ?? "";
  const fullTo = data.history[data.history.length - 1]?.date ?? "";
  const filteredHistory = (() => {
    if (!data.history.length) return [];
    let from = fullFrom,
      to = fullTo;
    if (period === "custom") {
      from = customFrom || fullFrom;
      to = customTo || fullTo;
    } else if (period === "currentYear") {
      from = `${new Date().getFullYear()}-01-01`;
    } else if (period !== "all") {
      const years =
        period === "1y" ? 1 : period === "2y" ? 2 : period === "3y" ? 3 : 5;
      const date = new Date(`${fullTo}T12:00:00`);
      date.setFullYear(date.getFullYear() - years);
      from = date.toISOString().slice(0, 10);
    }
    return data.history.filter(
      (point) => point.date >= from && point.date <= to,
    );
  })();
  const periodFirst = filteredHistory[0]?.totalMinor ?? null;
  const periodLast =
    filteredHistory[filteredHistory.length - 1]?.totalMinor ?? null;
  const periodChange =
    periodFirst !== null && periodLast !== null
      ? periodLast - periodFirst
      : null;
  const changePercent =
    periodFirst && periodChange !== null
      ? (periodChange / Math.abs(periodFirst)) * 100
      : null;
  return (
    <section className="assets-page">
      <div className="overview-heading">
        <div>
          <p className="eyebrow">{t("Vermögen")}</p>
          <h1>{t("Vermögensentwicklung")}</h1>
          <p className="intro">
            {t(
              "Entwicklung und Zusammensetzung auf Basis der importierten Salden.",
            )}
          </p>
        </div>
        <button className="primary-button" onClick={onAccounts}>
          {t("Konten verwalten")}
        </button>
      </div>
      <div className="asset-kpis">
        <article>
          <span>{t("Aktuelles Vermögen")}</span>
          <strong>{money(data.currentTotalMinor, data.currency)}</strong>
        </article>
        <article>
          <span>{t("Veränderung im gewählten Zeitraum")}</span>
          <strong className={(periodChange ?? 0) < 0 ? "negative" : "positive"}>
            {signedMoney(periodChange, data.currency)}
          </strong>
          <small>
            {changePercent === null
              ? t("Noch kein Vergleichswert")
              : `${changePercent >= 0 ? "+" : ""}${changePercent.toFixed(1)} %`}
          </small>
        </article>
      </div>
      <article className="dashboard-card wealth-chart-card">
        <div className="card-heading">
          <div>
            <p className="eyebrow">{t("Zeitverlauf")}</p>
            <h2>{t("Gesamtvermögen")}</h2>
            <small className="full-period">
              {t("Gesamte Datenbasis:")}{" "}
              {fullFrom ? `${shortDate(fullFrom)} – ${shortDate(fullTo)}` : "–"}
            </small>
          </div>
          <div className="wealth-chart-controls">
            <span>
              {filteredHistory.length > 1
                ? `${shortDate(filteredHistory[0].date)} – ${shortDate(filteredHistory[filteredHistory.length - 1].date)}`
                : t("Kein Vergleichszeitraum")}
            </span>
            <div className="account-picker-field">
              <span>{t("Konto / Depot")}</span>
              <details className="account-picker">
                <summary>
                  {selectedAccountIds.length === 0
                    ? t("Gesamtvermögen")
                    : selectedAccountIds.length === 1
                      ? accountLabel(data.accounts.find((account) => account.id === selectedAccountIds[0]))
                      : `${selectedAccountIds.length} ${t("ausgewählt")}`}
                </summary>
                <div className="account-picker-menu">
                  <label>
                    <input
                      type="checkbox"
                      checked={selectedAccountIds.length === 0}
                      onChange={() => setSelectedAccountIds([])}
                    />
                    {t("Gesamtvermögen")}
                  </label>
                  {data.accounts.map((account) => (
                    <label key={account.id}>
                      <input
                        type="checkbox"
                        checked={selectedAccountIds.includes(account.id)}
                        onChange={() =>
                          setSelectedAccountIds((selected) =>
                            selected.includes(account.id)
                              ? selected.filter((id) => id !== account.id)
                              : [...selected, account.id],
                          )
                        }
                      />
                      {accountLabel(account)}
                    </label>
                  ))}
                </div>
              </details>
            </div>
          </div>
        </div>
        <div className="period-controls">
          <div className="period-presets">
            {[
              ["all", t("Gesamt")],
              ["currentYear", t("Aktuelles Jahr")],
              ["1y", t("1 Jahr")],
              ["2y", t("2 Jahre")],
              ["3y", t("3 Jahre")],
              ["5y", t("5 Jahre")],
              ["custom", t("Eigener Zeitraum")],
            ].map(([value, label]) => (
              <button
                className={period === value ? "active" : ""}
                key={value}
                onClick={() => {
                  setPeriod(value as typeof period);
                  if (value === "custom") {
                    setCustomFrom(customFrom || fullFrom);
                    setCustomTo(customTo || fullTo);
                  }
                }}
              >
                {label}
              </button>
            ))}
          </div>
          {period === "custom" && (
            <div className="custom-period">
              <label>
                {t("Von")}
                <input
                  type="date"
                  lang={locale()}
                  min={fullFrom}
                  max={customTo || fullTo}
                  value={customFrom}
                  onChange={(event) => setCustomFrom(event.target.value)}
                />
              </label>
              <label>
                {t("Bis")}
                <input
                  type="date"
                  lang={locale()}
                  min={customFrom || fullFrom}
                  max={fullTo}
                  value={customTo}
                  onChange={(event) => setCustomTo(event.target.value)}
                />
              </label>
            </div>
          )}
        </div>
        <WealthChart
          history={filteredHistory}
          currency={data.currency}
          onSelectRange={(from, to) => {
            setCustomFrom(from);
            setCustomTo(to);
            setPeriod("custom");
          }}
        />
      </article>
      <div className="asset-breakdowns">
        <BreakdownCard
          translateTypes
          title={t("Nach Kategorie")}
          values={data.byType}
          total={data.currentTotalMinor}
          currency={data.currency}
        />
        <BreakdownCard
          title={t("Nach Anbieter")}
          values={data.byProvider}
          total={data.currentTotalMinor}
          currency={data.currency}
        />
      </div>
      <article className="dashboard-card asset-accounts">
        <div className="card-heading">
          <div>
            <p className="eyebrow">{t("Details")}</p>
            <h2>{t("Vermögenspositionen")}</h2>
          </div>
          <span>
            {data.accounts.length} {t("Positionen")}
          </span>
        </div>
        {data.accounts.map((account) => (
          <div className="asset-account" key={account.id}>
            <ProviderLogo
              name={account.provider}
              providerKey={account.providerKey}
              customLogo={account.logoDataUrl}
            />
            <div>
              <strong>{account.name}</strong>
              <small>
                {account.provider} · {typeLabel(account.accountType)}
              </small>
            </div>
            <span>
              {account.balanceDate
                ? tr`Stand ${shortDate(account.balanceDate)}`
                : t("Ohne Stichtag")}
            </span>
            <b>{money(account.balanceMinor, account.currency)}</b>
          </div>
        ))}
      </article>
    </section>
  );
}

function accountLabel(account: AssetAccount | undefined): string {
  return account ? `${account.provider} · ${account.name}` : "–";
}

function WealthChart({
  history,
  currency,
  onSelectRange,
}: {
  history: HistoryPoint[];
  currency: string;
  onSelectRange: (from: string, to: string) => void;
}) {
  const [hoveredIndex, setHoveredIndex] = useState<number | null>(null);
  const [dragSelection, setDragSelection] = useState<{
    startIndex: number;
    currentIndex: number;
  } | null>(null);
  const [measurement, setMeasurement] = useState<{
    startIndex: number;
    currentIndex: number;
  } | null>(null);
  const dragStartIndex = useRef<number | null>(null);
  const dragStartClientX = useRef<number | null>(null);
  const activePointerId = useRef<number | null>(null);
  const dragMode = useRef<"zoom" | "measure" | null>(null);
  const removeDragListeners = useRef<(() => void) | null>(null);
  const historySignature = history
    .map((point) => `${point.date}:${point.totalMinor}`)
    .join("|");
  useEffect(() => {
    removeDragListeners.current?.();
    setMeasurement(null);
    setDragSelection(null);
    dragStartIndex.current = null;
    dragStartClientX.current = null;
    activePointerId.current = null;
    dragMode.current = null;
  }, [historySignature]);
  useEffect(
    () => () => {
      removeDragListeners.current?.();
    },
    [],
  );
  if (!history.length)
    return (
      <div className="chart-empty">
        {t("Noch keine Saldo-Snapshots vorhanden.")}
      </div>
    );
  const chartHistory = history;
  const width = 900,
    height = 270,
    left = 92,
    right = 18,
    top = 16,
    bottom = 42;
  const values = chartHistory.map((point) => point.totalMinor);
  const { minimum, maximum, ticks } = axisScale(
    Math.min(...values),
    Math.max(...values),
  );
  const range = maximum - minimum;
  const startTime = Date.parse(chartHistory[0].date);
  const endTime = Date.parse(chartHistory[chartHistory.length - 1].date);
  const timeRange = Math.max(endTime - startTime, 1);
  const plotWidth = width - left - right;
  const points = chartHistory.map((point) => ({
    ...point,
    x: left + ((Date.parse(point.date) - startTime) / timeRange) * plotWidth,
    y: top + ((maximum - point.totalMinor) / range) * (height - top - bottom),
  }));
  const line = stepPath(points);
  const area = `${line} L ${points[points.length - 1].x} ${height - bottom} L ${points[0].x} ${height - bottom} Z`;
  const dateTicks = Array.from({ length: 5 }, (_, index) => {
    const timestamp = startTime + (timeRange * index) / 4;
    return {
      x: left + (plotWidth * index) / 4,
      date: new Date(timestamp).toISOString().slice(0, 10),
    };
  });
  const hoveredPoint = hoveredIndex === null ? null : points[hoveredIndex];
  const selectionStart = dragSelection
    ? points[Math.min(dragSelection.startIndex, dragSelection.currentIndex)]
    : null;
  const selectionEnd = dragSelection
    ? points[Math.max(dragSelection.startIndex, dragSelection.currentIndex)]
    : null;
  const measurementStart = measurement ? points[measurement.startIndex] : null;
  const measurementEnd = measurement ? points[measurement.currentIndex] : null;
  const measurementDifference =
    measurementStart && measurementEnd
      ? measurementEnd.totalMinor - measurementStart.totalMinor
      : null;
  const measurementPercent =
    measurementStart &&
    measurementDifference !== null &&
    measurementStart.totalMinor !== 0
      ? (measurementDifference / Math.abs(measurementStart.totalMinor)) * 100
      : null;
  const pointIndexAt = (clientX: number, svg: SVGSVGElement) => {
    const screenMatrix = svg.getScreenCTM();
    let chartX: number;
    if (screenMatrix) {
      const pointer = svg.createSVGPoint();
      pointer.x = clientX;
      pointer.y = 0;
      chartX = pointer.matrixTransform(screenMatrix.inverse()).x;
    } else {
      const bounds = svg.getBoundingClientRect();
      chartX = ((clientX - bounds.left) / bounds.width) * width;
    }
    let index = 0;
    while (index + 1 < points.length && points[index + 1].x <= chartX) index += 1;
    if (
      index + 1 < points.length &&
      Math.abs(points[index + 1].x - chartX) < Math.abs(points[index].x - chartX)
    ) {
      index += 1;
    }
    return index;
  };
  const finishDragSelection = (clientX: number, svg: SVGSVGElement) => {
    const startIndex = dragStartIndex.current;
    const mode = dragMode.current;
    const dragDistance = Math.abs(clientX - (dragStartClientX.current ?? clientX));
    const endIndex = pointIndexAt(clientX, svg);
    dragStartIndex.current = null;
    dragStartClientX.current = null;
    activePointerId.current = null;
    dragMode.current = null;
    setDragSelection(null);
    if (
      startIndex === null ||
      startIndex === endIndex ||
      (mode === "zoom" && dragDistance < 5)
    ) {
      if (mode === "measure") setMeasurement(null);
      return;
    }
    if (mode === "measure") {
      setMeasurement({ startIndex, currentIndex: endIndex });
      return;
    }
    const fromIndex = Math.min(startIndex, endIndex);
    const toIndex = Math.max(startIndex, endIndex);
    onSelectRange(points[fromIndex].date, points[toIndex].date);
  };
  const tooltipWidth = 176;
  const tooltipX = hoveredPoint
    ? Math.min(Math.max(hoveredPoint.x - tooltipWidth / 2, left), width - right - tooltipWidth)
    : 0;
  const tooltipY = hoveredPoint
    ? hoveredPoint.y > top + 46
      ? hoveredPoint.y - 42
      : hoveredPoint.y + 12
    : 0;
  const measurementTooltipWidth = 214;
  const measurementTooltipHeight = 54;
  const measurementTooltipX =
    measurementStart && measurementEnd
      ? Math.min(
          Math.max(
            (measurementStart.x + measurementEnd.x) / 2 -
              measurementTooltipWidth / 2,
            left,
          ),
          width - right - measurementTooltipWidth,
        )
      : 0;
  const measurementTooltipY =
    measurementStart && measurementEnd
      ? Math.min(measurementStart.y, measurementEnd.y) >
        top + measurementTooltipHeight + 8
        ? Math.min(measurementStart.y, measurementEnd.y) -
          measurementTooltipHeight -
          8
        : Math.min(
            Math.max(measurementStart.y, measurementEnd.y) + 8,
            height - bottom - measurementTooltipHeight,
          )
      : 0;
  const measurementClass =
    (measurementDifference ?? 0) < 0 ? "negative" : "positive";
  return (
    <div className="wealth-chart">
      <svg
        viewBox={`0 0 ${width} ${height}`}
        role="img"
        aria-label={t("Verlauf des Gesamtvermögens")}
      >
        <defs>
          <linearGradient id="wealth-fill" x1="0" y1="0" x2="0" y2="1">
            <stop offset="0" stopColor="#059669" stopOpacity=".15" />
            <stop offset="1" stopColor="#059669" stopOpacity="0" />
          </linearGradient>
        </defs>
        {ticks.map((tick) => {
          const y = top + ((maximum - tick) / range) * (height - top - bottom);
          return (
            <g key={tick}>
              <line
                x1={left}
                y1={y}
                x2={width - right}
                y2={y}
                className="chart-grid"
              />
              <text
                x={left - 12}
                y={y + 4}
                textAnchor="end"
                className="chart-axis-label"
              >
                {compactMoney(tick, currency)}
              </text>
            </g>
          );
        })}
        {dateTicks.map((tick, index) => (
          <g key={tick.date}>
            <line
              x1={tick.x}
              y1={height - bottom}
              x2={tick.x}
              y2={height - bottom + 5}
              className="chart-axis-tick"
            />
            <text
              x={tick.x}
              y={height - bottom + 20}
              textAnchor={index === 0 ? "start" : index === 4 ? "end" : "middle"}
              className="chart-axis-label"
            >
              {shortDate(tick.date)}
            </text>
          </g>
        ))}
        <path d={area} fill="url(#wealth-fill)" />
        <path d={line} className="chart-line" />
        <circle cx={points[0].x} cy={points[0].y} r="4" className="chart-dot">
          <title>
            {shortDate(points[0].date)}: {money(points[0].totalMinor, currency)}
          </title>
        </circle>
        <circle
          cx={points[points.length - 1].x}
          cy={points[points.length - 1].y}
          r="4"
          className="chart-dot"
        >
          <title>
            {shortDate(points[points.length - 1].date)}:{" "}
            {money(points[points.length - 1].totalMinor, currency)}
          </title>
        </circle>
        {selectionStart && selectionEnd && (
          <g className="chart-selection" pointerEvents="none">
            <rect
              x={selectionStart.x}
              y={top}
              width={Math.max(selectionEnd.x - selectionStart.x, 1)}
              height={height - top - bottom}
            />
            <line x1={selectionStart.x} y1={top} x2={selectionStart.x} y2={height - bottom} />
            <line x1={selectionEnd.x} y1={top} x2={selectionEnd.x} y2={height - bottom} />
          </g>
        )}
        {measurementStart &&
          measurementEnd &&
          measurement?.startIndex !== measurement?.currentIndex && (
            <g
              className={`chart-measurement ${measurementClass}`}
              pointerEvents="none"
            >
              <line
                x1={measurementStart.x}
                y1={measurementStart.y}
                x2={measurementEnd.x}
                y2={measurementEnd.y}
                className="chart-measurement-line"
              />
              <line
                x1={measurementStart.x}
                y1={measurementStart.y}
                x2={measurementEnd.x}
                y2={measurementStart.y}
                className="chart-measurement-guide"
              />
              <line
                x1={measurementEnd.x}
                y1={measurementStart.y}
                x2={measurementEnd.x}
                y2={measurementEnd.y}
                className="chart-measurement-guide"
              />
              <circle cx={measurementStart.x} cy={measurementStart.y} r="5" />
              <circle cx={measurementEnd.x} cy={measurementEnd.y} r="5" />
              <rect
                x={measurementTooltipX}
                y={measurementTooltipY}
                width={measurementTooltipWidth}
                height={measurementTooltipHeight}
                rx="7"
                className="chart-measurement-tooltip"
              />
              <text
                x={measurementTooltipX + measurementTooltipWidth / 2}
                y={measurementTooltipY + 17}
                textAnchor="middle"
              >
                <tspan className="chart-measurement-percent">
                  {measurementPercent === null
                    ? "–"
                    : `${measurementPercent >= 0 ? "+" : ""}${measurementPercent.toFixed(2)} %`}
                </tspan>
                <tspan
                  x={measurementTooltipX + measurementTooltipWidth / 2}
                  dy="15"
                >
                  {signedMoney(measurementDifference, currency)}
                </tspan>
                <tspan
                  x={measurementTooltipX + measurementTooltipWidth / 2}
                  dy="14"
                  className="chart-measurement-dates"
                >
                  {shortDate(measurementStart.date)} → {shortDate(measurementEnd.date)}
                </tspan>
              </text>
            </g>
          )}
        {hoveredPoint && !measurement && (
          <g className="chart-tooltip" pointerEvents="none">
            <line x1={hoveredPoint.x} y1={top} x2={hoveredPoint.x} y2={height - bottom} className="chart-hover-guide" />
            <circle cx={hoveredPoint.x} cy={hoveredPoint.y} r="5" className="chart-dot" />
            <rect x={tooltipX} y={tooltipY} width={tooltipWidth} height="34" rx="6" />
            <text x={tooltipX + tooltipWidth / 2} y={tooltipY + 14} textAnchor="middle">
              <tspan>{shortDate(hoveredPoint.date)}</tspan>
              <tspan x={tooltipX + tooltipWidth / 2} dy="14" className="chart-tooltip-value">
                {money(hoveredPoint.totalMinor, currency)}
              </tspan>
            </text>
          </g>
        )}
        <rect
          x={left}
          y={top}
          width={width - left}
          height={height - top - bottom}
          className="chart-hover-area"
          onPointerDown={(event) => {
            if (event.button !== 0) return;
            const svg = event.currentTarget.ownerSVGElement;
            if (!svg) return;
            removeDragListeners.current?.();
            const pointerId = event.pointerId;
            const index = pointIndexAt(event.clientX, svg);
            dragStartIndex.current = index;
            dragStartClientX.current = event.clientX;
            activePointerId.current = pointerId;
            dragMode.current = event.shiftKey ? "measure" : "zoom";
            if (event.shiftKey) {
              setDragSelection(null);
              setMeasurement({ startIndex: index, currentIndex: index });
            } else {
              setMeasurement(null);
              setDragSelection({ startIndex: index, currentIndex: index });
            }
            setHoveredIndex(index);

            const removeListeners = () => {
              window.removeEventListener("pointermove", handlePointerMove, true);
              window.removeEventListener("pointerup", handlePointerUp, true);
              window.removeEventListener("pointercancel", handlePointerCancel, true);
              window.removeEventListener("blur", handleWindowBlur);
              if (removeDragListeners.current === removeListeners) {
                removeDragListeners.current = null;
              }
            };
            const cancelDrag = () => {
              const mode = dragMode.current;
              removeListeners();
              dragStartIndex.current = null;
              dragStartClientX.current = null;
              activePointerId.current = null;
              dragMode.current = null;
              setDragSelection(null);
              if (mode === "measure") setMeasurement(null);
            };
            const handlePointerMove = (pointerEvent: PointerEvent) => {
              if (pointerEvent.pointerId !== pointerId) return;
              const currentIndex = pointIndexAt(pointerEvent.clientX, svg);
              setHoveredIndex(currentIndex);
              if (dragStartIndex.current !== null) {
                if (dragMode.current === "measure") {
                  setMeasurement({
                    startIndex: dragStartIndex.current,
                    currentIndex,
                  });
                } else {
                  setDragSelection({
                    startIndex: dragStartIndex.current,
                    currentIndex,
                  });
                }
              }
              pointerEvent.preventDefault();
            };
            const handlePointerUp = (pointerEvent: PointerEvent) => {
              if (pointerEvent.pointerId !== pointerId) return;
              removeListeners();
              finishDragSelection(pointerEvent.clientX, svg);
            };
            const handlePointerCancel = (pointerEvent: PointerEvent) => {
              if (pointerEvent.pointerId === pointerId) cancelDrag();
            };
            const handleWindowBlur = () => cancelDrag();

            window.addEventListener("pointermove", handlePointerMove, true);
            window.addEventListener("pointerup", handlePointerUp, true);
            window.addEventListener("pointercancel", handlePointerCancel, true);
            window.addEventListener("blur", handleWindowBlur);
            removeDragListeners.current = removeListeners;
            event.preventDefault();
          }}
          onPointerMove={(event) => {
            if (activePointerId.current !== null) return;
            const svg = event.currentTarget.ownerSVGElement;
            if (!svg) return;
            const index = pointIndexAt(event.clientX, svg);
            setHoveredIndex(index);
          }}
          onPointerLeave={() => {
            if (activePointerId.current === null) setHoveredIndex(null);
          }}
        />
      </svg>
      <div className="chart-labels">
        <strong>{tr`${chartHistory.length} Tageswerte`}</strong>
        <small>{t("Zum Zoomen ziehen · Shift + Ziehen zum Messen")}</small>
      </div>
      <span className="sr-only" role="status" aria-live="polite">
        {measurementStart && measurementEnd && measurementDifference !== null
          ? `${measurementPercent === null ? "" : `${measurementPercent.toFixed(2)} % · `}${signedMoney(measurementDifference, currency)}`
          : ""}
      </span>
    </div>
  );
}

function axisScale(min: number, max: number) {
  const spread = Math.max(max - min, Math.abs(max) * 0.1, 100_000);
  const roughStep = spread / 4;
  const magnitude = 10 ** Math.floor(Math.log10(roughStep));
  const normalized = roughStep / magnitude;
  const step =
    (normalized <= 1 ? 1 : normalized <= 2 ? 2 : normalized <= 5 ? 5 : 10) *
    magnitude;
  const minimum = Math.floor((min - spread * 0.04) / step) * step;
  const maximum = Math.ceil((max + spread * 0.04) / step) * step;
  const ticks = Array.from(
    { length: Math.round((maximum - minimum) / step) + 1 },
    (_, index) => minimum + index * step,
  );
  return { minimum, maximum, ticks };
}

function stepPath(
  points: Array<HistoryPoint & { x: number; y: number }>,
): string {
  if (points.length === 1) return `M ${points[0].x} ${points[0].y}`;
  let path = `M ${points[0].x} ${points[0].y}`;
  for (let index = 1; index < points.length; index += 1) {
    const point = points[index];
    path += ` H ${point.x} V ${point.y}`;
  }
  return path;
}

function BreakdownCard({
  title,
  values,
  total,
  currency,
  translateTypes = false,
}: {
  translateTypes?: boolean;
  title: string;
  values: Breakdown[];
  total: number;
  currency: string;
}) {
  const scale = Math.max(
    ...values.map((value) => Math.abs(value.amountMinor)),
    1,
  );
  return (
    <article className="dashboard-card breakdown-card">
      <p className="eyebrow">{t("Aufteilung")}</p>
      <h2>{title}</h2>
      <div>
        {values.map((value) => (
          <div className="breakdown-row" key={value.key}>
            <div>
              <strong>{translateTypes ? t(value.label) : value.label}</strong>
              <span>{money(value.amountMinor, currency)}</span>
            </div>
            <div className="breakdown-bar">
              <span
                style={{
                  width: `${(Math.abs(value.amountMinor) / scale) * 100}%`,
                }}
              />
            </div>
            <small>
              {total
                ? `${((value.amountMinor / total) * 100).toFixed(1)} %`
                : "–"}{" "}
              · {value.accountCount}{" "}
              {value.accountCount === 1 ? t("Konto") : t("Konten")}
            </small>
          </div>
        ))}
      </div>
    </article>
  );
}

function money(value: number | null, currency: string) {
  return value === null
    ? "–"
    : new Intl.NumberFormat(locale(), { style: "currency", currency }).format(
        value / 100,
      );
}
function compactMoney(value: number, currency: string) {
  return new Intl.NumberFormat(locale(), {
    style: "currency",
    currency,
    notation: "compact",
    maximumFractionDigits: 0,
  }).format(value / 100);
}
function signedMoney(value: number | null, currency: string) {
  if (value === null) return "–";
  return `${value >= 0 ? "+" : ""}${money(value, currency)}`;
}
function shortDate(value: string) {
  const [year, month, day] = value.slice(0, 10).split("-");
  return year && month && day ? `${day}.${month}.${year}` : value;
}
function typeLabel(value: string) {
  return t(
    (
      {
        cash: "Konto",
        savings: "Sparkonto",
        portfolio: "Depot",
        pillar3a: "Säule 3a",
        mortgage: "Hypothek",
        manual_asset: "Manuell verwaltete Position",
      } as Record<string, string>
    )[value] ?? value,
  );
}
