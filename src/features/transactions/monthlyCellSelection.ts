// Berechnet rechteckige Zellbereiche und deren Summe für die Monatsvergleichstabelle.
export interface MonthlyCell {
  row: number;
  month: number;
}

export interface MonthlyCellSelection {
  anchor: MonthlyCell;
  focus: MonthlyCell;
}

export function monthlySelectionContains(selections: readonly MonthlyCellSelection[], row: number, month: number) {
  return selections.some(selection => {
    const firstRow = Math.min(selection.anchor.row, selection.focus.row);
    const lastRow = Math.max(selection.anchor.row, selection.focus.row);
    const firstMonth = Math.min(selection.anchor.month, selection.focus.month);
    const lastMonth = Math.max(selection.anchor.month, selection.focus.month);
    return row >= firstRow && row <= lastRow && month >= firstMonth && month <= lastMonth;
  });
}

export function summarizeMonthlySelection(selections: readonly MonthlyCellSelection[], rows: number[][]) {
  let count = 0;
  let total = 0;
  rows.forEach((months, row) => months.forEach((amount, month) => {
    if (amount !== 0 && monthlySelectionContains(selections, row, month)) {
      count += 1;
      total += amount;
    }
  }));
  return { count, total };
}
