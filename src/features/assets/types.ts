// Typisierte Vermögensprojektionen und historische Bewertungspunkte.
import type { AccountType } from "../../domain/finance";
export interface HistoryPoint {
  date: string;
  totalMinor: number;
}
export interface Breakdown {
  key: string;
  label: string;
  amountMinor: number;
  accountCount: number;
}
export interface AssetAccount {
  id: number;
  provider: string;
  providerKey: string;
  name: string;
  accountType: AccountType;
  currency: string;
  balanceMinor: number | null;
  balanceDate: string | null;
  logoDataUrl: string | null;
}
export interface WealthData {
  currency: string;
  currentTotalMinor: number;
  firstTotalMinor: number | null;
  changeMinor: number | null;
  history: HistoryPoint[];
  byType: Breakdown[];
  byProvider: Breakdown[];
  accounts: AssetAccount[];
}
