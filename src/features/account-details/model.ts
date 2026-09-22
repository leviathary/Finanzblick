// Typen und reine Anzeigeableitungen für lesende Konto- und Depotdetails.
import type { Account } from "../accounts/types";
import type { HistoryPoint } from "../assets/types";
export interface PositionDetail {
  id: number; label: string; symbol: string | null; quantity: number | null;
  priceMinor: number | null; priceCurrency: string | null;
  valueMinor: number | null; valueCurrency: string | null; valuationDate: string | null;
  start: string; end: string | null;
}
export interface AccountDetails {
  account: Account; positions: PositionDetail[]; history: HistoryPoint[]; historyCurrency: string;
  transactions: { id: number; date: string; description: string; amountMinor: number; currency: string }[];
  hasMore: boolean;
}
export function localToday() {
  const now = new Date();
  return `${now.getFullYear()}-${String(now.getMonth()+1).padStart(2,"0")}-${String(now.getDate()).padStart(2,"0")}`;
}
export function isCurrent(position: PositionDetail, today = localToday()) {
  return position.start <= today && (!position.end || position.end >= today) && position.quantity !== 0;
}
export function positionShares(positions: PositionDetail[], today = localToday()) {
  const active = positions.filter(p => isCurrent(p, today));
  const currencies = new Set(active.map(p => p.valueCurrency));
  const total = active.reduce((sum, p) => sum + (p.valueMinor ?? 0), 0);
  if (currencies.size !== 1 || active.some(p => p.valueMinor === null || !p.valueCurrency || p.valueMinor < 0) || total <= 0) return new Map<number, number>();
  return new Map(active.map(p => [p.id, (p.valueMinor ?? 0) / total]));
}

export function periodHistory(history: HistoryPoint[], from: string, today: string) {
  const known = history.filter(p => p.date <= today);
  const result = known.filter(p => p.date >= from);
  const before = known.filter(p => p.date < from);
  const previous = before[before.length - 1];
  if (previous && from <= today && result[0]?.date !== from) result.unshift({ ...previous, date: from });
  const latest = result[result.length - 1];
  if (latest && latest.date < today) result.push({ ...latest, date: today });
  return result;
}
