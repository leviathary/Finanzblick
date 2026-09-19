// Gemeinsame Kontotypen, Eingabeoptionen und Anzeigeformatierung.
import { t, locale } from "../../i18n";
export const accountTypes = [
  ["cash", "Privatkonto"],
  ["savings", "Sparkonto"],
  ["portfolio", "Depot"],
  ["pillar3a", "Vorsorgekonto"],
  ["mortgage", "Hypothek"],
  ["credit_card", "Kreditkarte"],
  ["manual_asset", "Manuell verwaltete Position"],
];
export function supportsManualValuation(accountType: string): boolean {
  return accountType === "manual_asset" || accountType === "pillar3a";
}

export function typeLabel(value: string) {
  return t(accountTypes.find(([key]) => key === value)?.[1] ?? value);
}
export function money(value: number | null, currency: string) {
  return value === null
    ? t("Noch kein Saldo")
    : new Intl.NumberFormat(locale(), { style: "currency", currency }).format(
        value / 100,
      );
}
