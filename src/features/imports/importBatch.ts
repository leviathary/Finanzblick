import type { ImportAccount, ParsedStatement, SaveImportResult } from "./importTypes";
import type { SelectedStatement } from "./fileDetection";

export interface BatchItem {
  file: SelectedStatement & { path: string };
  parsed?: ParsedStatement;
  accountIds: Record<string, number>;
  reviewed: boolean;
  result?: SaveImportResult;
  error?: string;
  duplicateNotice?: string;
  alreadyImported?: boolean;
}

export function currencies(statement: ParsedStatement): string[] {
  return [...new Set([...statement.currencyBalances.map(balance => balance.currency), ...statement.transactions.map(row => row.currency)])];
}

export function matchingAccounts(accounts: ImportAccount[], statement: ParsedStatement, currency: string): ImportAccount[] {
  return accounts.filter(account => account.isActive && (statement.provider === "unknown" || account.providerKey === statement.provider) && account.currency === currency && (!statement.accountType || account.accountType === statement.accountType));
}

export function suggestAccounts(accounts: ImportAccount[], statement: ParsedStatement): Record<string, number> {
  const result: Record<string, number> = {};
  const reference = statement.format.toUpperCase() === "MT940" ? normalizeAccountReference(statement.accountName) : "";
  for (const currency of currencies(statement)) {
    const candidates = matchingAccounts(accounts, statement, currency);
    if (reference) {
      const matches = candidates.filter(account => normalizeAccountReference(account.externalReference ?? "") === reference);
      if (matches.length === 1) result[currency] = matches[0].id;
    } else if (candidates.length === 1) result[currency] = candidates[0].id;
  }
  return result;
}

function normalizeAccountReference(value: string): string {
  return value.trim().replace(/^IBAN\s*:?\s*/i, "").replace(/\s/g, "").toUpperCase();
}

export function hasAccounts(item: BatchItem, accounts: ImportAccount[]): boolean {
  const providerKeys = new Set(accounts.filter(account => Object.values(item.accountIds).includes(account.id)).map(account => account.providerKey));
  return Boolean(item.parsed && providerKeys.size === 1 && currencies(item.parsed).length && currencies(item.parsed).every(currency => matchingAccounts(accounts, item.parsed!, currency).some(account => account.id === item.accountIds[currency])));
}

export function readyToSave(item: BatchItem, accounts: ImportAccount[]): boolean {
  return !item.result && !item.alreadyImported && hasAccounts(item, accounts);
}

export function canRelease(item: BatchItem, accounts: ImportAccount[]): boolean {
  return !item.result && !item.alreadyImported && !item.error && !item.reviewed && hasAccounts(item, accounts);
}

export function displayedProvider(item: BatchItem, accounts: ImportAccount[]): string {
  if (item.file.provider !== "unknown" || !hasAccounts(item, accounts)) return item.file.provider;
  return accounts.find(account => Object.values(item.accountIds).includes(account.id))?.providerKey ?? "unknown";
}

export async function saveBatch(items: BatchItem[], save: (item: BatchItem) => Promise<SaveImportResult>, update: (path: string, change: Partial<BatchItem>) => void, stopped: () => boolean) {
  for (const item of items) {
    if (stopped()) break;
    try {
      update(item.file.path, { result: await save(item), error: undefined });
    } catch (reason) {
      update(item.file.path, { error: String(reason) });
    }
  }
}
