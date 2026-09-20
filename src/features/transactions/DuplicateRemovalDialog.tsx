// Bestätigt das reversible Ausblenden einer doppelten Buchung mit den wichtigsten Buchungsdetails.
import { useEffect, useRef } from "react";
import { createPortal } from "react-dom";
import { t } from "../../i18n";

export function DuplicateRemovalDialog({ description, date, account, amount, busy, error, onCancel, onConfirm }: {
  description: string; date: string; account: string; amount: string; busy: boolean; error: string | null;
  onCancel: () => void; onConfirm: () => void;
}) {
  const dialog = useRef<HTMLDialogElement>(null);
  useEffect(() => { dialog.current?.showModal(); }, []);
  return createPortal(<dialog ref={dialog} className="settlement-rule-dialog duplicate-removal-dialog"
    aria-labelledby="duplicate-removal-title"
    onCancel={event => { event.preventDefault(); if (!busy) onCancel(); }}>
    <h2 id="duplicate-removal-title">{t("Doppelte Buchung entfernen?")}</h2>
    <p>{t("Die Buchung wird aus Salden und Auswertungen entfernt. Die ursprüngliche Importspur bleibt erhalten und kann unter „Importierte Dateien“ wiederhergestellt werden.")}</p>
    <dl className="duplicate-removal-details">
      <div><dt>{t("Beschreibung")}</dt><dd>{description}</dd></div>
      <div><dt>{t("Datum")}</dt><dd>{date}</dd></div>
      <div><dt>{t("Konto")}</dt><dd>{account}</dd></div>
      <div><dt>{t("Betrag")}</dt><dd>{amount}</dd></div>
    </dl>
    {error && <p className="error-message" role="alert">{t(error)}</p>}
    <div className="settlement-dialog-actions">
      <button type="button" className="secondary-button" disabled={busy} onClick={onCancel}>{t("Abbrechen")}</button>
      <button type="button" className="danger-button" disabled={busy} onClick={onConfirm}>{busy ? t("Bitte warten …") : t("Als Duplikat entfernen")}</button>
    </div>
  </dialog>, document.body);
}
