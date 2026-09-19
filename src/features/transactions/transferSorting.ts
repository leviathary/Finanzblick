// Sortiert neutrale Buchungen für die Anzeige, ohne Daten oder Auswahl zu verändern.
export type TransferSortKey = "bookingDate" | "accountName" | "description" | "amountMinor" | "transferType";
export interface TransferSort { key: TransferSortKey; descending: boolean }
interface SortableTransfer {
  id: number; bookingDate: string; accountName: string; description: string;
  amountMinor: number; transferType: string;
}

export function nextTransferSort(current: TransferSort, key: TransferSortKey): TransferSort {
  return { key, descending: current.key === key ? !current.descending : key === "bookingDate" || key === "amountMinor" };
}

export function sortTransfers<T extends SortableTransfer>(rows: readonly T[], sort: TransferSort, language: string, typeLabel: (type: string) => string): T[] {
  const compareText = new Intl.Collator(language, { numeric: true, sensitivity: "base" }).compare;
  return [...rows].sort((a, b) => {
    const comparison = sort.key === "amountMinor" ? Math.abs(a.amountMinor) - Math.abs(b.amountMinor)
      : sort.key === "transferType" ? compareText(typeLabel(a.transferType), typeLabel(b.transferType))
      : compareText(a[sort.key], b[sort.key]);
    return (sort.descending ? -comparison : comparison) || b.bookingDate.localeCompare(a.bookingDate) || b.id - a.id;
  });
}
