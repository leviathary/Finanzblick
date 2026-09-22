// Verwendet den gemeinsamen Zeitreihenchart mit lokalen Zeitraumfiltern ohne Renditeberechnung.
import { useState } from "react";
import { t } from "../../i18n";
import { InteractiveTimelineChart } from "../../shared/charts/InteractiveTimelineChart";
import { monthsBefore, type DailyValue } from "../../shared/charts/timelineModel";
import { localToday, periodHistory } from "./model";

export function DetailChart({ history, currency, title }: { history: DailyValue[]; currency: string; title: string }) {
  const [period, setPeriod] = useState("ytd");
  const [controls, setControls] = useState<HTMLDivElement | null>(null);
  const today = localToday();
  const from = period === "all" ? "" : period === "ytd" ? `${today.slice(0,4)}-01-01` : monthsBefore(today, Number(period));
  const visible = periodHistory(history, from, today);
  return <article className="dashboard-card detail-chart">
    <div className="card-heading"><h2>{title}</h2><span>{currency}</span></div>
    <div className="detail-chart-toolbar">
      <div className="detail-periods" aria-label={t("Zeitraum")}>
        {[["1", "1M"], ["6", "6M"], ["ytd", "YTD"], ["12", t("1J")], ["all", t("Gesamt")]].map(([value, label]) =>
          <button key={value} type="button" aria-pressed={period === value} onClick={() => setPeriod(value)}>{label}</button>)}
      </div><div ref={setControls} className="detail-chart-tools wealth-chart-tools" />
    </div>
    {visible.length ? <InteractiveTimelineChart history={visible} currency={currency} controlsContainer={controls} ariaLabel={title} /> : <p className="intro">{t("Keine Bewertungsdaten in diesem Zeitraum.")}</p>}
  </article>;
}
