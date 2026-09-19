// Accessible multi-selection dropdown; an empty selection means all options.
import { useEffect, useRef } from "react";
import { t } from "../i18n";

export function MultiSelect({ label, allLabel, options, value, onChange }: {
  label: string; allLabel: string; options: { value: string; label: string }[];
  value: string[]; onChange: (value: string[]) => void;
}) {
  const root = useRef<HTMLDetailsElement>(null);
  useEffect(() => {
    const close = (event: PointerEvent) => {
      if (root.current && !root.current.contains(event.target as Node)) root.current.open = false;
    };
    document.addEventListener("pointerdown", close);
    return () => document.removeEventListener("pointerdown", close);
  }, []);
  const selected = options.filter(option => value.includes(option.value));
  return <div className="analysis-multiselect"><span>{label}</span>
    <details ref={root} onKeyDown={event => {
      if (event.key === "Escape") { root.current!.open = false; root.current!.querySelector("summary")?.focus(); }
    }}>
      <summary aria-label={label} title={selected.map(option => option.label).join(", ")}>
        {!value.length ? allLabel : selected.length === 1 ? selected[0].label : `${selected.length} ${t("ausgewählt")}`}
      </summary>
      <div className="analysis-multiselect-options" role="group" aria-label={label}>
        <label><input type="checkbox" checked={!value.length} onChange={() => onChange([])}/>{allLabel}</label>
        {options.map(option => <label key={option.value}><input type="checkbox" checked={value.includes(option.value)} onChange={() => onChange(value.includes(option.value) ? value.filter(key => key !== option.value) : [...value, option.value])}/>{option.label}</label>)}
      </div>
    </details>
  </div>;
}
