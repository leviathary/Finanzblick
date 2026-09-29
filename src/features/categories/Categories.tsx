// Verwaltet Kategorien sowie Regeln für Händler- und Branchenzuordnungen.

import { t, categoryName } from "../../i18n";
import { useEffect, useRef, useState } from "react";
import { invoke } from "@tauri-apps/api/core";
import { ContextHelp } from "../../shared/ContextHelp";
import "./categories.css";
interface Category { key: string; label: string; color: string; transactionCount: number; ruleCount: number }
interface IndustryRule { industry: string; categoryKey: string; transactionCount: number }
interface CategoryRule { ruleType: "merchant" | "industry"; label: string }
interface PendingRuleDeletion { categoryKey: string; rule: CategoryRule }
function ruleTypeLabel(rule:CategoryRule){return t(rule.ruleType==="merchant" ? "Händlerregel" : "Branchenregel");}
export function Categories() {
 const [items,setItems]=useState<Category[]>([]);
 const [industries,setIndustries]=useState<IndustryRule[]>([]);
 const [rules,setRules]=useState<Record<string,CategoryRule[]>>({});
 const [rulesLoading,setRulesLoading]=useState(false);
 const [deletingRule,setDeletingRule]=useState<PendingRuleDeletion|null>(null);
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
 const ruleDialog=useRef<HTMLDialogElement>(null);
 const origin=useRef<HTMLElement|null>(null);
 const ruleOrigin=useRef<HTMLButtonElement|null>(null);
 const createButton=useRef<HTMLButtonElement>(null);
 function closeEditor(){setEdit(null);setCreating(false);setLabel("");requestAnimationFrame(()=>origin.current?.focus());}
 function begin(item:Category, action:"edit"|"delete"|"merge", button:HTMLButtonElement){
  const details=button.closest("details");
  origin.current=details?.querySelector("summary") ?? null;
  if(details)details.open=false;
  setError("");setMessage("");
  if(action==="edit"){
   setEdit(item.key);setCreating(false);setLabel(item.label);setColor(item.color);setRulesLoading(true);
   void invoke<CategoryRule[]>("list_category_rules",{categoryKey:item.key})
    .then(result=>setRules(current=>({...current,[item.key]:result})))
    .catch(reason=>setError(String(reason)))
    .finally(()=>setRulesLoading(false));
  }
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
 async function removeRule() {
  if(!deletingRule)return;
  setBusy(true);setError("");setMessage("");
  try {
   const count=await invoke<number>("delete_category_rule",{categoryKey:deletingRule.categoryKey,ruleType:deletingRule.rule.ruleType,label:deletingRule.rule.label});
   const [updatedRules]=await Promise.all([invoke<CategoryRule[]>("list_category_rules",{categoryKey:deletingRule.categoryKey}),load()]);
   setRules(current=>({...current,[deletingRule.categoryKey]:updatedRules}));
   setMessage(`${t("Regel gelöscht.")} ${t("Betroffene Buchungen")}: ${count}. ${t("Automatisch zugeordnete Buchungen wurden auf „Sonstiges“ gesetzt.")}`);
   ruleDialog.current?.close();
  } catch(reason){setError(String(reason));}finally{setBusy(false);}
 }
 const editor=<form className="category-inline-editor" onSubmit={save} aria-label={edit ? t("Kategorie bearbeiten") : t("Neue Kategorie")} onKeyDown={event=>{if(event.key==="Escape"&&!busy){event.preventDefault();closeEditor();}}}>
   <label>{t("Name")}<input autoFocus required maxLength={80} value={label} disabled={busy} onChange={e=>setLabel(e.target.value)} /></label>
   <label>{t("Farbe")}<input type="color" value={color} disabled={busy} onChange={e=>setColor(e.target.value)} /></label>
   <button className="primary-button" disabled={busy}>{busy ? t("Bitte warten …") : t("Speichern")}</button>
   <button type="button" className="secondary-button" disabled={busy} onClick={closeEditor}>{t("Abbrechen")}</button>
   {edit && <section className="category-rule-list" aria-labelledby={`category-rules-${edit}`}>
    <div className="category-rule-heading"><strong id={`category-rules-${edit}`}>{t("Zuordnungsregeln")}</strong><ContextHelp label={t("Zuordnungsregeln erklären")}><span><strong>{t("Branchenregel")}:</strong> {t("Eine Branchenregel verwendet die Branche aus dem importierten Kreditkarten- oder Bankauszug, sofern der Anbieter sie mitliefert. Sie gilt für Buchungen derselben Branche.")}</span><span><strong>{t("Händlerregel")}:</strong> {t("Eine Händlerregel wird in Saldonaut beim Kategorisieren erstellt. Sie erkennt passende Buchungstexte desselben Händlers und gilt für bestehende sowie zukünftige Importe.")}</span></ContextHelp></div>
    {rulesLoading ? <p role="status">{t("Regeln werden geladen …")}</p> : (rules[edit] ?? []).length ? <ul>{rules[edit].map(rule=><li key={`${rule.ruleType}-${rule.label}`}><span>{ruleTypeLabel(rule)}</span><strong>{rule.label}</strong><button type="button" className="category-rule-delete" disabled={busy} aria-label={`${rule.label}: ${t("Regel löschen")}`} onClick={event=>{ruleOrigin.current=event.currentTarget;setDeletingRule({categoryKey:edit,rule});ruleDialog.current?.showModal();}}>{t("Löschen")}</button></li>)}</ul> : <p>{t("Für diese Kategorie bestehen keine Regeln.")}</p>}
   </section>}
   {error && <p className="error-message" role="alert">{t(error)}</p>}
 </form>;
 return <section className="transactions-page">
  <p className="eyebrow">{t("Verwaltung")}</p><h1>{t("Kategorien")}</h1>
  <p className="intro category-page-intro">{t("Passe Namen und Farben an deine Auswertung an. Eigene Kategorien stehen auch bei den Transaktionen zur Verfügung.")}</p>
  {error && !edit && !creating && !removing && <p className="error-message" role="alert">{t(error)}</p>}{message && <p className="import-result" role="status">{t(message)}</p>}
  <div className="category-create-action"><button type="button" ref={createButton} className="primary-button" disabled={busy || !!edit || creating} onClick={()=>{origin.current=createButton.current;setCreating(true);setLabel("");setColor("#527b91");setError("");}}>{t("Neue Kategorie")}</button></div>
  {creating && <div className="dashboard-card category-editor">{editor}</div>}
  <dialog ref={ruleDialog} className="settlement-rule-dialog category-rule-dialog" aria-labelledby="category-rule-dialog-title" onCancel={event=>{if(busy)event.preventDefault();}} onClose={()=>{setDeletingRule(null);setError("");requestAnimationFrame(()=>{if(ruleOrigin.current?.isConnected)ruleOrigin.current.focus();else document.querySelector<HTMLElement>('.category-inline-editor input')?.focus();});}}>
   {deletingRule && <><h2 id="category-rule-dialog-title">{t("Regel löschen")}</h2><p><strong>{ruleTypeLabel(deletingRule.rule)}:</strong> {deletingRule.rule.label}</p><p>{t("Automatisch durch diese Regel zugeordnete Buchungen werden auf „Sonstiges“ gesetzt. Manuelle Einzelentscheidungen bleiben erhalten. Die Regel gilt auch für zukünftige Importe nicht mehr.")}</p>{error && <p className="error-message" role="alert">{t(error)}</p>}<div className="category-dialog-actions"><button type="button" className="secondary-button" disabled={busy} onClick={()=>ruleDialog.current?.close()}>{t("Abbrechen")}</button><button type="button" className="danger-button" disabled={busy} onClick={()=>void removeRule()}>{busy?t("Bitte warten …"):t("Regel löschen")}</button></div></>}
  </dialog>
  <dialog ref={dialog} className="settlement-rule-dialog category-removal-dialog" aria-labelledby="category-removal-title" onCancel={event=>{if(busy)event.preventDefault();}} onClose={()=>{setRemoving(null);setError("");origin.current?.focus();}}>
  {removing && <>
   <h2 id="category-removal-title">{operation==="merge" ? t("Kategorie zusammenführen") : t("Kategorie löschen")}: {categoryName(removing.key, removing.label)}</h2>
   <p>{removing.transactionCount}  {t("Buchungen und")} {removing.ruleCount}  {t("Zuordnungsregeln werden in die Zielkategorie übernommen. Neue Importe, die bisher hier zugeordnet wurden, folgen ebenfalls dieser Auswahl.")}</p>
   <label>{t("Zielkategorie")}<select disabled={busy} value={target} onChange={e=>setTarget(e.target.value)}><option value="">{t("Bitte wählen")}</option>{items.filter(c=>c.key!==removing.key).map(c=><option key={c.key} value={c.key}>{categoryName(c.key, c.label)}</option>)}</select></label>
   {error && <p className="error-message" role="alert">{t(error)}</p>}
   <div className="category-dialog-actions"><button className="secondary-button" disabled={busy} onClick={()=>dialog.current?.close()}>{t("Abbrechen")}</button><button className={operation==="delete" ? "danger-button" : "primary-button"} disabled={busy || !target} onClick={()=>void remove()}>{busy ? t("Bitte warten …") : t("Übernehmen und Kategorie entfernen")}</button></div>
  </>}</dialog>
  <div className="dashboard-card managed-categories">{items.map(item=><div className="managed-category" key={item.key}>
   <span className="category-color" style={{background:item.color}} aria-hidden="true"/><div><strong>{categoryName(item.key,item.label)}</strong><small>{item.transactionCount} {t("Buchungen ·")} {item.ruleCount} {t("Regeln")}</small></div>
   <details className="category-menu" onToggle={event=>{if(event.currentTarget.open)document.querySelectorAll<HTMLDetailsElement>(".category-menu[open]").forEach(other=>{if(other!==event.currentTarget)other.open=false;});}} onKeyDown={event=>{if(event.key==="Escape"){event.currentTarget.open=false;event.currentTarget.querySelector("summary")?.focus();}}} onBlur={event=>{if(!event.currentTarget.contains(event.relatedTarget as Node))event.currentTarget.open=false;}}>
    <summary aria-label={t("Aktionen")+": "+categoryName(item.key,item.label)}>⋯</summary>
    <div className="category-menu-options">
     <a href={`#transactions?category=${encodeURIComponent(item.key)}&period=all&view=details${item.key==="income" ? "&mode=income" : ""}`} onClick={event=>{event.currentTarget.closest("details")?.removeAttribute("open");}}>{t("Buchungen anzeigen")}</a>
     <button disabled={busy || !!edit || creating} onClick={event=>begin(item,"edit",event.currentTarget)}>{t("Bearbeiten")}</button>
     <button disabled={busy || !!edit || creating || items.length<2} onClick={event=>begin(item,"merge",event.currentTarget)}>{t("Zusammenführen …")}</button>
     <button className="category-delete-option" disabled={busy || !!edit || creating || items.length<2} onClick={event=>begin(item,"delete",event.currentTarget)}>{t("Löschen …")}</button>
    </div>
   </details>
   {edit===item.key && editor}
  </div>)}</div>
  {industries.length>0 && <div className="dashboard-card industry-rules">
   <h2>{t("Kartenkäufe automatisch kategorisieren")}</h2>
   <p className="intro">{t("Wähle für jede Branche die passende Ausgabenkategorie – zum Beispiel „Lebensmittel & Haushalt“ für Lebensmittelgeschäfte. Deine Auswahl gilt für bestehende und künftig importierte Kartenkäufe.")}</p>
   <p className="settings-hint">{t("Selbst zugewiesene Kategorien und Regeln für einzelne Händler bleiben erhalten.")}</p>
   <div className="industry-rules-header" aria-hidden="true"><span>{t("Kreditkarten-Kategorie")}</span><span>{t("Saldonaut-Kategorie")}</span></div>
   {industries.map(rule=><label key={rule.industry}><span><strong>{rule.industry}</strong><small>{rule.transactionCount} {t("Buchungen")}</small></span><span className="industry-category-control"><span>{t("Saldonaut-Kategorie")}</span><select aria-label={`${t("Saldonaut-Kategorie")}: ${rule.industry}`} disabled={busy} value={rule.categoryKey} onChange={event=>void assignIndustry(rule.industry,event.target.value)}>{items.map(item=><option key={item.key} value={item.key}>{categoryName(item.key,item.label)}</option>)}</select></span></label>)}
  </div>}
 </section>;
}
