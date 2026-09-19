// Zeigt kompakte Buchungsaktionen und optional den Einstieg zur Kategorieänderung außerhalb der scrollenden Tabelle.
import { useEffect, useRef, useState } from "react";
import { createPortal } from "react-dom";
import { t, tr } from "../../i18n";

export type TransferType = "NONE" | "CREDIT_CARD_SETTLEMENT" | "INTERNAL_TRANSFER";

export function TransactionActions({ neutral, description, disabled, onChange, onEditCategory }: {
  neutral: boolean; description: string; disabled: boolean; onChange: (type: TransferType) => Promise<void>;
  onEditCategory?: () => void;
}) {
  const trigger = useRef<HTMLButtonElement>(null);
  const menu = useRef<HTMLDivElement>(null);
  const [position, setPosition] = useState<{ top: number; left: number } | null>(null);
  function close() { setPosition(null); trigger.current?.focus(); }
  useEffect(() => {
    if (!position) return;
    menu.current?.querySelector<HTMLButtonElement>("button")?.focus();
    const outside = (event: PointerEvent) => {
      if (!menu.current?.contains(event.target as Node) && !trigger.current?.contains(event.target as Node)) setPosition(null);
    };
    const key = (event: KeyboardEvent) => { if (event.key === "Escape") { event.preventDefault(); close(); } };
    const scroll = () => setPosition(null);
    document.addEventListener("pointerdown", outside);
    document.addEventListener("keydown", key);
    window.addEventListener("resize", scroll);
    window.addEventListener("scroll", scroll, true);
    return () => {
      document.removeEventListener("pointerdown", outside); document.removeEventListener("keydown", key);
      window.removeEventListener("resize", scroll); window.removeEventListener("scroll", scroll, true);
    };
  }, [position]);
  function choose(kind: TransferType) { close(); void onChange(kind); }
  return <>
    <button ref={trigger} type="button" className="transaction-more" disabled={disabled}
      aria-label={tr`Aktionen für ${description}`} aria-expanded={!!position} aria-haspopup="dialog"
      onClick={() => {
        if (position) { close(); return; }
        const rect = trigger.current!.getBoundingClientRect();
        setPosition({ left: Math.max(8, Math.min(rect.right - 300, window.innerWidth - 308)), top: Math.max(8, Math.min(rect.bottom + 4, window.innerHeight - (onEditCategory ? 280 : 230))) });
      }}>···</button>
    {position && createPortal(<div className="transaction-action-menu" role="dialog" aria-label={t("Buchungsaktionen")} ref={menu}
      style={position} onBlur={event => { if (!event.currentTarget.contains(event.relatedTarget)) setPosition(null); }}>
      {onEditCategory && <button type="button" onClick={() => { close(); onEditCategory(); }}>{t("Kategorie ändern …")}</button>}
      {neutral ? <button type="button" onClick={() => choose("NONE")}>{t("Als reguläre Buchung wiederherstellen")}</button> : <>
        <button type="button" onClick={() => choose("CREDIT_CARD_SETTLEMENT")}>{t("Als Kartenausgleich markieren")}</button>
        <button type="button" onClick={() => choose("INTERNAL_TRANSFER")}>{t("Als Übertrag zwischen eigenen Konten markieren")}</button>
      </>}
    </div>, document.body)}
  </>;
}
