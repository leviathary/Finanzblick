// Reine Regeln für die Freigabe eines Setup-Entwurfs; kein React- oder Backend-Zustand.
import type { Preview, Row, SetupRule } from "../types";

export type BankPaymentSort = "newest" | "oldest" | "largest" | "smallest";

export function bankPaymentStart(today: Date, months: number): string {
  // Local calendar dates, clamped at month end (e.g. April 30 -> February 28).
  const end = new Date(today.getFullYear(), today.getMonth(), today.getDate());
  const start = new Date(end.getFullYear(), end.getMonth() - months, 1);
  const lastDay = new Date(start.getFullYear(), start.getMonth() + 1, 0).getDate();
  start.setDate(Math.min(end.getDate(), lastDay));
  return localDate(start);
}

function localDate(value: Date): string {
  return `${value.getFullYear()}-${String(value.getMonth() + 1).padStart(2, "0")}-${String(value.getDate()).padStart(2, "0")}`;
}

export function filterBankPayments(rows: Row[], query: string, sort: BankPaymentSort = "newest", today = new Date(), months = 2): Row[] {
  const from = bankPaymentStart(today, months), to = localDate(today);
  const term = query.trim().toLowerCase();
  const filtered = rows.filter(row => row.amountMinor < 0 && row.bookingDate >= from && row.bookingDate <= to && row.description.toLowerCase().includes(term));
  filtered.sort((a, b) => (sort === "newest" || sort === "oldest"
    ? (sort === "newest" ? -1 : 1) * a.bookingDate.localeCompare(b.bookingDate)
    : (sort === "largest" ? -1 : 1) * (Math.abs(a.amountMinor) - Math.abs(b.amountMinor)))
    || b.id - a.id);
  return filtered;
}

export function draft(row: Row | undefined, preview: Preview | null): SetupRule | null {
  return row && preview
    ? { transactionId: row.id, prefix: preview.prefix, expectedIds: preview.matches.map(match => match.id) }
    : null;
}

export function validPreview(
  row: Row | undefined,
  preview: { loading: boolean; data: Preview | null },
): boolean {
  return !row || (!preview.loading && !!preview.data?.matches.length && preview.data.matches.length <= 5000);
}
