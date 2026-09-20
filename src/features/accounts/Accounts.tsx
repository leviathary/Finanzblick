// Verwaltet Banken, Konten, Vermögenseinbezug und manuell gepflegte Positionen.

import { t, locale } from "../../i18n";
import { useCallback, useEffect, useMemo, useState } from "react";
import { invoke } from "@tauri-apps/api/core";

import { useSettings } from "../../settings";
import { ActionMenu } from "../../shared/ActionMenu";
import { ProviderLogo } from "./ProviderLogo";

import type { Account } from "./types";
import { ManualPositions } from "../positions/ManualPositions";
import { AccountEditor } from "./AccountEditor";
import { accountTypes, supportsManualValuation, typeLabel, money } from "./presentation";

function accountMetadata(account: Account) {
  const activity = supportsManualValuation(account.accountType)
    ? `${account.manualValuationCount} ${account.manualValuationCount === 1 ? t("Position") : t("Positionen")}${account.manualQuantity !== null && account.manualUnitPriceMinor !== null ? ` · ${account.manualQuantity.toLocaleString(locale())} ${t("Einheiten")} × ${money(account.manualUnitPriceMinor, account.manualQuoteCurrency ?? account.currency)}` : ""}`
    : `${account.importCount} ${account.importCount === 1 ? t("Import") : t("Importe")}`;
  return [typeLabel(account.accountType), account.externalReference, activity]
    .filter(Boolean)
    .join(" · ");
}

export function Accounts() {
  const { settings } = useSettings();
  const [accounts, setAccounts] = useState<Account[]>([]);
  const [editing, setEditing] = useState<Account | null>(null);
  const [showCreate, setShowCreate] = useState(false);
  const [error, setError] = useState<string | null>(null);
  const [saving, setSaving] = useState(false);
  const [valuing, setValuing] = useState<Account | null>(null);
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
    };
    window.addEventListener("market-data-refreshed", refreshMarketValues);
    return () =>
      window.removeEventListener("market-data-refreshed", refreshMarketValues);
  }, [load]);
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
    if (field === "isActive" && account.isActive && !window.confirm(t("Dieses Konto archivieren? Die Historie bleibt erhalten."))) return;
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

  function openValuation(account: Account) {
    setEditing(null);
    setError(null);
    setValuing(account);
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
              {draft.accountType === "pillar3a" && (
                <small>
                  {t("Vorsorgekonten werden standardmässig nicht zum Gesamtvermögen gezählt.")}
                </small>
              )}
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
            {!supportsManualValuation(draft.accountType) && (
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
        <ManualPositions key={valuing.id} account={valuing} onClose={() => setValuing(null)} onChanged={load} onError={setError} />
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
                    · {group.accounts.length}{" "}
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
                    <div className="managed-account-meta">
                      <span>{accountMetadata(account)}</span>
                      {!account.includeInNetWorth && (
                        <span className="account-status-badge">{t("Nicht im Gesamtvermögen")}</span>
                      )}
                      {!account.isActive && (
                        <span className="account-status-badge">{t("Archiviert")}</span>
                      )}
                    </div>
                  </div>
                  <b className={`managed-account-amount${account.balanceMinor === null || account.balanceMinor === 0 ? " zero" : ""}`}>
                    {money(account.balanceMinor ?? 0, account.balanceCurrency)}
                  </b>
                  <ActionMenu label={t("Aktionen") + ": " + account.name} disabled={saving} actions={[
                    { label: t("Bearbeiten"), onClick: () => setEditing({ ...account }) },
                    { label: account.includeInNetWorth ? t("Vom Gesamtvermögen ausschließen") : t("Zum Gesamtvermögen zählen"), onClick: () => void toggle(account, "includeInNetWorth") },
                    account.importCount === 0 && account.manualValuationCount === 0
                      ? { label: t("Löschen"), onClick: () => void deleteAccount(account), separated: true, danger: true }
                      : { label: account.isActive ? t("Archivieren") : t("Aktivieren"), onClick: () => void toggle(account, "isActive"), separated: true, danger: account.isActive },
                  ]} />
                </div>
              ))}
            </article>
          ))}
        </div>
      )}
    </section>
  );
}
