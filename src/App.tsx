// Verknüpft Navigation, Ansichten, Einstellungen und Zugriffsschutz der Anwendung.

import { t } from "./i18n";
import { invoke, isTauri } from "@tauri-apps/api/core";
import { getCurrentWindow } from "@tauri-apps/api/window";
import "./App.css";
import { DataSecurity } from "./features/settings/DataSecurity";
import { Settings } from "./features/settings/Settings";
import { useSettings } from "./settings";
import { VaultGate, useVaultLock } from "./features/auth/VaultGate";
import { useEffect, useRef, useState } from "react";
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
import { MoonLanderLauncher } from "./features/moon-lander/MoonLanderLauncher";
import { OnboardingTour } from "./features/onboarding/OnboardingTour";
import { markOnboardingComplete, onboardingOpenEvent, shouldShowOnboarding } from "./features/onboarding/onboardingState";
import { NavIcon } from "./shared/navigation/NavIcon";
import { SectionTabs, SectionPanel } from "./shared/navigation/SectionTabs";

type Page = "holdings" | "chat" | "help" | "data-security" | "settings" | "categories" | "overview" | "accounts" | "assets" | "taxes" | "tax-years" | "transactions" | "transfers" | "cards" | "card-setup" | "imports" | "import-history";
type ImportKind = "bank" | "tax";

function pageFromHash(): Page {
  if (window.location.hash === "#chat") return "chat";
  if (window.location.hash.startsWith("#help")) return "help";
  if (window.location.hash === "#data-security") return "data-security";
  if (window.location.hash === "#settings") return "settings";
  if (window.location.hash === "#categories") return "categories";
  if (window.location.hash === "#import-history/tax" || window.location.hash === "#taxes/years") return "tax-years";
  if (window.location.hash.startsWith("#import-history")) return "import-history";
  if (window.location.hash.startsWith("#imports")) return "imports";
  if (window.location.hash.split("?")[0] === "#holdings") return "holdings";
  if (window.location.hash.split("?")[0] === "#banks") return "accounts";
  if (window.location.hash.split("?")[0] === "#assets") return "assets";
  if (window.location.hash === "#tax-history" || window.location.hash === "#taxes") return "taxes";
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
  const [showOnboarding, setShowOnboarding] = useState(shouldShowOnboarding);
  const onboardingReturnFocus = useRef<HTMLElement | null>(null);
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
  useEffect(() => {
    const syncPage = () => {
      if (window.location.hash === "#tax-history") {
        window.location.replace("#taxes");
        return;
      }
      if (window.location.hash === "#import-history/tax") {
        window.location.replace("#taxes/years");
        return;
      }
      const nextPage = pageFromHash();
      setPage(nextPage);
      if (nextPage === "imports") setImportKind(window.location.hash === "#imports/tax" ? "tax" : "bank");
    };
    syncPage();
    window.addEventListener("hashchange", syncPage);
    return () => window.removeEventListener("hashchange", syncPage);
  }, []);
  useEffect(() => {
    const open = () => {
      onboardingReturnFocus.current = document.activeElement instanceof HTMLElement ? document.activeElement : null;
      setShowOnboarding(true);
    };
    window.addEventListener(onboardingOpenEvent, open);
    return () => window.removeEventListener(onboardingOpenEvent, open);
  }, []);

  function navigate(next: Page) {
    window.location.hash = next === "accounts" ? "banks" : next;
    setPage(next);
  }

  function closeOnboarding(destination?: "overview" | "imports") {
    markOnboardingComplete();
    setShowOnboarding(false);
    if (destination) navigate(destination);
    else requestAnimationFrame(() => onboardingReturnFocus.current?.focus());
  }

  return (
    <main className="app-shell">
      <MoonLanderLauncher />
      {showOnboarding && <OnboardingTour onClose={closeOnboarding} />}
      <aside className="sidebar">
        <strong className="brand">Saldonaut</strong>
        <nav className="sidebar-primary" aria-label={t("Hauptnavigation")}>
          <div className="sidebar-nav-group" role="group" aria-label={t("Analyse")}>
            <a className={page === "overview" ? "active" : ""} aria-current={page === "overview" ? "page" : undefined} href="#overview"><NavIcon name="home"/><span>{t("Übersicht")}</span></a>
            <a className={page === "assets" ? "active" : ""} aria-current={page === "assets" ? "page" : undefined} href="#assets"><NavIcon name="chart"/><span>{t("Vermögen")}</span></a>
            <a className={page === "holdings" ? "active" : ""} aria-current={page === "holdings" ? "page" : undefined} href="#holdings"><NavIcon name="bank"/><span>{t("Konten & Depots")}</span></a>
            <a className={page === "transactions" || page === "transfers" || page === "cards" || page === "card-setup" ? "active" : ""} aria-current={page === "transactions" || page === "transfers" || page === "cards" || page === "card-setup" ? "page" : undefined} href="#transactions"><NavIcon name="transactions"/><span>{t("Transaktionen")}</span></a>
          </div>
          <div className="sidebar-nav-group" role="group" aria-label={t("Weitere Auswertungen und Werkzeuge")}>
            <a className={page === "taxes" || page === "tax-years" ? "active" : ""} aria-current={page === "taxes" || page === "tax-years" ? "page" : undefined} href="#taxes"><NavIcon name="tax"/><span>{t("Steuern")}</span></a>
            <a className={page === "chat" ? "active" : ""} aria-current={page === "chat" ? "page" : undefined} href="#chat"><NavIcon name="chat"/><span>{t("Finanzchat")}</span></a>
          </div>
          <div className="sidebar-nav-group" role="group" aria-label={t("Verwaltung")}>
            <a className={page === "imports" || page === "import-history" ? "active" : ""} aria-current={page === "imports" || page === "import-history" ? "page" : undefined} href="#imports"><NavIcon name="import"/><span>{t("Import")}</span></a>
            <a className={page === "accounts" ? "active" : ""} aria-current={page === "accounts" ? "page" : undefined} href="#banks"><NavIcon name="bank"/><span>{t("Kontenverwaltung")}</span></a>
            <a className={page === "categories" ? "active" : ""} aria-current={page === "categories" ? "page" : undefined} href="#categories"><NavIcon name="tag"/><span>{t("Kategorien")}</span></a>
          </div>
        </nav>
        <nav className="sidebar-secondary" aria-label={t("Kontonavigation")}>
          <a className={page === "help" ? "active" : ""} aria-current={page === "help" ? "page" : undefined} href="#help/start"><NavIcon name="help"/><span>{t("Hilfe")}</span></a>
          <a className={page === "data-security" ? "active" : ""} aria-current={page === "data-security" ? "page" : undefined} href="#data-security"><NavIcon name="history"/><span>{t("Daten & Sicherheit")}</span></a>
          <a className={page === "settings" ? "active" : ""} aria-current={page === "settings" ? "page" : undefined} href="#settings"><NavIcon name="settings"/><span>{t("Einstellungen")}</span></a>
          <button type="button" onClick={() => { void lockVault(); }}><NavIcon name="logout"/><span>{t("Abmelden")}</span></button>
        </nav>
      </aside>
      <div className="content">
        {(page === "imports" || page === "import-history") && <nav className="transaction-tabs" aria-label={t("Import")}>
          <a href={page === "imports" && importKind === "tax" ? "#imports/tax" : "#imports"} aria-current={page === "imports" ? "page" : undefined}>{t("Dateien importieren")}</a>
          {(page !== "imports" || importKind === "bank") && <a href="#import-history" aria-current={page === "import-history" ? "page" : undefined}>{t("Importierte Dateien")}</a>}
        </nav>}
        {page === "chat" ? <FinanceChat /> : page === "help" ? <Help /> : page === "data-security" ? <DataSecurity /> : page === "settings" ? <Settings /> : page === "overview" ? <Overview onImport={() => navigate("imports")} /> : page === "holdings" ? <AccountExplorer /> : page === "accounts" ? <Accounts /> : page === "assets" ? <Assets onAccounts={() => navigate("accounts")} onImport={() => navigate("imports")} /> : page === "taxes" ? <TaxHistory /> : page === "tax-years" ? <TaxHistory view="management" /> : page === "transactions" ? <Transactions /> : page === "transfers" ? <TransferManagement /> : page === "cards" ? <CreditCards /> : page === "card-setup" ? <CardSetupWizard /> : page === "categories" ? <Categories /> : null}
        <div hidden={page !== "imports"}>
          <SectionTabs id="import-kind" className="import-kind-tabs" label={t("Importart")} value={importKind} onChange={(kind) => { setImportKind(kind as ImportKind); window.location.hash = kind === "tax" ? "imports/tax" : "imports"; }} tabs={[
            { value: "bank", label: t("Bankauszüge") }, { value: "tax", label: t("Steuererklärungen") },
          ]} />
          <SectionPanel id="import-kind" value="bank" active={importKind}><ImportWizard enabled={page === "imports" && importKind === "bank"} /></SectionPanel>
          <SectionPanel id="import-kind" value="tax" active={importKind}><TaxHistory view="import" active={page === "imports" && importKind === "tax"} /></SectionPanel>
        </div>
        {page === "import-history" && <ImportHistory />}
      </div>
    </main>
  );
}

export default function ProtectedApp() {
  return <VaultGate><App /></VaultGate>;
}
