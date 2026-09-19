// Zeigt zeilenbezogene Aktionen in einem viewport-begrenzten Popover mit Tastatur- und Fokussteuerung.
import { useEffect, useRef, useState } from "react";
import { createPortal } from "react-dom";

export function ActionMenu({ label, disabled, actions }: {
  label: string;
  disabled?: boolean;
  actions: Array<{ label: string; onClick: () => void; separated?: boolean; danger?: boolean }>;
}) {
  const trigger = useRef<HTMLButtonElement>(null);
  const panel = useRef<HTMLDivElement>(null);
  const [position, setPosition] = useState<{ top: number; left: number } | null>(null);
  function close() { setPosition(null); trigger.current?.focus(); }
  useEffect(() => {
    if (!position) return;
    panel.current?.querySelector<HTMLButtonElement>("button")?.focus();
    const outside = (event: PointerEvent) => {
      if (!panel.current?.contains(event.target as Node) && !trigger.current?.contains(event.target as Node)) setPosition(null);
    };
    const scroll = () => setPosition(null);
    document.addEventListener("pointerdown", outside);
    window.addEventListener("resize", scroll);
    window.addEventListener("scroll", scroll, true);
    return () => {
      document.removeEventListener("pointerdown", outside);
      window.removeEventListener("resize", scroll);
      window.removeEventListener("scroll", scroll, true);
    };
  }, [position]);
  return <>
    <button ref={trigger} type="button" className="transaction-more row-action-trigger" disabled={disabled}
      aria-label={label} aria-expanded={!!position} aria-haspopup="dialog"
      onClick={() => {
        if (position) { close(); return; }
        const rect = trigger.current!.getBoundingClientRect();
        setPosition({ left: Math.max(8, Math.min(rect.right - 300, window.innerWidth - 308)), top: Math.max(8, Math.min(rect.bottom + 4, window.innerHeight - 230)) });
      }}>⋯</button>
    {position && createPortal(<div ref={panel} className="transaction-action-menu row-action-menu" role="dialog" aria-label={label} style={position}
      onBlur={event => { if (!event.currentTarget.contains(event.relatedTarget)) setPosition(null); }}
      onKeyDown={event => {
        if (event.key === "Escape") { event.preventDefault(); close(); }
        const buttons = Array.from(panel.current?.querySelectorAll<HTMLButtonElement>("button") ?? []);
        const index = buttons.indexOf(document.activeElement as HTMLButtonElement);
        const next = event.key === "ArrowDown" ? (index + 1) % buttons.length : event.key === "ArrowUp" ? (index + buttons.length - 1) % buttons.length : event.key === "Home" ? 0 : event.key === "End" ? buttons.length - 1 : -1;
        if (next >= 0) { event.preventDefault(); buttons[next]?.focus(); }
      }}>
      {actions.map(action => <button key={action.label} type="button" disabled={disabled}
        className={`${action.separated ? "row-action-separated" : ""}${action.danger ? " row-action-danger" : ""}`}
        onClick={() => { close(); action.onClick(); }}>{action.label}</button>)}
    </div>, document.body)}
  </>;
}
