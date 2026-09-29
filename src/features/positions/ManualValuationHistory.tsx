// Zeigt den Bewertungsverlauf einer manuellen Position und nimmt neue Stichtagswerte entgegen.

import { useEffect, useRef, useState } from "react";

import { locale, t } from "../../i18n";
import { InteractiveTimelineChart } from "../../shared/charts/InteractiveTimelineChart";
import type { PositionChart } from "../assets/positionHistory";
import { money } from "../accounts/presentation";
import { positionsApi } from "./api";
import { ManualValuationDialog } from "./ManualValuationDialog";
import { ManualValueEditor } from "./ManualValueEditor";
import type { ManualPosition, ManualValuation } from "./types";

interface Props {
  position: ManualPosition;
  currency: string;
  saving: boolean;
  notice: string | null;
  actionError: string | null;
  revision: number;
  onBack: () => void;
  onClearError: () => void;
  onSave: (date: string, amount: string) => Promise<boolean>;
  onUpdate: (valuationId: number, date: string, amount: string) => Promise<boolean>;
  onDelete: (valuationId: number) => Promise<boolean>;
}

export function ManualValuationHistory({ position, currency, saving, notice, actionError, revision, onBack, onClearError, onSave, onUpdate, onDelete }: Props) {
  const backButton = useRef<HTMLButtonElement>(null);
  const editOrigin = useRef<HTMLButtonElement | null>(null);
  const [chart, setChart] = useState<PositionChart | null>(null);
  const [valuations, setValuations] = useState<ManualValuation[]>([]);
  const [editing, setEditing] = useState<ManualValuation | null>(null);
  const [loading, setLoading] = useState(true);
  const [error, setError] = useState("");
  const [retry, setRetry] = useState(0);
  const [controls, setControls] = useState<HTMLDivElement | null>(null);

  useEffect(() => { backButton.current?.focus(); }, []);
  useEffect(() => {
    let active = true;
    setLoading(true);
    setError("");
    void Promise.all([positionsApi.history(position.accountId), positionsApi.valuations(position.id)])
      .then(([rows, savedValuations]) => {
        if (!active) return;
        setChart(rows.find(row => row.id === position.id) ?? null);
        setValuations(savedValuations);
      })
      .catch(reason => { if (active) setError(typeof reason === "string" ? reason : t("Der Bewertungsverlauf konnte nicht geladen werden.")); })
      .finally(() => { if (active) setLoading(false); });
    return () => { active = false; };
  }, [position.accountId, position.id, revision, retry]);

  const date = (value: string) => new Intl.DateTimeFormat(locale(), { dateStyle: "medium", timeZone: "UTC" }).format(new Date(`${value}T00:00:00Z`));
  const closeEditor = () => {
    setEditing(null);
    requestAnimationFrame(() => editOrigin.current?.focus());
  };

  return <>
    <div className="manual-position-back">
      <button ref={backButton} className="secondary-button" type="button" onClick={onBack}>
        <span aria-hidden="true">←</span> {t("Zurück zu Positionen")}
      </button>
    </div>
    <article className="dashboard-card manual-valuation-history">
      <div className="valuation-history-heading">
        <div>
          <p className="eyebrow">{t("Bewertungsverlauf")}</p>
          <h2>{position.label}</h2>
          <p className="settings-hint">{t("Neue Stichtagswerte ergänzen dieselbe Position. Frühere Bewertungen bleiben erhalten.")}</p>
        </div>
        <div className="valuation-history-current">
          <span>{t("Aktueller Wert")}</span>
          <strong>{money(position.amountMinor, position.valueCurrency)}</strong>
          <small>{t("Bewertet am")} {date(position.valuationDate)}</small>
        </div>
      </div>
      {notice && <div className="position-save-feedback" role="status"><span aria-hidden="true">✓</span>{notice}</div>}
      {!editing && actionError && <p className="error-message" role="alert">{actionError}</p>}
      <ManualValueEditor
        key={`${position.id}-new-${revision}`}
        position={position}
        currency={currency}
        saving={saving}
        onCancel={onBack}
        onSave={onSave}
      />
      <section className="valuation-history-chart" aria-labelledby="valuation-history-chart-title">
        <div className="valuation-history-chart-heading">
          <div>
            <p className="eyebrow">{t("Entwicklung")}</p>
            <h3 id="valuation-history-chart-title">{t("Positionswert im Zeitverlauf")}</h3>
          </div>
          <div ref={setControls} className="valuation-history-chart-tools wealth-chart-tools" />
        </div>
        {error ? <div className="valuation-history-error" role="alert"><span>{error}</span><button className="secondary-button" type="button" onClick={() => setRetry(value => value + 1)}>{t("Erneut versuchen")}</button></div>
          : loading ? <p role="status" className="settings-hint">{t("Bewertungen werden geladen …")}</p>
          : chart?.history.length ? <InteractiveTimelineChart history={chart.history} currency={currency} controlsContainer={controls} minimumValueSpanRatio={0.2} ariaLabel={t("Positionswert im Zeitverlauf")} />
          : <p className="settings-hint">{t("Noch keine Bewertungen vorhanden.")}</p>}
      </section>
      <section className="valuation-history-list" aria-labelledby="valuation-history-list-title">
        <div className="position-section-heading">
          <div>
            <p className="eyebrow">{t("Stichtage")}</p>
            <h3 id="valuation-history-list-title">{t("Gespeicherte Bewertungen")}</h3>
            <p className="settings-hint">{t("Manuell eingegebene Stichtagswerte können nachträglich korrigiert werden.")}</p>
          </div>
        </div>
        {loading ? <p role="status" className="settings-hint">{t("Bewertungen werden geladen …")}</p>
          : valuations.length ? <div className="valuation-list-table"><table>
            <thead><tr><th>{t("Datum")}</th><th>{t("Gesamtwert")}</th><th>{t("Aktion")}</th></tr></thead>
            <tbody>{valuations.map(valuation => <tr key={valuation.id} className={editing?.id === valuation.id ? "selected" : undefined}>
              <td>{date(valuation.valueDate)}</td>
              <td>{money(valuation.amountMinor, valuation.currency)}</td>
              <td><button className="text-button" type="button" onClick={event => { editOrigin.current = event.currentTarget; onClearError(); setEditing(valuation); }}>{t("Bearbeiten")}</button></td>
            </tr>)}</tbody>
          </table></div>
          : <p className="settings-hint">{t("Keine gespeicherten Bewertungen vorhanden.")}</p>}
      </section>
    </article>
    {editing && <ManualValuationDialog
      position={position}
      valuation={editing}
      currency={currency}
      saving={saving}
      error={actionError}
      canDelete={valuations.length > 1}
      onClose={closeEditor}
      onSave={(valueDate, amount) => onUpdate(editing.id, valueDate, amount)}
      onDelete={() => onDelete(editing.id)}
    />}
  </>;
}
