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

export type DateFormat = "auto" | "dmy" | "mdy" | "ymd";
export type NumberFormat = "auto" | "decimal-comma" | "decimal-point";

export interface TabularMapping {
  sheetName: string;
  headerRow: number;
  dataStartRow: number;
  dateColumn: number;
  descriptionColumns: number[];
  amountColumn: number | null;
  debitColumn: number | null;
  creditColumn: number | null;
  valueDateColumn: number | null;
  balanceColumn: number | null;
  currencyColumn: number | null;
  industryColumn: number | null;
  fixedCurrency: string;
  invertAmount: boolean;
  dateFormat: DateFormat;
  numberFormat: NumberFormat;
}

export interface SheetInspection {
  name: string;
  rowCount: number;
  columnCount: number;
  suggestedHeaderRow: number;
  headerDetected: boolean;
  headerScore: number;
  preview: string[][];
}

export interface TabularInspection { sheets: SheetInspection[]; }

export interface ImportMappingProfile {
  id: number;
  name: string;
  headerFingerprint: string;
  mapping: TabularMapping;
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

