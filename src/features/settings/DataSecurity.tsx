import { useState } from "react";
import { t } from "../../i18n";
import { DatabasePicker } from "./DatabasePicker";
import { BackupPanel } from "./BackupPanel";
import { PasswordSettings } from "./PasswordSettings";

export function DataSecurity() {
  const [revision, setRevision] = useState(0);
  return <section className="settings-page">
    <div className="overview-heading"><div><p className="eyebrow">Finanzblick</p><h1>{t("Daten & Sicherheit")}</h1></div></div>
    <nav className="settings-nav" aria-label={t("Daten & Sicherheit")}>
      {[["data-databases", "Finanzprofil"], ["data-backup", "Backup"], ["settings-security", "Passwort ändern"]].map(([id, label]) =>
        <button key={id} type="button" onClick={() => document.getElementById(id)?.scrollIntoView({ behavior: "smooth", block: "start" })}>{t(label)}</button>)}
    </nav>
    <section id="data-databases" className="dashboard-card settings-card"><DatabasePicker key={revision} allowCreate /></section>
    <BackupPanel onRestored={() => setRevision(current => current + 1)} />
    <PasswordSettings />
  </section>;
}
