// Verwaltet Positionen und Bewertungen eines einzelnen Kontos, getrennt von der Kontenübersicht.
import { useEffect, useRef, useState } from "react";
import { positionsApi } from "./api";
import { t, tr, locale } from "../../i18n";
import type { Account } from "../accounts/types";
import type { ManualPosition } from "./types";
import { money } from "../accounts/presentation";
import { assetTypes, valuationCurrencies, calculatedValuation, marketSourceLabel } from "./presentation";

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
  const [marketRefreshPending, setMarketRefreshPending] = useState(false);
  const [showPositionForm, setShowPositionForm] = useState(true);
  const [valuation, setValuation] = useState({
    id: null as number | null,
    label: "",
    amount: "",
    quantity: "",
    unitPrice: "",
    quoteCurrency: valuing.currency,
    exchangeRate: "1",
    date: new Date().toISOString().slice(0, 10),
    assetType: "other",
    identifierType: "ticker" as "isin" | "ticker",
    isin: "",
    ticker: "",
    method: "total" as "total" | "units",
    holdingStartDate: new Date().toISOString().slice(0, 10),
    holdingEndDate: "",
  });
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
      setError(
        t(
          "Die ISIN muss aus 12 Buchstaben und Ziffern bestehen und mit einem zweistelligen Ländercode beginnen.",
        ),
      );
      return;
    }
    if (
      useAutomaticPrice &&
      (quantity === null || !Number.isFinite(quantity) || quantity <= 0)
    ) {
      setError(t("Bitte eine gültige Menge eingeben."));
      return;
    }
    const holdingStartDate = useAutomaticPrice
      ? valuation.holdingStartDate
      : valuation.date;
    if (!holdingStartDate) {
      setError(t("Bitte ein Einstandsdatum eingeben."));
      return;
    }
    if (
      valuation.holdingEndDate &&
      valuation.holdingEndDate < holdingStartDate
    ) {
      setError(t("Das Verkaufsdatum darf nicht vor dem Einstandsdatum liegen."));
      return;
    }
    if (
      useUnits &&
      (quantity === null ||
        (!useAutomaticPrice && unitPrice === null) ||
        !Number.isFinite(quantity) ||
        (!useAutomaticPrice && !Number.isFinite(unitPrice)))
    ) {
      setError(t("Bitte Menge und Wert pro Einheit vollständig eingeben."));
      return;
    }
    if (
      useUnits &&
      !useAutomaticPrice &&
      (!Number.isFinite(exchangeRate) || exchangeRate <= 0)
    ) {
      setError(t("Bitte einen gültigen Wechselkurs eingeben."));
      return;
    }
    if (!Number.isFinite(amount) || amount < 0) {
      setError(t("Bitte einen gültigen Wert eingeben."));
      return;
    }
    setSaving(true);
    setError(null);
    try {
      await positionsApi.save({
          id: valuation.id,
          accountId: valuing.id,
          label: valuation.label,
          valuationDate: holdingStartDate,
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
      setValuation((current) => ({
        ...current,
        id: null,
        label: "",
        amount: "",
        quantity: "",
        unitPrice: "",
        isin: "",
        ticker: "",
        holdingStartDate: new Date().toISOString().slice(0, 10),
        holdingEndDate: "",
      }));
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
    } catch (reason) {
      setError(
        typeof reason === "string"
          ? reason
          : t("Der manuelle Wert konnte nicht gespeichert werden."),
      );
    } finally {
      setSaving(false);
    }
  }

  async function deletePosition(position: ManualPosition) {
    if (!window.confirm(t("Diese Position löschen?"))) return;
    await positionsApi.remove(position.id);
    const remainingPositions = await positionsApi.list(position.accountId);
    setPositions(remainingPositions);
    setShowPositionForm(remainingPositions.length === 0);
    await onChanged();
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
                    <div>
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
                    <b>
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
                    <button
                      className="text-button"
                      onClick={() => {
                        setValuationNotice(null);
                        setShowPositionForm(true);
                        setValuation((current) => ({
                          ...current,
                          id: position.id,
                          label: position.label,
                          amount: String(position.amountMinor / 100),
                          quantity:
                            position.quantity === null
                              ? ""
                              : String(position.quantity),
                          unitPrice:
                            position.unitPriceMinor === null
                              ? ""
                              : String(position.unitPriceMinor / 100),
                          quoteCurrency:
                            position.quoteCurrency ?? valuing.currency,
                          exchangeRate: String(position.exchangeRate ?? 1),
                          method:
                            position.quantity !== null ? "units" : "total",
                          assetType: position.assetType ?? "other",
                          identifierType: position.identifierType ?? "ticker",
                          isin:
                            position.identifierType === "isin"
                              ? (position.identifier ?? "")
                              : "",
                          ticker:
                            position.identifierType === "ticker"
                              ? (position.identifier ?? "")
                              : "",
                          holdingStartDate:
          position.holdingStartDate ?? position.valuationDate,
                          holdingEndDate: position.holdingEndDate ?? "",
                        }));
                      }}
                    >
                      {t("Bearbeiten")}
                    </button>
                    {!position.identifier && (
                      <button
                        className="text-button danger"
                        onClick={() => void deletePosition(position)}
                      >
                        {t("Löschen")}
                      </button>
                    )}
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
                onClick={() => {
                  setValuationNotice(null);
                  setShowPositionForm(true);
                }}
              >
                {t("Neue Position erfassen")}
              </button>
            </div>
          )}
          {showPositionForm && (
            <section className="position-form-section">
              <div className="position-section-heading">
                <div>
                  <p className="eyebrow">
                    {valuation.id ? t("Bearbeiten") : t("Neu erfassen")}
                  </p>
                  <h3>
                    {valuation.id
                      ? t("Position bearbeiten")
                      : t("Neue Position")}
                  </h3>
                </div>
              </div>
              <div className="valuation-setup">
                <label>
                  {t("Bezeichnung")}
                  <input
                    value={valuation.label}
                    onChange={(event) =>
                      setValuation({ ...valuation, label: event.target.value })
                    }
                    placeholder={t("z. B. Beispielaktie oder Mietkaution")}
                  />
                </label>
                <label>
                  {t("Anlagetyp")}
                  <select
                    value={valuation.assetType}
                    onChange={(event) =>
                      setValuation({
                        ...valuation,
                        assetType: event.target.value,
                        identifierType:
                          event.target.value === "crypto"
                            ? "ticker"
                            : valuation.identifierType,
                        isin: ["cash", "crypto"].includes(event.target.value)
                          ? ""
                          : valuation.isin,
                        ticker: event.target.value === "cash"
                          ? ""
                          : valuation.ticker,
                        method:
                          event.target.value === "cash"
                            ? "total"
                            : valuation.method,
                      })
                    }
                  >
                    {assetTypes.map(([value, label]) => (
                      <option value={value} key={value}>
                        {t(label)}
                      </option>
                    ))}
                  </select>
                </label>
                {valuation.assetType !== "cash" && (
                  <label>
                    {t(
                      valuation.assetType === "crypto"
                        ? "Yahoo-Ticker"
                        : "Wertpapierkennung",
                    )}
                    <div className="identifier-input">
                      <select
                        value={valuation.identifierType}
                        disabled={valuation.assetType === "crypto"}
                        onChange={(event) =>
                          setValuation({
                            ...valuation,
                            identifierType: event.target.value as
                              "isin" | "ticker",
                          })
                        }
                      >
                        <option value="isin">ISIN</option>
                        <option value="ticker">Ticker</option>
                      </select>
                      <input
                        maxLength={
                          valuation.identifierType === "isin" ? 12 : 32
                        }
                        value={
                          valuation.identifierType === "isin"
                            ? valuation.isin
                            : valuation.ticker
                        }
                        onChange={(event) =>
                          setValuation({
                            ...valuation,
                            [valuation.identifierType]:
                              event.target.value.toUpperCase(),
                            method: event.target.value.trim()
                              ? "units"
                              : valuation.method,
                          })
                        }
                        placeholder={
                          valuation.assetType === "crypto"
                            ? t("optional, z. B. BTC-USD")
                            : valuation.identifierType === "isin"
                            ? t("optional, z. B. CH0012032048")
                            : t("optional, z. B. XNAS:AAPL")
                        }
                      />
                    </div>
                    <small>
                      {t(
                        valuation.assetType === "crypto"
                          ? "Yahoo-Kryptowährungsticker, z. B. BTC-USD oder ETH-USD. Die Umrechnung in CHF erfolgt automatisch."
                          : "Kann später zur automatischen Kurszuordnung verwendet werden.",
                      )}
                    </small>
                  </label>
                )}
                <div className="valuation-method-block">
                  <span className="field-label">{t("Bewertungsmethode")}</span>
                  <div className="valuation-methods">
                    <button
                      type="button"
                      className={valuation.method === "total" ? "active" : ""}
                      onClick={() =>
                        setValuation({ ...valuation, method: "total" })
                      }
                    >
                      {t("Gesamtwert eingeben")}
                    </button>
                    {valuation.assetType !== "cash" && (
                      <button
                        type="button"
                        className={valuation.method === "units" ? "active" : ""}
                        onClick={() =>
                          setValuation({ ...valuation, method: "units" })
                        }
                      >
                        {t("Menge × Wert pro Einheit")}
                      </button>
                    )}
                  </div>
                </div>
              </div>
              <div className="management-form valuation-fields">
                {valuation.method === "total" ||
                valuation.assetType === "cash" ? (
                  <label>
                    {t("Gesamtwert")} ({valuing.currency})
                    <input
                      inputMode="decimal"
                      value={valuation.amount}
                      onChange={(event) =>
                        setValuation({
                          ...valuation,
                          amount: event.target.value,
                        })
                      }
                    />
                  </label>
                ) : (
                  <>
                    <label>
                      {t("Menge / Einheiten")}
                      <input
                        inputMode="decimal"
                        value={valuation.quantity}
                        onChange={(event) =>
                          setValuation({
                            ...valuation,
                            quantity: event.target.value,
                          })
                        }
                      />
                    </label>
                    {automaticValuation ? (
                      <label>
                        {t("Preis")}
                        <div className="amount-with-currency readonly-price">
                          <input
                            value={valuation.unitPrice}
                            placeholder={t("Noch nicht geladen")}
                            readOnly
                            aria-readonly="true"
                          />
                          <span>{valuation.quoteCurrency}</span>
                        </div>
                      </label>
                    ) : (
                      <>
                        <label>
                          {t("Wert pro Einheit")}
                          <div className="amount-with-currency">
                            <input
                              inputMode="decimal"
                              value={valuation.unitPrice}
                              onChange={(event) =>
                                setValuation({
                                  ...valuation,
                                  unitPrice: event.target.value,
                                })
                              }
                            />
                            <select
                              aria-label={t("Kurswährung")}
                              value={valuation.quoteCurrency}
                              onChange={(event) =>
                                setValuation({
                                  ...valuation,
                                  quoteCurrency: event.target.value,
                                  exchangeRate:
                                    event.target.value === valuing.currency
                                      ? "1"
                                      : valuation.exchangeRate,
                                })
                              }
                            >
                              {valuationCurrencies.map((currency) => (
                                <option key={currency}>{currency}</option>
                              ))}
                            </select>
                          </div>
                        </label>
                        {valuation.quoteCurrency !== valuing.currency && (
                          <label>
                            {tr`Umrechnung ${valuation.quoteCurrency} → ${valuing.currency}`}
                            <div className="exchange-rate-input">
                              <span>1 {valuation.quoteCurrency} =</span>
                              <input
                                aria-label={t("Wechselkurs")}
                                inputMode="decimal"
                                value={valuation.exchangeRate}
                                onChange={(event) =>
                                  setValuation({
                                    ...valuation,
                                    exchangeRate: event.target.value,
                                  })
                                }
                              />
                              <span>{valuing.currency}</span>
                            </div>
                            <small>
                              {t(
                                "Der Kurs rechnet den Anlagewert in die Kontowährung um.",
                              )}
                            </small>
                          </label>
                        )}
                        <div className="calculated-value">
                          <span>
                            {t("Umgerechneter Gesamtwert")} ({valuing.currency})
                          </span>
                          <strong>
                            {calculatedValuation(valuation, valuing.currency)}
                          </strong>
                        </div>
                      </>
                    )}
                  </>
                )}
                <div className="position-date-row">
                  <label>
                    {t("Einstandsdatum")}
                    <input
                      type="date"
                      lang={locale()}
                      value={
                        automaticValuation
                          ? valuation.holdingStartDate
                          : valuation.date
                      }
                      onChange={(event) =>
                        setValuation({
                          ...valuation,
                          date: event.target.value,
                          holdingStartDate: event.target.value,
                        })
                      }
                    />
                    <small>
                      {t(
                        "Ab dem Einstandsdatum zählt die Position zum Vermögen.",
                      )}
                    </small>
                  </label>
                  <label>
                    {t("Verkaufsdatum (optional)")}
                    <input
                      type="date"
                      lang={locale()}
                      min={
                        (automaticValuation
                          ? valuation.holdingStartDate
                          : valuation.date) || undefined
                      }
                      value={valuation.holdingEndDate}
                      onChange={(event) =>
                        setValuation({
                          ...valuation,
                          holdingEndDate: event.target.value,
                        })
                      }
                    />
                    <small>
                      {t(
                        "Ohne Verkaufsdatum bleibt die Position offen. Nach einem Verkauf bleibt ihre Historie erhalten.",
                      )}
                    </small>
                  </label>
                </div>
              </div>
              <div className="form-actions">
                {positions.length > 0 && (
                  <button
                    className="secondary-button"
                    type="button"
                    onClick={() => {
                      setValuationNotice(null);
                      setShowPositionForm(false);
                      setValuation((current) => ({
                        ...current,
                        id: null,
                        label: "",
                        amount: "",
                        quantity: "",
                        unitPrice: "",
                        isin: "",
                        ticker: "",
                      }));
                    }}
                  >
                    {t("Abbrechen")}
                  </button>
                )}
                <button
                  className="primary-button"
                  disabled={saving}
                  onClick={() => void saveValuation()}
                >
                  {t("Position speichern")}
                </button>
              </div>
            </section>
          )}
        </article>
      </>;
}
