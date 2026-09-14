import { t } from "../../i18n";
import { useState, type FormEvent } from "react";
import { invoke } from "@tauri-apps/api/core";
import { useSettings, type Language, type Region } from "../../settings";
import packageInfo from "../../../package.json";

export function Settings() {
  const { settings, saveSettings } = useSettings();
  const [draft, setDraft] = useState(settings);
  const [saving, setSaving] = useState(false);
  const [saveArea, setSaveArea] = useState<"general" | "market">("general");
  const [notice, setNotice] = useState("");
  const [error, setError] = useState("");
  const [refreshingPrices, setRefreshingPrices] = useState(false);
  const [marketNotice, setMarketNotice] = useState("");
  const [marketError, setMarketError] = useState("");

  function scrollToSection(id: string) {
    document.getElementById(id)?.scrollIntoView({ behavior: "smooth", block: "start" });
  }

  async function refreshPrices() {
    setSaveArea("market");
    setRefreshingPrices(true);
    setMarketNotice("");
    setMarketError("");
    try {
      await saveSettings(draft);
      const result = await invoke<{
        updatedPositions: number;
        storedDays: number;
        errors: string[];
      }>("refresh_market_data", { force: true });
      if (result.errors.length) setMarketError(result.errors.join(" · "));
      setMarketNotice(
        `${result.updatedPositions} ${t("Positionen aktualisiert")} · ${result.storedDays} ${t("Tageswerte gespeichert")}`,
      );
      window.dispatchEvent(new Event("market-data-refreshed"));
    } catch (reason) {
      setMarketError(String(reason));
    } finally {
      setRefreshingPrices(false);
    }
  }

  async function save(event: FormEvent, area: "general" | "market") {
    event.preventDefault();
    setSaveArea(area);
    setSaving(true);
    setNotice("");
    setError("");
    try {
      await saveSettings(draft);
      setNotice(t("Einstellungen gespeichert."));
    } catch (reason) {
      setError(String(reason));
    } finally {
      setSaving(false);
    }
  }


  return (
    <section className="settings-page">
      <div className="overview-heading">
        <div>
          <p className="eyebrow">Finanzblick</p>
          <h1>{t("Einstellungen")}</h1>
        </div>
      </div>
      <nav className="settings-nav" aria-label={t("Einstellungsbereiche")}>
        <button type="button" onClick={() => scrollToSection("settings-general")}>{t("Allgemein")}</button>
        <button type="button" onClick={() => scrollToSection("settings-market")}>{t("Automatische Marktpreise")}</button>
      </nav>
      <form
        id="settings-general"
        className="dashboard-card settings-card"
        onSubmit={(event) => void save(event, "general")}
      >
        <h2>{t("Allgemein")}</h2>
        <label>
          {t("Automatisch sperren nach")}
          <select
            value={draft.autoLockMinutes}
            disabled={saving}
            onChange={(e) =>
              setDraft({ ...draft, autoLockMinutes: Number(e.target.value) })
            }
          >
            {[1, 5, 10, 15, 30, 60].map((minutes) => (
              <option key={minutes} value={minutes}>
                {minutes} {minutes === 1 ? t("Minute") : t("Minuten")}
              </option>
            ))}
          </select>
        </label>
        <p className="settings-hint">
          {t(
            "Die Zeit zählt ab der letzten Bedienung. Beim Minimieren wird ebenfalls gesperrt.",
          )}
        </p>
        <label>
          {t("Sprache")}
          <select
            value={draft.language}
            disabled={saving}
            onChange={(e) =>
              setDraft({ ...draft, language: e.target.value as Language })
            }
          >
            <option value="de">Deutsch</option>
            <option value="en">English</option>
            <option value="fr">Français</option>
            <option value="it">Italiano</option>
          </select>
        </label>
        <label>
          {t("Ländereinstellungen")}
          <select
            value={draft.region}
            disabled={saving}
            onChange={(e) =>
              setDraft({ ...draft, region: e.target.value as Region })
            }
          >
            <option value="CH">{t("Schweiz")}</option>
            <option value="DE">{t("Deutschland")}</option>
            <option value="AT">{t("Österreich")}</option>
            <option value="FR">France</option>
            <option value="IT">Italia</option>
            <option value="GB">United Kingdom</option>
            <option value="US">United States</option>
          </select>
        </label>
        <p className="settings-hint">
          {t("Steuert die Darstellung von Datum und Zahlen. Gespeicherte Werte bleiben unverändert.")}
        </p>
        <label>
          {t("Standardwährung für neue Konten")}
          <select
            value={draft.defaultCurrency}
            disabled={saving}
            onChange={(e) =>
              setDraft({ ...draft, defaultCurrency: e.target.value })
            }
          >
            {["CHF", "EUR", "USD", "GBP"].map((currency) => (
              <option key={currency}>{currency}</option>
            ))}
          </select>
        </label>
        <p className="settings-hint">
          {t(
            "Wird beim Anlegen eines Kontos vorgeschlagen. Bestehende Konten und Beträge bleiben unverändert; es findet keine Währungsumrechnung statt.",
          )}
        </p>
        {saveArea === "general" && error && (
          <p role="alert" className="error-message">
            {t(error)}
          </p>
        )}
        {saveArea === "general" && notice && (
          <p role="status" className="settings-success">
            {t(notice)}
          </p>
        )}
        <button className="primary-button" disabled={saving}>
          {saving ? t("Wird gespeichert…") : t("Einstellungen speichern")}
        </button>
      </form>
      <form
        id="settings-market"
        className="dashboard-card settings-card"
        onSubmit={(event) => void save(event, "market")}
      >
        <h2>{t("Automatische Marktpreise")}</h2>
        <p className="settings-section-intro">
          {t("Optionale Datenquellen für automatische Tageskurse und Wechselkurse.")}
        </p>
        <label>
          {t("Marketstack-API-Schlüssel")}
          <input
            type="password"
            autoComplete="off"
            value={draft.marketstackApiKey}
            disabled={saving}
            onChange={(e) =>
              setDraft({ ...draft, marketstackApiKey: e.target.value.trim() })
            }
            placeholder={t("API-Schlüssel eingeben")}
          />
        </label>
        <label>
          {t("Alpha-Vantage-API-Schlüssel")}
          <input
            type="password"
            autoComplete="off"
            value={draft.alphaVantageApiKey}
            disabled={saving}
            onChange={(e) =>
              setDraft({ ...draft, alphaVantageApiKey: e.target.value.trim() })
            }
            placeholder={t("API-Schlüssel eingeben")}
          />
        </label>
        <p className="settings-hint">
          {t(
            "Optionale API-Schlüssel werden verschlüsselt in diesem Finanzprofil gespeichert. Ohne Schlüssel verwendet Finanzblick Yahoo Finance als Rückfall für Tageskurse und FX-Wechselkurse.",
          )}
        </p>
        <div className="settings-actions">
          <button className="primary-button" disabled={saving || refreshingPrices}>
            {saving && saveArea === "market"
              ? t("Wird gespeichert…")
              : t("Einstellungen speichern")}
          </button>
          <button
            className="secondary-button"
            type="button"
            disabled={saving || refreshingPrices}
            onClick={() => void refreshPrices()}
          >
            {refreshingPrices
              ? t("Kurse werden geladen …")
              : t("Kurse jetzt aktualisieren")}
          </button>
        </div>
        {marketError && (
          <p role="alert" className="error-message">
            {marketError}
          </p>
        )}
        {marketNotice && (
          <p role="status" className="settings-success">
            {marketNotice}
          </p>
        )}
        {saveArea === "market" && error && (
          <p role="alert" className="error-message">
            {t(error)}
          </p>
        )}
        {saveArea === "market" && notice && (
          <p role="status" className="settings-success">
            {t(notice)}
          </p>
        )}
      </form>
      <p className="settings-version">Finanzblick · Version {packageInfo.version}</p>
    </section>
  );
}
