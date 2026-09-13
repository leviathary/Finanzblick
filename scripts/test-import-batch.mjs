import assert from "node:assert/strict";
import { readFile } from "node:fs/promises";
import test from "node:test";
import ts from "typescript";

const source = await readFile(new URL("../src/features/imports/importBatch.ts", import.meta.url), "utf8");
const js = ts.transpileModule(source, { compilerOptions: { module: ts.ModuleKind.ESNext } }).outputText;
const { canRelease, displayedProvider, suggestAccounts, hasAccounts, readyToSave, saveBatch } = await import(`data:text/javascript;base64,${Buffer.from(js).toString("base64")}`);
const account = { id: 1, name: "Privat", provider: "UBS", providerKey: "ubs", currency: "CHF", accountType: "checking", isActive: true };
const parsed = { format: "CSV", accountName: "", provider: "ubs", accountType: "checking", transactions: [{ currency: "CHF" }], currencyBalances: [{ currency: "USD" }] };
const usd = { ...account, id: 2, currency: "USD" };
const item = { file: { path: "/statements/a.pdf" }, parsed, accountIds: { CHF: 1, USD: 2 }, reviewed: true };

test("bulk release excludes failed, saved, ambiguous and incomplete files", () => {
  const candidate = { ...item, reviewed: false };
  assert.equal(canRelease(candidate, [account, usd]), true);
  for (const changes of [{ alreadyImported: true }, { error: "Failed" }, { result: { duplicate: true } }, { reviewed: true }, { accountIds: { CHF: 1 } }, { parsed: undefined }]) {
    assert.equal(canRelease({ ...candidate, ...changes }, [account, usd]), false);
  }
  assert.equal(displayedProvider({ ...candidate, parsed: { ...parsed, provider: "unknown" }, file: { provider: "unknown" } }, [account, usd]), "ubs");
  assert.equal(displayedProvider({ ...candidate, file: { provider: "unknown" }, accountIds: {} }, [account, usd]), "unknown");
});

test("requires valid accounts for every currency without a separate release", () => {
  assert.equal(readyToSave(item, [account, usd]), true);
  assert.equal(readyToSave({ ...item, reviewed: false }, [account, usd]), true);
  assert.equal(hasAccounts(item, [account]), false);
  for (const changed of [{ isActive: false }, { providerKey: "migros" }, { accountType: "savings" }, { currency: "EUR" }]) {
    assert.equal(hasAccounts(item, [account, { ...usd, ...changed }]), false);
  }
  assert.equal(readyToSave({ ...item, result: { duplicate: true } }, [account, usd]), false);
});

test("suggests only unambiguous matching accounts", () => {
  assert.deepEqual(suggestAccounts([account, usd], parsed), { CHF: 1, USD: 2 });
  assert.deepEqual(suggestAccounts([account, { ...account, id: 3 }, usd], parsed), { USD: 2 });
});

test("MT940 selects by account reference, respecting currency, provider and ambiguity", () => {
  const statement = { ...parsed, format: "MT940", accountName: "CH00 0000 0000 0000 1000 1", currencyBalances: [] };
  const matching = { ...account, externalReference: "iban: ch0000000000000010001" };
  const other = { ...account, id: 3, externalReference: "CH0000000000000030003" };
  assert.deepEqual(suggestAccounts([other, matching], statement), { CHF: 1 });
  assert.deepEqual(suggestAccounts([other], statement), {});
  assert.deepEqual(suggestAccounts([matching, { ...matching, id: 4 }], statement), {});
  for (const change of [{ isActive: false }, { currency: "USD" }, { providerKey: "migros" }]) {
    assert.deepEqual(suggestAccounts([{ ...matching, ...change }], statement), {});
  }
});

test("unknown CSV provider uses a consistent account provider without guessing merchants", () => {
  const unknown = { ...parsed, provider: "unknown" };
  assert.deepEqual(suggestAccounts([account, usd], unknown), { CHF: 1, USD: 2 });
  assert.equal(hasAccounts({ ...item, parsed: unknown }, [account, usd]), true);
  assert.equal(hasAccounts({ ...item, parsed: unknown }, [account, { ...usd, providerKey: "migros" }]), false);
  assert.deepEqual(suggestAccounts([account, { ...account, id: 3, providerKey: "migros" }, usd], unknown), { USD: 2 });
});

test("continues after a save failure and preserves duplicate results", async () => {
  const results = new Map();
  const targets = ["first", "bad", "duplicate", "last"].map(path => ({ ...item, file: { path } }));
  await saveBatch(targets, async current => {
    if (current.file.path === "bad") throw new Error("File unavailable");
    return { importId: 1, duplicate: current.file.path === "duplicate" };
  }, (path, changes) => results.set(path, changes), () => false);
  assert.equal(results.size, 4);
  assert.match(results.get("bad").error, /File unavailable/);
  assert.equal(results.get("duplicate").result.duplicate, true);
  assert.ok(results.get("last").result);
  const retry = targets.map(current => ({ ...current, ...results.get(current.file.path) })).filter(current => readyToSave(current, [account, usd]));
  assert.deepEqual(retry.map(current => current.file.path), ["bad"]);
});

test("stops between files without discarding completed results", async () => {
  let stop = false;
  const completed = [];
  await saveBatch([item, item], async () => { stop = true; return { importId: 4 }; }, (_, change) => completed.push(change), () => stop);
  assert.equal(completed.length, 1);
  assert.equal(completed[0].result.importId, 4);
});
