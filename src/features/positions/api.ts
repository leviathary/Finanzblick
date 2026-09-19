// Typisierte Backend-Aufrufe für Positionsverwaltung und Kursaktualisierungen.
import { invoke } from "@tauri-apps/api/core";
import type { ManualPosition, MarketRefreshResult, SaveValuationRequest } from "./types";
export const positionsApi = {
  list: (accountId: number) => invoke<ManualPosition[]>("list_manual_positions", { accountId }),
  save: (request: SaveValuationRequest) => invoke<void>("save_manual_valuation", { request }),
  remove: (positionId: number) => invoke<void>("delete_manual_position", { positionId }),
  refresh: () => invoke<MarketRefreshResult>("refresh_market_data", { force: true }),
};
