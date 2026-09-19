// Leitet Kartenstatus und kontogebundene Tabellenfilter ab, ohne Beträge verschiedener Währungen zu summieren.
import type { Account, AccountRow, SettlementRule } from "./types";

export const ALL_CARDS = "ALL";

export function hasSettlementRule(account: Account, rules: SettlementRule[]): boolean {
  return rules.some(rule => rule.accountId === account.id && rule.currency === account.currency && rule.direction === 1);
}

export function selectCardRows(rows: AccountRow[], accountId: string, kind = "ALL"): AccountRow[] {
  return rows.filter(row => (accountId === ALL_CARDS || String(row.accountId) === accountId) && (kind === "ALL" || row.kind === kind))
    .sort((a, b) => b.bookingDate.localeCompare(a.bookingDate) || b.id - a.id);
}
