// Gemeinsame Reiter mit Tastaturnavigation und dauerhaft montierten Panels zum Erhalt ungespeicherter Eingaben.
import type { ReactNode } from "react";

export function SectionTabs({ id, label, tabs, value, onChange, className = "transaction-tabs" }: {
  id: string; label: string; tabs: { value: string; label: string }[]; value: string; onChange: (value: string) => void; className?: string;
}) {
  return <div className={className} role="tablist" aria-label={label}>
    {tabs.map((tab, index) => <button key={tab.value} type="button" role="tab" id={`${id}-tab-${tab.value}`}
      aria-controls={`${id}-panel-${tab.value}`} aria-selected={value === tab.value} tabIndex={value === tab.value ? 0 : -1}
      onClick={() => onChange(tab.value)} onKeyDown={event => {
        const next = event.key === "ArrowRight" ? (index + 1) % tabs.length : event.key === "ArrowLeft" ? (index + tabs.length - 1) % tabs.length : event.key === "Home" ? 0 : event.key === "End" ? tabs.length - 1 : null;
        if (next === null) return;
        event.preventDefault(); onChange(tabs[next].value);
        document.getElementById(`${id}-tab-${tabs[next].value}`)?.focus();
      }}>{tab.label}</button>)}
  </div>;
}

export function SectionPanel({ id, value, active, children }: { id: string; value: string; active: string; children: ReactNode }) {
  return <div className="section-tab-panel" role="tabpanel" id={`${id}-panel-${value}`} aria-labelledby={`${id}-tab-${value}`} hidden={active !== value}>{children}</div>;
}
