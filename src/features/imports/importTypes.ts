import type { ProviderId } from "./fileDetection";

export interface ParsedTransaction {
  bookingDate: string;
  valueDate: string | null;
  description: string;
  industry: string | null;
  amountMinor: number;
  balanceMinor: number | null;
  currency: string;
  confidence: number;
  sourceRow: number;
}

export interface ParsedStatement {
  provider: ProviderId;
  format: string;
  accountName: string;
  transactions: ParsedTransaction[];
  openingBalanceMinor: number | null;
  closingBalanceMinor: number | null;
  warnings: string[];
  currencyBalances: Array<{ currency: string; openingDate: string | null; openingBalanceMinor: number; closingBalanceMinor: number; closingDate: string }>;
  accountType: string | null;
}

export interface ImportAccount {
  id: number; name: string; provider: string; providerKey: string; currency: string; accountType: string; isActive: boolean;
  externalReference?: string | null;
}

export interface SaveImportResult {
  importId: number;
  accountId: number;
  insertedTransactions: number;
  duplicate: boolean;
}

export interface DatabaseStatus {
  path: string;
  accounts: number;
  imports: number;
  transactions: number;
}

