// Verwaltet ausschließlich den vierstufigen Karten-Setup-Workflow und seine Entwürfe.
import { TransactionTabs } from "../../transactions/TransactionTabs";
import { useEffect, useState } from "react";
import { t, tr } from "../../../i18n";
import { cardsApi } from "../api";
import type { Account, Row, Preview } from "../types";
import { money, date } from "../presentation";
import { PatternTable } from "./PatternTable";
import { draft, validPreview, filterBankPayments, bankPaymentStart, type BankPaymentSort } from "./model";
function usePreview(row:Row|undefined,prefix:string) {
  const requestKey=JSON.stringify([row?.id??null,prefix]);
  const [state,setState]=useState<{key:string;data:Preview|null;loading:boolean;error:string}>({key:"",data:null,loading:false,error:""});
  useEffect(()=>{
    let active=true;
    setState({key:requestKey,data:null,loading:!!row,error:""});
    if(row) void cardsApi.preview(row.id,prefix)
      .then(data=>{if(active)setState({key:requestKey,data,loading:false,error:""});})
      .catch(error=>{if(active)setState({key:requestKey,data:null,loading:false,error:String(error)});});
    return()=>{active=false;};
  },[row?.id,prefix]);
  return state.key===requestKey?state:{data:null,loading:!!row,error:""};
}
function RulePreview({value}:{value:ReturnType<typeof usePreview>}) {
  if(value.loading)return <p role="status">{t("Buchungen werden geladen…")}</p>;
  if(value.error)return <p role="alert" className="error-message">{t(value.error)}</p>;
  if(!value.data)return null;
  const p=value.data;
  return <div className="transfer-help"><strong>{p.accountName} · {p.currency} · {t(p.direction<0?"Belastung":"Gutschrift")}</strong>
    <p>{tr`${p.matches.length} passende Buchungen in der gesamten Historie. ${p.protectedCount} manuelle Entscheidungen werden nicht verändert.`}</p>
    <div className="settlement-preview"><table><tbody>{p.matches.map(r=><tr key={r.id}><td>{date(r.bookingDate)}</td><td>{r.description}</td><td>{money(r.amountMinor,p.currency)}</td></tr>)}</tbody></table></div>
  </div>;
}
export function CardSetupWizard() {
  const [accounts,setAccounts]=useState<Account[]>([]);
  const [card,setCard]=useState(new URLSearchParams(window.location.hash.split("?")[1]).get("account")??"");
  const [bank,setBank]=useState("");
  const [rows,setRows]=useState<Row[]>([]);
  const [bankRows,setBankRows]=useState<Row[]>([]);
  const [bankSearch,setBankSearch]=useState("");
  const [bankSort,setBankSort]=useState<BankPaymentSort>("newest");
  const [bankMonths,setBankMonths]=useState(2);
  const [step,setStep]=useState(1);
  const [cardSeed,setCardSeed]=useState("");
  const [bankSeed,setBankSeed]=useState("");
  const [cardPrefix,setCardPrefix]=useState("");
  const [bankPrefix,setBankPrefix]=useState("");
  const [past,setPast]=useState(true);
  const [future,setFuture]=useState(true);
  const [busy,setBusy]=useState(false);
  const [loading,setLoading]=useState(false);
  const [bankLoading,setBankLoading]=useState(false);
  const [error,setError]=useState("");
  useEffect(()=>{void cardsApi.accounts()
    .then(setAccounts)
    .catch(reason=>setError(String(reason)));},[]);
  useEffect(()=>{
    let active=true;setRows([]);setCardSeed("");setLoading(!!card);setError("");
    if(card)void cardsApi.rows(Number(card))
      .then(r=>{if(active)setRows(r);}).catch(e=>{if(active)setError(String(e));}).finally(()=>{if(active)setLoading(false);});
    return()=>{active=false;};
  },[card]);
  useEffect(()=>{
    let active=true;setBankRows([]);setBankSeed("");setBankMonths(2);setBankLoading(!!bank);setError("");
    if(bank)void cardsApi.rows(Number(bank))
      .then(r=>{if(active)setBankRows(r.filter(x=>x.amountMinor<0));}).catch(e=>{if(active)setError(String(e));}).finally(()=>{if(active)setBankLoading(false);});
    return()=>{active=false;};
  },[bank]);
  const cards=accounts.filter(a=>a.accountType==="credit_card"&&a.isActive);
  const banks=accounts.filter(a=>["checking","cash","savings"].includes(a.accountType)&&a.isActive);
  const cardRow=rows.find(r=>r.id===Number(cardSeed));
  const bankRow=bankRows.find(r=>r.id===Number(bankSeed));
  const cardPreview=usePreview(cardRow,cardPrefix);
  const bankPreview=usePreview(bankRow,bankPrefix);
  const credits=rows.filter(r=>r.amountMinor>0);
  function chooseCard(id:string) {
    setCardSeed(id);
    const row=rows.find(r=>r.id===Number(id));setCardPrefix(row?.description.split("·")[0].trim()??"");
  }
  const canSave=!!card&&(!!cardRow||!!bankRow)
    &&validPreview(cardRow,cardPreview)&&validPreview(bankRow,bankPreview)&&(!(cardRow||bankRow)||past||future);
  async function confirm() {
    setBusy(true);setError("");
    try {
      await cardsApi.confirm({cardId:Number(card),cardRule:draft(cardRow,cardPreview.data),bankRule:draft(bankRow,bankPreview.data),decisions:[],past,future});
      window.location.hash="transactions/cards?account="+card;
    }catch(e){setError(String(e));}finally{setBusy(false);}
  }
  function chooseBank(id:string) {
    setBankSeed(id);
    setBankPrefix(bankRows.find(row=>row.id===Number(id))?.description.split("·")[0].trim()??"");
  }
  function skipStep() {
    const hasDraft=step===2?!!cardSeed:!!bankSeed;
    if(hasDraft&&!window.confirm(t("Auswahl und vorgemerkte Änderungen dieses Schritts verwerfen und überspringen?")))return;
    if(step===2){chooseCard("");}
    if(step===3)chooseBank("");
    setStep(current=>current+1);
  }
  return <section className="transactions-page cards-wizard">
    <h1>{t("Karte einrichten")}</h1>
    <TransactionTabs active="cards" />
    {error&&<p className="error-message" role="alert">{t(error)}</p>}
    <aside className="wizard-logic-note">
      <strong>{t("Hinweis zur Buchungslogik:")}</strong>
      <ul>
        <li><strong>{t("Kartenkäufe:")}</strong> {t("Gelten als Konsumausgaben.")}</li>
        <li><strong>{t("Rechnungsausgleiche:")}</strong> {t("Werden neutralisiert – keine doppelten Ausgaben.")}</li>
        <li><strong>{t("Erstattungen:")}</strong> {t("Mindern die Ausgaben der jeweiligen Kategorie.")}</li>
      </ul>
      <p>{t("Ungeklärte Kartengutschriften sind noch nicht in den Ausgaben berücksichtigt.")}</p>
    </aside>
    <ol className="card-setup-steps">{["Karte wählen","Zahlungseingang auf der Karte","Abbuchung vom Bankkonto","Bestätigen & Anwenden"].map((label,index)=><li key={label} className={index+1<step?"step-passed":undefined} aria-current={step===index+1?"step":undefined}>{index+1}. {t(label)}</li>)}</ol>
    <div className="cards-wizard-body">
    {step===1&&<article className="dashboard-card">
      <label>{t("Kreditkartenkonto")}<select disabled={busy} value={card} onChange={e=>setCard(e.target.value)}><option value="">{t("Bitte Konto wählen")}</option>{cards.map(a=><option value={a.id} key={a.id}>{a.name} ({a.currency})</option>)}</select></label>
      {!cards.length&&<p>{t("Bitte zuerst ein Kreditkartenkonto anlegen und Kartenbuchungen importieren.")}</p>}
    </article>}
    {loading&&<p role="status">{t("Buchungen werden geladen…")}</p>}
    {step===2&&card&&!loading&&<article className="dashboard-card">
      <h2>{t("Welcher Zahlungseingang auf der Kreditkarte gleicht die Rechnung aus?")}</h2>
      <p><strong>{t("Kreditkartenkonto")}: {cards.find(a=>a.id===Number(card))?.name} ({cards.find(a=>a.id===Number(card))?.currency})</strong></p>
      <p>{t("Hier siehst du Gutschriften auf deiner Kreditkarte: positive Beträge (+). Wähle den Eingang deiner Rechnungszahlung, nicht eine Händlererstattung. Die Abbuchung vom Bankkonto folgt in Schritt 3.")}</p>
      <p>{t("Wähle direkt in der Tabelle eine Zahlung als Muster. Empfehlungen sind nur Vorschläge und werden nicht automatisch übernommen.")}</p>
      <PatternTable rows={credits} selectedId={cardSeed} disabled={busy} onSelect={chooseCard}/>
      {cardRow&&<><p>{t("Die gewählte Buchung dient als Beispiel für die Ausgleichsregel. Änderungen werden erst im letzten Schritt gespeichert.")}</p><label>{t("Buchungstext beginnt mit")}<input disabled={busy} value={cardPrefix} onChange={e=>setCardPrefix(e.target.value)}/></label><RulePreview value={cardPreview}/></>}
    </article>}
    {step===3&&<article className="dashboard-card"><h2>{t("Von welchem Konto bezahlst du die Kartenrechnung?")}</h2>
      <p>{t("Jetzt geht es um die andere Kontoseite: die Abbuchung vom Bankkonto mit negativem Betrag (−). Wähle zuerst das Bankkonto und dann die Buchung, mit der du die Kartenrechnung bezahlt hast – keinen einzelnen Einkauf.")}</p>
      <label>{t("Bankkonto")}<select disabled={busy} value={bank} onChange={e=>setBank(e.target.value)}><option value="">{t("Überspringen / Konto noch nicht importiert")}</option>{banks.map(a=><option key={a.id} value={a.id}>{a.name} ({a.currency})</option>)}</select></label>
      {bank&&<label>{t("Bankabbuchungen durchsuchen")}<input type="search" value={bankSearch} disabled={busy} placeholder={t("Buchungstext suchen")} onChange={e=>setBankSearch(e.target.value)}/></label>}
      {bank&&<p role="status">{tr`Angezeigt werden Abbuchungen der letzten ${bankMonths} Monate bis heute. Die Regel kann weiterhin auf die gesamte Historie angewendet werden.`}</p>}
      {bankLoading?<p role="status">{t("Buchungen werden geladen…")}</p>:bank&&<>
        <PatternTable rows={filterBankPayments(bankRows,bankSearch,bankSort,new Date(),bankMonths)} selectedId={bankSeed} disabled={busy} onSelect={chooseBank} sort={bankSort} onSort={setBankSort}/>
        {bankRows.some(row=>row.amountMinor<0&&row.bookingDate<bankPaymentStart(new Date(),bankMonths))&&<button type="button" className="secondary-button" disabled={busy} onClick={()=>setBankMonths(months=>months+2)}>{t("Mehr laden")}</button>}
      </>}
      {bankRow&&<><label>{t("Buchungstext beginnt mit")}<input disabled={busy} value={bankPrefix} onChange={e=>setBankPrefix(e.target.value)}/></label><RulePreview value={bankPreview}/></>}
    </article>}
    {step===4&&<article className="dashboard-card"><h2>{t("Bestätigen & Anwenden")}</h2>
      <p>{t("Bitte prüfe beide Regeln. Es werden keine Einkäufe mit Abrechnungen verknüpft. Kontosalden bleiben unverändert.")}</p>
      <h3>{t("Kreditkartenkonto")}</h3>{cardRow?<RulePreview value={cardPreview}/>:<p>{t("Keine Regel für diese Kontoseite.")}</p>}
      <h3>{t("Bankkonto")}</h3>{bankRow?<RulePreview value={bankPreview}/>:<p>{t("Keine Regel für diese Kontoseite.")}</p>}
      <label><input type="checkbox" disabled={busy} checked={past} onChange={e=>setPast(e.target.checked)}/>{t("Regeln auf bisherige Buchungen anwenden")}</label>
      <label><input type="checkbox" disabled={busy} checked={future} onChange={e=>setFuture(e.target.checked)}/>{t("Regel auch auf zukünftige Importe anwenden")}</label>
      <p>{t("Bestehende manuelle Entscheidungen bleiben geschützt. Offene Gutschriften kannst du unter Kartentransaktionen prüfen.")}</p>
    </article>}
    <footer className="wizard-footer" aria-label={t("Schrittnavigation")}>
      <div className="wizard-footer-back">
        {step>1&&<button type="button" className="secondary-button" disabled={busy} onClick={()=>setStep(s=>s-1)}>← {tr`Zurück zu Schritt ${step-1}`}</button>}
        <a href={"#transactions/cards?account="+card} aria-disabled={busy} onClick={e=>{if(busy)e.preventDefault();}}>{t("Abbrechen")}</a>
      </div>
      <div className="wizard-footer-next">
        {(step===2||step===3)&&<button type="button" className="secondary-button" disabled={busy||loading||bankLoading} onClick={skipStep}>{t("Schritt überspringen")}</button>}
        {step<4?<button type="button" className="primary-button" disabled={busy||loading||!cards.some(a=>a.id===Number(card))||(step===2&&(!cardRow||!validPreview(cardRow,cardPreview)))||(step===3&&(bankLoading||!validPreview(bankRow,bankPreview)))} onClick={()=>setStep(s=>s+1)}>{tr`Weiter zu Schritt ${step+1}`} →</button>:
        <button type="button" className="primary-button" disabled={busy||!canSave} onClick={()=>void confirm()}>{t("Einrichtung übernehmen")}</button>}
      </div>
    </footer>
    </div>
  </section>;
}
