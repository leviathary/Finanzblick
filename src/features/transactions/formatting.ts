// Formatiert Beträge, Datumswerte und Monatsnamen für die Buchungsanalyse.

import { locale } from "../../i18n";

export function money(value: number) {
  return new Intl.NumberFormat(locale(), { style: "currency", currency: "CHF" }).format(value / 100);
}

export function amountNumber(value: number) {
  return new Intl.NumberFormat(locale(), { minimumFractionDigits: 2, maximumFractionDigits: 2 }).format(value / 100);
}

export function shortDate(value: string) {
  const [year, month, day] = value.slice(0, 10).split("-");
  return `${day}.${month}.${year}`;
}

export function monthName(month: number) {
  return new Intl.DateTimeFormat(locale(), { month: "short" }).format(new Date(2020, month, 1));
}

export function monthNameLong(month: number) {
  return new Intl.DateTimeFormat(locale(), { month: "long" }).format(new Date(2020, month, 1));
}
