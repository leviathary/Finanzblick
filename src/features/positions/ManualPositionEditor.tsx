// Rendert und aktualisiert das Formular zum Erfassen oder Bearbeiten einer Depotposition.

import type { Dispatch, SetStateAction } from "react";

import { locale, t, tr } from "../../i18n";
import type { Account } from "../accounts/types";
import { assetTypes, calculatedValuation, valuationCurrencies } from "./presentation";

export interface ManualPositionDraft {
  id: number | null;
  label: string;
  amount: string;
  quantity: string;
  unitPrice: string;
  quoteCurrency: string;
  exchangeRate: string;
  date: string;
  assetType: string;
  identifierType: "isin" | "ticker";
  isin: string;
  ticker: string;
  method: "total" | "units";
  holdingStartDate: string;
  holdingEndDate: string;
}

interface Props {
  account: Account;
  positionsExist: boolean;
  valuation: ManualPositionDraft;
  setValuation: Dispatch<SetStateAction<ManualPositionDraft>>;
  automaticValuation: boolean;
  saving: boolean;
  setValuationNotice: (notice: string | null) => void;
  setShowPositionForm: (show: boolean) => void;
  onSave: () => Promise<void>;
}

export function ManualPositionEditor({ account, positionsExist, valuation, setValuation, automaticValuation, saving, setValuationNotice, setShowPositionForm, onSave }: Props) {
  return (
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
                    {t("Gesamtwert")} ({account.currency})
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
                                    event.target.value === account.currency
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
                        {valuation.quoteCurrency !== account.currency && (
                          <label>
                            {tr`Umrechnung ${valuation.quoteCurrency} → ${account.currency}`}
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
                              <span>{account.currency}</span>
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
                            {t("Umgerechneter Gesamtwert")} ({account.currency})
                          </span>
                          <strong>
                            {calculatedValuation(valuation, account.currency)}
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
                {positionsExist && (
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
                  onClick={() => void onSave()}
                >
                  {t("Position speichern")}
                </button>
              </div>
            </section>
  );
}
