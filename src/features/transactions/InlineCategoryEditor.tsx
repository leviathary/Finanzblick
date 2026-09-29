// Edits a transaction category explicitly, with scope disclosure and recoverable save errors.
import { useState } from "react";
import { categoryName, t, tr } from "../../i18n";

export function InlineCategoryEditor({ description, initialKey, options, onSave, onClose }: {
  description: string; initialKey: string; options: { key: string; label: string }[];
  onSave: (key: string, createMerchantRule: boolean) => Promise<void>; onClose: () => void;
}) {
  const [key, setKey] = useState(initialKey);
  const [scope, setScope] = useState<"merchant" | "single">("merchant");
  const createMerchantRule = scope === "merchant";
  const [saving, setSaving] = useState(false);
  const [error, setError] = useState("");
  return <form className="transaction-category-editor" aria-label={tr`Kategorie für ${description}`} aria-busy={saving}
    onKeyDown={event => { if(event.key === "Escape") { event.preventDefault(); event.stopPropagation(); if(!saving) onClose(); } }}
    onSubmit={async event => {
      event.preventDefault(); if(saving || key === initialKey && !createMerchantRule) return;
      setSaving(true); setError("");
      try { await onSave(key, createMerchantRule); onClose(); }
      catch(reason) { setError(typeof reason === "string" ? reason : t("Kategorie konnte nicht geändert werden.")); setSaving(false); }
    }}>
    <label>{t("Kategorie")}<select autoFocus value={key} disabled={saving} onChange={event=>setKey(event.target.value)}>
      {options.map(option=><option key={option.key} value={option.key}>{categoryName(option.key,option.label)}</option>)}
    </select></label>
    <fieldset className="category-scope-options" disabled={saving}>
      <legend>{t("Geltungsbereich")}</legend>
      <label><input type="radio" name={`category-scope-${description}`} value="merchant" checked={scope === "merchant"} onChange={()=>setScope("merchant")}/><span><strong>{t("Händlerregel erstellen")}</strong><small>{t("Passende bestehende Buchungen dieses Händlers ändern und zukünftige Importe automatisch zuordnen.")}</small></span></label>
      <label><input type="radio" name={`category-scope-${description}`} value="single" checked={scope === "single"} onChange={()=>setScope("single")}/><span><strong>{t("Nur diese Buchung kategorisieren")}</strong><small>{t("Nur die ausgewählte Buchung ändern. Für andere Buchungen wird keine Regel erstellt.")}</small></span></label>
    </fieldset>
    <button className="primary-button" type="submit" disabled={saving || key === initialKey && !createMerchantRule || !options.some(option=>option.key===key)}>{saving ? t("Bitte warten …") : t("Speichern")}</button>
    <button className="secondary-button" type="button" disabled={saving} onClick={onClose}>{t("Abbrechen")}</button>
    {error && <p className="error-message" role="alert">{error}</p>}
  </form>;
}
