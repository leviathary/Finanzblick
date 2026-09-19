// Leitet Kartenstatus und kontogebundene Tabellenfilter ab, ohne Beträge verschiedener Währungen zu summieren.
import type { Account, AccountRow, SettlementRule } from "./types";

export const ALL_CARDS = "ALL";
export type CardPeriod = "current" | "previous" | "custom" | "all";
export interface CardFilters { from?: string; to?: string; search?: string }
export type CardSortKey = "bookingDate" | "amountMinor" | "accountName" | "description";
export interface CardSort { key: CardSortKey; descending: boolean }

export function cardPeriodRange(period: CardPeriod, from: string, to: string, now = new Date()): CardFilters {
  if (period === "all") return {};
  if (period === "custom") return { from, to };
  const year = now.getFullYear() - (period === "previous" ? 1 : 0);
  return { from: `${year}-01-01`, to: `${year}-12-31` };
}

export function sortCardRows(rows: AccountRow[], sort: CardSort, language: string): AccountRow[] {
  return [...rows].sort((a, b) => {
    const comparison = sort.key === "amountMinor" ? Math.abs(a.amountMinor) - Math.abs(b.amountMinor)
      : a[sort.key].localeCompare(b[sort.key], language, { numeric: true, sensitivity: "base" });
    return (sort.descending ? -comparison : comparison) || b.bookingDate.localeCompare(a.bookingDate) || b.id - a.id;
  });
}

export function hasSettlementRule(account: Account, rules: SettlementRule[]): boolean {
  return rules.some(rule => rule.accountId === account.id && rule.currency === account.currency && rule.direction === 1);
}

export function selectCardRows(rows: AccountRow[], accountId: string, kind = "ALL", filters: CardFilters = {}): AccountRow[] {
  const search = filters.search?.trim().toLowerCase() ?? "";
  return rows.filter(row => (accountId === ALL_CARDS || String(row.accountId) === accountId) && (kind === "ALL" || row.kind === kind)
    && (!filters.from || row.bookingDate >= filters.from) && (!filters.to || row.bookingDate <= filters.to)
    && (!search || row.description.toLowerCase().includes(search) || row.accountName.toLowerCase().includes(search)))
    .sort((a, b) => b.bookingDate.localeCompare(a.bookingDate) || b.id - a.id);
}
