// Formatiert Vermögenswerte, Kontobezeichnungen und Datumsangaben.
import { t, locale } from "../../i18n";
import type { AssetAccount } from "./types";
export function accountLabel(account: AssetAccount | undefined): string {
  return account ? `${account.provider} · ${account.name}` : "–";
}

export function money(value: number | null, currency: string) {
  return value === null
    ? "–"
    : new Intl.NumberFormat(locale(), { style: "currency", currency }).format(
        value / 100,
      );
}
export function compactMoney(value: number, currency: string) {
  return new Intl.NumberFormat(locale(), {
    style: "currency",
    currency,
    notation: "compact",
    maximumFractionDigits: 0,
  }).format(value / 100);
}
export function signedMoney(value: number | null, currency: string) {
  if (value === null) return "–";
  return `${value >= 0 ? "+" : ""}${money(value, currency)}`;
}
export function shortDate(value: string) {
  const [year, month, day] = value.slice(0, 10).split("-");
  return year && month && day ? `${day}.${month}.${year}` : value;
}
export function typeLabel(value: string) {
  return t(
    (
      {
        cash: "Konto",
        checking: "Privatkonto",
        savings: "Sparkonto",
        portfolio: "Depot",
        pillar3a: "Vorsorgekonto",
        mortgage: "Hypothek",
        credit_card: "Kreditkarte",
        manual_asset: "Manuell verwaltete Position",
      } as Record<string, string>
    )[value] ?? value,
  );
}
