// Rendert lokale Zeitreihen mit Lightweight Charts; Navigation und Messung ändern keine Auswertungsfilter.
import { createPortal } from "react-dom";
import { useEffect, useId, useMemo, useRef, useState } from "react";
import { AreaSeries, LineSeries, ColorType, CrosshairMode, LineType, createChart, type IChartApi, type ISeriesApi, type Time } from "lightweight-charts";
import { isTauri } from "@tauri-apps/api/core";
import { openUrl } from "@tauri-apps/plugin-opener";
import { locale, t, tr } from "../../i18n";
import { measureTimeline, prepareTimeline, zoomRange, type DailyValue } from "./timelineModel";
import "./interactiveTimeline.css";

function timeDate(time: Time) {
  return typeof time === "string" ? time : typeof time === "number" ? new Date(time * 1000).toISOString().slice(0, 10) : `${time.year}-${String(time.month).padStart(2, "0")}-${String(time.day).padStart(2, "0")}`;
}
export function InteractiveTimelineChart({ history, currency, controlsContainer, indexed = false, comparison }: { history: DailyValue[]; currency: string; controlsContainer: HTMLElement | null; indexed?: boolean; comparison?: DailyValue[] }) {
  const comparisonSignature = JSON.stringify(comparison ?? []);
  const signature = JSON.stringify(history);
  const model = useMemo(() => {
    try { return prepareTimeline(JSON.parse(signature) as DailyValue[]); } catch { return null; }
  }, [signature]);
  const chartRef = useRef<IChartApi | null>(null);
  const seriesRef = useRef<ISeriesApi<"Area"> | null>(null);
  const hostRef = useRef<HTMLDivElement>(null);
  const modeRef = useRef(false);
  const indexRef = useRef(0);
  const anchorRef = useRef<number | null>(null);
  const stopDragRef = useRef<(() => void) | null>(null);
  const [measuring, setMeasuring] = useState(false);
  const [hovered, setHovered] = useState<DailyValue | null>(null);
  const [measurement, setMeasurement] = useState<{ start: number; end: number } | null>(null);
  const [geometry, setGeometry] = useState<{ x1: number; y1: number; x2: number; y2: number; tooltipX: number; tooltipY: number; visible: boolean } | null>(null);
  const tooltipRef = useRef<HTMLDivElement>(null);
  const [linkError, setLinkError] = useState(false);
  const helpId = useId();
  const region = locale();
  const money = (minor: number) => indexed ? new Intl.NumberFormat(region, { maximumFractionDigits: 2 }).format(minor / 100) + " " + t("Punkte") : new Intl.NumberFormat(region, { style: "currency", currency }).format(minor / 100);
  const date = (value: string) => new Intl.DateTimeFormat(region, { dateStyle: "medium", timeZone: "UTC" }).format(new Date(`${value}T00:00:00Z`));

  useEffect(() => {
    const host = hostRef.current;
    if (!host || !model?.points.length) return;
    const styles = getComputedStyle(host), color = (name: string) => styles.getPropertyValue(name).trim();
    const chart = createChart(host, {
      autoSize: true,
      layout: { background: { type: ColorType.Solid, color: color("--bg-surface") }, textColor: color("--text-secondary"), fontFamily: styles.fontFamily, fontSize: 12, attributionLogo: false },
      grid: { vertLines: { visible: false }, horzLines: { color: color("--border-subtle") } },
      rightPriceScale: { borderVisible: false, scaleMargins: { top: 0.12, bottom: 0.12 } },
      timeScale: { borderVisible: false, minBarSpacing: 0.01, timeVisible: false, rightOffset: 0, fixLeftEdge: true, fixRightEdge: true, lockVisibleTimeRangeOnResize: true },
      crosshair: { mode: CrosshairMode.Normal, vertLine: { labelBackgroundColor: color("--color-primary") }, horzLine: { labelBackgroundColor: color("--color-primary") } },
      localization: { locale: region, priceFormatter: (value: number) => money(value * 100), timeFormatter: (time: Time) => date(timeDate(time)) },
      handleScroll: { mouseWheel: false, pressedMouseMove: true, horzTouchDrag: true, vertTouchDrag: false },
      handleScale: { mouseWheel: true, pinch: true, axisPressedMouseMove: true, axisDoubleClickReset: true },
    });
    const series = chart.addSeries(AreaSeries, {
      lineColor: color("--accent-positive"), topColor: color("--accent-positive") + "47", bottomColor: color("--accent-positive") + "00",
      lineWidth: 2, lineType: LineType.Simple, priceLineVisible: false, lastValueVisible: false,
      pointMarkersVisible: model.points.length === 1, pointMarkersRadius: 4,
    });
    series.setData(model.data);
    const comparisonData = JSON.parse(comparisonSignature) as DailyValue[];
    const comparisonSeries = comparisonData.length ? chart.addSeries(LineSeries, { color: color("--chart-blue"), lineWidth: 2, priceLineVisible: false, lastValueVisible: false }) : null;
    comparisonSeries?.setData(prepareTimeline(comparisonData).data);
    const updateAppearance = () => {
      const palette = getComputedStyle(host), value = (name: string) => palette.getPropertyValue(name).trim();
      chart.applyOptions({
        layout: { background: { type: ColorType.Solid, color: value("--bg-surface") }, textColor: value("--text-secondary") },
        grid: { horzLines: { color: value("--border-subtle") } },
        crosshair: { vertLine: { labelBackgroundColor: value("--color-primary") }, horzLine: { labelBackgroundColor: value("--color-primary") } },
      });
      series.applyOptions({ lineColor: value("--accent-positive"), topColor: value("--accent-positive") + "47", bottomColor: value("--accent-positive") + "00" });
      comparisonSeries?.applyOptions({ color: value("--chart-blue") });
    };
    window.addEventListener("appearance-changed", updateAppearance);
    chartRef.current = chart; seriesRef.current = series;
    indexRef.current = model.points.length - 1; anchorRef.current = null;
    setHovered(null); setMeasurement(null); setGeometry(null);
    chart.timeScale().fitContent();
    chart.subscribeCrosshairMove(event => {
      if (!event.time || !event.point || event.point.x < 0 || event.point.y < 0) { setHovered(null); return; }
      const index = model.points.findIndex(point => point.date === timeDate(event.time!));
      if (index >= 0) { indexRef.current = index; setHovered(model.points[index]); } else setHovered(null);
    });
    // Capture Shift-drag before the chart's own mouse handler starts panning.
    const indexAt = (event: MouseEvent) => {
      const bounds = host.getBoundingClientRect(), x = (event.clientX - bounds.left) * host.clientWidth / bounds.width;
      let nearest = 0, distance = Infinity;
      model.points.forEach((point, index) => {
        const coordinate = chart.timeScale().timeToCoordinate(point.date);
        if (coordinate !== null && Math.abs(coordinate - x) < distance) { nearest = index; distance = Math.abs(coordinate - x); }
      });
      return nearest;
    };
    const mouseDown = (event: MouseEvent) => {
      if (event.button !== 0 || !(event.shiftKey || modeRef.current)) return;
      event.preventDefault(); event.stopPropagation(); host.focus(); stopDragRef.current?.();
      const start = indexAt(event), startX = event.clientX, startY = event.clientY;
      let moved = false;
      const move = (next: MouseEvent) => {
        next.preventDefault(); next.stopPropagation();
        if (Math.abs(next.clientX - startX) + Math.abs(next.clientY - startY) > 4) moved = true;
        setMeasurement({ start, end: indexAt(next) });
      };
      const clear = () => { window.removeEventListener("mousemove", move, true); window.removeEventListener("mouseup", up, true); window.removeEventListener("blur", cancel); stopDragRef.current = null; };
      const cancel = () => { clear(); anchorRef.current = null; setMeasurement(null); };
      const up = (next: MouseEvent) => {
        next.preventDefault(); next.stopPropagation(); clear();
        if (moved) { setMeasurement({ start, end: indexAt(next) }); anchorRef.current = null; }
        else if (anchorRef.current === null) { anchorRef.current = start; setMeasurement({ start, end: start }); }
        else { setMeasurement({ start: anchorRef.current, end: start }); anchorRef.current = null; }
      };
      window.addEventListener("mousemove", move, true); window.addEventListener("mouseup", up, true); window.addEventListener("blur", cancel);
      stopDragRef.current = clear;
    };
    host.addEventListener("mousedown", mouseDown, true);
    return () => { window.removeEventListener("appearance-changed", updateAppearance); stopDragRef.current?.(); host.removeEventListener("mousedown", mouseDown, true); chart.remove(); chartRef.current = null; seriesRef.current = null; };
  }, [model, currency, region, comparisonSignature, indexed]);

  useEffect(() => {
    const chart = chartRef.current, series = seriesRef.current;
    if (!chart || !series || !model || !measurement) { setGeometry(null); return; }
    const refresh = () => {
      const a = model.points[measurement.start], b = model.points[measurement.end];
      if (!a || !b) { setGeometry(null); return; }
      const x1 = chart.timeScale().timeToCoordinate(a.date), x2 = chart.timeScale().timeToCoordinate(b.date);
      const y1 = series.priceToCoordinate(a.totalMinor / 100), y2 = series.priceToCoordinate(b.totalMinor / 100);
      const width = hostRef.current?.clientWidth ?? 0, height = hostRef.current?.clientHeight ?? 0;
      const tooltipWidth = tooltipRef.current?.offsetWidth ?? 280, tooltipHeight = tooltipRef.current?.offsetHeight ?? 32;
      const next = x1 !== null && x2 !== null && y1 !== null && y2 !== null ? {
        x1, x2, y1, y2,
        tooltipX: Math.max(8, Math.min(x2 + 12, width - tooltipWidth - 8)),
        tooltipY: Math.max(8, Math.min(y2 - tooltipHeight - 12, height - tooltipHeight - 8)),
        visible: x2 >= -1 && x2 <= chart.timeScale().width() + 1 && y2 >= -1 && y2 <= height - chart.timeScale().height() + 1,
      } : null;
      setGeometry(previous => JSON.stringify(previous) === JSON.stringify(next) ? previous : next);
    };
    // Follows vertical axis scaling too, for which there is no range-change event.
    let frame = 0;
    const tick = () => { refresh(); frame = requestAnimationFrame(tick); };
    tick(); return () => cancelAnimationFrame(frame);
  }, [measurement, model, currency, region]);

  const reset = () => { stopDragRef.current?.(); anchorRef.current = null; setMeasurement(null); chartRef.current?.priceScale("right").applyOptions({ autoScale: true }); chartRef.current?.timeScale().fitContent(); };
  const zoom = (factor: number) => { const scale = chartRef.current?.timeScale(), range = scale?.getVisibleLogicalRange(); if (range) scale?.setVisibleLogicalRange(zoomRange(range, factor)); };
  const start = measurement && model?.points[measurement.start], end = measurement && model?.points[measurement.end];
  const result = start && end ? measureTimeline(start, end) : null;
  if (!model) return <p className="error-message" role="alert">{t("Die Chartdaten enthalten ungültige oder doppelte Tageswerte.")}</p>;
  if (!model.points.length) return <div className="chart-empty">{t("Noch keine Saldo-Snapshots vorhanden.")}</div>;
  return <div className="interactive-timeline">
    {controlsContainer && createPortal(<>
      <button type="button" className="wealth-measure-toggle" title={t("Messen")} aria-label={t("Messen")} aria-pressed={measuring} onClick={() => { modeRef.current = !measuring; setMeasuring(!measuring); anchorRef.current = null; setMeasurement(null); }}>
        <svg width="16" height="16" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="1.5" aria-hidden="true"><path d="m3 16 13-13 5 5L8 21Z M7 12l2 2m2-6 2 2m2-6 2 2"/></svg>
      </button>
      <button type="button" className="wealth-chart-reset" title={t("Ansicht zurücksetzen")} aria-label={t("Ansicht zurücksetzen")} onClick={reset}>
        <svg width="16" height="16" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="1.7" aria-hidden="true"><path d="M3 10a9 9 0 1 1 2 8M3 4v6h6"/></svg>
      </button>
    </>, controlsContainer)}
    <div className="interactive-timeline-stage">
      <div className="interactive-timeline-canvas" ref={hostRef} tabIndex={0} role="group" aria-label={t("Interaktiver Vermögensverlauf")} aria-describedby={helpId} onKeyDown={event => {
        if (["ArrowLeft", "ArrowRight", "Home", "End"].includes(event.key)) {
          event.preventDefault();
          const index = event.key === "Home" ? 0 : event.key === "End" ? model.points.length - 1 : Math.max(0, Math.min(model.points.length - 1, indexRef.current + (event.key === "ArrowLeft" ? -1 : 1)));
          indexRef.current = index;
          const point = model.points[index]; setHovered(point);
          const scale = chartRef.current?.timeScale(), logical = scale?.timeToIndex(point.date), range = scale?.getVisibleLogicalRange();
          if (logical !== null && logical !== undefined && range && (logical < range.from || logical > range.to)) { const half = (range.to - range.from) / 2; scale?.setVisibleLogicalRange({ from: logical - half, to: logical + half }); }
          seriesRef.current && chartRef.current?.setCrosshairPosition(point.totalMinor / 100, point.date, seriesRef.current);
        } else if (event.key === "Enter" && measuring) {
          event.preventDefault(); const index = indexRef.current;
          if (anchorRef.current === null) { anchorRef.current = index; setMeasurement({ start: index, end: index }); }
          else { setMeasurement({ start: anchorRef.current, end: index }); anchorRef.current = null; }
        } else if (event.key === "Escape") { stopDragRef.current?.(); anchorRef.current = null; setMeasurement(null); }
        else if (event.key === "+" || event.key === "=") { event.preventDefault(); zoom(0.7); }
        else if (event.key === "-") { event.preventDefault(); zoom(1.4); }
      }} />
      {geometry && <svg className="interactive-timeline-measure" aria-hidden="true"><line x1={geometry.x1} y1={geometry.y1} x2={geometry.x2} y2={geometry.y2}/><circle cx={geometry.x1} cy={geometry.y1} r="4"/><circle cx={geometry.x2} cy={geometry.y2} r="4"/></svg>}
      {result && geometry && <div ref={tooltipRef} className="interactive-timeline-tooltip" role="status" style={{ left: geometry.tooltipX, top: geometry.tooltipY, visibility: geometry.visible ? "visible" : "hidden" }}>
        {new Intl.NumberFormat(region, { minimumFractionDigits: 2, maximumFractionDigits: 2, signDisplay: "exceptZero" }).format(result.difference / 100)} {indexed ? t("Punkte") : currency} ({result.percent === null ? "–" : new Intl.NumberFormat(region, { maximumFractionDigits: 2, signDisplay: "exceptZero" }).format(result.percent) + "%"}) · {tr`${result.days} Tage`}
      </div>}
    </div>
    <p className="sr-only" id={helpId}>{t("Mausrad zum Zoomen · Ziehen zum Verschieben · Shift + Ziehen zum Messen. Tastatur: Pfeile für Tageswerte, +/− zum Zoomen.")} {t("Zwei Punkte anklicken oder mit Shift ziehen. Tastatur: Pfeiltasten, dann Enter für Start und Ende. Escape löscht die Messung.")}</p>
    <span className="sr-only" role="status">{hovered ? `${date(hovered.date)} · ${money(hovered.totalMinor)}` : ""}</span>
    <div className="interactive-timeline-attribution">
      <a href="https://www.tradingview.com/" target="_blank" rel="noreferrer" onClick={event => { if (isTauri()) { event.preventDefault(); void openUrl("https://www.tradingview.com/").catch(() => setLinkError(true)); } }}>Powered by TradingView Lightweight Charts™</a>
      <details><summary aria-label={t("Lizenzhinweis")} title={t("Lizenzhinweis")}>ⓘ</summary><div>TradingView Lightweight Charts™<br/>Copyright (с) 2025 TradingView, Inc. https://www.tradingview.com/</div></details>
    </div>
    {linkError && <p role="alert" className="error-message">{t("Der Link konnte nicht geöffnet werden.")}</p>}
  </div>;
}
