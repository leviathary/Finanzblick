// Erklärt Saldoverlauf und Auswertungsfilter ohne Benchmark- oder Depotfunktionen.
import { useId, useRef } from "react";
import { createPortal } from "react-dom";
import { t } from "../../i18n";

export function BalanceChartHelp() {
  const dialog = useRef<HTMLDialogElement>(null);
  const trigger = useRef<HTMLButtonElement>(null);
  const heading = useRef<HTMLHeadingElement>(null);
  const id = useId();
  return <>
    <button ref={trigger} type="button" className="wealth-chart-help" title={t("Hilfe zum Chart")} aria-label={t("Hilfe zum Chart")} aria-haspopup="dialog" onClick={() => { dialog.current?.showModal(); heading.current?.focus(); }}>
      <svg width="16" height="16" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="1.7" aria-hidden="true"><circle cx="12" cy="12" r="9"/><path d="M9.5 9a2.5 2.5 0 0 1 5 0c0 2-2.5 2-2.5 4M12 16v1"/></svg>
    </button>
    {createPortal(<dialog ref={dialog} className="settlement-rule-dialog chart-help-dialog" aria-labelledby={id} onClose={() => trigger.current?.focus()}>
      <h2 id={id} ref={heading} tabIndex={-1}>{t("Hilfe zum Chart")}</h2>
      <div className="chart-help-content">
        <section><h3>{t("Saldoverlauf")}</h3><p>{t("Der Saldoverlauf zeigt nur Privat- und Sparkonten. Die Bank- und Kontofilter im Chart gelten nur für die Grafik. Die Auswertung darunter hat eigene Filter und berücksichtigt auch Kreditkarten. Der Zeitraum gilt für beide Bereiche: YTD zeigt das laufende Kalenderjahr, Max die gesamte Historie; unter … wählst du einen eigenen Zeitraum.")}</p></section>
        <section><h3>{t("Zoomen und Verschieben")}</h3><p>{t("Zoome mit dem Mausrad oder einer Zwei-Finger-Geste und verschiebe den Ausschnitt durch Ziehen. Das ändert keine Kennzahlen oder Daten. Das Reset-Icon zeigt wieder den gesamten gewählten Zeitraum und entfernt die Messung.")}</p></section>
        <section><h3>{t("Messen und Tastatur")}</h3><p>{t("Shift + Ziehen misst die Veränderung zwischen zwei Tageswerten. Alternativ aktivierst du das Lineal und klickst Start und Ende an. Die Messung zeigt Betrag, Prozentänderung und Kalendertage.")}</p><p>{t("Mit Tab fokussierst du den Chart. Pfeiltasten links/rechts wählen Tageswerte, Home/End den ersten oder letzten Wert; +/− zoomt. Bei aktivem Lineal setzt Enter Start und Ende. Escape löscht die Messung.")}</p></section>
      </div>
      <form method="dialog" className="chart-help-footer"><button className="secondary-button">{t("Schließen")}</button></form>
    </dialog>, document.body)}
  </>;
}
