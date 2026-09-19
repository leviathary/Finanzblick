// Verwaltet Kategorien sowie Regeln für Händler- und Branchenzuordnungen.

import { t, categoryName } from "../../i18n";
import { useEffect, useRef, useState } from "react";
import { invoke } from "@tauri-apps/api/core";
import "./categories.css";
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
 const [creating,setCreating]=useState(false);
 const [operation,setOperation]=useState<"delete"|"merge">("merge");
 const dialog=useRef<HTMLDialogElement>(null);
 const origin=useRef<HTMLElement|null>(null);
 const createButton=useRef<HTMLButtonElement>(null);
 function closeEditor(){setEdit(null);setCreating(false);setLabel("");requestAnimationFrame(()=>origin.current?.focus());}
 function begin(item:Category, action:"edit"|"delete"|"merge", button:HTMLButtonElement){
  const details=button.closest("details");
  origin.current=details?.querySelector("summary") ?? null;
  if(details)details.open=false;
  setError("");setMessage("");
  if(action==="edit"){setEdit(item.key);setCreating(false);setLabel(item.label);setColor(item.color);}
  else {setRemoving(item);setTarget("");setOperation(action);dialog.current?.showModal();}
 }
 async function load() {const [categories,rules]=await Promise.all([invoke<Category[]>("list_categories"),invoke<IndustryRule[]>("list_industry_rules")]);setItems(categories);setIndustries(rules);}
 useEffect(()=>{void load().catch(reason=>setError(String(reason)));},[]);
 async function save(event: React.FormEvent) {
  event.preventDefault(); setBusy(true);setError("");setMessage("");
  try {await invoke("save_category",{key:edit,label,color});await load();setMessage(edit ? t("Kategorie aktualisiert.") : t("Kategorie hinzugefügt."));closeEditor();}
  catch(reason){setError(String(reason));}finally{setBusy(false);}
 }
 async function remove() {
  if(!removing || !target)return;
  setBusy(true);setError("");setMessage("");
  try {await invoke("remove_category",{key:removing.key,targetKey:target});await load();setMessage(t("Kategorie entfernt. Buchungen und Regeln wurden in die Zielkategorie übernommen."));dialog.current?.close();setRemoving(null);requestAnimationFrame(()=>createButton.current?.focus());}
  catch(reason){setError(String(reason));}finally{setBusy(false);}
 }
 async function assignIndustry(industry:string,categoryKey:string) {
  setBusy(true);setError("");setMessage("");
  try {await invoke("save_industry_rule",{industry,categoryKey});await load();setMessage(t("Branchenregel gespeichert. Bestehende und zukünftige Buchungen werden berücksichtigt."));}
  catch(reason){setError(String(reason));}finally{setBusy(false);}
 }
 const editor=<form className="category-inline-editor" onSubmit={save} aria-label={edit ? t("Kategorie bearbeiten") : t("Neue Kategorie")} onKeyDown={event=>{if(event.key==="Escape"&&!busy){event.preventDefault();closeEditor();}}}>
   <label>{t("Name")}<input autoFocus required maxLength={80} value={label} disabled={busy} onChange={e=>setLabel(e.target.value)} /></label>
   <label>{t("Farbe")}<input type="color" value={color} disabled={busy} onChange={e=>setColor(e.target.value)} /></label>
   <button className="primary-button" disabled={busy}>{busy ? t("Bitte warten …") : t("Speichern")}</button>
   <button type="button" className="secondary-button" disabled={busy} onClick={closeEditor}>{t("Abbrechen")}</button>
   {error && <p className="error-message" role="alert">{t(error)}</p>}
 </form>;
 return <section className="transactions-page">
  <p className="eyebrow">{t("Verwaltung")}</p><h1>{t("Kategorien")}</h1>
  <p className="intro">{t("Passe Namen und Farben an deine Auswertung an. Eigene Kategorien stehen auch bei den Transaktionen zur Verfügung.")}</p>
  {error && !edit && !creating && !removing && <p className="error-message" role="alert">{t(error)}</p>}{message && <p className="import-result" role="status">{t(message)}</p>}
  <div className="category-create-action"><button ref={createButton} className="secondary-button" disabled={busy || !!edit || creating} onClick={()=>{origin.current=createButton.current;setCreating(true);setLabel("");setColor("#527b91");setError("");}}>{t("Neue Kategorie")}</button></div>
  {creating && <div className="dashboard-card category-editor">{editor}</div>}
  <dialog ref={dialog} className="settlement-rule-dialog category-removal-dialog" aria-labelledby="category-removal-title" onCancel={event=>{if(busy)event.preventDefault();}} onClose={()=>{setRemoving(null);setError("");origin.current?.focus();}}>
  {removing && <>
   <h2 id="category-removal-title">{operation==="merge" ? t("Kategorie zusammenführen") : t("Kategorie löschen")}: {categoryName(removing.key, removing.label)}</h2>
   <p>{removing.transactionCount}  {t("Buchungen und")} {removing.ruleCount}  {t("Zuordnungsregeln werden in die Zielkategorie übernommen. Neue Importe, die bisher hier zugeordnet wurden, folgen ebenfalls dieser Auswahl.")}</p>
   <label>{t("Zielkategorie")}<select disabled={busy} value={target} onChange={e=>setTarget(e.target.value)}><option value="">{t("Bitte wählen")}</option>{items.filter(c=>c.key!==removing.key).map(c=><option key={c.key} value={c.key}>{categoryName(c.key, c.label)}</option>)}</select></label>
   {error && <p className="error-message" role="alert">{t(error)}</p>}
   <div className="category-dialog-actions"><button className="secondary-button" disabled={busy} onClick={()=>dialog.current?.close()}>{t("Abbrechen")}</button><button className={operation==="delete" ? "danger-button" : "primary-button"} disabled={busy || !target} onClick={()=>void remove()}>{busy ? t("Bitte warten …") : t("Übernehmen und Kategorie entfernen")}</button></div>
  </>}</dialog>
  <div className="dashboard-card managed-categories">{items.map(item=><div className="managed-category" key={item.key}>
   <span className="category-color" style={{background:item.color}}/><div><strong>{categoryName(item.key,item.label)}</strong><small>{item.transactionCount} {t("Buchungen ·")} {item.ruleCount} {t("Regeln")}</small></div>
   <details className="category-menu" onToggle={event=>{if(event.currentTarget.open)document.querySelectorAll<HTMLDetailsElement>(".category-menu[open]").forEach(other=>{if(other!==event.currentTarget)other.open=false;});}} onKeyDown={event=>{if(event.key==="Escape"){event.currentTarget.open=false;event.currentTarget.querySelector("summary")?.focus();}}} onBlur={event=>{if(!event.currentTarget.contains(event.relatedTarget as Node))event.currentTarget.open=false;}}>
    <summary aria-label={t("Aktionen")+": "+categoryName(item.key,item.label)}>⋯</summary>
    <div className="category-menu-options">
     <button disabled={busy || !!edit || creating} onClick={event=>begin(item,"edit",event.currentTarget)}>{t("Bearbeiten")}</button>
     <button disabled={busy || !!edit || creating || items.length<2} onClick={event=>begin(item,"merge",event.currentTarget)}>{t("Zusammenführen …")}</button>
     <button className="category-delete-option" disabled={busy || !!edit || creating || items.length<2} onClick={event=>begin(item,"delete",event.currentTarget)}>{t("Löschen …")}</button>
    </div>
   </details>
   {edit===item.key && editor}
  </div>)}</div>
  {industries.length>0 && <div className="dashboard-card industry-rules"><h2>{t("Kartenkäufe automatisch kategorisieren")}</h2><p className="intro">{t("Wähle für jede Branche die passende Ausgabenkategorie – zum Beispiel „Lebensmittel & Haushalt“ für Lebensmittelgeschäfte. Deine Auswahl gilt für bestehende und künftig importierte Kartenkäufe.")}</p><p className="settings-hint">{t("Selbst zugewiesene Kategorien und Regeln für einzelne Händler bleiben erhalten.")}</p>{industries.map(rule=><label key={rule.industry}><span><strong>{rule.industry}</strong><small>{rule.transactionCount} {t("Buchungen")}</small></span><select disabled={busy} value={rule.categoryKey} onChange={event=>void assignIndustry(rule.industry,event.target.value)}>{items.map(item=><option key={item.key} value={item.key}>{categoryName(item.key,item.label)}</option>)}</select></label>)}</div>}
 </section>;
}
