// Gemeinsame Beschriftungen und Formatierung der Kartenansicht und Einrichtung.
import { locale } from "../../i18n";
export const kindLabels:Record<string,string>={PURCHASE:"Kartenkauf",REFUND:"Händlererstattung",SETTLEMENT:"Kartenausgleich",TRANSFER:"Umbuchung",UNKNOWN:"Ungeklärte Gutschrift",REGULAR:"Reguläre Buchung"};
export const money=(amount:number,currency:string)=>new Intl.NumberFormat(locale(),{style:"currency",currency}).format(amount/100);
export const date=(value:string)=>new Intl.DateTimeFormat(locale()).format(new Date(value+"T12:00:00"));
