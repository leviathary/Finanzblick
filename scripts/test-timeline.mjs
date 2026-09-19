// Prüft Chart-Daten, Kalenderlücken, Messungen und die Trennung vom Auswertungszeitraum.
import assert from "node:assert/strict";
import { readFile } from "node:fs/promises";
import test from "node:test";
import ts from "typescript";
const source = await readFile(new URL("../src/shared/charts/timelineModel.ts", import.meta.url), "utf8");
const js = ts.transpileModule(source, { compilerOptions: { module: ts.ModuleKind.ESNext, target: ts.ScriptTarget.ES2020 } }).outputText;
const { prepareTimeline, measureTimeline, zoomRange, monthsBefore } = await import(`data:text/javascript;base64,${Buffer.from(js).toString("base64")}`);
test("timeline sorts a copy, converts minor units and preserves calendar gaps", () => {
  const input = [{ date: "2024-03-01", totalMinor: 12345 }, { date: "2024-02-28", totalMinor: -123 }];
  const original = structuredClone(input);
  const { data } = prepareTimeline(input);
  assert.deepEqual(data, [{ time: "2024-02-28", value: -1.23 }, { time: "2024-02-29" }, { time: "2024-03-01", value: 123.45 }]);
  assert.deepEqual(input, original);
  assert.deepEqual(prepareTimeline([]), { points: [], data: [] });
  assert.equal(prepareTimeline([input[0]]).data.length, 1);
});
test("timeline rejects invalid dates, duplicate days and non-finite values", () => {
  for (const date of ["2023-02-29", "2024-13-01", "not a date"]) assert.throws(() => prepareTimeline([{ date, totalMinor: 0 }]));
  assert.throws(() => prepareTimeline([{ date: "2024-01-01", totalMinor: Infinity }]));
  assert.throws(() => prepareTimeline([{ date: "2024-01-01", totalMinor: 1 }, { date: "2024-01-01", totalMinor: 2 }]));
});
test("measurement supports zero, negative and reversed endpoints without mutating balances", () => {
  assert.deepEqual(measureTimeline({ date: "2026-03-24", totalMinor: 0 }, { date: "2026-06-24", totalMinor: 123 }), { difference: 123, percent: null, days: 92 });
  assert.deepEqual(measureTimeline({ date: "2026-06-24", totalMinor: -200 }, { date: "2026-03-24", totalMinor: -100 }), { difference: 100, percent: 50, days: 92 });
  assert.deepEqual(measureTimeline({ date: "2026-03-24", totalMinor: 200 }, { date: "2026-03-24", totalMinor: 100 }), { difference: -100, percent: -50, days: 0 });
  assert.deepEqual(zoomRange({ from: 0, to: 100 }, 0.5), { from: 25, to: 75 });
});
test("month presets clamp month ends and preserve leap years", () => {
  assert.equal(monthsBefore("2024-03-31", 1), "2024-02-29");
  assert.equal(monthsBefore("2026-03-31", 1), "2026-02-28");
  assert.equal(monthsBefore("2026-03-31", 6), "2025-09-30");
});
test("wealth chart cannot change report dates and chart strings are translated", async () => {
  const wrapper = await readFile(new URL("../src/features/assets/WealthChart.tsx", import.meta.url), "utf8");
  assert.ok(!wrapper.includes("onSelectRange"));
  const component = await readFile(new URL("../src/shared/charts/InteractiveTimelineChart.tsx", import.meta.url), "utf8");
  assert.ok(!component.includes("onSelectRange"));
  const messages = JSON.parse(await readFile(new URL("../src/translations.json", import.meta.url), "utf8"));
  for (const [, key] of component.matchAll(/\bt\("([^"]+)"\)/g)) assert.equal(messages[key]?.length, 3, key);
});

test("benchmark comparison uses a common positive base and never changes report values", async () => {
  const source = await readFile(new URL("../src/features/assets/benchmarkModel.ts", import.meta.url), "utf8");
  const js = ts.transpileModule(source, { compilerOptions: { module: ts.ModuleKind.ESNext, target: ts.ScriptTarget.ES2020 } }).outputText;
  const { compareBenchmark } = await import(`data:text/javascript;base64,${Buffer.from(js).toString("base64")}`);
  const history = [{ date: "2026-01-01", totalMinor: 100 }, { date: "2026-01-02", totalMinor: 200 }, { date: "2026-01-03", totalMinor: 300 }, { date: "2026-01-04", totalMinor: 400 }];
  const benchmark = { name: "Test", currency: "USD", points: [{ date: "2026-01-02", close: 50 }, { date: "2026-01-04", close: 60 }] };
  const before = structuredClone({ history, benchmark });
  const result = compareBenchmark(history, benchmark);
  assert.equal(result.from, "2026-01-02");
  assert.equal(result.main[0].totalMinor, 10000);
  assert.equal(result.main[2].totalMinor, 20000);
  assert.deepEqual(result.comparison.map(p => p.totalMinor), [10000, 12000]);
  assert.equal(result.comparison.length, 2); // No synthetic price for the missing trading day.
  assert.deepEqual({ history, benchmark }, before);
  assert.equal(compareBenchmark([], benchmark), null);
  assert.equal(compareBenchmark(history, { ...benchmark, points: [] }), null);
  for (const totalMinor of [0, -1]) assert.equal(compareBenchmark(history.map(p => ({ ...p, totalMinor })), benchmark), null);
  assert.equal(compareBenchmark(history, { ...benchmark, points: [{ date: "2025-01-01", close: 5 }] }), null);
});

test("transaction balance chart shares interactive controls without changing report dates on zoom", async () => {
  const code = await readFile(new URL("../src/features/transactions/Transactions.tsx", import.meta.url), "utf8");
  assert.ok(code.includes("<InteractiveTimelineChart"));
  assert.ok(!code.includes("onSelectRange="));
  assert.ok(!code.includes('t("Gesamte Datenbasis:")'));
  assert.ok(!code.includes('t("Klicke auf eine Kategorie, um die einzelnen Buchungen zu sehen.")'));
  assert.ok(code.includes("monthsBefore(fullTo"));
  assert.ok(code.includes('invoke<TimelinePoint[]>("bank_balance_history"'));
  assert.ok(code.includes('item.accountType === "cash" || item.accountType === "savings"'));
  assert.ok(code.includes('label={t("Konto der Auswertung")}'));
  assert.ok(code.includes('providerKeys: provider, accountIds: account.map(Number)'));
  assert.ok(code.includes("accountId: balanceAccount ? Number(balanceAccount) : null"));
  const help = await readFile(new URL("../src/features/transactions/BalanceChartHelp.tsx", import.meta.url), "utf8");
  const messages = JSON.parse(await readFile(new URL("../src/translations.json", import.meta.url), "utf8"));
  for (const [, key] of help.matchAll(/\bt\("([^"]+)"\)/g)) assert.equal(messages[key]?.length, 3, key);
});

test("toolbar and benchmark labels are translated and obsolete chart heading is absent", async () => {
  const messages = JSON.parse(await readFile(new URL("../src/translations.json", import.meta.url), "utf8"));
  for (const name of ["BenchmarkPicker", "WealthChart", "ChartHelp"]) {
    const code = await readFile(new URL(`../src/features/assets/${name}.tsx`, import.meta.url), "utf8");
    for (const [, key] of code.matchAll(/\bt\("([^"]+)"\)/g)) assert.equal(messages[key]?.length, 3, key);
  }
  const assets = await readFile(new URL("../src/features/assets/Assets.tsx", import.meta.url), "utf8");
  assert.ok(assets.includes('aria-label={t("Chart-Steuerung")}'));
  assert.ok(!assets.includes('t("Kein Vergleichszeitraum")'));
  assert.ok(assets.includes("<ChartHelp />"));
  assert.ok(assets.includes('reportPeriod() ? "custom" : "currentYear"'), "Wealth defaults to YTD unless a report period is provided");
  const picker = await readFile(new URL("../src/features/assets/BenchmarkPicker.tsx", import.meta.url), "utf8");
  assert.ok(!picker.includes("Basis 100 am ersten gemeinsamen Datum"));
  assert.ok(!picker.includes("Quelle: Yahoo Finance"));
});

test("category donut and multi-select expose translated keyboard-accessible controls", async () => {
  const donut = await readFile(new URL("../src/features/transactions/CategoryDonut.tsx", import.meta.url), "utf8");
  const multi = await readFile(new URL("../src/shared/MultiSelect.tsx", import.meta.url), "utf8");
  const messages = JSON.parse(await readFile(new URL("../src/translations.json", import.meta.url), "utf8"));
  for (const code of [donut, multi]) {
    for (const [, key] of code.matchAll(/\bt\("([^"]+)"\)/g)) assert.equal(messages[key]?.length, 3, key);
  }
  assert.ok(donut.includes("positive.slice(0,8)"));
  assert.ok(donut.includes("positive.slice(8)"));
  assert.ok(donut.includes("item.amountMinor > 0"));
  const shared = await readFile(new URL("../src/shared/charts/DistributionDonut.tsx", import.meta.url), "utf8");
  assert.ok(shared.includes('role={onSelect ? "button" : "img"} tabIndex={0}'));
  assert.ok(multi.includes('type="checkbox"'));
  assert.ok(multi.includes('event.key === "Escape"'));
});

test("drilldown category changes require explicit inline save", async () => {
  const code = await readFile(new URL("../src/features/transactions/Transactions.tsx", import.meta.url), "utf8");
  const editor = await readFile(new URL("../src/features/transactions/InlineCategoryEditor.tsx", import.meta.url), "utf8");
  const actions = await readFile(new URL("../src/features/transactions/TransactionActions.tsx", import.meta.url), "utf8");
  const messages = JSON.parse(await readFile(new URL("../src/translations.json", import.meta.url), "utf8"));
  assert.ok(code.includes('<InlineCategoryEditor'));
  assert.ok(!code.includes('onChange={event => void changeCategory'));
  assert.ok(editor.includes('await onSave(key)'));
  assert.ok(editor.includes('event.key === "Escape"'));
  assert.ok(editor.includes('role="alert"'));
  assert.ok(actions.includes('onEditCategory'));
  for (const source of [editor, actions]) {
    for (const [, key] of source.matchAll(/\bt\("([^"]+)"\)/g)) assert.equal(messages[key]?.length, 3, key);
  }
});

test("provider allocation shares donut, excludes negative slices and translates states", async () => {
  const code = await readFile(new URL("../src/features/overview/ProviderDistribution.tsx", import.meta.url), "utf8");
  const overview = await readFile(new URL("../src/features/overview/Overview.tsx", import.meta.url), "utf8");
  const messages = JSON.parse(await readFile(new URL("../src/translations.json", import.meta.url), "utf8"));
  assert.ok(code.includes("<DistributionDonut"));
  assert.ok(code.includes("Math.max(0,item.balanceMinor)"));
  assert.ok(code.includes("total={total}"));
  assert.ok(!overview.includes('className="provider-bar"'));
  for (const [, key] of code.matchAll(/\bt\("([^"]+)"\)/g)) assert.equal(messages[key]?.length, 3, key);
});
