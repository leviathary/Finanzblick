// Öffnet das isolierte, erst bei Bedarf geladene Osterei innerhalb der entsperrten App.
import { useEffect, useRef, useState, type ComponentType } from "react";
import { createPortal } from "react-dom";
import { t } from "../../i18n";
import "./moonLander.css";

export function MoonLanderLauncher() {
  const dialog = useRef<HTMLDialogElement>(null);
  const origin = useRef<HTMLElement | null>(null);
  const [open, setOpen] = useState(false);
  const [Game, setGame] = useState<ComponentType | null>(null);
  const [failed, setFailed] = useState(false);
  useEffect(() => {
    const shortcut = (event: KeyboardEvent) => {
      const target = event.target;
      if (event.defaultPrevented || event.repeat || event.isComposing || event.altKey ||
          !(event.ctrlKey || event.metaKey) || !event.shiftKey || event.code !== "KeyL" ||
          (target instanceof HTMLElement && (target.isContentEditable || target.closest("input, textarea, select, [role='textbox']"))) ||
          document.querySelector("dialog[open], [aria-modal='true']")) return;
      event.preventDefault();
      origin.current = document.activeElement instanceof HTMLElement ? document.activeElement : null;
      setOpen(true);
    };
    window.addEventListener("keydown", shortcut);
    return () => window.removeEventListener("keydown", shortcut);
  }, []);
  useEffect(() => {
    if (!open) return;
    let active = true;
    const element = dialog.current!;
    element.showModal();
    setFailed(false);
    void import("./MoonLander").then(module => { if (active) setGame(() => module.default); })
      .catch(() => { if (active) setFailed(true); });
    return () => { active = false; element.close(); origin.current?.focus(); };
  }, [open]);
  return createPortal(<dialog ref={dialog} className="settlement-rule-dialog moon-lander-dialog" aria-labelledby="moon-lander-title"
    onCancel={event => { event.preventDefault(); setOpen(false); }} onClose={() => setOpen(false)}>
    {open && <><header className="moon-lander-header"><h2 id="moon-lander-title">Saldonaut · Moon Lander</h2>
      <button type="button" className="secondary-button" onClick={() => setOpen(false)}>{t("Schließen")} · Esc</button></header>
      {Game ? <Game /> : <p role="status">{failed ? t("Das Spiel konnte nicht geladen werden. Bitte schließen und erneut öffnen.") : t("Spiel wird geladen …")}</p>}
    </>}
  </dialog>, document.body);
}
