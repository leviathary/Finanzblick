// Zeigt Vermögensverteilung und Bewertungsverläufe mit auswählbaren Positionen.

import { t, tr, locale } from "../../i18n";
import { useEffect, useRef, useState } from "react";
import { invoke } from "@tauri-apps/api/core";
import { ProviderLogo } from "../accounts/ProviderLogo";
import { sumPositionHistory, type PositionChart } from "./positionHistory";
import { reportPeriod } from "../../domain/reportPeriod";

import type { WealthData } from "./types";
import { WealthChart } from "./WealthChart";
import { BenchmarkPicker } from "./BenchmarkPicker";
import { ChartHelp } from "./ChartHelp";
import type { BenchmarkData } from "./benchmarkModel";
import { monthsBefore } from "../../shared/charts/timelineModel";
import { BreakdownCard } from "./BreakdownCard";
import { accountLabel, money, signedMoney, shortDate, typeLabel } from "./presentation";

export function Assets({
  onAccounts,
  onImport,
}: {
  onAccounts: () => void;
  onImport: () => void;
}) {
  const [data, setData] = useState<WealthData | null>(null);
  const [error, setError] = useState<string | null>(null);
  const [period, setPeriod] = useState<
    "all" | "currentYear" | "1m" | "6m" | "1y" | "3y" | "5y" | "custom"
  >(() => reportPeriod() ? "custom" : "currentYear");
  const [customFrom, setCustomFrom] = useState(() => reportPeriod()?.from ?? "");
  const [chartControls, setChartControls] = useState<HTMLDivElement | null>(null);
  const [benchmark, setBenchmark] = useState<BenchmarkData | null>(null);
  const [customTo, setCustomTo] = useState(() => reportPeriod()?.to ?? "");
  const [selectedAccountIds, setSelectedAccountIds] = useState<number[]>([]);
  const [accountSearch, setAccountSearch] = useState("");
  const [positionData, setPositionData] = useState<{ accountId: number; positions: PositionChart[] } | null>(null);
  const [positionError, setPositionError] = useState("");
  const [positionIds, setPositionIds] = useState<number[] | null>(null);
  const [chartMode, setChartMode] = useState<"value" | "price">("value");
  const manualAccountId = selectedAccountIds.length === 1 && data?.accounts.some(account => account.id === selectedAccountIds[0] && (account.accountType === "manual_asset" || account.accountType === "pillar3a")) ? selectedAccountIds[0] : null;
  useEffect(() => {
    let cancelled = false;
    setPositionIds(null);
    setChartMode("value");
    setPositionData(null);
    setPositionError("");
    const load = () => {
      if (manualAccountId === null) return;
      void invoke<PositionChart[]>("position_chart_data", { accountId: manualAccountId })
        .then(positions => { if (!cancelled) { setPositionData({ accountId: manualAccountId, positions }); setPositionError(""); } })
        .catch(reason => { if (!cancelled) setPositionError(String(reason)); });
    };
    load();
    window.addEventListener("market-data-refreshed", load);
    return () => { cancelled = true; window.removeEventListener("market-data-refreshed", load); };
  }, [manualAccountId]);
  const positions = positionData?.accountId === manualAccountId ? positionData.positions : null;
  const chosenPositions = positions?.filter(position => positionIds === null || positionIds.includes(position.id)) ?? [];
  const singlePosition = chosenPositions.length === 1 ? chosenPositions[0] : null;
  const quoteCurrency = singlePosition?.prices[0]?.currency;
  const canShowPrice = !!quoteCurrency && !!singlePosition?.prices.every(point => point.currency === quoteCurrency);
  const showPrice = chartMode === "price" && canShowPrice;
  const chartCurrency = showPrice ? quoteCurrency! : data?.currency ?? "CHF";
  const chartHistory = positions ? (showPrice ? singlePosition!.prices : sumPositionHistory(chosenPositions)) : data?.history ?? [];
  const accountPickerRef = useRef<HTMLDetailsElement>(null);
  useEffect(() => {
    const closeOutside = (event: PointerEvent) => {
      const picker = accountPickerRef.current;
      if (picker?.open && event.target instanceof Node && !picker.contains(event.target)) {
        picker.open = false;
      }
    };
    const closeOnEscape = (event: KeyboardEvent) => {
      const picker = accountPickerRef.current;
      if (event.key === "Escape" && picker?.open) {
        picker.open = false;
        picker.querySelector("summary")?.focus();
      }
    };
    document.addEventListener("pointerdown", closeOutside, true);
    document.addEventListener("keydown", closeOnEscape);
    return () => {
      document.removeEventListener("pointerdown", closeOutside, true);
      document.removeEventListener("keydown", closeOnEscape);
    };
  }, []);
  useEffect(() => {
    let cancelled = false;
    const load = () =>
      void invoke<WealthData>("wealth_data", {
        accountIds: selectedAccountIds.length ? selectedAccountIds : null,
      })
        .then(result => { if (!cancelled) { setData(result); setError(null); } })
        .catch((reason) =>
          !cancelled && setError(
            typeof reason === "string"
              ? reason
              : t("Vermögensdaten konnten nicht geladen werden."),
          ),
        );
    load();
    window.addEventListener("market-data-refreshed", load);
    return () => { cancelled = true; window.removeEventListener("market-data-refreshed", load); };
  }, [selectedAccountIds]);
  if (error)
    return (
      <section className="assets-page">
        <p className="error-message">{t(error)}</p>
      </section>
    );
  if (!data)
    return (
      <section className="assets-page">
        <p className="intro">{t("Vermögen wird geladen…")}</p>
      </section>
    );
  if (!data.accounts.length)
    return (
      <section className="assets-page empty-overview">
        <p className="eyebrow">{t("Vermögen")}</p>
        <h1>{t("Noch keine Vermögensdaten")}</h1>
        <p className="intro">
          {t(
            "Importiere einen Auszug oder nimm ein Konto in die Vermögensberechnung auf.",
          )}
        </p>
        <div className="empty-actions">
          <button className="primary-button" onClick={onImport}>
            {t("Auszug importieren")}
          </button>
          <button className="secondary-button" onClick={onAccounts}>
            {t("Konten verwalten")}
          </button>
        </div>
      </section>
    );

  const sortedAccounts = [...data.accounts].sort((left, right) => {
    const amountDifference =
      Math.abs(right.balanceMinor ?? 0) - Math.abs(left.balanceMinor ?? 0);
    return amountDifference || left.name.localeCompare(right.name, locale());
  });
  const fullFrom = chartHistory[0]?.date ?? "";
  const fullTo = chartHistory[chartHistory.length - 1]?.date ?? "";
  const filteredHistory = (() => {
    if (!chartHistory.length) return [];
    let from = fullFrom,
      to = fullTo;
    if (period === "custom") {
      from = customFrom || fullFrom;
      to = customTo || fullTo;
    } else if (period === "currentYear") {
      from = `${new Date().getFullYear()}-01-01`;
    } else if (period === "1m" || period === "6m") {
      from = monthsBefore(fullTo, period === "1m" ? 1 : 6);
    } else if (period !== "all") {
      const years =
        period === "1y" ? 1 : period === "3y" ? 3 : 5;
      const date = new Date(`${fullTo}T12:00:00`);
      date.setFullYear(date.getFullYear() - years);
      from = date.toISOString().slice(0, 10);
    }
    return chartHistory.filter(
      (point) => point.date >= from && point.date <= to,
    );
  })();
  const periodFirst = filteredHistory[0]?.totalMinor ?? null;
  const periodLast =
    filteredHistory[filteredHistory.length - 1]?.totalMinor ?? null;
  const periodChange =
    periodFirst !== null && periodLast !== null
      ? periodLast - periodFirst
      : null;
  const changePercent =
    periodFirst && periodChange !== null
      ? (periodChange / Math.abs(periodFirst)) * 100
      : null;
  return (
    <section className="assets-page">
      <div className="overview-heading">
        <div>
          <p className="eyebrow">{t("Vermögen")}</p>
          <h1>{t("Vermögensentwicklung")}</h1>
          <p className="intro">
            {t(
              "Entwicklung und Zusammensetzung auf Basis der importierten Salden.",
            )}
          </p>
        </div>
        <button className="primary-button" onClick={onAccounts}>
          {t("Konten verwalten")}
        </button>
      </div>
      <div className="asset-kpis">
        <article>
          <span>{t(showPrice ? "Letzter Kurs" : positions ? "Positionswert" : "Aktuelles Vermögen")}</span>
          <strong>{money(positions ? chartHistory[chartHistory.length - 1]?.totalMinor ?? null : data.currentTotalMinor, chartCurrency)}</strong>
        </article>
        <article>
          <span>{t("Veränderung im gewählten Zeitraum")}</span>
          <strong className={(periodChange ?? 0) < 0 ? "negative" : "positive"}>
            {signedMoney(periodChange, chartCurrency)}
          </strong>
          <small>
            {changePercent === null
              ? t("Noch kein Vergleichswert")
              : `${changePercent >= 0 ? "+" : ""}${changePercent.toFixed(1)} %`}
          </small>
        </article>
      </div>
      <article className="dashboard-card wealth-chart-card">
        <div className="period-controls wealth-period-controls">
          <div className="wealth-timeframe-bar" role="group" aria-label={t("Chart-Steuerung")}>
            <div className="wealth-instrument-controls">
              <details className="account-picker" ref={accountPickerRef}>
                <summary aria-label={t("Konto / Depot")}>
                  <svg width="16" height="16" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="1.7" aria-hidden="true"><circle cx="10.5" cy="10.5" r="6.5"/><path d="m16 16 5 5"/></svg>
                  <span>
                  {selectedAccountIds.length === 0
                    ? t("Gesamtvermögen")
                    : selectedAccountIds.length === 1
                      ? accountLabel(data.accounts.find((account) => account.id === selectedAccountIds[0]))
                      : `${selectedAccountIds.length} ${t("ausgewählt")}`}
                  </span>
                </summary>
                <div className="account-picker-menu">
                  <input type="search" aria-label={t("Konten durchsuchen")} placeholder={t("Konten durchsuchen")} value={accountSearch} onChange={event => setAccountSearch(event.target.value)} />
                  {accountSearch.trim() && !data.accounts.some(account => accountLabel(account).toLocaleLowerCase().includes(accountSearch.trim().toLocaleLowerCase())) && <p className="benchmark-notice" role="status">{t("Keine passenden Konten gefunden.")}</p>}
                  <label>
                    <input
                      type="checkbox"
                      checked={selectedAccountIds.length === 0}
                      onChange={() => setSelectedAccountIds([])}
                    />
                    {t("Gesamtvermögen")}
                  </label>
                  {data.accounts.filter(account => accountLabel(account).toLocaleLowerCase().includes(accountSearch.trim().toLocaleLowerCase())).map((account) => (
                    <label key={account.id}>
                      <input
                        type="checkbox"
                        checked={selectedAccountIds.includes(account.id)}
                        onChange={() =>
                          setSelectedAccountIds((selected) =>
                            selected.includes(account.id)
                              ? selected.filter((id) => id !== account.id)
                              : [...selected, account.id],
                          )
                        }
                      />
                      {accountLabel(account)}
                    </label>
                  ))}
                </div>
              </details>
              <BenchmarkPicker from={fullFrom} onChange={setBenchmark} />
            </div>
          <div className="wealth-timeframes" role="group" aria-label={t("Zeitraum")}>
            {[
              ["1m", "1M", t("1 Monat")],
              ["6m", "6M", t("6 Monate")],
              ["currentYear", "YTD", t("Aktuelles Jahr")],
              ["1y", t("1J"), t("1 Jahr")],
              ["3y", t("3J"), t("3 Jahre")],
              ["5y", t("5J"), t("5 Jahre")],
              ["all", "Max", t("Gesamt")],
              ["custom", "…", t("Eigener Zeitraum")],
            ].map(([value, label, description]) => (
              <button
                type="button" title={description} aria-label={description} aria-pressed={period === value}
                className={period === value ? "active" : ""}
                key={value}
                onClick={() => {
                  setPeriod(value as typeof period);
                  if (value === "custom") {
                    setCustomFrom(customFrom || fullFrom);
                    setCustomTo(customTo || fullTo);
                  }
                }}
              >
                {label}
              </button>
            ))}
          </div>
          <div className="wealth-chart-tools"><div className="wealth-chart-tool-icons" ref={setChartControls} /><ChartHelp /></div>
          </div>
          {period === "custom" && (
            <div className="custom-period">
              <label>
                {t("Von")}
                <input
                  type="date"
                  lang={locale()}
                  min={fullFrom}
                  max={customTo || fullTo}
                  value={customFrom}
                  onChange={(event) => setCustomFrom(event.target.value)}
                />
              </label>
              <label>
                {t("Bis")}
                <input
                  type="date"
                  lang={locale()}
                  min={customFrom || fullFrom}
                  max={fullTo}
                  value={customTo}
                  onChange={(event) => setCustomTo(event.target.value)}
                />
              </label>
            </div>
          )}
        </div>
        <div className={manualAccountId !== null ? "position-chart-layout" : undefined}>
        <div className="position-chart-main">
        {positions && <div className="position-chart-modes">
          <button type="button" className="secondary-button" aria-pressed={!showPrice} onClick={() => setChartMode("value")}>{t("Positionswert")}</button>
          <button type="button" className="secondary-button" aria-pressed={showPrice} disabled={!canShowPrice} onClick={() => setChartMode("price")}>{t("Kurs")}</button>
          <small>{t(showPrice ? "Preis pro Anteil in Originalwährung" : "Wert der ausgewählten Positionen in CHF")}</small>
        </div>}
        {positions && !chosenPositions.length ? <p className="chart-empty">{t("Bitte mindestens eine Position auswählen.")}</p> : <WealthChart
          history={filteredHistory}
          currency={chartCurrency}
          controlsContainer={chartControls}
          benchmark={benchmark}
        />}
        </div>
        {manualAccountId !== null && <aside className="position-chart-list" aria-label={t("Positionen")}>
          <h3>{t("Positionen")}</h3>
          {positionError ? <p role="alert" className="error-message">{t(positionError)}</p> : !positions ? <p role="status">{t("Positionen werden geladen …")}</p> : <>
            <div className="position-list-actions">
              <button type="button" onClick={() => { setPositionIds(null); setChartMode("value"); }}>{t("Alle auswählen")}</button>
              <button type="button" onClick={() => { setPositionIds([]); setChartMode("value"); }}>{t("Auswahl aufheben")}</button>
            </div>
            {!positions.length && <p>{t("Keine Positionen vorhanden.")}</p>}
            {positions.map(position => <label key={position.id} className="position-chart-row">
              <input type="checkbox" checked={chosenPositions.some(selected => selected.id === position.id)} onChange={() => {
                setPositionIds(current => {
                  const selected = current ?? positions.map(item => item.id);
                  return selected.includes(position.id) ? selected.filter(id => id !== position.id) : [...selected, position.id];
                });
                setChartMode("value");
              }} />
              <span><strong>{position.label}</strong><small>{position.holdingEndDate ? `${t("Enddatum")}: ${shortDate(position.holdingEndDate)}` : t("Positionswert")}</small></span>
              <b>{money(position.history[position.history.length - 1]?.totalMinor ?? null, "CHF")}</b>
            </label>)}
          </>}
        </aside>}
        </div>
      </article>
      <div className="asset-breakdowns">
        <BreakdownCard
          translateTypes
          variant="category"
          title={t("Nach Kategorie")}
          values={data.byType}
          total={data.currentTotalMinor}
          currency={data.currency}
        />
        <BreakdownCard
          variant="provider"
          title={t("Nach Anbieter")}
          values={data.byProvider}
          total={data.currentTotalMinor}
          currency={data.currency}
          accounts={data.accounts}
        />
      </div>
      <article className="dashboard-card asset-accounts">
        <div className="card-heading">
          <div>
            <p className="eyebrow">{t("Details")}</p>
            <h2>{t("Vermögenspositionen")}</h2>
          </div>
          <span>
            {data.accounts.length} {t("Positionen")}
          </span>
        </div>
        {sortedAccounts.map((account) => (
          <div className="asset-account" key={account.id}>
            <ProviderLogo
              name={account.provider}
              providerKey={account.providerKey}
              customLogo={account.logoDataUrl}
            />
            <div>
              <strong>{account.name}</strong>
              <small>
                {account.provider} · {typeLabel(account.accountType)}
              </small>
            </div>
            <span>
              {account.balanceDate
                ? tr`Stand ${shortDate(account.balanceDate)}`
                : t("Ohne Stichtag")}
            </span>
            <b
              className={
                account.balanceMinor === null || account.balanceMinor === 0
                  ? "asset-account-amount zero"
                  : "asset-account-amount"
              }
            >
              {money(account.balanceMinor ?? 0, account.balanceCurrency)}
            </b>
          </div>
        ))}
      </article>
    </section>
  );
}
