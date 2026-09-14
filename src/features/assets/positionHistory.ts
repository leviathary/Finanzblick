export interface ChartPoint { date: string; totalMinor: number }
export interface PositionChart {
  id: number;
  label: string;
  holdingEndDate: string | null;
  history: ChartPoint[];
  prices: (ChartPoint & { currency: string })[];
}

export function sumPositionHistory(positions: PositionChart[]): ChartPoint[] {
  const changes = new Map<string, Map<number, number>>();
  for (const position of positions) {
    for (const point of position.history) {
      const day = changes.get(point.date) ?? new Map<number, number>();
      day.set(position.id, point.totalMinor);
      changes.set(point.date, day);
    }
  }
  const dates = [...changes.keys()].sort();
  if (!dates.length) return [];
  const latest = new Map<number, number>();
  const result: ChartPoint[] = [];
  const day = new Date(`${dates[0]}T00:00:00Z`);
  const end = dates[dates.length - 1];
  while (day.toISOString().slice(0, 10) <= end) {
    const date = day.toISOString().slice(0, 10);
    for (const [id, value] of changes.get(date) ?? []) latest.set(id, value);
    result.push({ date, totalMinor: [...latest.values()].reduce((sum, value) => sum + value, 0) });
    day.setUTCDate(day.getUTCDate() + 1);
  }
  return result;
}
