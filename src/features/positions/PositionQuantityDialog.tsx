// Erfasst einen datierten Kauf oder Verkauf, ohne frühere Positionsmengen zu überschreiben.
import { createPortal } from "react-dom";
import { useEffect, useRef, useState } from "react";

import { locale, t } from "../../i18n";
import type { ManualPosition } from "./types";

export function PositionQuantityDialog({ position, saving, error, onClose, onSave }: {
  position: ManualPosition;
  saving: boolean;
  error: string | null;
  onClose: () => void;
  onSave: (date: string, changeType: "buy" | "sell", quantity: string) => Promise<boolean>;
}) {
  const dialog = useRef<HTMLDialogElement>(null);
  const [changeType, setChangeType] = useState<"buy" | "sell">("buy");
  const [quantity, setQuantity] = useState("");
  const [date, setDate] = useState(new Date().toISOString().slice(0, 10));
  useEffect(() => { if (!dialog.current?.open) dialog.current?.showModal(); }, []);
  const parsed = Number(quantity.trim().replace(/[’' ]/g, "").replace(",", "."));
  const current = position.quantity ?? 0;
  const result = Number.isFinite(parsed) && parsed > 0
    ? current + (changeType === "buy" ? parsed : -parsed)
    : null;
  const units = (value: number) => new Intl.NumberFormat(locale(), { maximumFractionDigits: 9 }).format(value);

  return createPortal(<dialog
    ref={dialog}
    className="settlement-rule-dialog quantity-change-dialog"
    aria-labelledby="quantity-change-title"
    onClose={onClose}
    onCancel={event => { event.preventDefault(); if (!saving) dialog.current?.close(); }}
  >
    <h2 id="quantity-change-title">{t("Kauf oder Verkauf erfassen")}</h2>
    <p className="settings-hint">{position.label} · {t("Aktueller Bestand")}: {units(current)}</p>
    <fieldset className="quantity-change-type">
      <legend>{t("Vorgang")}</legend>
      <label><input type="radio" name="quantity-change" checked={changeType === "buy"} disabled={saving} onChange={() => setChangeType("buy")} /> {t("Kauf")}</label>
      <label><input type="radio" name="quantity-change" checked={changeType === "sell"} disabled={saving} onChange={() => setChangeType("sell")} /> {t("Verkauf")}</label>
    </fieldset>
    <div className="quantity-change-fields">
      <label>{t("Anzahl")}<input autoFocus inputMode="decimal" value={quantity} disabled={saving} onChange={event => setQuantity(event.target.value)} /></label>
      <label>{t("Datum")}<input type="date" lang={locale()} max={new Date().toISOString().slice(0, 10)} value={date} disabled={saving} onChange={event => setDate(event.target.value)} /></label>
    </div>
    {result !== null && <p className={`quantity-change-result${result < 0 ? " invalid" : ""}`}>
      <span>{t("Neuer Bestand")}</span><strong>{units(current)} → {units(result)}</strong>
    </p>}
    {error && <p className="error-message" role="alert">{t(error)}</p>}
    <div className="settlement-dialog-actions">
      <button type="button" className="secondary-button" disabled={saving} onClick={() => dialog.current?.close()}>{t("Abbrechen")}</button>
      <button type="button" className="primary-button" disabled={saving || !date || !quantity.trim() || result === null || result < 0} onClick={() => void onSave(date, changeType, quantity).then(saved => { if (saved) dialog.current?.close(); })}>
        {saving ? t("Bitte warten …") : changeType === "buy" ? t("Kauf erfassen") : t("Verkauf erfassen")}
      </button>
    </div>
  </dialog>, document.body);
}
