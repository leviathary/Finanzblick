// Zustandsloses Bearbeitungsformular für ein Konto; Speichern übernimmt die Kontenansicht.
import { t } from "../../i18n";
import type { Account } from "./types";
import type { AccountType } from "../../domain/finance";
import { accountTypes, supportsManualValuation, money } from "./presentation";
export function AccountEditor({
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
            onChange={(event) => {
              const accountType = event.target.value as AccountType;
              setAccount({
                ...account,
                accountType,
                includeInNetWorth:
                  accountType === "pillar3a" && account.accountType !== "pillar3a"
                    ? false
                    : account.includeInNetWorth,
              });
            }}
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
        {!supportsManualValuation(account.accountType) && (
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
      {supportsManualValuation(account.accountType) && (
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
      <label className="account-net-worth-toggle">
        <span>
          <strong>{t("Im Gesamtvermögen berücksichtigen")}</strong>
          <small>
            {account.accountType === "pillar3a"
              ? t("Vorsorgevermögen ist standardmässig ausgeblendet und kann hier einbezogen werden.")
              : t("Dieses Konto in Summen und Vermögensauswertungen einbeziehen.")}
          </small>
        </span>
        <input
          type="checkbox"
          role="switch"
          checked={account.includeInNetWorth}
          onChange={(event) =>
            setAccount({ ...account, includeInNetWorth: event.target.checked })
          }
        />
      </label>
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
