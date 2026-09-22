// Regressionen für Stichtage, fehlende Bewertungen und Übersetzungen der Leseansicht.
import assert from "node:assert/strict";
import { readFile } from "node:fs/promises";
import test from "node:test";
import ts from "typescript";
const source = await readFile(new URL("../src/features/account-details/model.ts", import.meta.url), "utf8");
const js = ts.transpileModule(source, { compilerOptions: { module: ts.ModuleKind.ESNext, target: ts.ScriptTarget.ES2022 } }).outputText;
const { isCurrent, positionShares, periodHistory } = await import(`data:text/javascript;base64,${Buffer.from(js).toString("base64")}`);
const base = { id:1, start:"2026-01-02",end:null,quantity:2,valueMinor:200,valueCurrency:"CHF" };
test("positions count from inception through end, not before or after", () => {
  assert(!isCurrent(base,"2026-01-01")); assert(isCurrent(base,"2026-01-02"));
  assert(isCurrent({...base,end:"2026-01-03"},"2026-01-03"));
  assert(!isCurrent({...base,end:"2026-01-03"},"2026-01-04"));
  assert(!isCurrent({...base,quantity:0},"2026-01-04"));
});
test("weights require complete comparable values", () => {
  assert.equal(positionShares([base,{...base,id:2,valueMinor:600}],"2026-01-04").get(1),.25);
  for(const other of [{valueMinor:null},{valueCurrency:"USD"},{valueMinor:-1}]) assert.equal(positionShares([base,{...base,id:2,...other}],"2026-01-04").size,0);
});
test("periods carry known balances but never invent a pre-inception value", () => {
  const points=[{date:"2026-01-02",totalMinor:200}];
  assert.deepEqual(periodHistory(points,"2026-01-01","2026-01-04"),[...points,{date:"2026-01-04",totalMinor:200}]);
  assert.deepEqual(periodHistory(points,"2026-01-03","2026-01-04"),[{date:"2026-01-03",totalMinor:200},{date:"2026-01-04",totalMinor:200}]);
  assert.deepEqual(periodHistory(points,"2025-01-01","2026-01-01"),[]);
});
test("all account detail labels have three translations", async () => {
  const messages=JSON.parse(await readFile(new URL("../src/translations.json",import.meta.url),"utf8"));
  for(const file of ["AccountExplorer.tsx","DetailChart.tsx"]) {
    const code=await readFile(new URL("../src/features/account-details/"+file,import.meta.url),"utf8");
    for(const match of code.matchAll(/\bt\("([^"\\]+)"\)/g)) assert.equal(messages[match[1]]?.length,3,match[1]);
  }
});
