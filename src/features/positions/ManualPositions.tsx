// Verwaltet Positionen und Bewertungen eines einzelnen Kontos, getrennt von der Kontenübersicht.
import { useEffect, useRef, useState } from "react";
import { positionsApi } from "./api";
import { t } from "../../i18n";
import type { Account } from "../accounts/types";
import type { ManualPosition } from "./types";
import { money } from "../accounts/presentation";
import { marketSourceLabel } from "./presentation";
import { ActionMenu } from "../../shared/ActionMenu";
import { ManualPositionDialog } from "./ManualPositionDialog";
import { ManualPositionEditor, type ManualPositionDraft } from "./ManualPositionEditor";
import { ManualValuationHistory } from "./ManualValuationHistory";
import { PositionQuantityWorkflow } from "./PositionQuantityWorkflow";
import { existingPositionDraft, newPositionDraft } from "./positionDraft";

type Props = {
  account: Account;
  onClose: () => void;
  onChanged: () => Promise<void>;
  onError: (message: string | null) => void;
};

export function ManualPositions({ account: valuing, onClose, onChanged, onError: setError }: Props) {
  const backButton = useRef<HTMLButtonElement>(null);
  const [saving, setSaving] = useState(false);
  const [valuationNotice, setValuationNotice] = useState<string | null>(null);
  const [valuationActionError, setValuationActionError] = useState<string | null>(null);
  const [marketRefreshPending, setMarketRefreshPending] = useState(false);
  const [showPositionForm, setShowPositionForm] = useState(true);
  const [editingPosition, setEditingPosition] = useState<ManualPosition | null>(null);
  const [positionEditorError, setPositionEditorError] = useState<string | null>(null);
  const [quantityPosition, setQuantityPosition] = useState<ManualPosition | null>(null);
  const [valuingPosition, setValuingPosition] = useState<ManualPosition | null>(null);
  const [historyRevision, setHistoryRevision] = useState(0);
  const [valuation, setValuation] = useState<ManualPositionDraft>(() => newPositionDraft(valuing.currency));
  const [positions, setPositions] = useState<ManualPosition[]>([]);

  useEffect(() => {
    backButton.current?.focus();
  }, []);

  useEffect(() => {
    let active = true;
    const refresh = (initial = false) => {
      void positionsApi.list(valuing.id)
        .then(rows => { if (active) { setPositions(rows); if (initial) setShowPositionForm(rows.length === 0); } })
        .catch(reason => { if (active) setError(typeof reason === "string" ? reason : t("Konten konnten nicht geladen werden.")); });
    };
    refresh(true);
    const handleRefresh = () => refresh();
    window.addEventListener("market-data-refreshed", handleRefresh);
    return () => { active = false; window.removeEventListener("market-data-refreshed", handleRefresh); };
  }, [valuing.id, setError]);
  const automaticValuation =
    valuation.method === "units" &&
    Boolean(
      (valuation.identifierType === "isin"
        ? valuation.isin
        : valuation.ticker
      ).trim(),
    );


  function reportPositionError(message: string) {
    setError(message);
    if (editingPosition) setPositionEditorError(message);
  }

  async function saveValuation() {
    const wasEditing = valuation.id !== null;
    const number = (value: string) =>
      Number(value.trim().replace("'", "").replace(",", "."));
    const useUnits =
      valuation.method === "units" && valuation.assetType !== "cash";
    const identifier =
      valuation.identifierType === "isin" ? valuation.isin : valuation.ticker;
    const useAutomaticPrice = useUnits && Boolean(identifier.trim());
    const quantity =
      useUnits && valuation.quantity ? number(valuation.quantity) : null;
    const unitPrice =
      useUnits && !useAutomaticPrice && valuation.unitPrice
        ? number(valuation.unitPrice)
        : null;
    const exchangeRate =
      useUnits &&
      !useAutomaticPrice &&
      valuation.quoteCurrency !== valuing.currency
        ? number(valuation.exchangeRate)
        : 1;
    const amount = useAutomaticPrice
      ? (positions.find((position) => position.id === valuation.id)
          ?.amountMinor ?? 0) / 100
      : useUnits && quantity !== null && unitPrice !== null
        ? quantity * unitPrice * exchangeRate
        : number(valuation.amount);
    if (
      valuation.identifierType === "isin" &&
      identifier.trim() &&
      !/^[A-Z]{2}[A-Z0-9]{10}$/.test(identifier.trim())
    ) {
      reportPositionError(t("Die ISIN muss aus 12 Buchstaben und Ziffern bestehen und mit einem zweistelligen Ländercode beginnen."));
      return false;
    }
    if (
      useAutomaticPrice &&
      (quantity === null || !Number.isFinite(quantity) || quantity <= 0)
    ) {
      reportPositionError(t("Bitte eine gültige Menge eingeben."));
      return false;
    }
    const holdingStartDate = valuation.holdingStartDate || valuation.date;
    if (!holdingStartDate) {
      reportPositionError(t("Bitte ein Einstandsdatum eingeben."));
      return false;
    }
    if (
      valuation.holdingEndDate &&
      valuation.holdingEndDate < holdingStartDate
    ) {
      reportPositionError(t("Das Verkaufsdatum darf nicht vor dem Einstandsdatum liegen."));
      return false;
    }
    if (
      useUnits &&
      (quantity === null ||
        (!useAutomaticPrice && unitPrice === null) ||
        !Number.isFinite(quantity) ||
        (!useAutomaticPrice && !Number.isFinite(unitPrice)))
    ) {
      reportPositionError(t("Bitte Menge und Wert pro Einheit vollständig eingeben."));
      return false;
    }
    if (
      useUnits &&
      !useAutomaticPrice &&
      (!Number.isFinite(exchangeRate) || exchangeRate <= 0)
    ) {
      reportPositionError(t("Bitte einen gültigen Wechselkurs eingeben."));
      return false;
    }
    if (!Number.isFinite(amount) || amount < 0) {
      reportPositionError(t("Bitte einen gültigen Wert eingeben."));
      return false;
    }
    setSaving(true);
    setError(null);
    setPositionEditorError(null);
    try {
      await positionsApi.save({
          id: valuation.id,
          accountId: valuing.id,
          label: valuation.label,
          valuationDate: wasEditing ? valuation.date : holdingStartDate,
          amountMinor: Math.round(amount * 100),
          quantity,
          unitPriceMinor:
            unitPrice === null ? null : Math.round(unitPrice * 100),
          quoteCurrency:
            useUnits && !useAutomaticPrice ? valuation.quoteCurrency : null,
          exchangeRate: useUnits && !useAutomaticPrice ? exchangeRate : null,
          assetType: valuation.assetType,
          identifierType: identifier.trim() ? valuation.identifierType : null,
          identifier: identifier.trim() || null,
          holdingStartDate,
          holdingEndDate: valuation.holdingEndDate || null,
      });
      setPositions(
        await positionsApi.list(valuing.id),
      );
      await onChanged();
      setValuation(newPositionDraft(valuing.currency));
      setValuationNotice(
        useAutomaticPrice
          ? t("Position gespeichert. Kurse werden im Hintergrund geladen.")
          : wasEditing
            ? t("Position wurde aktualisiert und ist oben aufgeführt.")
            : t("Position wurde gespeichert und ist oben aufgeführt."),
      );
      setShowPositionForm(false);
      if (useAutomaticPrice) {
        setMarketRefreshPending(true);
        void positionsApi.refresh()
          .then((refresh) => {
            setMarketRefreshPending(false);
            setValuationNotice(
              refresh.errors.length
                ? `${t("Position gespeichert. Der automatische Kurs konnte noch nicht geladen werden.")} ${refresh.errors.join(" · ")}`
                : t("Kursdaten wurden im Hintergrund aktualisiert."),
            );
            window.dispatchEvent(new Event("market-data-refreshed"));
          })
          .catch((reason) => {
            setMarketRefreshPending(false);
            setValuationNotice(
              `${t("Position gespeichert. Der automatische Kurs konnte noch nicht geladen werden.")} ${String(reason)}`,
            );
          });
      }
      return true;
    } catch (reason) {
      reportPositionError(
        typeof reason === "string"
          ? reason
          : t("Der manuelle Wert konnte nicht gespeichert werden."),
      );
      return false;
    } finally {
      setSaving(false);
    }
  }

  async function saveAdditionalValuation(position: ManualPosition, date: string, value: string) {
    const amount = Number(value.trim().replace(/[’']/g, "").replace(",", "."));
    if (!Number.isFinite(amount) || amount < 0) {
      const message = t("Bitte einen gültigen Wert eingeben.");
      setError(message); setValuationActionError(message);
      return false;
    }
    if (!date || position.holdingStartDate && date < position.holdingStartDate) {
      const message = t("Das Bewertungsdatum darf nicht vor dem Einstandsdatum liegen.");
      setError(message); setValuationActionError(message);
      return false;
    }
    if (position.holdingEndDate && date > position.holdingEndDate) {
      const message = t("Das Bewertungsdatum darf nicht nach dem Verkaufsdatum liegen.");
      setError(message); setValuationActionError(message);
      return false;
    }
    setSaving(true);
    setError(null);
    setValuationActionError(null);
    try {
      await positionsApi.save({
        id: position.id,
        accountId: position.accountId,
        label: position.label,
        valuationDate: date,
        amountMinor: Math.round(amount * 100),
        quantity: position.quantity,
        unitPriceMinor: null,
        quoteCurrency: null,
        exchangeRate: null,
        assetType: position.assetType ?? "other",
        identifierType: null,
        identifier: null,
        holdingStartDate: position.holdingStartDate ?? position.valuationDate,
        holdingEndDate: position.holdingEndDate,
      });
      const updatedPositions = await positionsApi.list(valuing.id);
      setPositions(updatedPositions);
      await onChanged();
      setValuingPosition(updatedPositions.find(current => current.id === position.id) ?? null);
      setHistoryRevision(current => current + 1);
      setValuationNotice(t("Neue Bewertung gespeichert. Frühere Bewertungen bleiben erhalten."));
      return true;
    } catch (reason) {
      const message = typeof reason === "string" ? reason : t("Der manuelle Wert konnte nicht gespeichert werden.");
      setError(message); setValuationActionError(message);
      return false;
    } finally {
      setSaving(false);
    }
  }

  async function updateStoredValuation(position: ManualPosition, valuationId: number, date: string, value: string) {
    const amount = Number(value.trim().replace(/[’']/g, "").replace(",", "."));
    if (!Number.isFinite(amount) || amount < 0) {
      const message = t("Bitte einen gültigen Wert eingeben.");
      setError(message); setValuationActionError(message);
      return false;
    }
    if (!date || position.holdingStartDate && date < position.holdingStartDate) {
      const message = t("Das Bewertungsdatum darf nicht vor dem Einstandsdatum liegen.");
      setError(message); setValuationActionError(message);
      return false;
    }
    if (position.holdingEndDate && date > position.holdingEndDate) {
      const message = t("Das Bewertungsdatum darf nicht nach dem Verkaufsdatum liegen.");
      setError(message); setValuationActionError(message);
      return false;
    }
    setSaving(true);
    setError(null);
    setValuationActionError(null);
    try {
      await positionsApi.updateValuation({
        id: valuationId,
        positionId: position.id,
        valueDate: date,
        amountMinor: Math.round(amount * 100),
      });
      const updatedPositions = await positionsApi.list(valuing.id);
      setPositions(updatedPositions);
      await onChanged();
      setValuingPosition(updatedPositions.find(current => current.id === position.id) ?? null);
      setHistoryRevision(current => current + 1);
      setValuationNotice(t("Bewertung korrigiert."));
      return true;
    } catch (reason) {
      const message = typeof reason === "string" ? reason : t("Die Bewertung konnte nicht korrigiert werden.");
      setError(message); setValuationActionError(message);
      return false;
    } finally {
      setSaving(false);
    }
  }

  async function deleteStoredValuation(position: ManualPosition, valuationId: number) {
    setSaving(true);
    setError(null);
    setValuationActionError(null);
    try {
      await positionsApi.deleteValuation({ id: valuationId, positionId: position.id });
      const updatedPositions = await positionsApi.list(valuing.id);
      setPositions(updatedPositions);
      await onChanged();
      setValuingPosition(updatedPositions.find(current => current.id === position.id) ?? null);
      setHistoryRevision(current => current + 1);
      setValuationNotice(t("Bewertung gelöscht."));
      return true;
    } catch (reason) {
      const message = typeof reason === "string" ? reason : t("Die Bewertung konnte nicht gelöscht werden.");
      setError(message); setValuationActionError(message);
      return false;
    } finally {
      setSaving(false);
    }
  }

  function startNewPosition() {
    setValuationNotice(null);
    setPositionEditorError(null);
    setEditingPosition(null);
    setValuingPosition(null);
    setValuation(newPositionDraft(valuing.currency));
    setShowPositionForm(true);
  }

  function startNewValuation(position: ManualPosition) {
    setValuationNotice(null);
    setShowPositionForm(false);
    setValuingPosition(position);
  }

  function startEditingPosition(position: ManualPosition) {
    setValuationNotice(null);
    setError(null);
    setPositionEditorError(null);
    setShowPositionForm(false);
    setEditingPosition(position);
    setValuation(existingPositionDraft(position, valuing.currency));
  }

  function cancelPositionEditor() {
    setValuationNotice(null);
    setPositionEditorError(null);
    setError(null);
    setEditingPosition(null);
    setShowPositionForm(false);
    setValuation(newPositionDraft(valuing.currency));
  }

  async function deletePosition(position: ManualPosition) {
    if (!window.confirm(t("Diese Position löschen?"))) return;
    setError(null);
    try {
      await positionsApi.remove(position.id);
      const remainingPositions = await positionsApi.list(position.accountId);
      setPositions(remainingPositions);
      setShowPositionForm(remainingPositions.length === 0);
      await onChanged();
    } catch (reason) {
      setError(typeof reason === "string" ? reason : t("Die Position konnte nicht gelöscht werden."));
    }
  }

  if (valuingPosition) {
    return <ManualValuationHistory
      position={valuingPosition}
      currency={valuing.currency}
      saving={saving}
      notice={valuationNotice}
      actionError={valuationActionError}
      revision={historyRevision}
      onBack={() => { setValuingPosition(null); setValuationNotice(null); setValuationActionError(null); setError(null); }}
      onClearError={() => { setValuationActionError(null); setError(null); }}
      onSave={(date, amount) => saveAdditionalValuation(valuingPosition, date, amount)}
      onUpdate={(valuationId, date, amount) => updateStoredValuation(valuingPosition, valuationId, date, amount)}
      onDelete={valuationId => deleteStoredValuation(valuingPosition, valuationId)}
    />;
  }

  return <>
        <div className="manual-position-back">
          <button ref={backButton} className="secondary-button" type="button" onClick={onClose}>
            <span aria-hidden="true">←</span> {t("Zurück zu Banken & Konten")}
          </button>
        </div>
        <article className="dashboard-card manual-valuation-form">
          <div className="card-heading">
            <div>
              <p className="eyebrow">{t("Positionen")}</p>
              <h2>
                {valuing.provider} · {valuing.name}
              </h2>
            </div>
          </div>
          <p className="settings-hint">
            {t(
              "Erfasse mehrere Positionen. Der Kontowert ergibt sich aus ihrer Summe.",
            )}
          </p>
          {valuationNotice && (
            <div className="position-save-feedback" role="status">
              <span aria-hidden="true">✓</span>
              {valuationNotice}
            </div>
          )}
          <section className="positions-section">
            <div className="position-section-heading">
              <div>
                <p className="eyebrow">{t("Bestehende Positionen")}</p>
                <h3>{t("Erfasste Werte")}</h3>
              </div>
              {positions.length > 0 && (
                <div className="positions-total">
                  <span>{t("Gesamtwert")}</span>
                  <strong>
                    {money(
                      positions.reduce(
                        (total, position) =>
                          total +
                          ((!position.holdingStartDate ||
                            position.holdingStartDate <=
                              new Date().toISOString().slice(0, 10)) &&
                          (!position.holdingEndDate ||
                            position.holdingEndDate >=
                              new Date().toISOString().slice(0, 10))
                            ? position.amountMinor
                            : 0),
                        0,
                      ),
                      positions.every(
                        (position) =>
                          position.valueCurrency === positions[0].valueCurrency,
                      )
                        ? positions[0].valueCurrency
                        : valuing.currency,
                    )}
                  </strong>
                </div>
              )}
            </div>
            {positions.length > 0 ? (
              <div className="manual-position-list">
                {positions.map((position) => (
                  <div key={position.id}>
                    <div className="position-summary">
                      <strong>{position.label}</strong>
                      <small>
                        {position.valuationDate}
                        {position.identifier
                          ? ` · ${position.identifier} · ${t("Automatisch bewertet")}${position.priceSource ? ` (${marketSourceLabel(position.priceSource)})` : ""}`
                          : ""}
                        {position.holdingStartDate
                          ? ` · ${t("ab")} ${position.holdingStartDate}`
                          : ""}
                        {position.holdingEndDate
                          ? ` · ${t("bis")} ${position.holdingEndDate}`
                          : ""}
                      </small>
                    </div>
                    <b className="position-amount">
                      {marketRefreshPending &&
                      position.identifier
                        ? t("Kurse werden geladen …")
                        : money(
                        (!position.holdingStartDate ||
                          position.holdingStartDate <=
                            new Date().toISOString().slice(0, 10)) &&
                          (!position.holdingEndDate ||
                            position.holdingEndDate >=
                              new Date().toISOString().slice(0, 10))
                          ? position.amountMinor
                          : 0,
                        position.valueCurrency,
                          )}
                    </b>
                    <ActionMenu
                      label={`${t("Aktionen")}: ${position.label}`}
                      disabled={saving}
                      actions={[
                        ...(!position.identifier
                          ? [{
                              label: t("Neue Bewertung"),
                              onClick: () => startNewValuation(position),
                            }]
                          : []),
                        ...(position.identifier && position.quantity !== null
                          ? [{
                              label: t("Kauf/Verkauf erfassen"),
                              onClick: () => {
                                setQuantityPosition(position);
                              },
                            }]
                          : []),
                        {
                          label: t("Position bearbeiten"),
                          onClick: () => startEditingPosition(position),
                        },
                        ...(position.canDelete
                          ? [{
                              label: t("Löschen"),
                              onClick: () => void deletePosition(position),
                              separated: true,
                              danger: true,
                            }]
                          : []),
                      ]}
                    />
                  </div>
                ))}
              </div>
            ) : (
              <p className="empty-position-hint">
                {t("Noch keine Position erfasst.")}
              </p>
            )}
          </section>
          {!showPositionForm && positions.length > 0 && (
            <div className="new-position-cta">
              <button
                className="secondary-button"
                type="button"
                onClick={startNewPosition}
              >
                {t("Neue Position erfassen")}
              </button>
            </div>
          )}
          {showPositionForm && (
            <ManualPositionEditor
              account={valuing}
              positionsExist={positions.length > 0}
              valuation={valuation}
              setValuation={setValuation}
              automaticValuation={automaticValuation}
              saving={saving}
              onCancel={cancelPositionEditor}
              onSave={saveValuation}
            />
          )}
        </article>
        {editingPosition && (
          <ManualPositionDialog
            positionLabel={editingPosition.label}
            saving={saving}
            error={positionEditorError}
            showQuantityHelp={valuation.method === "units"}
            onClose={cancelPositionEditor}
          >
            <ManualPositionEditor
              account={valuing}
              positionsExist
              valuation={valuation}
              setValuation={setValuation}
              automaticValuation={automaticValuation}
              saving={saving}
              showHeading={false}
              onCancel={cancelPositionEditor}
              onSave={async () => {
                const saved = await saveValuation();
                if (saved) setEditingPosition(null);
                return saved;
              }}
            />
          </ManualPositionDialog>
        )}
        {quantityPosition && (
          <PositionQuantityWorkflow
            position={quantityPosition}
            accountId={valuing.id}
            onChanged={onChanged}
            onPositionsChanged={setPositions}
            onNotice={setValuationNotice}
            onError={setError}
            onClose={() => setQuantityPosition(null)}
          />
        )}
      </>;
}
