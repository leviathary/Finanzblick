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
  onValue,
}: {
  account: Account;
  setAccount: (account: Account) => void;
  saving: boolean;
  onCancel: () => void;
  onSave: () => void;
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
