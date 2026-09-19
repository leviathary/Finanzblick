// Einheitliche Routennavigation unter dem Titel der Transaktionsansichten.
import { t } from "../../i18n";
export function TransactionTabs({ active }: { active: "all" | "transfers" | "cards" }) {
  return <nav className="transaction-tabs" aria-label={t("Transaktionen")}>
    <a href="#transactions" aria-current={active === "all" ? "page" : undefined}>{t("Alle Transaktionen")}</a>
    <a href="#transactions/transfers" aria-current={active === "transfers" ? "page" : undefined}>{t("Umbuchungen & Ausgleiche")}</a>
    <a href="#transactions/cards" aria-current={active === "cards" ? "page" : undefined}>{t("Kartentransaktionen")}</a>
  </nav>;
}
