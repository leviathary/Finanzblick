// Zeigt die Bearbeitung einer bestehenden Depotposition in einem fokussierten Modal-Dialog.
import { createPortal } from "react-dom";
import { useLayoutEffect, useRef, type ReactNode } from "react";

import { t } from "../../i18n";
import { ContextHelp } from "../../shared/ContextHelp";

export function ManualPositionDialog({ positionLabel, saving, error, showQuantityHelp, onClose, children }: {
  positionLabel: string;
  saving: boolean;
  error: string | null;
  showQuantityHelp: boolean;
  onClose: () => void;
  children: ReactNode;
}) {
  const dialog = useRef<HTMLDialogElement>(null);

  useLayoutEffect(() => {
    if (!dialog.current?.open) {
      dialog.current?.showModal();
      dialog.current?.querySelector<HTMLInputElement>("input:not([disabled])")?.focus();
    }
  }, []);

  return createPortal(
    <dialog
      ref={dialog}
      className="settlement-rule-dialog position-edit-dialog"
      aria-labelledby="position-edit-dialog-title"
      onClose={onClose}
      onCancel={(event) => {
        event.preventDefault();
        if (!saving) dialog.current?.close();
      }}
    >
      <div className="position-edit-dialog-heading">
        <h2 id="position-edit-dialog-title">{t("Position bearbeiten")}</h2>
        {showQuantityHelp && (
          <ContextHelp label={t("Bestandsänderungen erklären")}>
            {t("Käufe und Verkäufe werden über das Drei-Punkte-Menü der Position erfasst. Ein Verkauf des gesamten Bestands setzt das Enddatum automatisch.")}
          </ContextHelp>
        )}
      </div>
      <p className="settings-hint">{positionLabel}</p>
      {error && <p className="error-message" role="alert">{error}</p>}
      {children}
    </dialog>,
    document.body,
  );
}
