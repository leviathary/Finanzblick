// Zeichnet das Tresor-Raumschiff als SVG und steuert eine lokale, pausierbare Spielrunde.
import { useEffect, useRef, useState } from "react";
import { t } from "../../i18n";
import { FEET, PAD, MAX_VX, MAX_VY, newFlight, stepFlight, type Flight, type Controls } from "./model";

const emptyControls = (): Controls => ({ thrust: false, left: false, right: false });
const stars = Array.from({ length: 65 }, (_, i) => ({ x: (i * 137 + 29) % 900, y: (i * 79 + 17) % 330, r: i % 4 === 0 ? 1.5 : .8 }));
const coins = Array.from({ length: 95 }, (_, i) => ({ x: (i * 83 + 18) % 900, y: 397 + (i * 17) % 43, silver: i % 3 === 0 }));

export default function MoonLander() {
  const flight = useRef(newFlight());
  const controls = useRef(emptyControls());
  const stage = useRef<HTMLDivElement>(null);
  const [view, setView] = useState(flight.current);
  const update = (next: Flight) => { flight.current = next; setView(next); };
  const pause = () => {
    controls.current = emptyControls();
    if (flight.current.status === "flying") update({ ...flight.current, status: "paused" });
  };
  useEffect(() => {
    stage.current?.focus();
    const release = (event: KeyboardEvent) => {
      if (event.code === "ArrowUp") controls.current.thrust = false;
      if (event.code === "ArrowLeft") controls.current.left = false;
      if (event.code === "ArrowRight") controls.current.right = false;
    };
    const visibility = () => { if (document.hidden) pause(); };
    window.addEventListener("keyup", release);
    window.addEventListener("blur", pause);
    document.addEventListener("visibilitychange", visibility);
    let frame = 0, last = 0, accumulator = 0;
    const animate = (now: number) => {
      accumulator += last ? Math.min((now - last) / 1000, .05) : 0;
      last = now;
      while (accumulator >= 1 / 120) {
        flight.current = stepFlight(flight.current, controls.current, 1 / 120);
        accumulator -= 1 / 120;
      }
      setView(flight.current);
      frame = requestAnimationFrame(animate);
    };
    frame = requestAnimationFrame(animate);
    return () => {
      cancelAnimationFrame(frame);
      window.removeEventListener("keyup", release);
      window.removeEventListener("blur", pause);
      document.removeEventListener("visibilitychange", visibility);
    };
  }, []);
  const start = () => {
    controls.current = emptyControls();
    update({ ...(flight.current.status === "paused" ? flight.current : newFlight()), status: "flying" });
    stage.current?.focus();
  };
  const message = view.status === "landed" ? t("Dein Vermögen ist sicher gelandet!") : view.status === "crashed" ? t("Etwas zu offensiv investiert. Versuch es noch einmal!") : view.status === "paused" ? t("Flug pausiert.") : view.status === "ready" ? t("Bereit zur Mondlandung?") : t("Lande den Tresor sanft auf der Plattform.");
  return <div className="moon-lander-game">
    <div className="moon-lander-hud">
      <span>{t("Treibstoff")}: {Math.ceil(view.fuel)} %</span>
      <span>{t("Höhe")}: {Math.max(0, Math.round(PAD.top - view.y - FEET))} m</span>
      <span>{t("Sinktempo")}: {Math.round(view.vy)} / {MAX_VY} m/s</span>
      <span>{t("Seitentempo")}: {Math.round(Math.abs(view.vx))} / {MAX_VX} m/s</span>
    </div>
    <div ref={stage} className="moon-lander-stage" tabIndex={0} role="group" aria-label={t("Spielfeld")} aria-describedby="moon-lander-controls"
      onBlur={pause} onKeyDown={event => {
        if (event.ctrlKey || event.metaKey || event.altKey) return;
        if (["ArrowUp", "ArrowLeft", "ArrowRight"].includes(event.code)) {
          event.preventDefault(); event.stopPropagation();
          if (flight.current.status !== "flying") return;
          if (event.code === "ArrowUp") controls.current.thrust = true;
          if (event.code === "ArrowLeft") controls.current.left = true;
          if (event.code === "ArrowRight") controls.current.right = true;
        }
      }}>
      <svg viewBox="0 0 900 440" aria-hidden="true">
        <rect width="900" height="440" fill="#0b1220" />
        {stars.map((star, i) => <circle key={i} {...star} cx={star.x} cy={star.y} fill="#afbdd0" opacity={i % 2 ? .45 : .8} />)}
        <circle cx="140" cy="95" r="46" fill="#1c293c" /><path d="M126 52 Q164 93 123 137 A46 46 0 0 0 126 52" fill="#394960" />
        <path d="M0 384 H900 V440 H0Z" fill="#665337" />
        {coins.map((coin, i) => <g key={i} transform={`translate(${coin.x} ${coin.y}) rotate(${i % 2 ? -12 : 8})`}>
          <ellipse cy="4" rx="22" ry="8" fill={coin.silver ? "#64748b" : "#967440"} stroke="#0b1220" />
          <ellipse rx="22" ry="8" fill={coin.silver ? "#afbdd0" : "#c6a56a"} stroke="#0b1220" />
          <ellipse rx="16" ry="5" fill="none" stroke={coin.silver ? "#64748b" : "#967440"} />
        </g>)}
        <path d={`M${PAD.left} 360 H${PAD.right} V384 H${PAD.left}Z`} fill="#394960" stroke="#afbdd0" />
        <path d={`M${PAD.left + 8} 360 H${PAD.right - 8}`} stroke="#77b3a0" strokeWidth="5" />
        <path d="M660 369 H680 M670 364 V377" stroke="#afbdd0" strokeWidth="3" />
        <g transform={`translate(${view.x} ${view.y})`} opacity={view.status === "crashed" ? .6 : 1}>
          {view.status === "flying" && controls.current.thrust && view.fuel > 0 && <path d="M-9 19 L0 53 L9 19Z" fill="#e4b569" />}
          <path d="M-16 14 L-23 28 H-29 M16 14 L23 28 H29" fill="none" stroke="#afbdd0" strokeWidth="4" />
          <rect x="-23" y="-24" width="46" height="44" rx="5" fill="#64748b" stroke="#dbeafe" strokeWidth="2" />
          <rect x="-19" y="-20" width="38" height="35" rx="3" fill="#394960" />
          <circle cy="-2" r="13" fill={view.status === "landed" ? "#c6a56a" : "#afbdd0"} stroke="#0b1220" strokeWidth="2" />
          <circle cy="-2" r="5" fill="#394960" />
          <path d="M0-13 V9 M-11-2 H11" stroke="#394960" strokeWidth="2" />
          <path d="M-17-28 V-33" stroke="#afbdd0" strokeWidth="2" /><circle cx="-17" cy="-34" r="3" fill="#c6a56a" />
          {view.status === "crashed" && <path d="M-14-19 L4-4 L-5 3 L12 16" fill="none" stroke="#0b1220" strokeWidth="3" />}
        </g>
      </svg>
    </div>
    <p id="moon-lander-controls" className="moon-lander-instructions">{t("↑ Schub · ← → Steuern · Esc Zurück. Zum Landen beide Tempo-Grenzen einhalten. Tab pausiert den Flug.")}</p>
    <footer className="moon-lander-footer"><p role="status">{message}</p>
      <button type="button" className="primary-button" onPointerDown={event => event.preventDefault()} onClick={view.status === "flying" ? pause : start}>
        {view.status === "flying" ? t("Pause") : view.status === "paused" ? t("Weiterfliegen") : view.status === "ready" ? t("Starten") : t("Neuer Versuch")}
      </button>
    </footer>
  </div>;
}
