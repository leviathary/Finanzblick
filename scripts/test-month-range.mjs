import assert from "node:assert/strict";
import { readFile } from "node:fs/promises";
import test from "node:test";
import ts from "typescript";
const source = await readFile(new URL("../src/features/transactions/monthRange.ts", import.meta.url), "utf8");
const js = ts.transpileModule(source, { compilerOptions: { module: ts.ModuleKind.ESNext } }).outputText;
const { monthRange, shiftMonth } = await import(`data:text/javascript;base64,${Buffer.from(js).toString("base64")}`);
test("month selection includes the whole calendar month, including leap years", () => {
  for (const [month, days] of [["2026-01",31],["2026-04",30],["2026-02",28],["2024-02",29],["2100-02",28],["2000-02",29]]) {
    assert.deepEqual(monthRange(month), { from: `${month}-01`, to: `${month}-${days}` });
  }
  for (const month of ["", "2026-13", "2026-00", "2026-2", "invalid"]) assert.equal(monthRange(month), null);
});
test("month navigation crosses years and respects input limits", () => {
  assert.equal(shiftMonth("2026-01", -1), "2025-12");
  assert.equal(shiftMonth("2025-12", 1), "2026-01");
  assert.equal(shiftMonth("2024-03", -1), "2024-02");
  assert.equal(shiftMonth("1000-01", -1), "");
  assert.equal(shiftMonth("9999-12", 1), "");
});
