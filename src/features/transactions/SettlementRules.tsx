// Listet bestätigte Importregeln auf und erlaubt ihre Deaktivierung ohne bestehende Markierungen zu ändern.
import { useEffect, useState } from "react";
import { invoke } from "@tauri-apps/api/core";
import { t } from "../../i18n";

interface Rule { id: number; accountName: string; currency: string; direction: number; prefix: string }
export function SettlementRules() {
  const [rules, setRules] = useState<Rule[]>([]);
  const [error, setError] = useState("");
  const [busy, setBusy] = useState(false);
  async function load() { setRules(await invoke<Rule[]>("list_settlement_rules")); }
  useEffect(() => { void load().catch(reason => setError(String(reason))); }, []);
  async function remove(id: number) {
    setBusy(true); setError("");
    try { await invoke("delete_settlement_rule", { ruleId: id }); await load(); }
    catch (reason) { setError(String(reason)); }
    finally { setBusy(false); }
  }
  return <details className="dashboard-card settlement-rules">
    <summary>{t("Automatische Kartenausgleichsregeln")} ({rules.length})</summary>
    <p>{t("Deaktivieren stoppt die Regel für zukünftige Importe. Bestehende Markierungen bleiben erhalten und können einzeln oder gesammelt wiederhergestellt werden.")}</p>
    {error && <p role="alert" className="error-message">{t(error)}</p>}
    {rules.map(rule => <div className="settlement-rule" key={rule.id}>
      <div><strong>{rule.accountName} · {rule.currency} · {t(rule.direction < 0 ? "Belastung" : "Gutschrift")}</strong><p>{rule.prefix}</p></div>
      <button type="button" className="secondary-button" disabled={busy} onClick={() => void remove(rule.id)}>{t("Regel deaktivieren")}</button>
    </div>)}
    {!rules.length && <p>{t("Keine automatischen Regeln gespeichert.")}</p>}
  </details>;
}
