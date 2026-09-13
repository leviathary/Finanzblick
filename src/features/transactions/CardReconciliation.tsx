import { t, tr, locale } from "../../i18n";
import { useEffect, useState } from "react";
import { invoke } from "@tauri-apps/api/core";
interface Statement {source:string;account:string;currency:string;firstDate:string;lastDate:string;debits:number;credits:number;payments:number;unmatchedCredits:number;closingBalance:number|null;paymentDates:string|null}
interface Result {statements:Statement[];unmatchedPayments:{date:string;account:string;currency:string;amount:number}[]}
export function CardReconciliation() {
 const [data,setData]=useState<Result|null>(null);const [error,setError]=useState("");
 useEffect(()=>{invoke<Result>("card_reconciliation").then(setData).catch(e=>setError(String(e)));},[]);
 return <article className="dashboard-card reconciliation"><p className="eyebrow">{t("Kreditkartenabgleich")}</p><h2>{t("Kartenbuchungen und LSV-Zahlungen")}</h2>
 <p className="intro">{t("Alle importierten Zeiträume und Konten. Jede Datei wird separat gezeigt: Die darin enthaltene LSV-Gutschrift kann die vorherige Abrechnung bezahlen. Ohne explizite Rechnungszuordnung ist die Differenz zwischen Einkäufen und Zahlungen kein offener Rechnungsbetrag.")}</p>
 {error && <p role="alert">{t(error)}</p>}{!data && !error && <p>{t("Lade Abgleich…")}</p>}
 {data && <>
 {data.unmatchedPayments.length>0 && <div className="import-result"><strong>{t("Bankzahlungen ohne eindeutig passende Kartengutschrift")}</strong>{data.unmatchedPayments.map((p,i)=><p key={i}>{date(p.date)} · {p.account} · {money(p.amount,p.currency)}  {t("— Gegenbuchung fehlt oder ist nicht eindeutig.")}</p>)}</div>}
 {!data.statements.length && <p>{t("Keine Kreditkartenbuchungen importiert.")}</p>}
 {data.statements.map((s,i)=><details key={i} className="reconciliation-item"><summary><strong>{s.source}</strong><span>{date(s.firstDate)} – {date(s.lastDate)} · {s.account}</span><span>{s.unmatchedCredits>0 ? t("Zahlung nicht zugeordnet") : s.payments>0 ? t("LSV-Gegenbuchung gefunden") : t("Keine LSV-Gutschrift in dieser Datei")}</span></summary>
 <div className="transaction-kpis"><article><span>{t("Kartenbelastungen in der Datei")}</span><strong>{money(s.debits,s.currency)}</strong></article><article><span>{t("Gutschriften in der Datei")}</span><strong>{money(s.credits,s.currency)}</strong></article><article><span>{t("Davon mit Bankzahlung abgeglichen")}</span><strong>{money(s.payments,s.currency)}</strong><small>{s.paymentDates?.split(",").map(date).join(", ") || t("Keine zugeordnete Zahlung")}</small></article></div>
 <p>{t("Veränderung durch diese Buchungen:")} <strong>{money(s.debits-s.credits,s.currency)}</strong>  {t("(Belastungen minus Gutschriften).")}</p>
 <p>{s.closingBalance===null ? t("Kein Schlusssaldo gespeichert: Eine offene Kartenschuld lässt sich aus dieser Datei nicht verlässlich bestimmen.") : tr`Gespeicherter Schlusssaldo: ${money(s.closingBalance,s.currency)}. Ein negativer Saldo entspricht einer Kartenschuld; er gilt für diesen Auszug, nicht für heute.`}</p>
 {s.unmatchedCredits>0 && <p>{t("Noch nicht zugeordnete Zahlungsgutschriften:")} {money(s.unmatchedCredits,s.currency)}.</p>}
 </details>)}
 </>}
 </article>;
}
function money(n:number,currency:string){return new Intl.NumberFormat(locale(),{style:"currency",currency}).format(n/100);}
function date(s:string){return s.slice(0,10).split("-").reverse().join(".");}
