// Stellt Vermögensanteile als gemeinsamen Donut und detaillierte, farbcodierte Liste dar.
import { locale, t } from "../../i18n";
import {
  DistributionDonut,
  type DonutSegment,
} from "../../shared/charts/DistributionDonut";
import { distributionColor } from "../../shared/charts/distributionColors";
import { ProviderLogo } from "../accounts/ProviderLogo";
import type { AssetAccount, Breakdown } from "./types";
import { money } from "./presentation";
export function BreakdownCard({
  title,
  values,
  total,
  currency,
  variant,
  accounts = [],
  translateTypes = false,
}: {
  translateTypes?: boolean;
  title: string;
  values: Breakdown[];
  total: number;
  currency: string;
  variant: "category" | "provider";
  accounts?: AssetAccount[];
}) {
  const sorted = [...values].sort(
    (left, right) => right.amountMinor - left.amountMinor,
  );
  const keys = values.map((value) => value.key).sort();
  const label = (value: Breakdown) =>
    translateTypes ? t(value.label) : value.label;
  const color = (key: string) => distributionColor(key, keys);
  const positive = sorted.filter((value) => value.amountMinor > 0);
  const positiveTotal = positive.reduce(
    (sum, value) => sum + value.amountMinor,
    0,
  );
  const segments: DonutSegment[] = positive
    .slice(0, variant === "category" ? 8 : positive.length)
    .map((value) => ({
      key: value.key,
      label: label(value),
      color: color(value.key),
      amountMinor: value.amountMinor,
      keys: [value.key],
    }));
  const remaining = variant === "category" ? positive.slice(8) : [];
  if (remaining.length) segments.push({
    key: "__remaining",
    label: t("Weitere Kategorien"),
    color: "var(--text-secondary)",
    amountMinor: remaining.reduce(
      (sum, value) => sum + value.amountMinor,
      0,
    ),
    keys: remaining.map((value) => value.key),
  });
  return (
    <article className="dashboard-card breakdown-card">
      <p className="eyebrow">{t("Aufteilung")}</p>
      <h2>{title}</h2>
      <div className="breakdown-card-content">
        <DistributionDonut
          segments={segments}
          total={total}
          currency={currency}
          label={t("Gesamtvermögen")}
        />
        <div className="breakdown-list">
          {sorted.map((value) => {
            const provider =
              variant === "provider"
                ? accounts.find(
                    (account) => account.providerKey === value.key,
                  )
                : undefined;
            return (
              <div
                className={`breakdown-list-row ${variant}`}
                key={value.key}
              >
                <span
                  className="breakdown-dot"
                  style={{ background: color(value.key) }}
                  aria-hidden="true"
                />
                {variant === "provider" && (
                  <ProviderLogo
                    name={value.label}
                    providerKey={value.key}
                    customLogo={provider?.logoDataUrl ?? null}
                  />
                )}
                <div className="breakdown-identity">
                  <strong>{label(value)}</strong>
                  <small>
                    {value.accountCount}{" "}
                    {value.accountCount === 1 ? t("Konto") : t("Konten")}
                  </small>
                </div>
                <div className="breakdown-value">
                  <strong>{money(value.amountMinor, currency)}</strong>
                  <small>
                    {value.amountMinor < 0
                      ? variant === "provider"
                        ? t("Negativer Saldo")
                        : "–"
                      : positiveTotal
                        ? new Intl.NumberFormat(locale(), {
                            style: "percent",
                            minimumFractionDigits: 1,
                            maximumFractionDigits: 1,
                          }).format(value.amountMinor / positiveTotal)
                        : "–"}
                  </small>
                </div>
              </div>
            );
          })}
        </div>
      </div>
      {variant === "provider" &&
        sorted.some((value) => value.amountMinor < 0) && (
          <p className="breakdown-note">
            {t(
              "Der Ring und die Prozentanteile zeigen nur positive Anbietersalden. Negative Salden sind im Gesamtvermögen enthalten.",
            )}
          </p>
        )}
      {variant === "provider" && !positiveTotal && (
        <p className="breakdown-note">
          {t("Keine positiven Anbietersalden vorhanden.")}
        </p>
      )}
      {variant === "category" &&
        sorted.some((value) => value.amountMinor < 0) && (
          <p className="breakdown-note">
            {t(
              "Der Ring zeigt positive Kategoriesummen. Negative Summen bleiben in der Liste und im Gesamtbetrag berücksichtigt.",
            )}
          </p>
        )}
    </article>
  );
}
