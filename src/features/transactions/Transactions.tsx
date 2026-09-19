// Zeigt und analysiert Buchungen mit Zeitfiltern, Kategorien und Einnahmen-/Ausgabenauswertungen.

import { t, tr, locale, categoryName } from "../../i18n";
import { useCallback, useEffect, useMemo, useRef, useState } from "react";
import { invoke } from "@tauri-apps/api/core";

import { TimelineChart, type TimelinePoint } from "../../shared/charts/TimelineChart";
import { selectAmountRange } from "./amountSelection";
import { TransactionActions, type TransferType } from "./TransactionActions";
import { SettlementRuleDialog } from "./SettlementRuleDialog";
import { reportPeriod } from "../../domain/reportPeriod";

interface Category { key: string; label: string; color: string; amountMinor: number; transactionCount: number }
interface Month { month: string; amountMinor: number }
interface Transaction { id: number; bookingDate: string; description: string; industry: string | null; amountMinor: number; currency: string; categoryKey: string; categoryLabel: string; categoryColor: string; categorySource: "manual" | "merchant" | "industry" | "description"; provider: string; providerKey: string; accountName: string; excludedFromTotals: boolean; isCardSettlement: boolean; isCard: boolean; isManuallyOverridden: boolean; expenseMinor: number; incomeMinor: number }
interface Provider { provider: string; providerKey: string }
interface Account { id: number; name: string; providerKey: string; currency: string; isActive: boolean }
interface Analysis { incomeTransactions: Transaction[]; totalIncomeMinor: number; incomeCount: number; totalSpendMinor: number; transactionCount: number; firstDate: string | null; lastDate: string | null; categories: Category[]; months: Month[]; history: TimelinePoint[]; transactions: Transaction[]; providers: Provider[] }


export function Transactions() {
  const [ruleTransaction, setRuleTransaction] = useState<Transaction | null>(null);
  const [savingSettlement, setSavingSettlement] = useState(false);
  const [detailMode, setDetailMode] = useState<"income" | "expense">("expense");
  const [analysisView, setAnalysisView] = useState<"category" | "month">("category");
  const [sort, setSort] = useState<{ key: "bookingDate" | "description" | "accountName" | "amountMinor"; descending: boolean }>({ key: "bookingDate", descending: true });
  const [search, setSearch] = useState("");
  const [minimum, setMinimum] = useState("");
  const [rowLimit, setRowLimit] = useState(200);
  const [data, setData] = useState<Analysis | null>(null);
  const dragSelection = useRef<{ key: string; base: string[]; selecting: boolean } | null>(null);
  const suppressCategoryClick = useRef(false);
  const categoryAnchor = useRef<string | null>(null);
  const [category, setCategory] = useState<string[]>([]);
  const [provider, setProvider] = useState("");
  const [categoryOptions, setCategoryOptions] = useState<{ key: string; label: string }[]>([]);
  useEffect(() => { invoke<{ key: string; label: string }[]>("list_categories").then(setCategoryOptions).catch(() => setError(t("Kategorien konnten nicht geladen werden."))); }, []);
  const [accounts, setAccounts] = useState<Account[]>([]);
  const [account, setAccount] = useState("");
  const [from, setFrom] = useState(() => reportPeriod()?.from ?? `${new Date().getFullYear()}-01-01`);
  const [to, setTo] = useState(() => reportPeriod()?.to ?? "");
  const [selectedPeriod, setSelectedPeriod] = useState<"all" | "currentYear" | "1y" | "2y" | "3y" | "5y" | "custom">(() => reportPeriod() ? "custom" : "currentYear");
  const [comparisonYear, setComparisonYear] = useState(() => String(new Date().getFullYear()));
  const [monthlyDrilldown, setMonthlyDrilldown] = useState<{ categoryKey: string | null; year: string; month: number } | null>(null);
  const [error, setError] = useState<string | null>(null);
  const [loading, setLoading] = useState(true);
  const [categoryMessage, setCategoryMessage] = useState<string | null>(null);
  const loadRevision = useRef(0);
  const load = useCallback(async () => {
    const revision = ++loadRevision.current;
    setLoading(true);
    try { const result = await invoke<Analysis>("transaction_analysis", { from: from || null, to: to || null, providerKey: provider || null, accountId: account ? Number(account) : null }); if (revision === loadRevision.current) { setData(result); setError(null); } }
    catch (reason) { if (revision === loadRevision.current) setError(typeof reason === "string" ? reason : t("Transaktionen konnten nicht geladen werden.")); }
    finally { if (revision === loadRevision.current) setLoading(false); }
  }, [from, provider, to, account]);
  useEffect(() => { void load(); }, [load]);
  useEffect(() => { invoke<Account[]>("list_accounts").then(setAccounts).catch(() => setError(t("Konten konnten nicht geladen werden."))); }, []);
  const categorizedTransactions = useMemo(() => {
    const transactions = detailMode === "income"
      ? data?.incomeTransactions.filter(item => item.incomeMinor !== 0)
      : data?.transactions.filter(item => item.expenseMinor !== 0);
    return transactions?.filter(item => item.currency === "CHF") ?? [];
  }, [data, detailMode]);
  const visibleTransactions = useMemo(
    () => categorizedTransactions.filter(item => !category.length || category.includes(item.categoryKey)),
    [categorizedTransactions, category],
  );
  const drilldownTransactions = useMemo(() => {
    const neutral = (detailMode === "income" ? data?.incomeTransactions : data?.transactions)?.filter(item => item.excludedFromTotals && (!category.length || category.includes(item.categoryKey))) ?? [];
    const listed = [...visibleTransactions, ...neutral];
    if (!monthlyDrilldown) return listed;
    const yearMonth = `${monthlyDrilldown.year}-${String(monthlyDrilldown.month + 1).padStart(2, "0")}`;
    return listed.filter(item => item.bookingDate.startsWith(yearMonth)
      && (!monthlyDrilldown.categoryKey || item.categoryKey === monthlyDrilldown.categoryKey));
  }, [monthlyDrilldown, visibleTransactions, data, detailMode, category]);
  const comparisonYears = useMemo(
    () => [...new Set(categorizedTransactions.map(item => item.bookingDate.slice(0, 4)))].sort((a, b) => b.localeCompare(a)),
    [categorizedTransactions],
  );
  useEffect(() => {
    if (comparisonYears.length && !comparisonYears.includes(comparisonYear)) setComparisonYear(comparisonYears[0]);
  }, [comparisonYear, comparisonYears]);
  useEffect(() => { setMonthlyDrilldown(null); }, [comparisonYear]);
  const contribution = (item: Transaction) => detailMode === "income" ? item.incomeMinor : item.expenseMinor;
  const comparisonMonths = useMemo(() => {
    const rows = new Map<string, { key: string; label: string; color: string; months: number[]; total: number }>();
    for (const item of visibleTransactions) {
      if (item.bookingDate.slice(0, 4) !== comparisonYear) continue;
      const row = rows.get(item.categoryKey) ?? {
        key: item.categoryKey,
        label: item.categoryLabel,
        color: item.categoryColor,
        months: Array(12).fill(0),
        total: 0,
      };
      const amount = contribution(item);
      const month = Number(item.bookingDate.slice(5, 7)) - 1;
      row.months[month] += amount;
      row.total += amount;
      rows.set(item.categoryKey, row);
    }
    return [...rows.values()].sort((a, b) => b.total - a.total || a.label.localeCompare(b.label, locale()));
  }, [comparisonYear, visibleTransactions]);
  const comparisonMonthRange = useMemo(() => {
    const year = Number(comparisonYear);
    const current = new Date();
    const start = from.startsWith(`${comparisonYear}-`) ? Number(from.slice(5, 7)) - 1 : 0;
    const end = to.startsWith(`${comparisonYear}-`)
      ? Number(to.slice(5, 7)) - 1
      : year === current.getFullYear() ? current.getMonth() : 11;
    return { start: Math.max(0, start), end: Math.min(11, Math.max(start, end)) };
  }, [comparisonYear, from, to]);
  const comparisonMonthCount = comparisonMonthRange.end - comparisonMonthRange.start + 1;
  const comparisonTotals = useMemo(
    () => Array.from({ length: 12 }, (_, month) => comparisonMonths.reduce((sum, row) => sum + row.months[month], 0)),
    [comparisonMonths],
  );
  const monthlyDrilldownLabel = useMemo(() => {
    if (!monthlyDrilldown) return null;
    const selected = categorizedTransactions.find(item => item.categoryKey === monthlyDrilldown.categoryKey);
    const categoryLabel = selected
      ? categoryName(selected.categoryKey, selected.categoryLabel)
      : t("Alle Kategorien");
    return `${categoryLabel} · ${monthNameLong(monthlyDrilldown.month)} ${monthlyDrilldown.year}`;
  }, [categorizedTransactions, monthlyDrilldown]);
  const drilldownTotal = drilldownTransactions.reduce((sum, item) => sum + contribution(item), 0);
  const filteredTransactions = useMemo(() => {
    const query = search.trim().toLocaleLowerCase(locale());
    const threshold = Math.max(0, Number(minimum) || 0) * 100;
    return drilldownTransactions.filter(item => Math.abs(item.amountMinor) >= threshold &&
      (!query || `${item.description} ${item.industry ?? ""} ${item.accountName} ${item.provider} ${item.excludedFromTotals || item.isCardSettlement ? t("Rechnungsausgleich") : ""}`.toLocaleLowerCase(locale()).includes(query)))
      .sort((a, b) => {
        const comparison = sort.key === "amountMinor" ? Math.abs(a.amountMinor) - Math.abs(b.amountMinor)
          : a[sort.key].localeCompare(b[sort.key], locale(), { numeric: true, sensitivity: "base" });
        return (sort.descending ? -comparison : comparison) || b.bookingDate.localeCompare(a.bookingDate) || b.id - a.id;
      });
  }, [drilldownTransactions, search, minimum, sort]);
  useEffect(() => { setRowLimit(200); }, [category, search, minimum, sort, from, to, provider, monthlyDrilldown]);
  const [selectedAmounts, setSelectedAmounts] = useState<number[]>([]);
  const amountDrag = useRef<{ start: number; base: number[]; selecting: boolean } | null>(null);
  const amountRows = filteredTransactions.slice(0, rowLimit);
  const selectedAmountRows = amountRows.filter(item => selectedAmounts.includes(item.id));
  const selectedAmountTotal = selectedAmountRows.reduce((sum, item) => sum + contribution(item), 0);
  useEffect(() => {
    setSelectedAmounts([]); amountDrag.current = null;
  }, [from, to, provider, account, detailMode, category, search, minimum, monthlyDrilldown]);
  useEffect(() => {
    const ordered = filteredTransactions.slice(0, rowLimit).map(item => item.id);
    const move = (event: MouseEvent) => {
      const drag = amountDrag.current;
      if (!drag) return;
      if (!(event.buttons & 1)) { amountDrag.current = null; return; }
      const row = document.elementFromPoint(event.clientX, event.clientY)?.closest<HTMLElement>("[data-amount-row]");
      if (!row) return;
      event.preventDefault();
      setSelectedAmounts(selectAmountRange(drag.base, ordered, drag.start, Number(row.dataset.amountRow), drag.selecting));
    };
    const stop = () => { amountDrag.current = null; };
    window.addEventListener("mousemove", move);
    window.addEventListener("mouseup", stop);
    window.addEventListener("blur", stop);
    return () => { stop(); window.removeEventListener("mousemove", move); window.removeEventListener("mouseup", stop); window.removeEventListener("blur", stop); };
  }, [filteredTransactions, rowLimit]);
  const filteredTotal = filteredTransactions.reduce((sum, item) => sum + contribution(item), 0);
  const hasDetailFilter = search.trim() !== "" || Number(minimum) > 0;
  function sortBy(key: typeof sort.key) {
    setSort(current => ({ key, descending: current.key === key ? !current.descending : key === "amountMinor" || key === "bookingDate" }));
  }
  function sortHeader(key: typeof sort.key, label: string) {
    return <button className="expense-sort" aria-pressed={sort.key === key} onClick={() => sortBy(key)}
      aria-label={tr`${label}: ${(sort.key === key ? !sort.descending : key === "amountMinor" || key === "bookingDate") ? t("absteigend") : t("aufsteigend")} sortieren`}>
      {label} <span aria-hidden="true">{sort.key === key ? sort.descending ? "↓" : "↑" : "↕"}</span>
    </button>;
  }
  const displayedMonths = useMemo(() => {
    if (!category.length && detailMode === "expense") return data?.months ?? [];
    const months = new Map<string, number>();
    visibleTransactions.forEach(item => months.set(item.bookingDate.slice(0, 7), (months.get(item.bookingDate.slice(0, 7)) ?? 0) + contribution(item)));
    return Array.from(months, ([month, amountMinor]) => ({ month, amountMinor })).sort((left, right) => left.month.localeCompare(right.month));
  }, [category, data, visibleTransactions, detailMode]);
  const breakdown = useMemo(() => {
    if (detailMode === "expense") return data?.categories ?? [];
    const groups = new Map<string, Category>();
    for (const item of (data?.incomeTransactions ?? []).filter(item => item.incomeMinor !== 0)) {
      const group = groups.get(item.categoryKey) ?? { key: item.categoryKey, label: item.categoryLabel, color: item.categoryColor, amountMinor: 0, transactionCount: 0 };
      group.amountMinor += item.amountMinor; group.transactionCount += 1; groups.set(item.categoryKey, group);
    }
    return [...groups.values()].sort((a,b) => b.amountMinor - a.amountMinor || a.label.localeCompare(b.label));
  }, [data, detailMode]);
  const breakdownTotal = detailMode === "income"
    ? data?.totalIncomeMinor ?? 0
    : breakdown.reduce((sum, item) => sum + item.amountMinor, 0);
  const selectedCategories = breakdown.filter(item => category.includes(item.key));
  useEffect(() => {
    if (loading || !data) return;
    const available = new Set(breakdown.map(item => item.key));
    setCategory(current => current.every(key => available.has(key)) ? current : current.filter(key => available.has(key)));
  }, [breakdown, loading, data]);
  const selectedCategory = category.length === 1 ? selectedCategories[0] : undefined;
  const selectionLabel = category.length > 1 ? tr`${category.length} Kategorien ausgewählt` : selectedCategory ? categoryName(selectedCategory.key, selectedCategory.label) : undefined;
  const selectedTotal = category.length ? selectedCategories.reduce((sum, item) => sum + item.amountMinor, 0) : breakdownTotal;
  const averageMonthly = displayedMonths.length ? selectedTotal / displayedMonths.length : 0;
  const fullFrom = data?.history[0]?.date ?? "";
  const fullTo = data?.history[data.history.length - 1]?.date ?? "";
  const timelineHistory = useMemo(
    () => data?.history.filter(item => (!from || item.date >= from) && (!to || item.date <= to)) ?? [],
    [data, from, to],
  );

  useEffect(() => { categoryAnchor.current = null; }, [detailMode, provider, account, from, to]);
  useEffect(() => { setMonthlyDrilldown(null); }, [category, detailMode, provider, account, from, to]);
  useEffect(() => {
    function move(event: MouseEvent) {
      const drag = dragSelection.current;
      if (!drag) return;
      if (!(event.buttons & 1)) { dragSelection.current = null; return; }
      const element = document.elementFromPoint(event.clientX, event.clientY)?.closest<HTMLElement>("[data-category-key]");
      const key = element?.dataset.categoryKey;
      if (!key || key === drag.key && !suppressCategoryClick.current) return;
      const start = breakdown.findIndex(item => item.key === drag.key);
      const end = breakdown.findIndex(item => item.key === key);
      if (start < 0 || end < 0) return;
      suppressCategoryClick.current = true;
      categoryAnchor.current = drag.key;
      const range = breakdown.slice(Math.min(start,end),Math.max(start,end)+1).map(item => item.key);
      setCategory(drag.selecting ? [...new Set([...drag.base, ...range])] : drag.base.filter(key => !range.includes(key)));
      setSearch(""); setMinimum("");
    }
    function stop() { dragSelection.current = null; }
    window.addEventListener("mousemove", move);
    window.addEventListener("mouseup", stop);
    window.addEventListener("blur", stop);
    return () => { stop(); window.removeEventListener("mousemove", move); window.removeEventListener("mouseup", stop); window.removeEventListener("blur", stop); };
  }, [breakdown]);
  function selectCategory(key: string, shift: boolean) {
    const anchorIndex = breakdown.findIndex(item => item.key === categoryAnchor.current);
    const targetIndex = breakdown.findIndex(item => item.key === key);
    if (shift && anchorIndex >= 0 && targetIndex >= 0) {
      const range = breakdown.slice(Math.min(anchorIndex, targetIndex), Math.max(anchorIndex, targetIndex) + 1).map(item => item.key);
      setCategory(current => [...new Set([...current, ...range])]);
    } else {
      categoryAnchor.current = key;
      setCategory(current => current.includes(key) ? current.filter(item => item !== key) : [...current, key]);
    }
    setSearch(""); setMinimum("");
  }
  function showDetails(mode: "income" | "expense") {
    setDetailMode(mode); setCategory([]); setMonthlyDrilldown(null); setSearch(""); setMinimum(""); setRowLimit(200);
    requestAnimationFrame(() => document.getElementById("transaction-details")?.scrollIntoView({ behavior: "smooth", block: "start" }));
  }
  function showMonthlyTransactions(categoryKey: string | null, month: number) {
    setMonthlyDrilldown({ categoryKey, year: comparisonYear, month });
    setSearch(""); setMinimum(""); setRowLimit(200);
    requestAnimationFrame(() => document.getElementById("transaction-details")?.scrollIntoView({ behavior: "smooth", block: "start" }));
  }
  function choosePeriod(period: "all" | "currentYear" | "1y" | "2y" | "3y" | "5y" | "custom") {
    setSelectedPeriod(period);
    if (period === "all") { setFrom(""); setTo(""); return; }
    if (period === "custom") {
      setFrom(current => current || fullFrom);
      setTo(current => current || fullTo);
      return;
    }
    if (!fullTo) return;
    if (period === "currentYear") {
      setFrom(`${new Date().getFullYear()}-01-01`);
      setTo(fullTo);
      return;
    }
    const years = period === "1y" ? 1 : period === "2y" ? 2 : period === "3y" ? 3 : 5;
    const end = new Date(`${fullTo}T12:00:00`);
    const start = new Date(end);
    start.setFullYear(start.getFullYear() - years);
    setFrom(start.toISOString().slice(0, 10));
    setTo(fullTo);
  }

  async function changeSettlement(item: Transaction, transferType: TransferType) {
    if (transferType === "CREDIT_CARD_SETTLEMENT") { setRuleTransaction(item); return; }
    setSavingSettlement(true);
    try {
      await invoke("set_transaction_transfers", { transactionIds: [item.id], transferType });
      await load();
    } catch (reason) { setError(typeof reason === "string" ? reason : t("Markierung konnte nicht gespeichert werden.")); }
    finally { setSavingSettlement(false); }
  }

  async function changeCategory(transactionId: number, categoryKey: string) {
    try { const count = await invoke<number>("set_transaction_category", { transactionId, categoryKey }); setCategoryMessage(tr`Kategorie für ${count} passende Buchungen übernommen. Die Zuordnung gilt auch für zukünftige Importe.`); await load(); }
    catch (reason) { setError(typeof reason === "string" ? reason : t("Kategorie konnte nicht geändert werden.")); }
  }

  if (!data && loading) return <section className="transactions-page transaction-analysis-page"><p className="intro">{t("Transaktionen werden ausgewertet…")}</p></section>;
  return <section className="transactions-page transaction-analysis-page">
    {ruleTransaction && <SettlementRuleDialog transaction={ruleTransaction} onClose={() => setRuleTransaction(null)} onSaved={load} />}
    <div className="overview-heading"><div><p className="eyebrow">{t("Transaktionen")}</p><h1>{t("Deine Einnahmen und Ausgaben")}</h1><p className="intro">{t("Klicke auf eine Kategorie, um die einzelnen Buchungen zu sehen.")}</p></div></div>
    {error && <p className="error-message">{t(error)}</p>}{categoryMessage && <p className="import-result" role="status">{categoryMessage}</p>}
    {data && <>
      {data.transactions.some(row => row.isCard && row.amountMinor > 0 && !row.excludedFromTotals && row.expenseMinor === 0) && <aside className="card-credit-notice">
        <p role="status">{t("Ungeklärte Kartengutschriften sind noch nicht in den Ausgaben berücksichtigt.")}</p>
        <div className="card-credit-notice-actions">
          <a className="primary-button" href="#transactions/cards/setup">{t("Karte einrichten")} <span aria-hidden="true">→</span></a>
          <a className="card-credit-review-link" href="#transactions/cards">{t("Kartentransaktionen prüfen")}</a>
        </div>
      </aside>}
      {[...data.transactions, ...data.incomeTransactions].some(item => item.isCardSettlement) && <p className="import-result" role="status">{t("Rechnungsausgleiche sind neutralisiert. Bitte die zugehörigen Kartenkäufe separat importieren; ihre Vollständigkeit wird nicht geprüft.")}</p>}
      <article className="dashboard-card wealth-chart-card transaction-timeline">
        <div className="card-heading">
          <div><p className="eyebrow">{t("Zeitverlauf")}</p><h2>{t("Saldoverlauf")}</h2><small className="full-period">{t("Gesamte Datenbasis:")} {fullFrom ? `${shortDate(fullFrom)} – ${shortDate(fullTo)}` : "–"}</small></div>
          <div className="wealth-chart-controls">
            <span>{timelineHistory.length > 1 ? `${shortDate(timelineHistory[0].date)} – ${shortDate(timelineHistory[timelineHistory.length - 1].date)}` : t("Kein Vergleichszeitraum")}</span>
            <div className="transaction-timeline-filters">
              <label>{t("Bank")}<select aria-label={t("Bank")} value={provider} onChange={event => { setProvider(event.target.value); setAccount(""); setCategory([]); }}><option value="">{t("Alle Anbieter")}</option>{data.providers.map(item => <option value={item.providerKey} key={item.providerKey}>{item.provider}</option>)}</select></label>
              <label>{t("Konto")}<select aria-label={t("Konto")} value={account} onChange={event => { setAccount(event.target.value); setCategory([]); }}><option value="">{t("Alle Konten")}</option>{accounts.filter(item => item.isActive && (!provider || item.providerKey === provider)).map(item => <option key={item.id} value={item.id}>{item.name} ({item.currency})</option>)}</select></label>
            </div>
          </div>
        </div>
        <div className="period-controls">
          <div className="period-presets" role="group" aria-label={t("Zeitraum auswählen")}>
            {([['all', t('Gesamt')], ['currentYear', t('Aktuelles Jahr')], ['1y', t('1 Jahr')], ['2y', t('2 Jahre')], ['3y', t('3 Jahre')], ['5y', t('5 Jahre')], ['custom', t('Eigener Zeitraum')]] as const).map(([value, label]) => <button className={selectedPeriod === value ? "active" : ""} key={value} onClick={() => choosePeriod(value)}>{label}</button>)}
          </div>
          {selectedPeriod === "custom" && <div className="custom-period">
            <label>{t("Von")}<input type="date" lang={locale()} min={fullFrom} max={to || fullTo} value={from} onChange={event => { setFrom(event.target.value); setSelectedPeriod("custom"); }} /></label>
            <label>{t("Bis")}<input type="date" lang={locale()} min={from || fullFrom} max={fullTo} value={to} onChange={event => { setTo(event.target.value); setSelectedPeriod("custom"); }} /></label>
          </div>}
        </div>
        <TimelineChart history={timelineHistory} currency="CHF" ariaLabel={t("Verlauf der Kontosalden")} emptyLabel={t("Keine Salden für diesen Zeitverlauf vorhanden.")} onSelectRange={(rangeFrom, rangeTo) => { setFrom(rangeFrom); setTo(rangeTo); setSelectedPeriod("custom"); }} />
      </article>
      <div className="transaction-kpis cashflow-summary">
        <article><button className="cashflow-link" onClick={() => showDetails("income")} aria-pressed={detailMode === "income"}><span>{t("Einnahmen im Zeitraum")}</span><strong className="income-value">{money(data.totalIncomeMinor)}</strong><small>{data.incomeCount}  {t("Gutschriften anzeigen →")}</small></button></article>
        <article><button className="cashflow-link" onClick={() => showDetails("expense")} aria-pressed={detailMode === "expense"}><span>{t("Ausgaben im Zeitraum")}</span><strong>{money(data.totalSpendMinor)}</strong><small>{data.transactionCount}  {t("Belastungen anzeigen →")}</small></button></article>
        <article><span>{t("Differenz")}</span><strong className={data.totalIncomeMinor >= data.totalSpendMinor ? "income-value" : "deficit-value"}>{money(data.totalIncomeMinor - data.totalSpendMinor)}</strong><small>{t("Einnahmen minus Ausgaben")}</small></article>
      </div>
      <div className="transaction-kpis"><article><span>{selectionLabel ? selectionLabel : detailMode === "income" ? t("Einnahmen gesamt") : t("Ausgaben gesamt")}</span><strong>{money(selectedTotal)}</strong><small>{visibleTransactions.length}  {t("Buchungen im gewählten Zeitraum")}</small></article><article><span>{t("Durchschnitt pro Monat")}</span><strong>{money(averageMonthly)}</strong><small>{displayedMonths.length}  {t("Kalendermonate mit Buchungen")}</small></article><article><span>{t("Zeitraum")}</span><strong>{data.firstDate && data.lastDate ? `${shortDate(data.firstDate)} – ${shortDate(data.lastDate)}` : "–"}</strong><small>{provider ? data.providers.find(item => item.providerKey === provider)?.provider : t("Alle Anbieter")}</small></article></div>
      <div className="transaction-analysis-grid categories-full-width">
        <article className={`dashboard-card category-analysis ${analysisView === "month" ? "monthly-comparison" : ""}`}>
          <div className="card-heading">
            <div>
              <p className="eyebrow">{t("Aufteilung")}</p>
              <div className="analysis-view-tabs" role="tablist" aria-label={t("Darstellung auswählen")}>
                <button role="tab" aria-selected={analysisView === "category"} onClick={() => { setAnalysisView("category"); setMonthlyDrilldown(null); }}>{detailMode === "income" ? t("Einnahmen nach Kategorie") : t("Ausgaben nach Kategorie")}</button>
                <button role="tab" aria-selected={analysisView === "month"} onClick={() => setAnalysisView("month")}>{detailMode === "income" ? t("Einnahmen nach Monat") : t("Ausgaben nach Monat")}</button>
              </div>
            </div>
            {category.length > 0 && <button className="secondary-button" onClick={() => { setCategory([]); categoryAnchor.current = null; }}>{t("Auswahl aufheben")}</button>}
          </div>
          <div className="category-controls">
            <div className="quick-periods category-mode" role="group" aria-label={t("Aufteilung auswählen")}>{(["expense", "income"] as const).map(mode => <button key={mode} aria-pressed={detailMode === mode} onClick={() => { setDetailMode(mode); setCategory([]); setSearch(""); setMinimum(""); setRowLimit(200); }}>{mode === "income" ? t("Einnahmen") : t("Ausgaben")}</button>)}</div>
          </div>
          {analysisView === "category" ? <>
            {!breakdown.length && <p className="intro">{t("Keine")} {detailMode === "income" ? t("Einnahmen") : t("Ausgaben")}  {t("für diese Auswahl.")}</p>}
            <div className="category-selection-summary" role="status"><span>{category.length ? selectionLabel : t("Alle Kategorien")} · {visibleTransactions.length}  {t("Buchungen")}</span><strong>{money(selectedTotal)}</strong></div>
            <p className="intro">{t("Klicken, mit gedrückter linker Maustaste ziehen oder mit Shift-Klick einen Bereich auswählen.")}</p>
            <div className="category-drilldown category-multiselect">{breakdown.map(item => <button aria-pressed={category.includes(item.key)} className={category.includes(item.key) ? "selected" : ""} key={item.key} data-category-key={item.key} onMouseDown={event => {
              suppressCategoryClick.current = false;
              if (event.button === 0 && !event.shiftKey) dragSelection.current = { key: item.key, base: category, selecting: !category.includes(item.key) };
            }} onClick={event => { if (suppressCategoryClick.current && event.detail > 0) { suppressCategoryClick.current = false; return; } selectCategory(item.key, event.shiftKey); }}><span className="category-check" aria-hidden="true">{category.includes(item.key) ? "☑" : "☐"}</span><span className="category-color" style={{ background: item.color }}/><div><strong>{categoryName(item.key, item.label)}</strong><small>{item.transactionCount}  {t("Buchungen")}</small></div><b>{money(item.amountMinor)}</b><span className="category-share">{breakdownTotal ? `${(item.amountMinor / breakdownTotal * 100).toFixed(1)} %` : "–"}</span></button>)}</div>
          </> : <>
            <div className="monthly-comparison-toolbar">
              <p className="intro">{t("Kategorien im gewählten Jahr vergleichen.")}</p>
              <div className="monthly-comparison-options">
                <span>{t("Alle Beträge in CHF")}</span>
                <label className="comparison-year-select">{t("Jahr")}
                  <select value={comparisonYear} onChange={event => { setComparisonYear(event.target.value); setMonthlyDrilldown(null); }} disabled={!comparisonYears.length}>
                    {comparisonYears.length
                      ? comparisonYears.map(year => <option key={year} value={year}>{year}</option>)
                      : <option value={comparisonYear}>{comparisonYear}</option>}
                  </select>
                </label>
              </div>
            </div>
            {comparisonMonths.length ? <div className="monthly-comparison-scroll">
              <table>
                <thead><tr>
                  <th scope="col">{t("Kategorie")}</th>
                  {Array.from({ length: 12 }, (_, month) => <th scope="col" className={month < comparisonMonthRange.start || month > comparisonMonthRange.end ? "outside-range" : ""} key={month}>{monthName(month)}</th>)}
                  <th scope="col">{t("Ø / Monat")}</th>
                </tr></thead>
                <tbody>
                  {comparisonMonths.map(row => {
                    const average = row.total / comparisonMonthCount;
                    return <tr key={row.key}>
                      <th scope="row"><span className="category-color" style={{ background: row.color }}/><span>{categoryName(row.key, row.label)}</span></th>
                      {row.months.map((amount, month) => {
                        const selected = monthlyDrilldown?.categoryKey === row.key && monthlyDrilldown.year === comparisonYear && monthlyDrilldown.month === month;
                        const className = `${month < comparisonMonthRange.start || month > comparisonMonthRange.end ? "outside-range " : ""}${amount > 0 && average > 0 && amount > average * 1.35 ? "high-spend " : ""}${selected ? "selected-month-cell" : ""}`.trim();
                        return <td className={className} key={month}>{amount
                          ? <button className="monthly-value-button" aria-pressed={selected} onClick={() => showMonthlyTransactions(row.key, month)} aria-label={`${categoryName(row.key, row.label)} · ${monthNameLong(month)} ${comparisonYear}: ${money(amount)}`}>{amountNumber(amount)}</button>
                          : "–"}</td>;
                      })}
                      <td className="row-average">{amountNumber(average)}</td>
                    </tr>;
                  })}
                </tbody>
                <tfoot><tr>
                  <th scope="row">{t("Gesamt")}</th>
                  {comparisonTotals.map((amount, month) => {
                    const selected = monthlyDrilldown?.categoryKey === null && monthlyDrilldown.year === comparisonYear && monthlyDrilldown.month === month;
                    return <td className={`${month < comparisonMonthRange.start || month > comparisonMonthRange.end ? "outside-range " : ""}${selected ? "selected-month-cell" : ""}`.trim()} key={month}>{amount
                      ? <button className="monthly-value-button" aria-pressed={selected} onClick={() => showMonthlyTransactions(null, month)} aria-label={`${t("Gesamt")} · ${monthNameLong(month)} ${comparisonYear}: ${money(amount)}`}>{amountNumber(amount)}</button>
                      : "–"}</td>;
                  })}
                  <td>{amountNumber(comparisonTotals.reduce((sum, amount) => sum + amount, 0) / comparisonMonthCount)}</td>
                </tr></tfoot>
              </table>
            </div> : <p className="intro monthly-comparison-empty">{t("Keine Buchungen für dieses Jahr vorhanden.")}</p>}
          </>}
        </article>
      </div>
      <article id="transaction-details" className="dashboard-card drilldown-table">
        <div className="card-heading"><div><p className="eyebrow">Drilldown</p><h2>{monthlyDrilldownLabel ?? (selectionLabel ? selectionLabel : detailMode === "income" ? t("Alle Einnahmen") : t("Alle Ausgaben"))} <span className="drilldown-total">{money(monthlyDrilldown ? drilldownTotal : selectedTotal)}</span></h2></div><div className="drilldown-heading-actions"><span>{drilldownTransactions.length}  {t("Buchungen")}</span>{monthlyDrilldown && <button className="secondary-button" onClick={() => setMonthlyDrilldown(null)}>{t("Monatsauswahl aufheben")}</button>}</div></div>
        <p className="intro">{t("Kategorieänderungen gelten auch für passende Buchungen desselben Händlers und zukünftige Importe.")}</p>
        <div className="drilldown-filters">
          <label>{t("Buchungen durchsuchen")}<input type="search" placeholder={t("Beschreibung, Konto oder Bank")} value={search} onChange={event => setSearch(event.target.value)} /></label>
          <label>{t("Mindestbetrag (CHF)")}<input type="number" min="0" step="0.01" placeholder="0.00" value={minimum} onChange={event => setMinimum(event.target.value)} /></label>
          <button className="secondary-button" onClick={() => setSort({ key: "amountMinor", descending: true })}>{t("Grösste Beträge zuerst")}</button>
          {hasDetailFilter && <button className="text-button" onClick={() => { setSearch(""); setMinimum(""); }}>{t("Filter zurücksetzen")}</button>}
        </div>
        <p className="drilldown-result" role="status">{hasDetailFilter ? tr`${filteredTransactions.length} von ${drilldownTransactions.length} Buchungen · Gefilterte Summe: ${money(filteredTotal)}` : t("Spaltenüberschrift anklicken, um die Sortierung zu ändern.")}</p>
        <div className="expense-table"><div className="expense-row header"><span>{sortHeader("bookingDate", t("Datum"))}</span><span>{sortHeader("description", t("Beschreibung"))}</span><span>{t("Kategorie")}</span><span>{sortHeader("accountName", t("Konto"))}</span><span>{sortHeader("amountMinor", t("Betrag"))}</span><span aria-label={t("Aktionen")} /></div>
          {filteredTransactions.slice(0, rowLimit).map(item => <div className={`expense-row ${item.excludedFromTotals ? "card-detail-row" : ""}`} data-amount-row={item.id} key={item.id}><span>{shortDate(item.bookingDate)}</span><div><strong title={item.description}>{item.description}</strong><small>{item.provider}{item.industry ? ` · ${item.industry}` : ""}{item.excludedFromTotals && <> · <span className="transfer-badge">{t(item.isCardSettlement ? "Kartenausgleich" : "Umbuchung")}</span></>}</small></div><div><select aria-label={tr`Kategorie für ${item.description}`} value={item.categoryKey} style={{ borderLeftColor: item.categoryColor }} onChange={event => void changeCategory(item.id, event.target.value)}>{categoryOptions.map(({key, label}) => <option value={key} key={key}>{categoryName(key, label)}</option>)}</select><small>{item.excludedFromTotals ? t("Nicht in Auswertungen enthalten") : item.categorySource === "manual" ? t("Manuell gewählt") : item.categorySource === "merchant" ? t("Anhand Händlerregel") : item.categorySource === "industry" ? t("Anhand Branche") : t("Anhand Buchungstext")}</small></div><span>{item.accountName}</span><button type="button" className={`amount-select ${item.amountMinor > 0 ? "income-amount" : ""}`} aria-pressed={selectedAmounts.includes(item.id)} aria-label={tr`Betrag auswählen: ${money(Math.abs(item.amountMinor))} · ${item.description}`} onMouseDown={event => {
            if (event.button !== 0) return;
            event.preventDefault(); event.currentTarget.focus();
            const selecting = !selectedAmounts.includes(item.id);
            amountDrag.current = { start: item.id, base: selectedAmounts, selecting };
            setSelectedAmounts(selectAmountRange(selectedAmounts, amountRows.map(row => row.id), item.id, item.id, selecting));
          }} onClick={event => { if (event.detail === 0) setSelectedAmounts(current => current.includes(item.id) ? current.filter(id => id !== item.id) : [...current, item.id]); }}>{money(item.excludedFromTotals ? item.amountMinor : contribution(item))}</button><TransactionActions neutral={item.excludedFromTotals} description={item.description} disabled={savingSettlement} onChange={kind => changeSettlement(item, kind)} /></div>)}
          {!filteredTransactions.length && <p className="intro">{t("Keine Buchungen für diese Auswahl gefunden.")}</p>}
          {filteredTransactions.length > rowLimit && <button className="secondary-button" onClick={() => setRowLimit(limit => limit + 200)}>{t("Weitere Buchungen anzeigen (")}{rowLimit}  {t("von")} {filteredTransactions.length})</button>}
        </div>
        <div className="amount-selection-summary">
          <div role="status" aria-live="polite"><span>{tr`${selectedAmountRows.length} Beträge ausgewählt`}</span><strong>{money(selectedAmountTotal)}</strong></div>
          {selectedAmountRows.length > 0 ? <button className="secondary-button" onClick={() => setSelectedAmounts([])}>{t("Auswahl aufheben")}</button> : <span>{t("Beträge anklicken oder mit gedrückter Maustaste darüberziehen.")}</span>}
        </div>
      </article>
    </>}
  </section>;
}

function money(value: number) { return new Intl.NumberFormat(locale(), { style: "currency", currency: "CHF" }).format(value / 100); }
function amountNumber(value: number) { return new Intl.NumberFormat(locale(), { minimumFractionDigits: 2, maximumFractionDigits: 2 }).format(value / 100); }
function shortDate(value: string) { const [year, month, day] = value.slice(0, 10).split("-"); return `${day}.${month}.${year}`; }
function monthName(month: number) { return new Intl.DateTimeFormat(locale(), { month: "short" }).format(new Date(2020, month, 1)); }
function monthNameLong(month: number) { return new Intl.DateTimeFormat(locale(), { month: "long" }).format(new Date(2020, month, 1)); }
