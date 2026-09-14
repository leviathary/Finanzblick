import { t, tr, locale } from "../../i18n";
import { useEffect, useState } from "react";
import { invoke, isTauri } from "@tauri-apps/api/core";
import { ProviderLogo } from "../accounts/ProviderLogo";

interface AccountSummary {
  id: number;
  provider: string;
  providerKey: string;
  name: string;
  accountType: string;
  currency: string;
  balanceCurrency: string;
  balanceMinor: number | null;
  balanceDate: string | null;
  includeInNetWorth: boolean;
  logoDataUrl: string | null;
}

interface ProviderSummary {
  provider: string;
  providerKey: string;
  balanceMinor: number;
  accountCount: number;
  logoDataUrl: string | null;
}

interface DashboardData {
  totalBalanceMinor: number;
  currency: string;
  accounts: AccountSummary[];
  providers: ProviderSummary[];
}

export function Overview({ onImport }: { onImport: () => void }) {
  const [data, setData] = useState<DashboardData | null>(null);
  const [error, setError] = useState<string | null>(null);

  useEffect(() => {
    if (!isTauri()) {
      setError(t("Die lokalen Finanzdaten sind in der Desktop-App verfügbar."));
      return;
    }
    const load = () =>
      void invoke<DashboardData>("dashboard_data")
        .then(setData)
        .catch((reason) =>
          setError(
            typeof reason === "string"
              ? reason
              : t("Die Übersicht konnte nicht geladen werden."),
          ),
        );
    load();
    window.addEventListener("market-data-refreshed", load);
    return () => window.removeEventListener("market-data-refreshed", load);
  }, []);

  if (error)
    return (
      <section className="overview">
        <p className="error-message">{t(error)}</p>
      </section>
    );
  if (!data)
    return (
      <section className="overview">
        <p className="intro">{t("Übersicht wird geladen…")}</p>
      </section>
    );

  if (data.accounts.length === 0) {
    return (
      <section className="overview empty-overview">
        <p className="eyebrow">{t("Übersicht")}</p>
        <h1>{t("Deine Finanzen an einem Ort")}</h1>
        <p className="intro">
          {t(
            "Importiere deinen ersten Bankauszug, damit Konten und Vermögen hier erscheinen.",
          )}
        </p>
        <button className="primary-button" type="button" onClick={onImport}>
          {t("Ersten Auszug importieren")}
        </button>
      </section>
    );
  }

  const includedAccounts = data.accounts.filter(
    (account) => account.includeInNetWorth,
  );
  const valuedAccounts = includedAccounts.filter(
    (account) => account.balanceMinor !== null,
  ).length;
  const totalProviderScale = Math.max(Math.abs(data.totalBalanceMinor), 1);
  return (
    <section className="overview">
      <div className="overview-heading">
        <div>
          <p className="eyebrow">{t("Übersicht")}</p>
          <h1>{t("Dein Vermögen")}</h1>
        </div>
      </div>

      <div className="summary-grid overview-summary-grid">
        <article className="summary-card primary">
          <span>{t("Gesamtvermögen")}</span>
          <strong>{formatMoney(data.totalBalanceMinor, data.currency)}</strong>
          <small>
            {data.accounts.length}{" "}
            {data.accounts.length === 1 ? t("Konto") : t("Konten")} {t("bei")}{" "}
            {data.providers.length}{" "}
            {data.providers.length === 1 ? t("Anbieter") : t("Anbietern")}
          </small>
        </article>
        <article className="summary-card">
          <span>{t("Konten im Vermögen")}</span>
          <strong>{includedAccounts.length}</strong>
          <small>
            {valuedAccounts} {t("mit aktuellem Wert")}
          </small>
        </article>
        <article className="summary-card">
          <span>{t("Anbieter im Vermögen")}</span>
          <strong>{data.providers.length}</strong>
          <small>{t("im Gesamtvermögen berücksichtigt")}</small>
        </article>
      </div>

      <article className="dashboard-card overview-section-card">
        <div className="card-heading">
          <div>
            <p className="eyebrow">{t("Aufteilung")}</p>
            <h2>{t("Vermögen nach Anbieter")}</h2>
          </div>
        </div>
        <div className="provider-list">
          {data.providers.map((provider) => (
            <div className="provider-row" key={provider.providerKey}>
              <div className="provider-label">
                <ProviderLogo
                  name={provider.provider}
                  providerKey={provider.providerKey}
                  customLogo={provider.logoDataUrl}
                />
                <div>
                  <strong>{provider.provider}</strong>
                  <small>
                    {provider.accountCount}{" "}
                    {provider.accountCount === 1 ? t("Konto") : t("Konten")}
                  </small>
                </div>
                <b>{formatMoney(provider.balanceMinor, data.currency)}</b>
              </div>
              <div className="provider-bar">
                <span
                  style={{
                    width: `${Math.min(100, (Math.abs(provider.balanceMinor) / totalProviderScale) * 100)}%`,
                  }}
                />
              </div>
            </div>
          ))}
        </div>
      </article>

      <article className="dashboard-card accounts-card overview-section-card">
        <div className="card-heading">
          <div>
            <p className="eyebrow">{t("Konten")}</p>
            <h2>{t("Aktuelle Salden")}</h2>
          </div>
          <span>
            {data.accounts.length} {t("insgesamt")}
          </span>
        </div>
        <div className="account-table">
          {data.accounts.map((account) => (
            <div className="account-row" key={account.id}>
              <div className="account-identity">
                <ProviderLogo
                  name={account.provider}
                  providerKey={account.providerKey}
                  customLogo={account.logoDataUrl}
                />
                <div>
                  <strong>{account.name}</strong>
                  <small>
                    {account.provider} · {accountTypeLabel(account.accountType)}
                  </small>
                </div>
              </div>
              <span>
                {account.balanceDate
                  ? tr`Stand ${formatDate(account.balanceDate)}`
                  : t("Kein Saldo")}
              </span>
              <b>{formatMoney(account.balanceMinor, account.balanceCurrency)}</b>
            </div>
          ))}
        </div>
      </article>
    </section>
  );
}

function formatMoney(value: number | null, currency: string): string {
  if (value === null) return "–";
  return new Intl.NumberFormat(locale(), {
    style: "currency",
    currency,
  }).format(value / 100);
}

function formatDate(value: string): string {
  const [year, month, day] = value.slice(0, 10).split("-");
  return year && month && day ? `${day}.${month}.${year}` : value;
}

function accountTypeLabel(value: string): string {
  return value === "pillar3a"
    ? t("Säule 3a")
    : value === "cash"
      ? t("Konto")
      : value === "credit_card"
        ? t("Kreditkarte")
        : value === "manual_asset"
          ? t("Manuell verwaltete Position")
          : value;
}
