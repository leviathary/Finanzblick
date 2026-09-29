// Bearbeitet oder löscht eine einzelne manuelle Bewertung in einem fokussierten Modal-Dialog.
import { createPortal } from "react-dom";
import { useEffect, useRef, useState } from "react";

import { locale, t } from "../../i18n";
import { money } from "../accounts/presentation";
import { ManualValueEditor } from "./ManualValueEditor";
import type { ManualPosition, ManualValuation } from "./types";

export function ManualValuationDialog({ position, valuation, currency, saving, error, canDelete, onClose, onSave, onDelete }: {
  position: ManualPosition;
  valuation: ManualValuation;
  currency: string;
  saving: boolean;
  error: string | null;
  canDelete: boolean;
  onClose: () => void;
  onSave: (date: string, amount: string) => Promise<boolean>;
  onDelete: () => Promise<boolean>;
}) {
  const dialog = useRef<HTMLDialogElement>(null);
  const [confirmingDelete, setConfirmingDelete] = useState(false);
  useEffect(() => { dialog.current?.showModal(); }, []);
  const close = () => dialog.current?.close();
  const date = new Intl.DateTimeFormat(locale(), { dateStyle: "medium", timeZone: "UTC" })
    .format(new Date(`${valuation.valueDate}T00:00:00Z`));

  return createPortal(<dialog
    ref={dialog}
    className="settlement-rule-dialog valuation-edit-dialog"
    aria-labelledby="valuation-edit-dialog-title"
    onClose={onClose}
    onCancel={event => {
      event.preventDefault();
      if (saving) return;
      if (confirmingDelete) setConfirmingDelete(false);
      else close();
    }}
  >
    {confirmingDelete ? <>
      <h2 id="valuation-edit-dialog-title">{t("Bewertung löschen?")}</h2>
      <p>{t("Diese manuell eingegebene Bewertung wird dauerhaft gelöscht. Die übrigen Stichtage bleiben erhalten.")}</p>
      <p className="valuation-delete-summary"><strong>{date}</strong><strong>{money(valuation.amountMinor, valuation.currency)}</strong></p>
      {error && <p className="error-message" role="alert">{error}</p>}
      <div className="settlement-dialog-actions">
        <button className="secondary-button" type="button" disabled={saving} onClick={() => setConfirmingDelete(false)}>{t("Abbrechen")}</button>
        <button className="danger-button" type="button" disabled={saving} onClick={() => void onDelete().then(deleted => { if (deleted) close(); })}>{saving ? t("Bitte warten …") : t("Bewertung löschen")}</button>
      </div>
    </> : <>
      <h2 id="valuation-edit-dialog-title">{t("Bewertung bearbeiten")}</h2>
      <p className="settings-hint">{position.label} · {date}</p>
      {error && <p className="error-message" role="alert">{error}</p>}
      <ManualValueEditor
        position={position}
        valuation={valuation}
        currency={currency}
        saving={saving}
        showHeading={false}
        deleteDisabled={!canDelete}
        onDelete={() => setConfirmingDelete(true)}
        onCancel={close}
        onSave={async (valueDate, amount) => {
          const saved = await onSave(valueDate, amount);
          if (saved) close();
          return saved;
        }}
      />
      {!canDelete && <p className="settings-hint">{t("Die einzige Bewertung einer Position kann nicht gelöscht werden.")}</p>}
    </>}
  </dialog>, document.body);
}
