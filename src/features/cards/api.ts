// Kapselt die stabilen Tauri-Commands für Kartenansicht und Einrichtung.
import { invoke } from "@tauri-apps/api/core";
import type { Account, Category, Row, Decision, Preview, SetupRequest, SettlementRule } from "./types";
import type { TransferType } from "../transactions/TransactionActions";
export const cardsApi = {
  accounts: () => invoke<Account[]>("list_accounts"),
  rules: () => invoke<SettlementRule[]>("list_settlement_rules"),
  categories: () => invoke<Category[]>("list_categories"),
  rows: (accountId: number) => invoke<Row[]>("list_card_setup_transactions", { accountId }),
  preview: (transactionId: number, prefix: string) => invoke<Preview>("preview_settlement_rule", { transactionId, prefix }),
  decide: (decision: Decision) => invoke<void>("set_card_credit_decision", { decision }),
  transfer: (transactionId: number, transferType: TransferType) => invoke<void>("set_transaction_transfers", { transactionIds: [transactionId], transferType }),
  confirm: (request: SetupRequest) => invoke<void>("confirm_card_setup", { request }),
};
