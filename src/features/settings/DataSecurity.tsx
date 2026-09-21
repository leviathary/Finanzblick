// Bündelt die Oberflächen für Datenbankprofile, Sicherungen und Passwortverwaltung.

import { useState } from "react";
import { t } from "../../i18n";
import { DatabasePicker } from "./DatabasePicker";
import { BackupPanel } from "./BackupPanel";
import { PasswordSettings } from "./PasswordSettings";
import { SectionTabs, SectionPanel } from "../../shared/navigation/SectionTabs";

export function DataSecurity() {
  const [revision, setRevision] = useState(0);
  const [tab, setTab] = useState("profiles");
  return <section className="settings-page">
    <div className="overview-heading"><div><p className="eyebrow">Saldonaut</p><h1>{t("Daten & Sicherheit")}</h1></div></div>
    <SectionTabs id="security" label={t("Daten & Sicherheit")} value={tab} onChange={setTab} tabs={[
      { value: "profiles", label: t("Finanzprofile") }, { value: "backup", label: t("Backup") }, { value: "password", label: t("Passwort ändern") },
    ]} />
    <SectionPanel id="security" value="profiles" active={tab}><section id="data-databases" className="dashboard-card settings-card"><DatabasePicker key={revision} allowCreate /></section></SectionPanel>
    <SectionPanel id="security" value="backup" active={tab}><BackupPanel onRestored={() => setRevision(current => current + 1)} /></SectionPanel>
    <SectionPanel id="security" value="password" active={tab}><PasswordSettings /></SectionPanel>
  </section>;
}
