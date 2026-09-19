// Bereitet unveränderte Tageswerte vor und berechnet reine Anzeige-/Messwerte.
export interface DailyValue { date: string; totalMinor: number }
export function prepareTimeline(history: DailyValue[]) {
  const points = [...history].sort((a, b) => a.date.localeCompare(b.date));
  for (let index = 0; index < points.length; index++) {
    const point = points[index], date = new Date(`${point.date}T00:00:00Z`);
    if (!/^\d{4}-\d{2}-\d{2}$/.test(point.date) || !Number.isFinite(date.getTime()) || date.toISOString().slice(0, 10) !== point.date || !Number.isFinite(point.totalMinor) || points[index - 1]?.date === point.date) throw new Error("Invalid or duplicate timeline point");
  }
  // Whitespace preserves calendar gaps without inventing balances.
  const data: Array<{ time: string; value?: number }> = [];
  for (let index = 0; index < points.length; index++) {
    const point = points[index];
    if (index) for (let day = Date.parse(points[index - 1].date) + 86400000; day < Date.parse(point.date); day += 86400000) data.push({ time: new Date(day).toISOString().slice(0, 10) });
    data.push({ time: point.date, value: point.totalMinor / 100 });
  }
  return { points, data };
}
export function measureTimeline(start: DailyValue, end: DailyValue) {
  const difference = end.totalMinor - start.totalMinor;
  return { difference, percent: start.totalMinor === 0 ? null : difference / Math.abs(start.totalMinor) * 100, days: Math.round(Math.abs(Date.parse(end.date) - Date.parse(start.date)) / 86400000) };
}
export function monthsBefore(date: string, months: number) {
  const end = new Date(`${date}T00:00:00Z`), day = end.getUTCDate();
  end.setUTCDate(1); end.setUTCMonth(end.getUTCMonth() - months);
  const last = new Date(Date.UTC(end.getUTCFullYear(), end.getUTCMonth() + 1, 0)).getUTCDate();
  end.setUTCDate(Math.min(day, last));
  return end.toISOString().slice(0, 10);
}
export function zoomRange(range: { from: number; to: number }, factor: number) {
  const middle = (range.from + range.to) / 2, half = Math.max(1, (range.to - range.from) * factor / 2);
  return { from: middle - half, to: middle + half };
}
