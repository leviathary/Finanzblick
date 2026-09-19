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
  const picker = await readFile(new URL("../src/features/assets/BenchmarkPicker.tsx", import.meta.url), "utf8");
  assert.ok(!picker.includes("Basis 100 am ersten gemeinsamen Datum"));
  assert.ok(!picker.includes("Quelle: Yahoo Finance"));
});
