// Direkte Einfachauswahl eines Buchungsmusters; Vorschläge werden nie automatisch ausgewählt.
import { t, tr } from "../../../i18n";
import type { BankPaymentSort } from "./model";
import type { Row } from "../types";
import { date, money, kindLabels } from "../presentation";

interface Props {
  rows: Row[];
  selectedId: string;
  disabled: boolean;
  onSelect: (id: string) => void;
  sort?: BankPaymentSort;
  onSort?: (sort: BankPaymentSort) => void;
}

export function PatternTable({ rows, selectedId, disabled, onSelect, sort, onSort }: Props) {
  return <div className="transfer-table-scroll pattern-table-scroll"><table className="transfer-table pattern-table">
    <thead><tr>{["Datum", "Beschreibung", "Betrag", "Einordnung", "Aktionen"].map(label => {
      const sortable = !!onSort && (label === "Datum" || label === "Betrag");
      const descending = label === "Datum" ? "newest" : "largest";
      const ascending = label === "Datum" ? "oldest" : "smallest";
      const active = sort === descending || sort === ascending;
      const next = sort === descending ? ascending : descending;
      return <th scope="col" key={label} aria-sort={sortable && active ? sort === descending ? "descending" : "ascending" : undefined}>
        {sortable ? <button type="button" className="expense-sort" disabled={disabled} aria-pressed={active}
          aria-label={tr`${t(label)}: ${next === descending ? t("absteigend") : t("aufsteigend")} sortieren`}
          onClick={() => onSort?.(next)}>{t(label)} <span aria-hidden="true">{active ? sort === descending ? "↓" : "↑" : "↕"}</span></button> : t(label)}
      </th>;
    })}</tr></thead>
    <tbody>{rows.length === 0 && <tr><td colSpan={5}>{t("Keine Buchungen für diese Auswahl gefunden.")}</td></tr>}
      {rows.map(row => {
        const selected = selectedId === String(row.id);
        return <tr key={row.id} className={selected ? "pattern-selected" : undefined}>
          <td>{date(row.bookingDate)}</td>
          <td><div className="pattern-description">{row.description}{row.suggestedSettlement && row.kind === "UNKNOWN" && <span className="pattern-recommendation">{t("Empfohlener Ausgleich")}</span>}</div></td>
          <td>{money(row.amountMinor, row.currency)}</td>
          <td>{t(kindLabels[row.kind] ?? "Reguläre Buchung")}</td>
          <td><div className="pattern-actions"><button type="button" className={selected ? "primary-button" : "secondary-button"} disabled={disabled} aria-pressed={selected}
            aria-label={`${t(selected ? "Ausgewählt" : "Als Muster wählen")}: ${row.description}`}
            onClick={() => onSelect(String(row.id))}>
            {selected && <span aria-hidden="true">✓ </span>}{t(selected ? "Ausgewählt" : "Als Muster wählen")}
          </button></div></td>
        </tr>;
      })}
    </tbody>
  </table></div>;
}
