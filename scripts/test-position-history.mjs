import assert from "node:assert/strict";
import { readFile } from "node:fs/promises";
import test from "node:test";
import ts from "typescript";

const source = await readFile(new URL("../src/features/assets/positionHistory.ts", import.meta.url), "utf8");
const js = ts.transpileModule(source, { compilerOptions: { module: ts.ModuleKind.ESNext, target: ts.ScriptTarget.ES2020 } }).outputText;
const { sumPositionHistory } = await import(`data:text/javascript;base64,${Buffer.from(js).toString("base64")}`);
const first = { id: 1, history: [{ date: "2020-01-01", totalMinor: 100 }, { date: "2020-01-03", totalMinor: 200 }, { date: "2020-01-04", totalMinor: 0 }] };
const second = { id: 2, history: [{ date: "2020-01-02", totalMinor: 50 }, { date: "2020-01-05", totalMinor: 70 }] };

test("positions sum with staggered starts, carried values and sale reset", () => {
  assert.deepEqual(sumPositionHistory([first, second]).map(point => point.totalMinor), [100, 150, 250, 50, 70]);
});
test("single selection keeps only its own values and fills missing days", () => {
  assert.deepEqual(sumPositionHistory([first]).map(point => point.totalMinor), [100, 100, 200, 0]);
  assert.deepEqual(sumPositionHistory([]), []);
});
