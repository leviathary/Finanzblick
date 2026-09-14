import { locale, t } from "../../i18n";

export type Preview = { payload: string; instructions: string; followUp?: boolean };

export function SourcePreview({ preview }: { preview: Preview }) {
  const data = JSON.parse(preview.payload).data;
  const cardsOnly = data.accountScope === "credit_cards";
  const money = (amount: number | null | undefined) => amount == null ? "—" : new Intl.NumberFormat(locale(), { style: "currency", currency: "CHF" }).format(amount / 100);
  return <div className="chat-sources">
    {cardsOnly && <p className="chat-note"><strong>{t("Nur Kreditkarten")}</strong> — {t("Keine Daten anderer Konten, keine Einkommens- oder Vermögensübersicht.")}</p>}
    <p>{data.period.from} – {data.period.to} · CHF</p>
    {data.mode === "details" && <section className="chat-detail-preview">
      <h3>{t("Detailtransaktionen")}: {data.detailCoverage.count}</h3>
      <p>{t("Enthält nur Datum, Beträge, Währungen und Kategorien. Keine Buchungstexte oder Kontodaten. Fremdwährungen sind nicht in den CHF-Summen enthalten.")}</p>
      <details><summary>{t("Alle freigegebenen Buchungen anzeigen")}</summary><div className="chat-detail-table"><table><thead><tr><th>{t("Datum")}</th><th>{t("Kategorie")}</th><th>{t("Betrag")}</th></tr></thead><tbody>{data.detailTransactions.map((row: { id: number; bookingDate: string; categoryLabel: string; amountMinor: number; currency: string }) => <tr key={row.id}><td>{row.bookingDate}</td><td>{row.categoryLabel}</td><td>{row.currency} {new Intl.NumberFormat(locale(), { minimumFractionDigits: 2, maximumFractionDigits: 2 }).format(row.amountMinor / 100)}</td></tr>)}</tbody></table></div></details>
    </section>}
    <dl className="chat-totals">
      <div><dt>{t(cardsOnly ? "Kreditkartenbelastungen" : "Ausgaben nach Kategorien")}</dt><dd>{money(data.spending.totalMinor)}</dd></div>
      {cardsOnly && <div><dt>{t("Kreditkartengutschriften")}</dt><dd>{money(data.creditCardActivity.creditsMinor)}</dd></div>}
      {!cardsOnly && <><div><dt>{t("Geldzufluss")}</dt><dd>{money(data.cashFlow.inMinor)}</dd></div>
      <div><dt>{t("Geldabfluss")}</dt><dd>{money(data.cashFlow.outMinor)}</dd></div>
      <div><dt>{t("Netto-Geldfluss")}</dt><dd>{money(data.cashFlow.netMinor)}</dd></div>
      <div><dt>{t("Vermögensänderung im Datenzeitraum")}</dt><dd>{money(data.wealth?.changeMinor)}</dd></div></>}
    </dl>
    {data.wealth?.first && data.wealth?.last && <p className="chat-note">{t("Vermögen")}: {data.wealth.first.date} – {data.wealth.last.date}</p>}
    <p className="chat-note">{cardsOnly ? t("Summen nur in CHF. Gutschriften können Rückzahlungen oder Erstattungen sein und sind kein Einkommen.") : t("Nur vorhandene CHF-Daten. Interne Überträge sind nicht generell bereinigt; Vermögensänderung ist keine Rendite. Fehlende oder ältere Daten können das Bild verändern.")}</p>
    <details><summary>{t("Vollständige Anfrageinhalte")}</summary>
      <pre>{preview.payload}</pre>
      <details><summary>{t("Instruktionen zum Finanzmodell")}</summary><pre>{preview.instructions}</pre></details>
    </details>
    {!cardsOnly && <nav className="chat-actions" aria-label={t("Lokale Auswertungen")}>
      <a href={`#transactions?from=${data.period.from}&to=${data.period.to}`}>{t("Ausgaben & Geldfluss öffnen")}</a><a href={`#assets?from=${data.period.from}&to=${data.period.to}`}>{t("Vermögen öffnen")}</a>
    </nav>}
  </div>;
}

