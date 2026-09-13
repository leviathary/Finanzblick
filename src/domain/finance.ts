export type AccountType = "cash" | "savings" | "portfolio" | "pillar3a" | "mortgage";
export type InstitutionType = "bank" | "insurance" | "broker" | "pension";

export interface FinancialInstitution {
  id: string;
  name: string;
  type: InstitutionType;
}

export interface Account {
  id: string;
  institutionId: string;
  name: string;
  type: AccountType;
  currency: string;
  externalReference?: string;
}

export interface Transaction {
  id: string;
  accountId: string;
  bookingDate: string;
  amountMinor: number;
  currency: string;
  description: string;
  categoryId: string | null;
  importId: string;
}

export interface BalanceSnapshot {
  id: string;
  accountId: string;
  capturedAt: string;
  amountMinor: number;
  currency: string;
}
