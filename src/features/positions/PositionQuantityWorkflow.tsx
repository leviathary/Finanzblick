// Koordiniert Speicherung, Rückmeldung und Dialogzustand einer manuellen Bestandsänderung.
import { useState } from "react";

import { t } from "../../i18n";
import { positionsApi } from "./api";
import { PositionQuantityDialog } from "./PositionQuantityDialog";
import type { ManualPosition } from "./types";

export function PositionQuantityWorkflow({ position, accountId, onChanged, onPositionsChanged, onNotice, onError, onClose }: {
  position: ManualPosition;
  accountId: number;
  onChanged: () => Promise<void>;
  onPositionsChanged: (positions: ManualPosition[]) => void;
  onNotice: (notice: string) => void;
  onError: (message: string | null) => void;
  onClose: () => void;
}) {
  const [saving, setSaving] = useState(false);
  const [error, setError] = useState<string | null>(null);

  async function save(date: string, changeType: "buy" | "sell", quantity: string) {
    setSaving(true);
    setError(null);
    onError(null);
    try {
      await positionsApi.saveQuantityChange({ positionId: position.id, effectiveDate: date, changeType, quantity });
      onPositionsChanged(await positionsApi.list(accountId));
      await onChanged();
      onNotice(t(changeType === "buy" ? "Kauf erfasst." : "Verkauf erfasst."));
      return true;
    } catch (reason) {
      const message = typeof reason === "string" ? reason : t("Die Bestandsänderung konnte nicht gespeichert werden.");
      setError(message);
      onError(message);
      return false;
    } finally {
      setSaving(false);
    }
  }

  return <PositionQuantityDialog
    position={position}
    saving={saving}
    error={error}
    onClose={() => { onError(null); onClose(); }}
    onSave={save}
  />;
}
