// Lesende Konto-/Depotübersicht mit Positionsdetails, Charts und gezieltem Wechsel zur Verwaltung.
import { useEffect, useRef, useState } from "react";
import { invoke } from "@tauri-apps/api/core";
import { t, locale } from "../../i18n";
import type { Account } from "../accounts/types";
import { money, supportsManualValuation, typeLabel } from "../accounts/presentation";
import { ProviderLogo } from "../accounts/ProviderLogo";
import type { PositionChart } from "../assets/positionHistory";
import { DetailChart } from "./DetailChart";
import { isCurrent, localToday, positionShares, type AccountDetails, type PositionDetail } from "./model";
import "./accountDetails.css";

function selectedId() { const value = new URLSearchParams(window.location.hash.split("?")[1]).get("account"); return value && /^\d+$/.test(value) ? Number(value) : null; }
const number = (value: number | null) => value === null ? "—" : new Intl.NumberFormat(locale(), { maximumFractionDigits: 9 }).format(value);
const date = (value: string) => new Intl.DateTimeFormat(locale()).format(new Date(`${value.slice(0,10)}T00:00:00`));

export function AccountExplorer() {
  const [id, setId] = useState(selectedId);
  const [accounts, setAccounts] = useState<Account[] | null>(null);
  const [details, setDetails] = useState<AccountDetails | null>(null);
  const [error, setError] = useState("");
  const [revision, setRevision] = useState(0);
  const [search, setSearch] = useState("");
  const [archived, setArchived] = useState(false);
  const [limit, setLimit] = useState(50);
  const [bookingSort, setBookingSort] = useState<{ key: "date" | "description" | "amountMinor"; ascending: boolean }>({ key: "date", ascending: false });
  const [selectedPosition, setSelectedPosition] = useState<number | null>(null);
  const [charts, setCharts] = useState<PositionChart[]>([]);
  const [chartError, setChartError] = useState("");
  const [chartLoading, setChartLoading] = useState(false);
  const [mode, setMode] = useState<"value" | "price">("value");
  const [past, setPast] = useState(false);
  const [sort, setSort] = useState<{ key: keyof PositionDetail | "share"; ascending: boolean }>({ key: "valueMinor", ascending: false });
  const heading = useRef<HTMLHeadingElement>(null);
  const positionHeading = useRef<HTMLHeadingElement>(null);
  useEffect(() => { if (selectedPosition !== null) positionHeading.current?.focus(); }, [selectedPosition]);
  useEffect(() => {
    const change = () => { setId(selectedId()); setSelectedPosition(null); setLimit(50); };
    const refresh = () => setRevision(v => v+1);
    window.addEventListener("hashchange", change); window.addEventListener("market-data-refreshed", refresh);
    return () => { window.removeEventListener("hashchange", change); window.removeEventListener("market-data-refreshed", refresh); };
  }, []);
  useEffect(() => {
    let cancelled = false;
    setError(""); setDetails(null);
    void Promise.all([invoke<Account[]>("list_accounts"), id === null ? Promise.resolve(null) : invoke<AccountDetails>("account_details", { accountId: id, limit })])
      .then(([list, detail]) => { if (!cancelled) { setAccounts(list); setDetails(detail); } })
      .catch(e => { if (!cancelled) setError(String(e)); });
    return () => { cancelled = true; };
  }, [id, revision, limit]);
  useEffect(() => { if (details || (id === null && accounts)) heading.current?.focus(); }, [id, details, accounts]);
  useEffect(() => {
    let cancelled = false; setCharts([]); setChartError("");
    if (selectedPosition === null || id === null) return;
    setChartLoading(true);
    void invoke<PositionChart[]>("position_chart_data", { accountId: id })
      .then(rows => { if (!cancelled) setCharts(rows); })
      .catch(e => { if (!cancelled) setChartError(String(e)); })
      .finally(() => { if (!cancelled) setChartLoading(false); });
    return () => { cancelled = true; };
  }, [id, selectedPosition, revision]);
  const account = details?.account;
  const depot = account && supportsManualValuation(account.accountType);
  const shares = positionShares(details?.positions ?? []);
  const positions = (details?.positions ?? []).filter(p => past || isCurrent(p)).sort((a,b) => {
    const av = sort.key === "share" ? shares.get(a.id) : a[sort.key];
    const bv = sort.key === "share" ? shares.get(b.id) : b[sort.key];
    if (av == null) return bv == null ? 0 : 1;
    if (bv == null) return -1;
    const result = typeof av === "number" && typeof bv === "number" ? av-bv : String(av).localeCompare(String(bv), locale());
    return sort.ascending ? result : -result;
  });
  const selected = details?.positions.find(p => p.id === selectedPosition);
  const chart = charts.find(p => p.id === selectedPosition);
  const chartCurrency = mode === "value" ? "CHF" : chart?.prices[0]?.currency ?? account?.currency ?? "CHF";
  const chartHistory = mode === "value" ? chart?.history ?? [] : chart?.prices.filter(p => p.currency === chartCurrency) ?? [];
  const bookings = [...(details?.transactions ?? [])].sort((a,b) => {
    const key = bookingSort.key;
    const result = key === "amountMinor" ? a.currency.localeCompare(b.currency) || Math.abs(a.amountMinor)-Math.abs(b.amountMinor) : a[key].localeCompare(b[key],locale());
    return bookingSort.ascending ? result : -result;
  });
  function bookingHeader(key: "date" | "description" | "amountMinor", label: string) {
    return <th aria-sort={bookingSort.key === key ? bookingSort.ascending ? "ascending" : "descending" : "none"}><button className="expense-sort" onClick={() => setBookingSort({ key, ascending: bookingSort.key === key ? !bookingSort.ascending : false })}>{label} {bookingSort.key === key ? bookingSort.ascending ? "↑" : "↓" : "↕"}</button></th>;
  }
  function sortHeader(key: keyof PositionDetail | "share", label: string) {
    return <th aria-sort={sort.key === key ? sort.ascending ? "ascending" : "descending" : "none"}><button className="expense-sort" onClick={() => setSort({ key, ascending: sort.key === key ? !sort.ascending : key === "label" })}>{label} {sort.key === key ? sort.ascending ? "↑" : "↓" : "↕"}</button></th>;
  }
  return <section className="account-explorer">
    <header className="page-heading"><div><p className="eyebrow">{t("Konten & Depots")}</p><h1 ref={heading} tabIndex={-1}>{account?.name ?? t("Konten & Depots")}</h1></div>
      <a className="secondary-button" href={id === null ? "#banks" : `#banks?account=${id}`}>{t("Verwalten")}</a></header>
    {id !== null && <a className="secondary-button detail-back" href="#holdings"><span aria-hidden="true">←</span> {t("Zurück zu Konten & Depots")}</a>}
    {error ? <div role="alert" className="error-banner">{error}<button className="secondary-button" onClick={() => setRevision(v => v+1)}>{t("Erneut versuchen")}</button></div> : !accounts || (id !== null && !details) ? <p role="status">{t("Daten werden geladen …")}</p> : id === null ? <>
      <div className="detail-filters"><label>{t("Suchen")}<input type="search" value={search} onChange={e => setSearch(e.target.value)} /></label><label className="detail-checkbox"><input type="checkbox" checked={archived} onChange={e => setArchived(e.target.checked)} />{t("Archivierte Konten anzeigen")}</label></div>
      <article className="dashboard-card detail-account-list">
        {accounts.filter(a => (archived || a.isActive) && `${a.name} ${a.provider} ${typeLabel(a.accountType)}`.toLocaleLowerCase().includes(search.toLocaleLowerCase())).map(a => <a className="detail-account-link" href={`#holdings?account=${a.id}`} key={a.id}>
          <ProviderLogo name={a.provider} providerKey={a.providerKey} customLogo={a.logoDataUrl}/><div><strong>{a.name}</strong><small>{a.provider} · {typeLabel(a.accountType)}{!a.isActive && ` · ${t("Archiviert")}`}{!a.includeInNetWorth && ` · ${t("Nicht im Gesamtvermögen")}`}</small></div><div className="detail-number"><strong>{money(a.balanceMinor, a.balanceCurrency)}</strong><small>{a.balanceDate ? date(a.balanceDate) : t("Ohne Stichtag")}</small></div><span aria-hidden="true">→</span>
        </a>)}
        {!accounts.some(a => (archived || a.isActive) && `${a.name} ${a.provider} ${typeLabel(a.accountType)}`.toLocaleLowerCase().includes(search.toLocaleLowerCase())) && <p>{t("Keine passenden Konten gefunden.")}</p>}
      </article>
    </> : details && account && <>
      <article className="dashboard-card detail-summary"><ProviderLogo name={account.provider} providerKey={account.providerKey} customLogo={account.logoDataUrl}/><div><strong>{account.provider}</strong><small>{typeLabel(account.accountType)} · {account.externalReference ?? ""}</small>{!account.isActive && <small>{t("Archiviert")}</small>}{!account.includeInNetWorth && <small>{t("Nicht im Gesamtvermögen")}</small>}</div><div className="detail-total"><small>{t("Aktueller Wert")}</small><strong>{money(account.balanceMinor, account.balanceCurrency)}</strong><small>{account.balanceDate ? date(account.balanceDate) : t("Ohne Stichtag")}</small></div></article>
      <DetailChart key={id} title={depot ? t("Depotwert im Zeitverlauf") : t("Saldoverlauf")} history={details.history} currency={details.historyCurrency}/>
      {depot && <p className="detail-note">{t("Wertentwicklung, keine Rendite: Bestandsänderungen beeinflussen den Verlauf. Positionen zählen erst ab ihrem Stichtag. Fehlende Bewertungen sind nicht enthalten.")}</p>}
      {depot ? <article className="dashboard-card">
        <div className="card-heading"><h2>{t("Positionen")}</h2><label className="detail-checkbox"><input type="checkbox" checked={past} onChange={e => setPast(e.target.checked)} />{t("Beendete und zukünftige Positionen anzeigen")}</label></div>
        <div className="detail-table-scroll"><table className="detail-table"><thead><tr>{sortHeader("label", t("Bezeichnung"))}{sortHeader("symbol", t("Symbol"))}{sortHeader("quantity", t("Menge"))}{sortHeader("priceMinor", t("Kurs"))}{sortHeader("valueMinor", t("Positionswert"))}{sortHeader("share", t("Anteil"))}</tr></thead><tbody>
          {positions.map(p => <tr key={p.id} aria-selected={p.id === selectedPosition}><td><button data-position={p.id} className="detail-position-button" onClick={() => { setSelectedPosition(p.id); setMode("value"); }}>{p.label}</button><small>{p.start > localToday() ? t("Zukünftig") : !isCurrent(p) ? t("Beendet") : p.valuationDate ? date(p.valuationDate) : t("Noch nicht bewertet")}</small></td><td>{p.symbol ?? "—"}</td><td>{number(p.quantity)}</td><td>{p.priceMinor !== null && p.priceCurrency ? money(p.priceMinor, p.priceCurrency) : "—"}</td><td>{isCurrent(p) ? p.valueMinor !== null && p.valueCurrency ? money(p.valueMinor, p.valueCurrency) : t("Noch nicht bewertet") : "—"}</td><td>{shares.has(p.id) ? new Intl.NumberFormat(locale(), { style:"percent",maximumFractionDigits:1 }).format(shares.get(p.id)!) : "—"}</td></tr>)}
        </tbody></table></div>{!positions.length && <p>{t("Keine Positionen in dieser Auswahl.")}</p>}
      </article> : <article className="dashboard-card"><h2>{t("Buchungen")}</h2><div className="detail-table-scroll"><table className="detail-table"><thead><tr>{bookingHeader("date",t("Datum"))}{bookingHeader("description",t("Beschreibung"))}{bookingHeader("amountMinor",t("Betrag"))}</tr></thead><tbody>{bookings.map(row => <tr key={row.id}><td>{date(row.date)}</td><td>{row.description}</td><td>{money(row.amountMinor,row.currency)}</td></tr>)}</tbody></table></div>{!details.transactions.length && <p>{t("Keine Buchungen vorhanden.")}</p>}{details.hasMore && limit < 5000 && <button className="secondary-button" onClick={() => setLimit(v => v+50)}>{t("Weitere laden")}</button>}</article>}
      {selected && <section className="detail-position-section" aria-label={selected.label}><div className="card-heading"><h2 ref={positionHeading} tabIndex={-1}>{selected.label}</h2><div className="detail-periods"><button aria-pressed={mode === "value"} onClick={() => setMode("value")}>{t("Positionswert")}</button><button aria-pressed={mode === "price"} onClick={() => setMode("price")}>{t("Kurs")}</button><button onClick={() => { setSelectedPosition(null); requestAnimationFrame(() => document.querySelector<HTMLButtonElement>(`[data-position="${selected.id}"]`)?.focus()); }}>{t("Schließen")}</button></div></div>{chartError ? <div role="alert">{chartError}<button className="secondary-button" onClick={() => setRevision(v => v+1)}>{t("Erneut versuchen")}</button></div> : chartLoading ? <p role="status">{t("Daten werden geladen …")}</p> : <DetailChart key={`${selected.id}-${mode}`} title={mode === "value" ? t("Positionswert") : t("Kursverlauf")} currency={chartCurrency} history={chartHistory}/>}</section>}
    </>}
  </section>;
}
