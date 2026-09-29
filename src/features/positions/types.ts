// DTOs für manuelle Positionen, Bewertungen und Kursaktualisierungen.
export interface ManualPosition {
  id: number;
  accountId: number;
  label: string;
  valuationDate: string;
  amountMinor: number;
  valueCurrency: string;
  quantity: number | null;
  unitPriceMinor: number | null;
  quoteCurrency: string | null;
  exchangeRate: number | null;
  assetType: string | null;
  identifierType: "isin" | "ticker" | null;
  identifier: string | null;
  priceSource: string | null;
  holdingStartDate: string | null;
  holdingEndDate: string | null;
  canDelete: boolean;
}

export interface ManualValuation {
  id: number;
  positionId: number;
  valueDate: string;
  amountMinor: number;
  currency: string;
}

export interface UpdateManualValuationRequest {
  id: number;
  positionId: number;
  valueDate: string;
  amountMinor: number;
}

export interface DeleteManualValuationRequest {
  id: number;
  positionId: number;
}

export interface PositionQuantityChangeRequest {
  positionId: number;
  effectiveDate: string;
  changeType: "buy" | "sell";
  quantity: string;
}

export interface MarketRefreshResult {
  updatedPositions: number;
  storedDays: number;
  skippedPositions: number;
  errors: string[];
}


export interface SaveValuationRequest {
  id: number | null;
  accountId: number;
  label: string;
  valuationDate: string;
  amountMinor: number;
  quantity: number | null;
  unitPriceMinor: number | null;
  quoteCurrency: string | null;
  exchangeRate: number | null;
  assetType: string;
  identifierType: "isin" | "ticker" | null;
  identifier: string | null;
  holdingStartDate: string;
  holdingEndDate: string | null;
}
