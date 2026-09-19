// Normiert Vermögens- und Indexverläufe am gemeinsamen Start, ohne gespeicherte Werte zu verändern.
import type { DailyValue } from "../../shared/charts/timelineModel";
export interface BenchmarkData { name: string; currency: string; points: { date: string; close: number }[] }
export function compareBenchmark(history: DailyValue[], benchmark: BenchmarkData) {
  const main = [...history].sort((a, b) => a.date.localeCompare(b.date));
  const prices = [...benchmark.points].filter(p => Number.isFinite(p.close) && p.close > 0).sort((a, b) => a.date.localeCompare(b.date));
  const byDate = new Map(prices.map(p => [p.date, p.close]));
  // Do not skip a zero/negative first common balance: that would silently change the comparison period.
  const start = main.find(p => byDate.has(p.date));
  if (!start || !Number.isFinite(start.totalMinor) || start.totalMinor <= 0) return null;
  const end = [main[main.length - 1].date, prices[prices.length - 1].date].sort()[0];
  if (start.date >= end) return null;
  const within = (date: string) => date >= start.date && date <= end;
  return {
    from: start.date,
    main: main.filter(p => within(p.date)).map(p => ({ date: p.date, totalMinor: p.totalMinor / start.totalMinor * 10000 })),
    comparison: prices.filter(p => within(p.date)).map(p => ({ date: p.date, totalMinor: p.close / byDate.get(start.date)! * 10000 })),
  };
}
