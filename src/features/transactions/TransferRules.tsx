// Verwaltet benannte Umbuchungs- und Kartenausgleichsregeln mit Vorschau und Bearbeitung.
import { useEffect, useState } from "react";
import { invoke } from "@tauri-apps/api/core";
import { t } from "../../i18n";
import { TransferRuleEditor, type TransferRule, type RuleAccount } from "./TransferRuleEditor";
import "./transferRules.css";

export function TransferRules({ onChanged }: { onChanged: () => Promise<void> }) {
  const [rules, setRules] = useState<TransferRule[]>([]);
  const [accounts, setAccounts] = useState<RuleAccount[]>([]);
  const [editor, setEditor] = useState<{ rule: TransferRule | null } | null>(null);
  const [loading, setLoading] = useState(true);
  const [error, setError] = useState("");
  const [busy, setBusy] = useState(false);
  async function load() {
    setLoading(true); setError("");
    try {
      const [rules, accounts] = await Promise.all([invoke<TransferRule[]>("list_transfer_rules"), invoke<RuleAccount[]>("list_accounts")]);
      setRules(rules); setAccounts(accounts);
    } finally { setLoading(false); }
  }
  useEffect(() => { void load().catch(reason => setError(String(reason))); }, []);
  async function remove(id: number) {
    setBusy(true); setError("");
    try { await invoke("delete_settlement_rule", { ruleId: id }); await load(); }
    catch (reason) { setError(String(reason)); }
    finally { setBusy(false); }
  }
  return <details className="dashboard-card settlement-rules">
    <summary>{t("Automatische Umbuchungsregeln")} ({rules.length})</summary>
    <p>{t("Passende Buchungen werden als eigene Umbuchung oder Kartenausgleich neutralisiert. Diese Regeln filtern nicht die Tabelle darunter.")}</p>
    <p>{t("Deaktivieren stoppt die Regel für zukünftige Importe. Bestehende Markierungen bleiben erhalten und können einzeln oder gesammelt wiederhergestellt werden.")}</p>
    {error && <p role="alert" className="error-message">{t(error)}</p>}
    {loading && <p role="status">{t("Buchungen werden geladen…")}</p>}
    <button type="button" className="primary-button" disabled={busy || loading || !accounts.length || !!error} onClick={() => setEditor({ rule: null })}>{t("Neue Umbuchungsregel")}</button>
    {rules.map(rule => <div className="settlement-rule" key={rule.id}>
      <div><strong>{rule.name || `${rule.accountName} · ${t(rule.transferType === "INTERNAL_TRANSFER" ? "Eigene Umbuchung" : "Kartenausgleich")}`}</strong>
        <p>{t("Konto")}: {rule.accountName} · {rule.currency} · {t(rule.direction < 0 ? "Belastung" : "Gutschrift")}</p>
        <p>{t("Buchungstext beginnt mit")}: <q>{rule.prefix}</q></p>
        <p>{t("Regeltyp")}: {t(rule.transferType === "INTERNAL_TRANSFER" ? "Eigene Umbuchung" : "Kartenausgleich")}</p></div>
      <div className="transfer-rule-actions"><button type="button" className="secondary-button" disabled={busy || loading} onClick={() => setEditor({ rule })}>{t("Regel bearbeiten")}</button>
        <button type="button" className="secondary-button" disabled={busy || loading} onClick={() => void remove(rule.id)}>{t("Regel deaktivieren")}</button></div>
    </div>)}
    {!loading && !error && !rules.length && <p>{t("Keine automatischen Regeln gespeichert.")}</p>}
    {editor && <TransferRuleEditor rule={editor.rule} accounts={accounts} onClose={() => setEditor(null)} onSaved={() => {
      setEditor(null); void load().catch(reason => setError(String(reason))); void onChanged();
    }} />}
  </details>;
}
