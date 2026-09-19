// Eingabeoptionen und Anzeigehelfer für Positionsbewertungen.
import { money } from "../accounts/presentation";
export const assetTypes = [
  ["stock", "Aktie"],
  ["option", "Option"],
  ["crypto", "Kryptowährung"],
  ["cash", "Cash / Geldbetrag"],
  ["fund", "Fonds / ETF"],
  ["bond", "Obligation"],
  ["other", "Sonstige Anlage"],
];
export const valuationCurrencies = [
  "CHF",
  "EUR",
  "USD",
  "AUD",
  "GBP",
  "CAD",
  "JPY",
  "NZD",
];

export function marketSourceLabel(source: string): string {
  return source === "alpha_vantage"
    ? "Alpha Vantage"
    : source === "marketstack"
      ? "Marketstack"
      : source === "yahoo"
        ? "Yahoo Finance"
        : source;
}

export function calculatedValuation(
  valuation: {
    quantity: string;
    unitPrice: string;
    exchangeRate: string;
    quoteCurrency: string;
  },
  currency: string,
) {
  const quantity = Number(valuation.quantity.replace(",", "."));
  const price = Number(valuation.unitPrice.replace(",", "."));
  const rate =
    valuation.quoteCurrency === currency
      ? 1
      : Number(valuation.exchangeRate.replace(",", "."));
  return valuation.quantity &&
    valuation.unitPrice &&
    Number.isFinite(quantity * price * rate)
    ? money(Math.round(quantity * price * rate * 100), currency)
    : "–";
}
