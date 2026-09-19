// Zeigt gespeicherte Ausgleichsregeln je Kartenkonto und führt zum kontospezifischen Setup.
import { t } from "../../i18n";
import type { Account, SettlementRule } from "./types";
import { hasSettlementRule } from "./overviewModel";

export function CardAccountStatus({ accounts, rules }: { accounts: Account[]; rules: SettlementRule[] }) {
  return <ul className="cards-status-list">{accounts.map(account => {
    const configured = hasSettlementRule(account, rules);
    return <li key={account.id}>
      <span className="cards-status-name">{account.name} <small>{account.currency}</small></span>
      <span className={`cards-status-badge ${configured ? "configured" : "pending"}`}>
        <span aria-hidden="true">{configured ? "✓" : "⚠"}</span>
        {t(configured ? "Ausgleich konfiguriert" : "Ausgleich nicht eingerichtet")}
      </span>
      <a href={`#transactions/cards/setup?account=${account.id}`} aria-label={`${t("Setup anpassen")}: ${account.name}`}>{t("Setup anpassen")}</a>
    </li>;
  })}</ul>;
}
