// Bündelt die Kontexthilfe des Vermögenscharts in einem tastaturbedienbaren Dialog ohne Chart-Zustandsänderungen.
import { useId, useRef } from "react";
import { createPortal } from "react-dom";
import { t } from "../../i18n";

export function ChartHelp() {
  const dialog = useRef<HTMLDialogElement>(null);
  const trigger = useRef<HTMLButtonElement>(null);
  const heading = useRef<HTMLHeadingElement>(null);
  const titleId = useId();
  return <>
    <button ref={trigger} type="button" className="wealth-chart-help" title={t("Hilfe zum Chart")} aria-label={t("Hilfe zum Chart")} aria-haspopup="dialog" onClick={() => { dialog.current?.showModal(); heading.current?.focus(); }}>
      <svg width="16" height="16" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="1.7" aria-hidden="true"><circle cx="12" cy="12" r="9"/><path d="M9.5 9a2.5 2.5 0 0 1 5 0c0 2-2.5 2-2.5 4M12 16v1"/></svg>
    </button>
    {createPortal(<dialog ref={dialog} className="settlement-rule-dialog chart-help-dialog" aria-labelledby={titleId} onClose={() => trigger.current?.focus()}>
      <h2 id={titleId} ref={heading} tabIndex={-1}>{t("Hilfe zum Chart")}</h2>
      <div className="chart-help-content">
        <section><h3>{t("Konten und Positionen")}</h3><p>{t("Wähle links Gesamtvermögen oder suche einzelne Konten und Depots. Mehrere ausgewählte Konten werden zusammengefasst. Bei unterstützten Depots kannst du einzelne Positionen wählen. Positionswert zeigt deren Gesamtwert; Kurs zeigt den Preis pro Anteil und ist nur bei einer einzelnen Position mit verfügbaren Kursdaten aktiv.")}</p></section>
        <section><h3>{t("Zeiträume und Kennzahlen")}</h3><p>{t("Die Zeitraumknöpfe filtern den Verlauf und die Veränderungskennzahl. YTD bedeutet laufendes Kalenderjahr, Max die gesamte verfügbare Historie. Unter … wählst du einen eigenen Zeitraum. Der aktuelle Vermögenswert bleibt der neueste verfügbare Stand.")}</p></section>
        <section><h3>{t("Zoomen und Verschieben")}</h3><p>{t("Zoome mit dem Mausrad oder einer Zwei-Finger-Geste und verschiebe den Ausschnitt durch Ziehen. Das ändert keine Kennzahlen oder Daten. Das Reset-Icon zeigt wieder den gesamten gewählten Zeitraum und entfernt die Messung.")}</p></section>
        <section><h3>{t("Messen und Tastatur")}</h3><p>{t("Halte Shift gedrückt und ziehe zwischen zwei Punkten. Alternativ aktivierst du das Lineal und klickst Start und Ende an. Das Ergebnis zeigt Differenz, Prozentänderung und Kalendertage; im Benchmark-Modus die Differenz in Indexpunkten.")}</p><p>{t("Mit Tab fokussierst du den Chart. Pfeiltasten links/rechts wählen Tageswerte, Home/End den ersten oder letzten Wert; +/− zoomt. Bei aktivem Lineal setzt Enter Start und Ende. Escape löscht die Messung.")}</p></section>
        <section><h3>{t("Benchmarks verstehen")}</h3><p>{t("Unter Vergleichen wählst du SMI oder S&P 500. Kein Vergleich stellt die normale Betragsachse wieder her. Die Benchmark-Auswahl verändert keine Vermögenskennzahlen.")}</p><p>{t("Basis 100 am ersten gemeinsamen Datum. Index in Originalwährung, ohne Dividenden. Vermögensänderungen enthalten Ein- und Auszahlungen; dies ist kein Renditevergleich.")}</p><p>{t("Fehlt ein gemeinsamer Zeitraum mit positivem Startwert, bleibt der normale Chart sichtbar. Bei Ladefehlern kannst du im Vergleichsmenü erneut versuchen, die Daten abzurufen.")}</p></section>
        <section><h3>{t("Datenquelle und Datenschutz")}</h3><p>{t("Quelle: Yahoo Finance. Beim Auswählen werden Index und Startdatum übermittelt, keine Finanzdaten.")}</p></section>
      </div>
      <form method="dialog" className="chart-help-footer"><button className="secondary-button">{t("Schließen")}</button></form>
    </dialog>, document.body)}
  </>;
}
