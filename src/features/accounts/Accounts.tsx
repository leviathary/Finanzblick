import { t, tr, locale } from "../../i18n";
import { useCallback, useEffect, useMemo, useState } from "react";
import { invoke } from "@tauri-apps/api/core";

import { useSettings } from "../../settings";
import { ProviderLogo } from "./ProviderLogo";

interface Account {
  id: number;
  institutionId: number;
  provider: string;
  providerKey: string;
  institutionType: string;
  name: string;
  accountType: string;
  currency: string;
  externalReference: string | null;
  isActive: boolean;
  includeInNetWorth: boolean;
  balanceMinor: number | null;
  balanceDate: string | null;
  importCount: number;
  manualValuationCount: number;
  manualQuantity: number | null;
  manualUnitPriceMinor: number | null;
  manualQuoteCurrency: string | null;
  manualExchangeRate: number | null;
  logoDataUrl: string | null;
}
interface ManualPosition {
  id: number;
  accountId: number;
  label: string;
  valuationDate: string;
  amountMinor: number;
  valueCurrency: string;
  quantity: number | null;
  unitPriceMinor: number | null;
  quoteCurrency: string | null;
  exchangeRate: number | null;
  assetType: string | null;
  identifierType: "isin" | "ticker" | null;
  identifier: string | null;
  priceSource: string | null;
  holdingStartDate: string | null;
  holdingEndDate: string | null;
}

interface MarketRefreshResult {
  updatedPositions: number;
  storedDays: number;
  skippedPositions: number;
  errors: string[];
}

const accountTypes = [
  ["cash", "Privatkonto"],
  ["savings", "Sparkonto"],
  ["portfolio", "Depot"],
  ["pillar3a", "Säule 3a"],
  ["mortgage", "Hypothek"],
  ["credit_card", "Kreditkarte"],
  ["manual_asset", "Manuell verwaltete Position"],
];
const assetTypes = [
  ["stock", "Aktie"],
  ["option", "Option"],
  ["crypto", "Kryptowährung"],
  ["cash", "Cash / Geldbetrag"],
  ["fund", "Fonds / ETF"],
  ["bond", "Obligation"],
  ["other", "Sonstige Anlage"],
];
const valuationCurrencies = [
  "CHF",
  "EUR",
  "USD",
  "AUD",
  "GBP",
  "CAD",
  "JPY",
  "NZD",
];

export function Accounts() {
  const { settings } = useSettings();
  const [accounts, setAccounts] = useState<Account[]>([]);
  const [editing, setEditing] = useState<Account | null>(null);
  const [showCreate, setShowCreate] = useState(false);
  const [error, setError] = useState<string | null>(null);
  const [saving, setSaving] = useState(false);
  const [valuing, setValuing] = useState<Account | null>(null);
  const [valuationNotice, setValuationNotice] = useState<string | null>(null);
  const [marketRefreshPending, setMarketRefreshPending] = useState(false);
  const [showPositionForm, setShowPositionForm] = useState(true);
  const [valuation, setValuation] = useState({
    id: null as number | null,
    label: "",
    amount: "",
    quantity: "",
    unitPrice: "",
    quoteCurrency: "CHF",
    exchangeRate: "1",
    date: new Date().toISOString().slice(0, 10),
    assetType: "other",
    identifierType: "isin" as "isin" | "ticker",
    isin: "",
    ticker: "",
    method: "total" as "total" | "units",
    holdingStartDate: new Date().toISOString().slice(0, 10),
    holdingEndDate: "",
  });
  const [positions, setPositions] = useState<ManualPosition[]>([]);
  const [draft, setDraft] = useState({
    institutionName: "",
    institutionType: "bank",
    accountName: "",
    accountType: "cash",
    currency: settings.defaultCurrency,
    externalReference: "",
  });

  const load = useCallback(async () => {
    try {
      setAccounts(await invoke<Account[]>("list_accounts"));
      setError(null);
    } catch (reason) {
      setError(
        typeof reason === "string"
          ? reason
          : t("Konten konnten nicht geladen werden."),
      );
    }
  }, []);
  useEffect(() => {
    void load();
  }, [load]);
  useEffect(() => {
    const refreshMarketValues = () => {
      void load();
      if (valuing) {
        void invoke<ManualPosition[]>("list_manual_positions", {
          accountId: valuing.id,
        }).then(setPositions);
      }
    };
    window.addEventListener("market-data-refreshed", refreshMarketValues);
    return () =>
      window.removeEventListener("market-data-refreshed", refreshMarketValues);
  }, [load, valuing]);
  const groups = useMemo(
    () =>
      Object.values(
        accounts.reduce<
          Record<string, { name: string; key: string; accounts: Account[] }>
        >((result, account) => {
          result[account.providerKey] ??= {
            name: account.provider,
            key: account.providerKey,
            accounts: [],
          };
          result[account.providerKey].accounts.push(account);
          return result;
        }, {}),
      ),
    [accounts],
  );
  const automaticValuation =
    valuation.method === "units" &&
    Boolean(
      (valuation.identifierType === "isin"
        ? valuation.isin
        : valuation.ticker
      ).trim(),
    );

  async function createAccount() {
    setSaving(true);
    setError(null);
    try {
      await invoke("create_account", {
        request: {
          ...draft,
          externalReference: draft.externalReference || null,
        },
      });
      setDraft({
        institutionName: "",
        institutionType: "bank",
        accountName: "",
        accountType: "cash",
        currency: settings.defaultCurrency,
        externalReference: "",
      });
      setShowCreate(false);
      await load();
    } catch (reason) {
      setError(
        typeof reason === "string"
          ? reason
          : t("Konto konnte nicht angelegt werden."),
      );
    } finally {
      setSaving(false);
    }
  }

  async function saveAccount(account: Account) {
    setSaving(true);
    setError(null);
    try {
      await invoke("update_account", {
        request: {
          id: account.id,
          name: account.name,
          institutionName: editing?.id === account.id ? account.provider : null,
          accountType: account.accountType,
          currency: account.currency,
          externalReference: account.externalReference || null,
          isActive: account.isActive,
          includeInNetWorth: account.includeInNetWorth,
        },
      });
      setEditing(null);
      await load();
    } catch (reason) {
      setError(
        typeof reason === "string"
          ? reason
          : t("Konto konnte nicht gespeichert werden."),
      );
    } finally {
      setSaving(false);
    }
  }

  async function toggle(
    account: Account,
    field: "isActive" | "includeInNetWorth",
  ) {
    await saveAccount({ ...account, [field]: !account[field] });
  }

  async function deleteAccount(account: Account) {
    if (
      !window.confirm(
        t(
          "Dieses Konto endgültig löschen? Diese Aktion kann nicht rückgängig gemacht werden.",
        ),
      )
    )
      return;
    setSaving(true);
    setError(null);
    try {
      await invoke("delete_account", { accountId: account.id });
      await load();
    } catch (reason) {
      setError(
        typeof reason === "string"
          ? reason
          : t("Konto konnte nicht gelöscht werden."),
      );
    } finally {
      setSaving(false);
    }
  }

  async function saveValuation() {
    if (!valuing) return;
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
      await invoke("save_manual_valuation", {
        request: {
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
        },
      });
      setPositions(
        await invoke<ManualPosition[]>("list_manual_positions", {
          accountId: valuing.id,
        }),
      );
      await load();
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
        void invoke<MarketRefreshResult>("refresh_market_data", { force: true })
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

  async function openValuation(account: Account) {
    setEditing(null);
    setValuing(account);
    setValuationNotice(null);
    setValuation({
      id: null,
      label: "",
      amount: "",
      quantity: "",
      unitPrice: "",
      quoteCurrency: account.currency,
      exchangeRate: "1",
      date: new Date().toISOString().slice(0, 10),
      assetType: "other",
      identifierType: "ticker",
      isin: "",
      ticker: "",
      method: "total",
      holdingStartDate: new Date().toISOString().slice(0, 10),
      holdingEndDate: "",
    });
    try {
      const loadedPositions = await invoke<ManualPosition[]>(
        "list_manual_positions",
        { accountId: account.id },
      );
      setPositions(loadedPositions);
      setShowPositionForm(loadedPositions.length === 0);
    } catch {
      setPositions([]);
      setShowPositionForm(true);
    }
  }

  async function deletePosition(position: ManualPosition) {
    if (!window.confirm(t("Diese Position löschen?"))) return;
    await invoke("delete_manual_position", { positionId: position.id });
    const remainingPositions = await invoke<ManualPosition[]>(
      "list_manual_positions",
      { accountId: position.accountId },
    );
    setPositions(remainingPositions);
    setShowPositionForm(remainingPositions.length === 0);
    await load();
  }

  async function setLogo(institutionId: number, file: File | null) {
    if (!file) return;
    if (file.size > 2 * 1024 * 1024) {
      setError(t("Das Logo darf maximal 2 MB gross sein."));
      return;
    }
    const dataUrl = await new Promise<string>((resolve, reject) => {
      const reader = new FileReader();
      reader.onload = () => resolve(String(reader.result));
      reader.onerror = () => reject(reader.error);
      reader.readAsDataURL(file);
    });
    setSaving(true);
    setError(null);
    try {
      await invoke("set_institution_logo", { institutionId, dataUrl });
      setEditing((current) =>
        current?.institutionId === institutionId
          ? { ...current, logoDataUrl: dataUrl }
          : current,
      );
      await load();
    } catch (reason) {
      setError(
        typeof reason === "string"
          ? reason
          : t("Das Logo konnte nicht gespeichert werden."),
      );
    } finally {
      setSaving(false);
    }
  }

  async function removeLogo(institutionId: number) {
    setSaving(true);
    setError(null);
    try {
      await invoke("set_institution_logo", { institutionId, dataUrl: null });
      setEditing((current) =>
        current?.institutionId === institutionId
          ? { ...current, logoDataUrl: null }
          : current,
      );
      await load();
    } catch (reason) {
      setError(
        typeof reason === "string"
          ? reason
          : t("Das Logo konnte nicht entfernt werden."),
      );
    } finally {
      setSaving(false);
    }
  }

  return (
    <section className="accounts-page">
      <div className="overview-heading">
        <div>
          <p className="eyebrow">{t("Verwaltung")}</p>
          <h1>{t("Banken & Konten")}</h1>
          <p className="intro">
            {t(
              "Verwalte deine Bankbeziehungen und bestimme, welche Konten zum Gesamtvermögen zählen.",
            )}
          </p>
          <p className="accounts-archive-hint">
            {t(
              "Archivierte Konten bleiben mit ihrer Historie erhalten, erscheinen aber nicht mehr als aktive Konten. Konten ohne Importe können endgültig gelöscht werden.",
            )}
          </p>
        </div>
        <button className="primary-button" onClick={() => setShowCreate(true)}>
          {t("Konto hinzufügen")}
        </button>
      </div>
      {error && <p className="error-message">{t(error)}</p>}
      {showCreate && (
        <article className="dashboard-card account-form">
          <div className="card-heading">
            <h2>{t("Neues Konto")}</h2>
            <button
              className="text-button"
              onClick={() => setShowCreate(false)}
            >
              {t("Schliessen")}
            </button>
          </div>
          <div className="management-form">
            <label>
              {t("Bank oder Anbieter")}
              <input
                value={draft.institutionName}
                onChange={(event) =>
                  setDraft({ ...draft, institutionName: event.target.value })
                }
                placeholder={t(
                  draft.institutionType === "self_custody"
                    ? "z. B. eigene Verwaltung"
                    : "z. B. ZKB",
                )}
              />
            </label>
            <label>
              {t("Anbietertyp")}
              <select
                value={draft.institutionType}
                onChange={(event) => {
                  const institutionType = event.target.value;
                  setDraft({
                    ...draft,
                    institutionType,
                    accountType:
                      institutionType === "self_custody"
                        ? "manual_asset"
                        : draft.accountType,
                    currency:
                      institutionType === "self_custody"
                        ? "USD"
                        : draft.currency,
                    externalReference:
                      institutionType === "self_custody"
                        ? ""
                        : draft.externalReference,
                  });
                }}
              >
                <option value="bank">{t("Bank")}</option>
                <option value="insurance">{t("Versicherung")}</option>
                <option value="broker">Broker</option>
                <option value="pension">{t("Vorsorge")}</option>
                <option value="self_custody">
                  {t("Selbstverwahrung (eigene Wallet)")}
                </option>
              </select>
            </label>
            <label>
              {t("Kontoname")}
              <input
                value={draft.accountName}
                onChange={(event) =>
                  setDraft({ ...draft, accountName: event.target.value })
                }
                placeholder={t("z. B. Sparkonto")}
              />
            </label>
            <label>
              {t("Kontotyp")}
              <select
                value={draft.accountType}
                onChange={(event) =>
                  setDraft({ ...draft, accountType: event.target.value })
                }
              >
                {accountTypes.map(([value, label]) => (
                  <option value={value} key={value}>
                    {t(label)}
                  </option>
                ))}
              </select>
            </label>
            <label>
              {t("Währung")}
              <input
                maxLength={3}
                value={draft.currency}
                onChange={(event) =>
                  setDraft({
                    ...draft,
                    currency: event.target.value.toUpperCase(),
                  })
                }
              />
            </label>
            {draft.accountType !== "manual_asset" && (
              <label>
                {t("IBAN / Vertragsnummer")}
                <input
                  value={draft.externalReference}
                  onChange={(event) =>
                    setDraft({
                      ...draft,
                      externalReference: event.target.value,
                    })
                  }
                  placeholder="optional"
                />
              </label>
            )}
          </div>
          <div className="form-actions">
            <button
              className="primary-button"
              disabled={saving}
              onClick={createAccount}
            >
              {saving ? t("Wird gespeichert…") : t("Konto anlegen")}
            </button>
          </div>
        </article>
      )}
      {editing && (
        <article className="dashboard-card account-edit-screen">
          <div className="card-heading">
            <div>
              <p className="eyebrow">{t("Konto bearbeiten")}</p>
              <h2>
                {editing.provider} · {editing.name}
              </h2>
            </div>
          </div>
          <AccountEditor
            account={editing}
            setAccount={setEditing}
            saving={saving}
            onCancel={() => setEditing(null)}
            onSave={() => void saveAccount(editing)}
            onSetLogo={(file) => void setLogo(editing.institutionId, file)}
            onRemoveLogo={() => void removeLogo(editing.institutionId)}
            onValue={() => void openValuation(editing)}
          />
        </article>
      )}
      {valuing && (
        <article className="dashboard-card manual-valuation-form">
          <div className="card-heading">
            <div>
              <p className="eyebrow">{t("Manuelle Positionen")}</p>
              <h2>
                {valuing.provider} · {valuing.name}
              </h2>
            </div>
            <button className="text-button" onClick={() => setValuing(null)}>
              {t("Abbrechen")}
            </button>
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
      )}
      {!valuing && !editing && (
        <div className="institution-list">
          {groups.map((group) => (
            <article className="institution-card" key={group.key}>
              <header>
                <ProviderLogo
                  name={group.name}
                  providerKey={group.key}
                  customLogo={group.accounts[0]?.logoDataUrl}
                />
                <div>
                  <h2>{group.name}</h2>
                  <small>
                    {group.accounts.length}{" "}
                    {group.accounts.length === 1 ? t("Konto") : t("Konten")}
                  </small>
                </div>
              </header>
              {group.accounts.map((account) => (
                <div
                  className={`managed-account${account.isActive ? "" : " archived"}`}
                  key={account.id}
                >
                  <div>
                    <strong>{account.name}</strong>
                    <small>
                      {typeLabel(account.accountType)}
                      {account.externalReference
                        ? ` · ${account.externalReference}`
                        : ""}
                    </small>
                    <span>
                      {account.accountType === "manual_asset"
                        ? `${account.manualValuationCount} ${account.manualValuationCount === 1 ? t("Position") : t("Positionen")}${account.manualQuantity !== null && account.manualUnitPriceMinor !== null ? ` · ${account.manualQuantity.toLocaleString(locale())} ${t("Einheiten")} × ${money(account.manualUnitPriceMinor, account.manualQuoteCurrency ?? account.currency)}` : ""}`
                        : `${account.importCount} ${account.importCount === 1 ? t("Import") : t("Importe")}`}
                      {!account.includeInNetWorth
                        ? t(" · Nicht im Vermögen")
                        : ""}
                      {!account.isActive ? t(" · Archiviert") : ""}
                    </span>
                  </div>
                  <b>{money(account.balanceMinor, account.currency)}</b>
                  <div className="account-actions">
                    <button
                      className="text-button"
                      onClick={() => setEditing({ ...account })}
                    >
                      {t("Bearbeiten")}
                    </button>
                    <button
                      className="text-button"
                      onClick={() => void toggle(account, "includeInNetWorth")}
                    >
                      {account.includeInNetWorth
                        ? t("Aus Vermögen")
                        : t("Zum Vermögen")}
                    </button>
                    {account.importCount === 0 &&
                    account.manualValuationCount === 0 ? (
                      <button
                        className="text-button danger"
                        disabled={saving}
                        onClick={() => void deleteAccount(account)}
                      >
                        {t("Löschen")}
                      </button>
                    ) : (
                      <button
                        className="text-button danger"
                        onClick={() => void toggle(account, "isActive")}
                      >
                        {account.isActive ? t("Archivieren") : t("Aktivieren")}
                      </button>
                    )}
                  </div>
                </div>
              ))}
            </article>
          ))}
        </div>
      )}
    </section>
  );
}

function marketSourceLabel(source: string): string {
  return source === "alpha_vantage"
    ? "Alpha Vantage"
    : source === "marketstack"
      ? "Marketstack"
      : source === "yahoo"
        ? "Yahoo Finance"
        : source;
}

function AccountEditor({
  account,
  setAccount,
  saving,
  onCancel,
  onSave,
  onSetLogo,
  onRemoveLogo,
  onValue,
}: {
  account: Account;
  setAccount: (account: Account) => void;
  saving: boolean;
  onCancel: () => void;
  onSave: () => void;
  onSetLogo: (file: File | null) => void;
  onRemoveLogo: () => void;
  onValue: () => void;
}) {
  return (
    <div className="account-editor">
      <div className="management-form compact">
        <label>
          {t("Kontoname")}
          <input
            value={account.name}
            onChange={(event) =>
              setAccount({ ...account, name: event.target.value })
            }
          />
        </label>
        <label>
          {t("Kontotyp")}
          <select
            value={account.accountType}
            onChange={(event) =>
              setAccount({
                ...account,
                accountType: event.target.value,
              })
            }
          >
            {accountTypes.map(([value, label]) => (
              <option value={value} key={value}>
                {t(label)}
              </option>
            ))}
          </select>
        </label>
        <label>
          {t("Währung")}
          <input
            maxLength={3}
            value={account.currency}
            onChange={(event) =>
              setAccount({
                ...account,
                currency: event.target.value.toUpperCase(),
              })
            }
          />
        </label>
        {account.accountType !== "manual_asset" && (
          <label>
            {t("IBAN / Vertragsnummer")}
            <input
              value={account.externalReference ?? ""}
              onChange={(event) =>
                setAccount({
                  ...account,
                  externalReference: event.target.value,
                })
              }
            />
          </label>
        )}
      </div>
      {account.accountType === "manual_asset" && (
        <div className="account-value-editor">
          <div>
            <strong>{t("Positionen")}</strong>
            <small>
              {account.balanceDate
                ? `${t("Letzter Stand")}: ${account.balanceDate} · ${money(account.balanceMinor, account.currency)}`
                : t("Noch keine Position erfasst.")}
            </small>
          </div>
          <button className="secondary-button" type="button" onClick={onValue}>
            {t("Positionen verwalten")}
          </button>
        </div>
      )}
      <div className="account-logo-editor">
        <div className="form-grid institution-name-field">
          <label>
            {t("Bank / Anbieter")}
            <input maxLength={120} disabled={saving} value={account.provider} onChange={(event) => setAccount({ ...account, provider: event.target.value })} />
          </label>
        </div>
        <small>{t("Der Name gilt für alle Konten dieser Bank oder dieses Anbieters.")}</small>
        <span>{t("Logo der Bank oder des Anbieters")}</span>
        <label className="secondary-button">
          {t("Eigenes Logo hochladen")}
          <input
            type="file"
            accept="image/png,image/jpeg,image/webp,image/svg+xml"
            disabled={saving}
            onChange={(event) => {
              onSetLogo(event.target.files?.[0] ?? null);
              event.currentTarget.value = "";
            }}
          />
        </label>
        {account.logoDataUrl && (
          <button
            className="text-button danger"
            disabled={saving}
            onClick={onRemoveLogo}
          >
            {t("Logo entfernen")}
          </button>
        )}
        <small>{t("PNG, JPEG, WebP oder SVG · maximal 2 MB")}</small>
      </div>
      <div className="form-actions">
        <button className="secondary-button" onClick={onCancel}>
          {t("Abbrechen")}
        </button>
        <button className="primary-button" disabled={saving} onClick={onSave}>
          {t("Speichern")}
        </button>
      </div>
    </div>
  );
}

function typeLabel(value: string) {
  return t(accountTypes.find(([key]) => key === value)?.[1] ?? value);
}
function money(value: number | null, currency: string) {
  return value === null
    ? t("Noch kein Saldo")
    : new Intl.NumberFormat(locale(), { style: "currency", currency }).format(
        value / 100,
      );
}
function calculatedValuation(
  valuation: {
    quantity: string;
    unitPrice: string;
    exchangeRate: string;
    quoteCurrency: string;
  },
  currency: string,
) {
  const quantity = Number(valuation.quantity.replace(",", "."));
  const price = Number(valuation.unitPrice.replace(",", "."));
  const rate =
    valuation.quoteCurrency === currency
      ? 1
      : Number(valuation.exchangeRate.replace(",", "."));
  return valuation.quantity &&
    valuation.unitPrice &&
    Number.isFinite(quantity * price * rate)
    ? money(Math.round(quantity * price * rate * 100), currency)
    : "–";
}
