// Bietet eine sofort wirksame, gerätebezogene Farbschema-Auswahl mit zugänglichem Speicherstatus.
import { useState, useSyncExternalStore } from "react";
import { t } from "../../i18n";
import { getAppearance, saveAppearance, subscribeAppearance, type Appearance } from "./appearance";
export function AppearanceSettings() {
  const value = useSyncExternalStore(subscribeAppearance, getAppearance);
  const [saving, setSaving] = useState(false);
  const [error, setError] = useState(false);
  return <section className="dashboard-card settings-card" id="settings-appearance">
    <h2>{t("Darstellung")}</h2>
    <label htmlFor="appearance-preference">{t("Farbschema")}</label>
    <select id="appearance-preference" value={value} disabled={saving} onChange={async event => {
      const next = event.target.value as Appearance;
      setSaving(true); setError(false);
      try { await saveAppearance(next); } catch { setError(true); }
      finally { setSaving(false); }
    }}>
      <option value="system">{t("Systemeinstellung")}</option>
      <option value="light">{t("Hell")}</option>
      <option value="dark">{t("Dunkel")}</option>
    </select>
    <p className="settings-hint">{t("Wird sofort auf diesem Gerät gespeichert und gilt auch für die Anmeldung. Systemeinstellung folgt dem Farbschema des Betriebssystems.")}</p>
    {error && <p role="alert" className="error-message">{t("Die Darstellung konnte nicht gespeichert werden. Bitte versuche es erneut.")}</p>}
  </section>;
}
