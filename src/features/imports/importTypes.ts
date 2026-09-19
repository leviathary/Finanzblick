// Definiert die gemeinsamen Frontend-Datentypen für Importvorschau, Zuordnung und Speicherung.
import type { AccountType } from "../../domain/finance";

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
  transactionKind: "cash_transaction" | "security_trade" | "dividend" | "interest" | "fee" | "corporate_action" | string;
  referenceNamespace: string | null;
  externalReference: string | null;
  counterpartyName: string | null;
  remittanceInformation: string | null;
  securityDetails: {
    isin: string | null;
    valorNumber: string | null;
    quantity: string | null;
    price: string | null;
    priceCurrency: string | null;
    exchangeRate: string | null;
    grossAmountMinor: number | null;
    feesMinor: number | null;
    taxesMinor: number | null;
    withholdingTaxMinor: number | null;
    accruedInterestMinor: number | null;
  } | null;
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
  accountType: AccountType | null;
  documentType: string | null;
  documentDate: string | null;
  documentValueDate: string | null;
  recordDefinitionId: string | null;
  accountReference: string | null;
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
  id: number; name: string; provider: string; providerKey: string; currency: string; accountType: AccountType; isActive: boolean;
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

