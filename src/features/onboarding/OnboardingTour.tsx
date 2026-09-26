// Zeigt eine visuelle, vierstufige Einführung nach dem ersten Entsperren und aus der Hilfe heraus.

import { useEffect, useId, useRef, useState, type ReactNode } from "react";
import { t, tr } from "../../i18n";
import { NavIcon, type NavIconName } from "../../shared/navigation/NavIcon";
import "./onboarding.css";

type Destination = "overview" | "imports";

type Props = {
  onClose: (destination?: Destination) => void;
};

function VisualFrame({ label, children }: { label: string; children: ReactNode }) {
  return <div className="onboarding-visual" aria-hidden="true">
    <div className="onboarding-window-bar"><span/><span/><span/><strong>{label}</strong></div>
    {children}
  </div>;
}

function WelcomeVisual() {
  return <VisualFrame label="Saldonaut">
    <div className="onboarding-aggregator">
      <div className="onboarding-source-list">
        <div><span><NavIcon name="bank"/></span><strong>{t("Bankkonto")}</strong></div>
        <div><span><NavIcon name="chart"/></span><strong>{t("Depot")}</strong></div>
        <div><span><NavIcon name="bank"/></span><strong>{t("Vorsorge")}</strong></div>
      </div>
      <span className="onboarding-aggregator-arrow">→</span>
      <div className="onboarding-summary-preview">
        <header><NavIcon name="home"/><strong>{t("Gemeinsame Übersicht")}</strong></header>
        <div><span>{t("Gesamtvermögen")}</span><strong>CHF 284'650</strong></div>
        <footer><span>{t("6 Konten")}</span><span>{t("3 Anbieter")}</span></footer>
      </div>
    </div>
  </VisualFrame>;
}

function PrivacyVisual() {
  return <VisualFrame label={t("Daten & Sicherheit")}>
    <div className="onboarding-flow">
      <div className="onboarding-flow-card"><NavIcon name="import"/><strong>{t("Dokumente")}</strong><span>{t("Bank & Steuern")}</span></div>
      <span className="onboarding-flow-arrow">→</span>
      <div className="onboarding-flow-card active"><NavIcon name="settings"/><strong>{t("Lokal verarbeitet")}</strong><span>{t("Auf deinem Gerät")}</span></div>
      <span className="onboarding-flow-arrow">→</span>
      <div className="onboarding-flow-card"><NavIcon name="history"/><strong>{t("Finanzprofil")}</strong><span>{t("Verschlüsselt")}</span></div>
    </div>
    <div className="onboarding-privacy-status"><span>✓</span>{t("Keine Dokumente werden hochgeladen")}</div>
  </VisualFrame>;
}

function ImportVisual() {
  return <VisualFrame label={t("Import")}>
    <div className="onboarding-import-tabs"><strong>{t("Bankauszüge")}</strong><span>{t("Steuererklärungen")}</span></div>
    <div className="onboarding-drop-preview">
      <span className="onboarding-preview-icon"><NavIcon name="import"/></span>
      <strong>{t("Dateien oder Ordner hierher ziehen")}</strong>
      <span>{t("oder")}</span>
      <span className="onboarding-preview-button">{t("Dateien auswählen")}</span>
      <small>PDF&nbsp;&nbsp; XLSX&nbsp;&nbsp; CSV&nbsp;&nbsp; MT940</small>
    </div>
  </VisualFrame>;
}

const destinations: Array<{ icon: NavIconName; title: string; detail: string }> = [
  { icon: "home", title: "Übersicht", detail: "Gesamtvermögen und Anbieter auf einen Blick" },
  { icon: "chart", title: "Vermögen", detail: "Entwicklung und Aufteilung deines Vermögens" },
  { icon: "bank", title: "Konten & Depots", detail: "Salden, Depots und einzelne Positionen" },
  { icon: "transactions", title: "Transaktionen", detail: "Einnahmen, Ausgaben und Kategorien analysieren" },
];

function OverviewVisual() {
  return <VisualFrame label={t("Deine Finanzen")}>
    <div className="onboarding-destination-grid">
      {destinations.map(item => <div className="onboarding-destination" key={item.title}>
        <span><NavIcon name={item.icon}/></span>
        <div><strong>{t(item.title)}</strong><small>{t(item.detail)}</small></div>
      </div>)}
    </div>
  </VisualFrame>;
}

export function OnboardingTour({ onClose }: Props) {
  const dialogRef = useRef<HTMLDialogElement>(null);
  const headingRef = useRef<HTMLHeadingElement>(null);
  const [step, setStep] = useState(0);
  const titleId = useId();
  const descriptionId = useId();
  const steps = [
    {
      eyebrow: t("Willkommen"),
      title: t("Deine Finanzen an einem Ort"),
      body: t("Saldonaut ist dein Finanzaggregator und bündelt Konten, Depots und Finanzdaten verschiedener Anbieter in einer gemeinsamen Übersicht."),
      visual: <WelcomeVisual/>,
    },
    {
      eyebrow: t("Daten & Sicherheit"),
      title: t("Deine Daten bleiben bei dir"),
      body: t("Dokumente bleiben auf deinem Gerät. Nur bestätigte Finanzdaten werden verschlüsselt im lokalen Profil gespeichert."),
      visual: <PrivacyVisual/>,
    },
    {
      eyebrow: t("Import"),
      title: t("Starte mit deinem ersten Import"),
      body: t("Ziehe Bankauszüge, Depotbestände oder Steuererklärungen hinein. Saldonaut erkennt den Dokumenttyp und zeigt dir vor dem Speichern eine Vorschau."),
      visual: <ImportVisual/>,
    },
    {
      eyebrow: t("Orientierung"),
      title: t("Was du wo findest"),
      body: t("Von der Gesamtsicht bis zur einzelnen Buchung hat jeder Bereich eine klare Aufgabe."),
      visual: <OverviewVisual/>,
    },
  ];
  const current = steps[step];

  useEffect(() => {
    const dialog = dialogRef.current;
    dialog?.showModal();
    requestAnimationFrame(() => headingRef.current?.focus());
    return () => dialog?.close();
  }, []);

  useEffect(() => {
    requestAnimationFrame(() => headingRef.current?.focus());
  }, [step]);

  return <dialog ref={dialogRef} className="onboarding-dialog" aria-labelledby={titleId} aria-describedby={descriptionId}
    onCancel={event => { event.preventDefault(); onClose(); }}>
    <header className="onboarding-header">
      <div>
        <strong>{t("Willkommen bei Saldonaut")}</strong>
        <span>{tr`Schritt ${step + 1} von ${steps.length}`}</span>
      </div>
      <button type="button" className="onboarding-skip" onClick={() => onClose()}>{t("Überspringen")}</button>
    </header>
    <div className="onboarding-progress" aria-hidden="true">
      {steps.map((_, index) => <span key={index} className={index <= step ? "active" : ""}/>) }
    </div>
    <section className="onboarding-content">
      <div className="onboarding-copy">
        <p className="eyebrow">{current.eyebrow}</p>
        <h2 id={titleId} ref={headingRef} tabIndex={-1}>{current.title}</h2>
        <p id={descriptionId}>{current.body}</p>
      </div>
      {current.visual}
    </section>
    <footer className="onboarding-footer">
      {step > 0 ? <button type="button" className="secondary-button" onClick={() => setStep(value => value - 1)}>{t("Zurück")}</button> : <span/>}
      {step < steps.length - 1
        ? <button type="button" className="primary-button" onClick={() => setStep(value => value + 1)}>{t("Weiter")}</button>
        : <div className="onboarding-final-actions">
          <button type="button" className="secondary-button" onClick={() => onClose("overview")}>{t("Zur Übersicht")}</button>
          <button type="button" className="primary-button" onClick={() => onClose("imports")}>{t("Ersten Import starten")}</button>
        </div>}
    </footer>
  </dialog>;
}
