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

export type PositionQuantityChangeKind = "opening" | "purchase" | "sale" | "adjustment";
export interface PositionQuantityChange {
  id: number;
  date: string;
  kind: PositionQuantityChangeKind;
  change: number;
  balance: number;
}

export type AccountSection = "bank" | "cards" | "investments" | "pension";

function accountSection(accountType: Account["accountType"]): AccountSection {
  if (accountType === "credit_card") return "cards";
  if (accountType === "portfolio" || accountType === "manual_asset") return "investments";
  if (accountType === "pillar3a") return "pension";
  return "bank";
}

export function groupedAccounts<T extends Pick<Account, "accountType">>(accounts: T[]) {
  const groups = new Map<AccountSection, T[]>([
    ["bank", []],
    ["cards", []],
    ["investments", []],
    ["pension", []],
  ]);
  for (const account of accounts) groups.get(accountSection(account.accountType))?.push(account);
  return [...groups].filter(([, grouped]) => grouped.length > 0).map(([section, grouped]) => ({ section, accounts: grouped }));
}

export function usesPositionValuation(accountType: string, positionCount: number) {
  return accountType === "portfolio" || accountType === "manual_asset" || (accountType === "pillar3a" && positionCount > 0);
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
