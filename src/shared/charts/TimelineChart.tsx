// Zeichnet interaktive Zeitverläufe für datierte Geldbeträge.

import { useEffect, useId, useRef, useState } from "react";

import { locale, t, tr } from "../../i18n";

export interface TimelinePoint {
  date: string;
  totalMinor: number;
}

export function TimelineChart({
  history,
  currency,
  onSelectRange,
  ariaLabel,
  emptyLabel = t("Noch keine Saldo-Snapshots vorhanden."),
}: {
  history: TimelinePoint[];
  currency: string;
  onSelectRange: (from: string, to: string) => void;
  ariaLabel: string;
  emptyLabel?: string;
}) {
  const [hoveredIndex, setHoveredIndex] = useState<number | null>(null);
  const [dragSelection, setDragSelection] = useState<{
    startIndex: number;
    currentIndex: number;
  } | null>(null);
  const dragStartIndex = useRef<number | null>(null);
  const dragStartClientX = useRef<number | null>(null);
  const chartRef = useRef<HTMLDivElement>(null);
  const [chartWidth, setChartWidth] = useState(900);
  const gradientId = useId().replace(/:/g, "");
  const hasHistory = history.length > 0;
  useEffect(() => {
    const chart = chartRef.current;
    if (!chart) return;
    const updateWidth = (width: number) => setChartWidth(Math.max(520, Math.round(width)));
    updateWidth(chart.clientWidth);
    const observer = new ResizeObserver((entries) => updateWidth(entries[0]?.contentRect.width ?? chart.clientWidth));
    observer.observe(chart);
    return () => observer.disconnect();
  }, [hasHistory]);
  if (!history.length) return <div className="chart-empty">{emptyLabel}</div>;

  const width = chartWidth,
    height = 270,
    left = 92,
    right = 18,
    top = 16,
    bottom = 42;
  const values = history.map((point) => point.totalMinor);
  const { minimum, maximum, ticks } = axisScale(
    Math.min(...values),
    Math.max(...values),
  );
  const range = maximum - minimum;
  const startTime = Date.parse(history[0].date);
  const endTime = Date.parse(history[history.length - 1].date);
  const timeRange = Math.max(endTime - startTime, 1);
  const plotWidth = width - left - right;
  const points = history.map((point) => ({
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
    const dragDistance = Math.abs(clientX - (dragStartClientX.current ?? clientX));
    dragStartIndex.current = null;
    dragStartClientX.current = null;
    setDragSelection(null);
    if (startIndex === null || dragDistance < 5) return;
    const endIndex = pointIndexAt(clientX, svg);
    if (startIndex === endIndex) return;
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

  return (
    <div className="wealth-chart" ref={chartRef}>
      <svg viewBox={`0 0 ${width} ${height}`} role="img" aria-label={ariaLabel}>
        <defs>
          <linearGradient id={gradientId} x1="0" y1="0" x2="0" y2="1">
            <stop offset="0" stopColor="var(--accent-positive)" stopOpacity=".28" />
            <stop offset="1" stopColor="var(--accent-positive)" stopOpacity=".02" />
          </linearGradient>
        </defs>
        {ticks.map((tick) => {
          const y = top + ((maximum - tick) / range) * (height - top - bottom);
          return (
            <g key={tick}>
              <line x1={left} y1={y} x2={width - right} y2={y} className="chart-grid" />
              <text x={left - 12} y={y + 4} textAnchor="end" className="chart-axis-label">
                {compactMoney(tick, currency)}
              </text>
            </g>
          );
        })}
        {dateTicks.map((tick, index) => (
          <g key={tick.date}>
            <line x1={tick.x} y1={height - bottom} x2={tick.x} y2={height - bottom + 5} className="chart-axis-tick" />
            <text x={tick.x} y={height - bottom + 20} textAnchor={index === 0 ? "start" : index === 4 ? "end" : "middle"} className="chart-axis-label">
              {shortDate(tick.date)}
            </text>
          </g>
        ))}
        <path d={area} fill={`url(#${gradientId})`} />
        <path d={line} className="chart-line" />
        <circle cx={points[0].x} cy={points[0].y} r="4" className="chart-dot">
          <title>{shortDate(points[0].date)}: {money(points[0].totalMinor, currency)}</title>
        </circle>
        <circle cx={points[points.length - 1].x} cy={points[points.length - 1].y} r="4" className="chart-dot">
          <title>{shortDate(points[points.length - 1].date)}: {money(points[points.length - 1].totalMinor, currency)}</title>
        </circle>
        {selectionStart && selectionEnd && (
          <g className="chart-selection" pointerEvents="none">
            <rect x={selectionStart.x} y={top} width={Math.max(selectionEnd.x - selectionStart.x, 1)} height={height - top - bottom} />
            <line x1={selectionStart.x} y1={top} x2={selectionStart.x} y2={height - bottom} />
            <line x1={selectionEnd.x} y1={top} x2={selectionEnd.x} y2={height - bottom} />
          </g>
        )}
        {hoveredPoint && (
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
          onMouseDown={(event) => {
            if (event.button !== 0) return;
            const svg = event.currentTarget.ownerSVGElement;
            if (!svg) return;
            const index = pointIndexAt(event.clientX, svg);
            dragStartIndex.current = index;
            dragStartClientX.current = event.clientX;
            setDragSelection({ startIndex: index, currentIndex: index });
            setHoveredIndex(index);
            event.preventDefault();
          }}
          onMouseMove={(event) => {
            const svg = event.currentTarget.ownerSVGElement;
            if (!svg) return;
            const index = pointIndexAt(event.clientX, svg);
            setHoveredIndex(index);
            if (dragStartIndex.current !== null) {
              setDragSelection({ startIndex: dragStartIndex.current, currentIndex: index });
            }
          }}
          onMouseUp={(event) => {
            const svg = event.currentTarget.ownerSVGElement;
            if (svg) finishDragSelection(event.clientX, svg);
          }}
          onMouseLeave={() => {
            setHoveredIndex(null);
            dragStartIndex.current = null;
            dragStartClientX.current = null;
            setDragSelection(null);
          }}
        />
      </svg>
      <div className="chart-labels">
        <strong>{tr`${history.length} Tageswerte`}</strong>
        <small>{t("Zum Zoomen im Diagramm ziehen")}</small>
      </div>
    </div>
  );
}

function axisScale(min: number, max: number) {
  const spread = Math.max(max - min, Math.abs(max) * 0.1, 100_000);
  const roughStep = spread / 4;
  const magnitude = 10 ** Math.floor(Math.log10(roughStep));
  const normalized = roughStep / magnitude;
  const step = (normalized <= 1 ? 1 : normalized <= 2 ? 2 : normalized <= 5 ? 5 : 10) * magnitude;
  const minimum = Math.floor((min - spread * 0.04) / step) * step;
  const maximum = Math.ceil((max + spread * 0.04) / step) * step;
  const ticks = Array.from({ length: Math.round((maximum - minimum) / step) + 1 }, (_, index) => minimum + index * step);
  return { minimum, maximum, ticks };
}

function stepPath(points: Array<TimelinePoint & { x: number; y: number }>): string {
  if (points.length === 1) return `M ${points[0].x} ${points[0].y}`;
  let path = `M ${points[0].x} ${points[0].y}`;
  for (let index = 1; index < points.length; index += 1) {
    const point = points[index];
    path += ` H ${point.x} V ${point.y}`;
  }
  return path;
}

function money(value: number, currency: string) {
  return new Intl.NumberFormat(locale(), { style: "currency", currency }).format(value / 100);
}

function compactMoney(value: number, currency: string) {
  return new Intl.NumberFormat(locale(), {
    style: "currency",
    currency,
    notation: "compact",
    maximumFractionDigits: 0,
  }).format(value / 100);
}

function shortDate(value: string) {
  const [year, month, day] = value.slice(0, 10).split("-");
  return year && month && day ? `${day}.${month}.${year}` : value;
}
