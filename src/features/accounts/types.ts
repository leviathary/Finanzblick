// DTO für verwaltete Konten und deren aktuelle Bewertungsübersicht.
import type { AccountType } from "../../domain/finance";
export interface Account {
  id: number;
  institutionId: number;
  provider: string;
  providerKey: string;
  institutionType: string;
  name: string;
  accountType: AccountType;
  currency: string;
  externalReference: string | null;
  isActive: boolean;
  includeInNetWorth: boolean;
  balanceMinor: number | null;
  balanceDate: string | null;
  balanceCurrency: string;
  importCount: number;
  manualValuationCount: number;
  manualQuantity: number | null;
  manualUnitPriceMinor: number | null;
  manualQuoteCurrency: string | null;
  manualExchangeRate: number | null;
  logoDataUrl: string | null;
}
