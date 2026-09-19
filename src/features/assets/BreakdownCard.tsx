// Stellt eine vorberechnete Vermögensaufteilung dar.
import { t } from "../../i18n";
import type { Breakdown } from "./types";
import { money } from "./presentation";
export function BreakdownCard({
  title,
  values,
  total,
  currency,
  translateTypes = false,
}: {
  translateTypes?: boolean;
  title: string;
  values: Breakdown[];
  total: number;
  currency: string;
}) {
  const scale = Math.max(
    ...values.map((value) => Math.abs(value.amountMinor)),
    1,
  );
  return (
    <article className="dashboard-card breakdown-card">
      <p className="eyebrow">{t("Aufteilung")}</p>
      <h2>{title}</h2>
      <div>
        {values.map((value) => (
          <div className="breakdown-row" key={value.key}>
            <div>
              <strong>{translateTypes ? t(value.label) : value.label}</strong>
              <span>{money(value.amountMinor, currency)}</span>
            </div>
            <div className="breakdown-bar">
              <span
                style={{
                  width: `${(Math.abs(value.amountMinor) / scale) * 100}%`,
                }}
              />
            </div>
            <small>
              {total
                ? `${((value.amountMinor / total) * 100).toFixed(1)} %`
                : "–"}{" "}
              · {value.accountCount}{" "}
              {value.accountCount === 1 ? t("Konto") : t("Konten")}
            </small>
          </div>
        ))}
      </div>
    </article>
  );
}
