// Zeigt Kartenbuchungen und persistiert explizite Einordnungen; enthält keinen Setup-Zustand.
import { useEffect, useState } from "react";
import { categoryName, locale, t, tr } from "../../i18n";
import { cardsApi } from "./api";
import type { Account, Row, Category, Decision, SettlementRule, AccountRow } from "./types";
import { kindLabels, money, date } from "./presentation";
import { SettlementRuleDialog } from "../transactions/SettlementRuleDialog";
import { TransactionActions, type TransferType } from "../transactions/TransactionActions";
import { CardAccountStatus } from "./CardAccountStatus";
import { ALL_CARDS, hasSettlementRule, selectCardRows, cardPeriodRange, sortCardRows, type CardPeriod, type CardSort, type CardSortKey } from "./overviewModel";

export function CreditCards() {
  const [snapshot,setSnapshot]=useState<{cards:Account[];categories:Category[];rules:SettlementRule[];rows:AccountRow[]}|null>(null);
  const [card,setCard]=useState(new URLSearchParams(window.location.hash.split("?")[1]).get("account")??ALL_CARDS);
  const [busy,setBusy]=useState(false);
  const [loading,setLoading]=useState(true);
  const [loadError,setLoadError]=useState("");
  const [error,setError]=useState("");
  const [filter,setFilter]=useState("ALL");
  const [period,setPeriod]=useState<CardPeriod>("current");
  const [from,setFrom]=useState(`${new Date().getFullYear()}-01-01`);
  const [to,setTo]=useState(`${new Date().getFullYear()}-12-31`);
  const [search,setSearch]=useState("");
  const [limit,setLimit]=useState(50);
  const [sort,setSort]=useState<CardSort>({key:"bookingDate",descending:true});
  useEffect(()=>{setLimit(50);},[card,filter,period,from,to,search,sort]);
  const [dialog,setDialog]=useState<Row|null>(null);
  const [reload,setReload]=useState(0);
  useEffect(()=>{
    let active=true;
    setLoading(true);setLoadError("");
    async function load() {
      const [accounts,categories,rules]=await Promise.all([cardsApi.accounts(),cardsApi.categories(),cardsApi.rules()]);
      const cards=accounts.filter(account=>account.accountType==="credit_card"&&account.isActive);
      const groups=await Promise.all(cards.map(async account=>(await cardsApi.rows(account.id)).map(row=>({...row,accountId:account.id,accountName:account.name}))));
      if(!active)return;
      setSnapshot({cards,categories:categories.filter(category=>!["income","credit_card"].includes(category.key)),rules,rows:groups.flat()});
      setCard(current=>current===ALL_CARDS||cards.some(account=>String(account.id)===current)?current:ALL_CARDS);
    }
    void load().catch(reason=>{if(active)setLoadError(String(reason));}).finally(()=>{if(active)setLoading(false);});
    return()=>{active=false;};
  },[reload]);
  const cards=snapshot?.cards??[];
  const categories=snapshot?.categories??[];
  const rules=snapshot?.rules??[];
  const rows=selectCardRows(snapshot?.rows??[],card);
  const invalidPeriod=period==="custom"&&(!from||!to||from>to);
  const filteredRows=invalidPeriod?[]:sortCardRows(selectCardRows(rows,ALL_CARDS,filter,{...cardPeriodRange(period,from,to),search}),sort,locale());
  const visibleRows=filteredRows.slice(0,limit);
  function showUnresolved() {
    setFilter("UNKNOWN");setPeriod("all");setSearch("");setLimit(50);setSort({key:"bookingDate",descending:true});
  }
  function nextSort(key:CardSortKey):CardSort {
    return {key,descending:sort.key===key?!sort.descending:key==="bookingDate"||key==="amountMinor"};
  }
  const unresolvedCount=rows.filter(row=>row.kind==="UNKNOWN").length;
  const configuredCards=cards.filter(account=>hasSettlementRule(account,rules));
  const configuredCount=configuredCards.length;
  const statusCards=card===ALL_CARDS?cards:cards.filter(account=>String(account.id)===card);
  function decisionFor(row:Row,kind:"REFUND"|"UNKNOWN"):Decision {
    return {transactionId:row.id,kind,categoryKey:kind==="REFUND"?(categories.some(c=>c.key===row.categoryKey)?row.categoryKey:"other"):null};
  }
  async function saveDecision(row:Row,kind:"REFUND"|"UNKNOWN",categoryKey?:string) {
    setBusy(true);setError("");
    try {await cardsApi.decide({...decisionFor(row,kind),...(categoryKey?{categoryKey}:{})});setReload(v=>v+1);}
    catch(e){setError(String(e));}finally{setBusy(false);}
  }
  async function action(row:Row,kind:TransferType) {
    if(kind==="CREDIT_CARD_SETTLEMENT"){setDialog(row);return;}
    setBusy(true);setError("");
    try{await cardsApi.transfer(row.id,kind);setReload(v=>v+1);}
    catch(e){setError(String(e));}finally{setBusy(false);}
  }

  return <section className="transactions-page cards-page">
    <h1>{t("Kartentransaktionen")}</h1>
    {error && <p className="error-message" role="alert">{t(error)}</p>}
    {dialog && <SettlementRuleDialog transaction={dialog} onClose={()=>setDialog(null)} onSaved={async()=>{setReload(v=>v+1);}} />}
    {loading&&<p role="status">{t("Buchungen werden geladen…")}</p>}
    {loadError&&<div className="error-message" role="alert"><p>{t(loadError)}</p><button type="button" className="secondary-button" onClick={()=>setReload(value=>value+1)}>{t("Erneut versuchen")}</button></div>}
    {!loading&&!loadError&&snapshot&&<>
    <article className={`dashboard-card cards-setup-entry ${configuredCount>0?"compact-mode":""}`}>
      <span className="cards-setup-icon" aria-hidden="true"><svg viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="1.6"><rect x="3" y="5" width="18" height="14" rx="3"/><path d="M3 10h18M7 15h4"/></svg></span>
      <div className="cards-setup-copy"><h2>{t("Kartenabrechnungen richtig zuordnen")}</h2><p>{configuredCount>0?tr`${configuredCount} von ${cards.length} Karten sind für den automatischen Rechnungsausgleich eingerichtet.`:t("Der Assistent führt dich durch die Auswahl deiner Karte und die Zuordnung der Ausgleichszahlungen. So werden deine Ausgaben nicht doppelt gezählt.")}</p>
        {configuredCards.length>0&&<ul className="cards-configured-summary" aria-label={t("Ausgleich konfiguriert")}>
          {configuredCards.map(account=><li key={account.id}><span aria-hidden="true">✓</span><span>{account.name} <small>({account.currency})</small></span></li>)}
        </ul>}
      </div>
      <a className="primary-button cards-setup-link" href="#transactions/cards/setup">{t(configuredCount>0?"Weitere Karte einrichten":"Karte einrichten")}<span aria-hidden="true">{configuredCount>0?"+":"→"}</span></a>
    </article>
    <details className="cards-explanation"><summary>{t("Wie werden Kartenbuchungen berücksichtigt?")}</summary><p>{t("Kartenkäufe zählen als Ausgaben. Bestätigte Händlererstattungen mindern sie. Rechnungsausgleiche zählen auf beiden Konten nicht nochmals. Ungeklärte Gutschriften bleiben bis zur Prüfung aus den Summen ausgeschlossen; die Auswertung ist dann unvollständig.")}</p></details>
    {!cards.length&&<p className="cards-empty-hint">{t("Bitte zuerst ein Kreditkartenkonto anlegen und Kartenbuchungen importieren.")}</p>}
    {cards.length>0&&<article className="dashboard-card cards-bookings">
      <div className="cards-bookings-heading">
        <label className="cards-account-select">{t("Kreditkartenkonto")}
          <select disabled={busy||!!dialog} value={card} onChange={event=>setCard(event.target.value)}>
            <option value={ALL_CARDS}>{t("Alle Kartenkonten")}</option>
            {cards.map(account=><option key={account.id} value={account.id}>{account.name} ({account.currency})</option>)}
          </select>
        </label>
        <div className="cards-type-filter"><label>{t("Buchungstyp")}
          <select disabled={busy||!!dialog} value={filter} onChange={event=>setFilter(event.target.value)}>
            <option value="ALL">{t("Alle Transaktionen")}</option>
            <option value="PURCHASE">{t("Nur Einkäufe")}</option>
            <option value="REFUND">{t("Nur Erstattungen")}</option>
            <option value="UNKNOWN">{t("Ungeklärte Gutschriften")} ({unresolvedCount})</option>
            <option value="SETTLEMENT">{t("Kartenausgleich")}</option>
            <option value="TRANSFER">{t("Umbuchung")}</option>
          </select>
        </label>{unresolvedCount>0&&<button type="button" className="cards-unresolved-count" disabled={busy||!!dialog}
          aria-pressed={filter==="UNKNOWN"&&period==="all"&&!search.trim()} aria-controls="card-transactions-table"
          title={t("Ungeklärte Gutschriften aus allen Jahren anzeigen")}
          onClick={showUnresolved}>{tr`${unresolvedCount} ungeklärte Gutschriften`} <span aria-hidden="true">→</span></button>}</div>
      </div>
      <div className="cards-history-filters">
        <label>{t("Zeitraum")}<select aria-label={t("Zeitraum")} disabled={busy||!!dialog} value={period} onChange={e=>setPeriod(e.target.value as CardPeriod)}>
          <option value="current">{t("Aktuelles Jahr")}</option><option value="previous">{t("Vorjahr")}</option>
          <option value="custom">{t("Eigener Zeitraum")}</option><option value="all">{t("Gesamte Historie")}</option>
        </select></label>
        {period==="custom"&&<><label>{t("Von")}<input type="date" value={from} max={to||undefined} disabled={busy||!!dialog} onChange={e=>setFrom(e.target.value)}/></label>
          <label>{t("Bis")}<input type="date" value={to} min={from||undefined} disabled={busy||!!dialog} onChange={e=>setTo(e.target.value)}/></label></>}
        <label>{t("Buchungen durchsuchen")}<input type="search" value={search} disabled={busy||!!dialog} onChange={e=>setSearch(e.target.value)} placeholder={t("Händler oder Buchungstext suchen")}/></label>
      </div>
      {invalidPeriod&&<p className="error-message" role="alert">{t("Bitte einen gültigen Zeitraum mit Start- und Enddatum wählen.")}</p>}
      <CardAccountStatus accounts={statusCards} rules={rules}/>
      <p className="cards-status-hint">{t("Der Status betrifft die Regel auf dem Kartenkonto. Die Bankseite und die Vollständigkeit der Importe werden damit nicht bestätigt.")}</p>
      {rows.some(row=>row.kind==="UNKNOWN")&&<p className="cards-review-notice" role="status">{t("Ungeklärte Gutschriften sind noch nicht in den Auswertungen enthalten. Bitte prüfe ihre Einordnung.")}</p>}
      <div className="transfer-table-scroll" id="card-transactions-table"><table className="transfer-table">
        <thead><tr>{([["bookingDate","Datum"],["accountName","Konto"],["description","Beschreibung"],["amountMinor","Betrag"]] as const).map(([key,label])=><th key={key} scope="col" aria-sort={sort.key===key?sort.descending?"descending":"ascending":undefined}>
          <button type="button" className="expense-sort" disabled={busy||!!dialog} aria-pressed={sort.key===key}
            aria-label={tr`${t(label)}: ${nextSort(key).descending?t("absteigend"):t("aufsteigend")} sortieren`} onClick={()=>setSort(nextSort(key))}>
            {t(label)} <span aria-hidden="true">{sort.key===key?sort.descending?"↓":"↑":"↕"}</span>
          </button></th>)}<th scope="col">{t("Einordnung")}</th><th scope="col">{t("Aktionen")}</th></tr></thead>
        <tbody>{!visibleRows.length&&<tr><td colSpan={6}>{t("Keine Buchungen für diese Auswahl gefunden.")}</td></tr>}{visibleRows.map(row=><tr key={row.id}>
          <td>{date(row.bookingDate)}</td><td>{row.accountName}<small>{row.currency}</small></td><td>{row.description}{row.suggestedSettlement&&row.kind==="UNKNOWN"&&<small>{t("Ausgleich vorgeschlagen")}</small>}</td><td>{money(row.amountMinor,row.currency)}</td>
          <td>{t(kindLabels[row.kind]??"Reguläre Buchung")}{row.kind==="REFUND"&&<select aria-label={t("Kategorie")} disabled={busy} value={row.categoryKey} onChange={e=>void saveDecision(row,"REFUND",e.target.value)}>{categories.map(c=><option key={c.key} value={c.key}>{categoryName(c.key,c.label)}</option>)}</select>}</td>
          <td><TransactionActions description={row.description} neutral={["SETTLEMENT","TRANSFER"].includes(row.kind)} disabled={busy} onChange={kind=>action(row,kind)}/>
            {row.kind==="UNKNOWN"&&<button disabled={busy} className="secondary-button" onClick={()=>void saveDecision(row,"REFUND")}>{t("Als Händlererstattung bestätigen")}</button>}
            {row.kind==="REFUND"&&<button disabled={busy} className="secondary-button" onClick={()=>void saveDecision(row,"UNKNOWN")}>{t("Einordnung zurücksetzen")}</button>}
          </td>
        </tr>)}</tbody>
      </table></div>
      <div className="cards-history-footer"><p role="status">{tr`${visibleRows.length} von ${filteredRows.length} Buchungen angezeigt`}</p>
        {visibleRows.length<filteredRows.length&&<button type="button" className="secondary-button" disabled={busy||!!dialog} onClick={()=>setLimit(value=>value+50)}>{t("Weitere laden")}</button>}
      </div>
    </article>}
    </>}
  </section>;
}
