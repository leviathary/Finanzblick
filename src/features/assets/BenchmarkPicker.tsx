// Lädt einen ausdrücklich gewählten Index über den bestehenden Kursadapter; Fehler bleiben lokal am Chart.
import { useEffect, useRef, useState } from "react";
import { invoke } from "@tauri-apps/api/core";
import { t } from "../../i18n";
import type { BenchmarkData } from "./benchmarkModel";

export function BenchmarkPicker({ from, onChange }: { from: string; onChange: (value: BenchmarkData | null) => void }) {
  const [selected, setSelected] = useState("");
  const [loading, setLoading] = useState(false);
  const [failed, setFailed] = useState(false);
  const [retry, setRetry] = useState(0);
  const picker = useRef<HTMLDetailsElement>(null);
  useEffect(() => {
    const close = (event: PointerEvent) => { if (picker.current && event.target instanceof Node && !picker.current.contains(event.target)) picker.current.open = false; };
    document.addEventListener("pointerdown", close);
    return () => document.removeEventListener("pointerdown", close);
  }, []);
  useEffect(() => {
    let cancelled = false;
    onChange(null); setFailed(false); setLoading(false);
    if (!selected || !from) return;
    setLoading(true);
    void invoke<BenchmarkData>("benchmark_data", { benchmark: selected, from }).then(result => {
      if (!cancelled) onChange(result);
    }).catch(() => { if (!cancelled) setFailed(true); }).finally(() => { if (!cancelled) setLoading(false); });
    return () => { cancelled = true; };
  }, [selected, from, retry, onChange]);
  return <details className="benchmark-picker" ref={picker} onKeyDown={event => {
    if (event.key === "Escape" && picker.current) { picker.current.open = false; picker.current.querySelector("summary")?.focus(); }
  }}>
    <summary aria-label={t("Vergleichen")}><span aria-hidden="true">{loading ? "…" : "+"}</span> {selected ? selected === "smi" ? "SMI" : "S&P 500" : t("Vergleichen")}{failed && <span aria-label={t("Vergleich nicht verfügbar")}> ⚠</span>}</summary>
    <div className="benchmark-menu">
      <label htmlFor="benchmark-index">{t("Vergleichsindex")}</label>
      <select id="benchmark-index" value={selected} onChange={event => setSelected(event.target.value)}>
        <option value="">{t("Kein Vergleich")}</option><option value="smi">SMI · CHF</option><option value="sp500">S&P 500 · USD</option>
      </select>
      {loading && <p role="status">{t("Vergleich wird geladen …")}</p>}
      {failed && <div role="alert"><p>{t("Vergleich nicht verfügbar")}</p><button type="button" className="secondary-button" onClick={() => setRetry(n => n + 1)}>{t("Erneut versuchen")}</button></div>}
    </div>
  </details>;
}
