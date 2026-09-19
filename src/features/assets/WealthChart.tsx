// Bindet den lokalen interaktiven Chart an die Vermögensprojektion an, ohne Auswertungsfilter zu ändern.
import { InteractiveTimelineChart } from "../../shared/charts/InteractiveTimelineChart";
import type { HistoryPoint } from "./types";
import { compareBenchmark, type BenchmarkData } from "./benchmarkModel";
import { t } from "../../i18n";

export function WealthChart({ history, currency, controlsContainer, benchmark }: { history: HistoryPoint[]; currency: string; controlsContainer: HTMLElement | null; benchmark?: BenchmarkData | null }) {
  const compared = benchmark ? compareBenchmark(history, benchmark) : null;
  return <>
    {benchmark && !compared && <p role="status" className="benchmark-notice">{t("Kein gemeinsamer Vergleichszeitraum mit positivem Startwert verfügbar.")}</p>}
    {compared && benchmark && <div className="benchmark-legend" title={`${t("Basis 100")}: ${compared.from}`}>
      <span><i aria-hidden="true" />{t("Auswahl")} · {t("Basis 100")}</span>
      <span><i className="benchmark-swatch" aria-hidden="true" />{benchmark.name} · {benchmark.currency} · {t("Basis 100")}</span>
    </div>}
    <InteractiveTimelineChart history={compared?.main ?? history} currency={currency} controlsContainer={controlsContainer} indexed={!!compared} comparison={compared?.comparison} />
  </>;
}
