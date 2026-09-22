// Verknüpft Navigation, Ansichten, Einstellungen und Zugriffsschutz der Anwendung.

import { t } from "./i18n";
import { invoke, isTauri } from "@tauri-apps/api/core";
import { getCurrentWindow } from "@tauri-apps/api/window";
import "./App.css";
import { DataSecurity } from "./features/settings/DataSecurity";
import { Settings } from "./features/settings/Settings";
import { useSettings } from "./settings";
import { VaultGate, useVaultLock } from "./features/auth/VaultGate";
import { useEffect, useState, type ReactNode } from "react";
import { ImportWizard } from "./features/imports/ImportWizard";
import { ImportHistory } from "./features/imports/ImportHistory";
import { Overview } from "./features/overview/Overview";
import { Accounts } from "./features/accounts/Accounts";
import { AccountExplorer } from "./features/account-details/AccountExplorer";
import { Assets } from "./features/assets/Assets";
import { CreditCards } from "./features/cards/CreditCards";
import { CardSetupWizard } from "./features/cards/setup/CardSetupWizard";
import { TransferManagement } from "./features/transactions/TransferManagement";
import { Transactions } from "./features/transactions/Transactions";
import { TaxHistory } from "./features/tax-history/TaxHistory";

import { FinanceChat } from "./features/chat/FinanceChat";
import { Categories } from "./features/categories/Categories";
import { Help } from "./features/help/Help";

type Page = "holdings" | "chat" | "help" | "data-security" | "settings" | "categories" | "overview" | "accounts" | "assets" | "tax-history" | "transactions" | "transfers" | "cards" | "card-setup" | "imports" | "import-history";
type ImportKind = "bank" | "tax";

type NavIconName = "chat" | "help" | "home" | "bank" | "chart" | "tax" | "transactions" | "tag" | "import" | "history" | "settings" | "logout";

function NavIcon({ name }: { name: NavIconName }) {
  const paths: Record<NavIconName, ReactNode> = {
    chat: <><path d="M4 4h16v13H9l-5 4V4Z"/><path d="M8 8h8M8 12h5"/></>,
    help: <><circle cx="12" cy="12" r="9"/><path d="M9.7 9a2.5 2.5 0 1 1 3.7 2.2c-.9.5-1.4 1-1.4 2.1M12 17h.01"/></>,
    home: <><path d="M3 11.5 12 4l9 7.5"/><path d="M5.5 10v10h13V10M9 20v-6h6v6"/></>,
    bank: <><path d="M3 9h18L12 4 3 9Z"/><path d="M5 9v8m4-8v8m6-8v8m4-8v8M3 20h18"/></>,
    chart: <><path d="M4 19V5"/><path d="M4 19h16"/><path d="m7 15 4-5 3 3 5-7"/></>,
    tax: <><path d="M6 3h9l4 4v14H6z"/><path d="M15 3v5h5M9 12h6M9 16h6"/></>,
    transactions: <><rect x="4" y="3" width="16" height="18" rx="2"/><path d="M8 8h.01M11 8h5M8 12h.01M11 12h5M8 16h.01M11 16h5"/></>,
    tag: <><path d="M20 13 13 20l-9-9V4h7l9 9Z"/><circle cx="8.5" cy="8.5" r="1"/></>,
    import: <><path d="M12 3v12m-4-4 4 4 4-4"/><path d="M5 16v4h14v-4"/></>,
    history: <><path d="M4 7h16v13H4zM8 4h8l2 3H6l2-3Z"/><path d="M9 12h6M9 16h4"/></>,
    settings: <><circle cx="12" cy="12" r="3"/><path d="M19.4 15a1.7 1.7 0 0 0 .34 1.88l.06.06-2.83 2.83-.06-.06a1.7 1.7 0 0 0-1.88-.34 1.7 1.7 0 0 0-1.03 1.56V21h-4v-.08A1.7 1.7 0 0 0 8.96 19.4a1.7 1.7 0 0 0-1.88.34l-.06.06-2.83-2.83.06-.06A1.7 1.7 0 0 0 4.6 15 1.7 1.7 0 0 0 3 14H3v-4h.08A1.7 1.7 0 0 0 4.6 9a1.7 1.7 0 0 0-.34-1.88l-.06-.06 2.83-2.83.06.06A1.7 1.7 0 0 0 9 4.6 1.7 1.7 0 0 0 10 3h4v.08A1.7 1.7 0 0 0 15 4.6a1.7 1.7 0 0 0 1.88-.34l.06-.06 2.83 2.83-.06.06A1.7 1.7 0 0 0 19.4 9 1.7 1.7 0 0 0 21 10v4h-.08A1.7 1.7 0 0 0 19.4 15Z"/></>,
    logout: <><path d="M10 4H5v16h5M14 8l4 4-4 4M18 12H9"/></>,
  };
  return <svg className="nav-icon" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="1.7" strokeLinecap="round" strokeLinejoin="round" aria-hidden="true" focusable="false">{paths[name]}</svg>;
}

function ImportKindTabs({ value, onChange }: { value: ImportKind; onChange: (value: ImportKind) => void }) {
  return <div className="import-kind-tabs" role="tablist" aria-label={t("Importart")}>
    <button type="button" role="tab" aria-selected={value === "bank"} onClick={() => onChange("bank")}>{t("Bankauszüge")}</button>
    <button type="button" role="tab" aria-selected={value === "tax"} onClick={() => onChange("tax")}>{t("Steuererklärungen")}</button>
  </div>;
}

function pageFromHash(): Page {
  if (window.location.hash === "#chat") return "chat";
  if (window.location.hash.startsWith("#help")) return "help";
  if (window.location.hash === "#data-security") return "data-security";
  if (window.location.hash === "#settings") return "settings";
  if (window.location.hash === "#categories") return "categories";
  if (window.location.hash.startsWith("#import-history")) return "import-history";
  if (window.location.hash.startsWith("#imports")) return "imports";
  if (window.location.hash.split("?")[0] === "#holdings") return "holdings";
  if (window.location.hash.split("?")[0] === "#banks") return "accounts";
  if (window.location.hash.split("?")[0] === "#assets") return "assets";
  if (window.location.hash === "#tax-history") return "tax-history";
  if (window.location.hash.split("?")[0] === "#transactions/cards/setup") return "card-setup";
  if (window.location.hash.split("?")[0] === "#transactions/cards") return "cards";
  if (window.location.hash.split("?")[0] === "#transactions/transfers") return "transfers";
  if (window.location.hash.split("?")[0] === "#transactions") return "transactions";
  return "overview";
}

function App() {
  useSettings();
  const lockVault = useVaultLock();
  const [page, setPage] = useState<Page>(pageFromHash);
  useEffect(() => {
    let active = true;
    const setTitle = (title: string) => {
      document.title = title;
      if (isTauri()) void getCurrentWindow().setTitle(title).catch(() => {});
    };
    setTitle("Saldonaut");
    void invoke<boolean>("demo_status").then(demo => {
      if (active) setTitle(demo ? "Saldonaut - Demo" : "Saldonaut");
    }).catch(() => {});
    return () => { active = false; setTitle("Saldonaut"); };
  }, []);
  const [importKind, setImportKind] = useState<ImportKind>(window.location.hash === "#imports/tax" ? "tax" : "bank");
  const [managementKind, setManagementKind] = useState<ImportKind>(window.location.hash === "#import-history/tax" ? "tax" : "bank");

  useEffect(() => {
    const syncPage = () => {
      const nextPage = pageFromHash();
      setPage(nextPage);
      if (nextPage === "imports") setImportKind(window.location.hash === "#imports/tax" ? "tax" : "bank");
      if (nextPage === "import-history") setManagementKind(window.location.hash === "#import-history/tax" ? "tax" : "bank");
    };
    window.addEventListener("hashchange", syncPage);
    return () => window.removeEventListener("hashchange", syncPage);
  }, []);

  function navigate(next: Page) {
    window.location.hash = next === "accounts" ? "banks" : next;
    setPage(next);
  }

  return (
    <main className="app-shell">
      <aside className="sidebar">
        <strong className="brand">Saldonaut</strong>
        <nav className="sidebar-primary" aria-label={t("Hauptnavigation")}>
          <div className="sidebar-nav-group" role="group" aria-label={t("Analyse")}>
            <a className={page === "overview" ? "active" : ""} href="#overview"><NavIcon name="home"/><span>{t("Übersicht")}</span></a>
            <a className={page === "assets" ? "active" : ""} href="#assets"><NavIcon name="chart"/><span>{t("Vermögen")}</span></a>
            <a className={page === "holdings" ? "active" : ""} aria-current={page === "holdings" ? "page" : undefined} href="#holdings"><NavIcon name="bank"/><span>{t("Konten & Depots")}</span></a>
            <a className={page === "transactions" || page === "transfers" || page === "cards" || page === "card-setup" ? "active" : ""} href="#transactions"><NavIcon name="transactions"/><span>{t("Transaktionen")}</span></a>
          </div>
          <div className="sidebar-nav-group" role="group" aria-label={t("Weitere Auswertungen und Werkzeuge")}>
            <a className={page === "tax-history" ? "active" : ""} href="#tax-history"><NavIcon name="tax"/><span>{t("Steuerhistorie")}</span></a>
            <a className={page === "chat" ? "active" : ""} href="#chat"><NavIcon name="chat"/><span>{t("Finanzchat")}</span></a>
          </div>
          <div className="sidebar-nav-group" role="group" aria-label={t("Verwaltung")}>
            <a className={page === "imports" || page === "import-history" ? "active" : ""} aria-current={page === "imports" || page === "import-history" ? "page" : undefined} href="#imports"><NavIcon name="import"/><span>{t("Import")}</span></a>
            <a className={page === "accounts" ? "active" : ""} href="#banks"><NavIcon name="bank"/><span>{t("Kontenverwaltung")}</span></a>
            <a className={page === "categories" ? "active" : ""} href="#categories"><NavIcon name="tag"/><span>{t("Kategorien")}</span></a>
          </div>
        </nav>
        <nav className="sidebar-secondary" aria-label={t("Kontonavigation")}>
          <a className={page === "help" ? "active" : ""} href="#help/start"><NavIcon name="help"/><span>{t("Hilfe")}</span></a>
          <a className={page === "data-security" ? "active" : ""} href="#data-security"><NavIcon name="history"/><span>{t("Daten & Sicherheit")}</span></a>
          <a className={page === "settings" ? "active" : ""} href="#settings"><NavIcon name="settings"/><span>{t("Einstellungen")}</span></a>
          <button type="button" onClick={() => { void lockVault(); }}><NavIcon name="logout"/><span>{t("Abmelden")}</span></button>
        </nav>
      </aside>
      <div className="content">
        {(page === "imports" || page === "import-history") && <nav className="transaction-tabs" aria-label={t("Import")}>
          <a href={ (page === "imports" ? importKind : managementKind) === "tax" ? "#imports/tax" : "#imports"} aria-current={page === "imports" ? "page" : undefined}>{t("Dateien importieren")}</a>
          <a href={(page === "imports" ? importKind : managementKind) === "tax" ? "#import-history/tax" : "#import-history"} aria-current={page === "import-history" ? "page" : undefined}>{t("Importierte Dateien")}</a>
        </nav>}
        {page === "chat" ? <FinanceChat /> : page === "help" ? <Help /> : page === "data-security" ? <DataSecurity /> : page === "settings" ? <Settings /> : page === "overview" ? <Overview onImport={() => navigate("imports")} /> : page === "holdings" ? <AccountExplorer /> : page === "accounts" ? <Accounts /> : page === "assets" ? <Assets onAccounts={() => navigate("accounts")} onImport={() => navigate("imports")} /> : page === "tax-history" ? <TaxHistory /> : page === "transactions" ? <Transactions /> : page === "transfers" ? <TransferManagement /> : page === "cards" ? <CreditCards /> : page === "card-setup" ? <CardSetupWizard /> : page === "categories" ? <Categories /> : null}
        <div hidden={page !== "imports"}>
          <ImportKindTabs value={importKind} onChange={(kind) => { setImportKind(kind); window.location.hash = kind === "tax" ? "imports/tax" : "imports"; }} />
          <div hidden={importKind !== "bank"}><ImportWizard enabled={page === "imports" && importKind === "bank"} /></div>
          <div hidden={importKind !== "tax"}><TaxHistory view="import" active={page === "imports" && importKind === "tax"} /></div>
        </div>
        {page === "import-history" && <div>
          <ImportKindTabs value={managementKind} onChange={(kind) => { setManagementKind(kind); window.location.hash = kind === "tax" ? "import-history/tax" : "import-history"; }} />
          {managementKind === "bank" ? <ImportHistory /> : <TaxHistory view="management" />}
        </div>}
      </div>
    </main>
  );
}

export default function ProtectedApp() {
  return <VaultGate><App /></VaultGate>;
}
