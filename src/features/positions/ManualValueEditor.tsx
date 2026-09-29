// Erfasst einen neuen datierten Gesamtwert für eine bestehende manuell bewertete Position.
import { useState } from "react";

import { locale, t } from "../../i18n";
import type { ManualPosition, ManualValuation } from "./types";

export function ManualValueEditor({ position, valuation, currency, saving, showHeading = true, onCancel, onSave, onDelete, deleteDisabled = false }: {
  position: ManualPosition;
  valuation?: ManualValuation | null;
  currency: string;
  saving: boolean;
  showHeading?: boolean;
  onCancel: () => void;
  onSave: (date: string, amount: string) => Promise<boolean>;
  onDelete?: () => void;
  deleteDisabled?: boolean;
}) {
  const today = new Date().toISOString().slice(0, 10);
  const latestAllowedDate = position.holdingEndDate && position.holdingEndDate < today
    ? position.holdingEndDate
    : today;
  const editing = Boolean(valuation);
  const [date, setDate] = useState(valuation?.valueDate ?? latestAllowedDate);
  const [amount, setAmount] = useState(String((valuation?.amountMinor ?? position.amountMinor) / 100));
  const heading = editing ? t("Bewertung bearbeiten") : t("Neue Bewertung");
  const explanation = editing ? null : t("Der neue Wert gilt ab diesem Stichtag. Frühere Bewertungen bleiben erhalten.");
  const saveLabel = editing ? t("Änderungen speichern") : t("Bewertung speichern");

  return <section className="position-form-section position-value-form">
    {showHeading && <div className="position-section-heading">
      <div>
        <p className="eyebrow">{t("Bewertung")}</p>
        <h3>{heading}</h3>
        <p className="settings-hint">{position.label}</p>
      </div>
    </div>}
    {explanation && <p className="settings-hint">{explanation}</p>}
    <div className="management-form valuation-fields">
      <label>
        {t("Gesamtwert")} ({currency})
        <input autoFocus inputMode="decimal" value={amount} onChange={event=>setAmount(event.target.value)} />
      </label>
      <label>
        {t("Bewertungsdatum")}
        <input type="date" lang={locale()} min={position.holdingStartDate ?? undefined} max={latestAllowedDate} value={date} onChange={event=>setDate(event.target.value)} />
      </label>
    </div>
    <div className="form-actions">
      {onDelete && <button className="danger-button valuation-delete-action" type="button" disabled={saving || deleteDisabled} onClick={onDelete}>{t("Löschen")}</button>}
      <button className="secondary-button" type="button" disabled={saving} onClick={onCancel}>{t("Abbrechen")}</button>
      <button className="primary-button" type="button" disabled={saving || !date || !amount.trim()} onClick={()=>void onSave(date, amount)}>{saving ? t("Bitte warten …") : saveLabel}</button>
    </div>
  </section>;
}
