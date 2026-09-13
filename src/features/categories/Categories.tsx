import { t, tr, categoryName } from "../../i18n";
import { useEffect, useState } from "react";
import { invoke } from "@tauri-apps/api/core";
interface Category { key: string; label: string; color: string; transactionCount: number; ruleCount: number }
interface IndustryRule { industry: string; categoryKey: string; transactionCount: number }
export function Categories() {
 const [items,setItems]=useState<Category[]>([]);
 const [industries,setIndustries]=useState<IndustryRule[]>([]);
 const [edit,setEdit]=useState<string|null>(null);
 const [label,setLabel]=useState("");
 const [color,setColor]=useState("#527b91");
 const [removing,setRemoving]=useState<Category|null>(null);
 const [target,setTarget]=useState("");
 const [busy,setBusy]=useState(false);
 const [error,setError]=useState("");
 const [message,setMessage]=useState("");
 async function load() {const [categories,rules]=await Promise.all([invoke<Category[]>("list_categories"),invoke<IndustryRule[]>("list_industry_rules")]);setItems(categories);setIndustries(rules);}
 useEffect(()=>{void load().catch(reason=>setError(String(reason)));},[]);
 async function save(event: React.FormEvent) {
  event.preventDefault(); setBusy(true);setError("");setMessage("");
  try {await invoke("save_category",{key:edit,label,color});await load();setMessage(edit ? t("Kategorie aktualisiert.") : t("Kategorie hinzugefügt."));setEdit(null);setLabel("");}
  catch(reason){setError(String(reason));}finally{setBusy(false);}
 }
 async function remove() {
  if(!removing || !target)return;
  setBusy(true);setError("");setMessage("");
  try {await invoke("remove_category",{key:removing.key,targetKey:target});await load();setMessage(t("Kategorie entfernt. Buchungen und Regeln wurden in die Zielkategorie übernommen."));setRemoving(null);if(edit===removing.key){setEdit(null);setLabel("");}}
  catch(reason){setError(String(reason));}finally{setBusy(false);}
 }
 async function assignIndustry(industry:string,categoryKey:string) {
  setBusy(true);setError("");setMessage("");
  try {await invoke("save_industry_rule",{industry,categoryKey});await load();setMessage(t("Branchenregel gespeichert. Bestehende und zukünftige Buchungen werden berücksichtigt."));}
  catch(reason){setError(String(reason));}finally{setBusy(false);}
 }
 return <section className="transactions-page">
  <p className="eyebrow">{t("Verwaltung")}</p><h1>{t("Kategorien")}</h1>
  <p className="intro">{t("Passe Namen und Farben an deine Auswertung an. Eigene Kategorien stehen auch bei den Transaktionen zur Verfügung.")}</p>
  {error && <p className="error-message" role="alert">{t(error)}</p>}{message && <p className="import-result" role="status">{t(message)}</p>}
  <form className="dashboard-card category-editor" onSubmit={save}>
   <h2>{edit ? t("Kategorie bearbeiten") : t("Neue Kategorie")}</h2>
   <div className="drilldown-filters"><label>{t("Name")}<input required maxLength={80} value={label} onChange={e=>setLabel(e.target.value)} /></label><label>{t("Farbe")}<input type="color" value={color} onChange={e=>setColor(e.target.value)} /></label><button className="primary-button" disabled={busy}>{edit ? t("Änderungen speichern") : t("Kategorie hinzufügen")}</button>{edit && <button type="button" className="text-button" disabled={busy} onClick={()=>{setEdit(null);setLabel("");}}>{t("Abbrechen")}</button>}</div>
  </form>
  {removing && <div className="dashboard-card category-editor" role="region" aria-label={t("Kategorie entfernen")}>
   <h2>„{categoryName(removing.key, removing.label)}{t("“ löschen oder zusammenführen")}</h2>
   <p>{removing.transactionCount}  {t("Buchungen und")} {removing.ruleCount}  {t("Zuordnungsregeln werden in die Zielkategorie übernommen. Neue Importe, die bisher hier zugeordnet wurden, folgen ebenfalls dieser Auswahl.")}</p>
   <div className="drilldown-filters"><label>{t("Zielkategorie")}<select value={target} onChange={e=>setTarget(e.target.value)}><option value="">{t("Bitte wählen")}</option>{items.filter(c=>c.key!==removing.key).map(c=><option key={c.key} value={c.key}>{categoryName(c.key, c.label)}</option>)}</select></label><button className="primary-button" disabled={busy || !target} onClick={()=>void remove()}>{t("Übernehmen und Kategorie entfernen")}</button><button className="text-button" disabled={busy} onClick={()=>setRemoving(null)}>{t("Abbrechen")}</button></div>
  </div>}
  <div className="dashboard-card managed-categories">{items.map(item=><div className="managed-category" key={item.key}><span className="category-color" style={{background:item.color}}/><div><strong>{categoryName(item.key, item.label)}</strong><small>{item.transactionCount}  {t("Buchungen ·")} {item.ruleCount}  {t("Regeln")}</small></div><button className="secondary-button" aria-label={tr`Bearbeiten: ${categoryName(item.key, item.label)}`} disabled={busy} onClick={()=>{setEdit(item.key);setLabel(item.label);setColor(item.color);setRemoving(null);window.scrollTo({top:0,behavior:"smooth"});}}>{t("Bearbeiten")}<span className="sr-only">: {categoryName(item.key, item.label)}</span></button><button className="text-button" aria-label={tr`Löschen / Zusammenführen: ${categoryName(item.key, item.label)}`} disabled={busy || items.length<2} onClick={()=>{setRemoving(item);setTarget("");window.scrollTo({top:0,behavior:"smooth"});}}>{t("Löschen / Zusammenführen")}<span className="sr-only">: {categoryName(item.key, item.label)}</span></button></div>)}</div>
  {industries.length>0 && <div className="dashboard-card industry-rules"><h2>{t("Branchen automatisch zuordnen")}</h2><p className="intro">{t("Die Branche stammt unverändert aus dem Kreditkartenauszug. Händlerregeln und manuelle Kategorien haben weiterhin Vorrang.")}</p>{industries.map(rule=><label key={rule.industry}><span><strong>{rule.industry}</strong><small>{rule.transactionCount} {t("Buchungen")}</small></span><select disabled={busy} value={rule.categoryKey} onChange={event=>void assignIndustry(rule.industry,event.target.value)}>{items.map(item=><option key={item.key} value={item.key}>{categoryName(item.key,item.label)}</option>)}</select></label>)}</div>}
 </section>;
}
