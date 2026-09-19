// Edits a transaction category explicitly, with scope disclosure and recoverable save errors.
import { useState } from "react";
import { categoryName, t, tr } from "../../i18n";

export function InlineCategoryEditor({ description, initialKey, options, onSave, onClose }: {
  description: string; initialKey: string; options: { key: string; label: string }[];
  onSave: (key: string) => Promise<void>; onClose: () => void;
}) {
  const [key, setKey] = useState(initialKey);
  const [saving, setSaving] = useState(false);
  const [error, setError] = useState("");
  return <form className="transaction-category-editor" aria-label={tr`Kategorie für ${description}`} aria-busy={saving}
    onKeyDown={event => { if(event.key === "Escape") { event.preventDefault(); event.stopPropagation(); if(!saving) onClose(); } }}
    onSubmit={async event => {
      event.preventDefault(); if(saving || key === initialKey) return;
      setSaving(true); setError("");
      try { await onSave(key); onClose(); }
      catch(reason) { setError(typeof reason === "string" ? reason : t("Kategorie konnte nicht geändert werden.")); setSaving(false); }
    }}>
    <label>{t("Kategorie")}<select autoFocus value={key} disabled={saving} onChange={event=>setKey(event.target.value)}>
      {options.map(option=><option key={option.key} value={option.key}>{categoryName(option.key,option.label)}</option>)}
    </select></label>
    <button className="primary-button" type="submit" disabled={saving || key === initialKey || !options.some(option=>option.key===key)}>{saving ? t("Bitte warten …") : t("Speichern")}</button>
    <button className="secondary-button" type="button" disabled={saving} onClick={onClose}>{t("Abbrechen")}</button>
    <p>{t("Kategorieänderungen gelten auch für passende Buchungen desselben Händlers und zukünftige Importe.")}</p>
    {error && <p className="error-message" role="alert">{error}</p>}
  </form>;
}
