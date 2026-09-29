// Typisierte Backend-Aufrufe für Positionsverwaltung und Kursaktualisierungen.
import { invoke } from "@tauri-apps/api/core";
import type { PositionChart } from "../assets/positionHistory";
import type { DeleteManualValuationRequest, ManualPosition, ManualValuation, MarketRefreshResult, PositionQuantityChangeRequest, SaveValuationRequest, UpdateManualValuationRequest } from "./types";
export const positionsApi = {
  list: (accountId: number) => invoke<ManualPosition[]>("list_manual_positions", { accountId }),
  valuations: (positionId: number) => invoke<ManualValuation[]>("list_manual_valuations", { positionId }),
  history: (accountId: number) => invoke<PositionChart[]>("position_chart_data", { accountId }),
  save: (request: SaveValuationRequest) => invoke<void>("save_manual_valuation", { request }),
  updateValuation: (request: UpdateManualValuationRequest) => invoke<void>("update_manual_valuation", { request }),
  deleteValuation: (request: DeleteManualValuationRequest) => invoke<void>("delete_manual_valuation", { request }),
  remove: (positionId: number) => invoke<void>("delete_manual_position", { positionId }),
  saveQuantityChange: (request: PositionQuantityChangeRequest) => invoke<void>("save_position_quantity_change", { request }),
  refresh: () => invoke<MarketRefreshResult>("refresh_market_data", { force: true }),
};
