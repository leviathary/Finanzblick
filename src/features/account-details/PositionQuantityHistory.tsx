// Zeigt die datierten Bestandsänderungen einer ausgewählten Depotposition als reine Lesehistorie.
import { useEffect, useState } from "react";
import { invoke } from "@tauri-apps/api/core";
import { locale, t } from "../../i18n";
import type { PositionQuantityChange, PositionQuantityChangeKind } from "./model";

const formatDate = (value: string) => new Intl.DateTimeFormat(locale()).format(new Date(`${value.slice(0, 10)}T00:00:00`));
const formatNumber = (value: number) => new Intl.NumberFormat(locale(), { maximumFractionDigits: 9 }).format(value);

function kindLabel(kind: PositionQuantityChangeKind) {
  if (kind === "opening") return t("Anfangsbestand");
  if (kind === "purchase") return t("Kauf");
  if (kind === "sale") return t("Verkauf");
  return t("Bestandsanpassung");
}

function formatChange(row: PositionQuantityChange) {
  const amount = formatNumber(Math.abs(row.change));
  if (row.kind === "opening") return amount;
  if (row.change > 0) return `+${amount}`;
  if (row.change < 0) return `−${amount}`;
  return amount;
}

export function PositionQuantityHistory({ positionId, revision }: { positionId: number; revision: number }) {
  const [rows, setRows] = useState<PositionQuantityChange[] | null>(null);
  const [error, setError] = useState("");
  const [retry, setRetry] = useState(0);

  useEffect(() => {
    let cancelled = false;
    setRows(null);
    setError("");
    void invoke<PositionQuantityChange[]>("position_quantity_history", { positionId })
      .then(result => { if (!cancelled) setRows(result); })
      .catch(() => { if (!cancelled) setError(t("Bestandsänderungen konnten nicht geladen werden.")); });
    return () => { cancelled = true; };
  }, [positionId, revision, retry]);

  return <article className="dashboard-card position-quantity-history">
    <h3>{t("Bestandsänderungen")}</h3>
    {error ? <div className="error-banner" role="alert">{error}<button className="secondary-button" onClick={() => setRetry(value => value + 1)}>{t("Erneut versuchen")}</button></div>
      : rows === null ? <p role="status">{t("Daten werden geladen …")}</p>
      : rows.length === 0 ? <p>{t("Keine Bestandsänderungen vorhanden.")}</p>
      : <div className="detail-table-scroll"><table className="detail-table position-change-table">
        <thead><tr><th>{t("Datum")}</th><th>{t("Vorgang")}</th><th>{t("Anzahl")}</th><th>{t("Bestand danach")}</th></tr></thead>
        <tbody>{rows.map(row => <tr key={row.id}><td>{formatDate(row.date)}</td><td>{kindLabel(row.kind)}</td><td>{formatChange(row)}</td><td>{formatNumber(row.balance)}</td></tr>)}</tbody>
      </table></div>}
  </article>;
}
