// Erstellt konsistente Formularentwürfe für neue und bestehende Depotpositionen.
import type { ManualPosition } from "./types";
import type { ManualPositionDraft } from "./ManualPositionEditor";

export function newPositionDraft(currency: string): ManualPositionDraft {
  const today = new Date().toISOString().slice(0, 10);
  return {
    id: null,
    label: "",
    amount: "",
    quantity: "",
    unitPrice: "",
    quoteCurrency: currency,
    exchangeRate: "1",
    date: today,
    assetType: "other",
    identifierType: "ticker",
    isin: "",
    ticker: "",
    method: "total",
    holdingStartDate: today,
    holdingEndDate: "",
  };
}

export function existingPositionDraft(position: ManualPosition, currency: string): ManualPositionDraft {
  return {
    id: position.id,
    label: position.label,
    amount: String(position.amountMinor / 100),
    quantity: position.quantity === null ? "" : String(position.quantity),
    unitPrice: position.unitPriceMinor === null ? "" : String(position.unitPriceMinor / 100),
    quoteCurrency: position.quoteCurrency ?? currency,
    exchangeRate: String(position.exchangeRate ?? 1),
    method: position.quantity !== null ? "units" : "total",
    assetType: position.assetType ?? "other",
    identifierType: position.identifierType ?? "ticker",
    isin: position.identifierType === "isin" ? (position.identifier ?? "") : "",
    ticker: position.identifierType === "ticker" ? (position.identifier ?? "") : "",
    date: position.valuationDate,
    holdingStartDate: position.holdingStartDate ?? position.valuationDate,
    holdingEndDate: position.holdingEndDate ?? "",
  };
}
