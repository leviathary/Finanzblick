// Prüft Kontozuordnung, Vorschläge und Fehlerbehandlung beim Stapelimport.

import assert from "node:assert/strict";
import { readFile } from "node:fs/promises";
import test from "node:test";
import ts from "typescript";

const source = await readFile(new URL("../src/features/imports/importBatch.ts", import.meta.url), "utf8");
const wizardSource = await readFile(new URL("../src/features/imports/ImportWizard.tsx", import.meta.url), "utf8");
const appSource = await readFile(new URL("../src/App.tsx", import.meta.url), "utf8");
const swissquoteReviewSource = await readFile(new URL("../src/features/imports/PositionSnapshotImport.tsx", import.meta.url), "utf8");
const applicationCss = await readFile(new URL("../src/styles/application.css", import.meta.url), "utf8");
const desktopCapability = JSON.parse(await readFile(new URL("../src-tauri/capabilities/default.json", import.meta.url), "utf8"));
const js = ts.transpileModule(source, { compilerOptions: { module: ts.ModuleKind.ESNext } }).outputText;
const { canRelease, displayedProvider, suggestAccounts, hasAccountReferenceMismatch, hasAccounts, orderedBatchItems, orderedPreviewTransactionIndices, readyPositionSnapshot, readyToSave, saveBatch, unresolvedDuplicateCount } = await import(`data:text/javascript;base64,${Buffer.from(js).toString("base64")}`);
const account = { id: 1, name: "Privat", provider: "UBS", providerKey: "ubs", currency: "CHF", accountType: "checking", isActive: true };
const parsed = { format: "CSV", accountName: "", provider: "ubs", accountType: "checking", transactions: [{ currency: "CHF" }], currencyBalances: [{ currency: "USD" }] };
const usd = { ...account, id: 2, currency: "USD" };
const item = { file: { path: "/statements/a.pdf" }, parsed, accountIds: { CHF: 1, USD: 2 }, reviewed: true };

test("snapshot release needs a date and eligible active account, including unchanged and empty snapshots", () => {
  const ready = { positionSnapshot: { snapshotDate: "2026-01-01", eligibleAccountIds: [1], positions: [], changes: [] }, positionAccountId: 1 };
  assert.equal(readyPositionSnapshot(ready, [account]), true);
  assert.equal(readyPositionSnapshot({ ...ready, positionSnapshot: { ...ready.positionSnapshot, snapshotDate: null } }, [account]), false);
  assert.equal(readyPositionSnapshot(ready, [{ ...account, isActive: false }]), false);
  for (const change of [{ positionAccountId: 2 }, { alreadyImported: true }, { error: "invalid" }, { positionResult: {} }]) {
    assert.equal(readyPositionSnapshot({ ...ready, ...change }, [account]), false);
  }
});

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

test("potential duplicates block import until every row has an explicit decision", () => {
  const duplicateCheck = { suspectedTransactions: [{ transactionIndex: 0 }, { transactionIndex: 2 }] };
  const blocked = { ...item, duplicateCheck, duplicateResolutions: {} };
  assert.equal(unresolvedDuplicateCount(blocked), 2);
  assert.equal(readyToSave(blocked, [account, usd]), false);
  const partial = { ...blocked, duplicateResolutions: { 0: "skip" } };
  assert.equal(unresolvedDuplicateCount(partial), 1);
  assert.equal(readyToSave(partial, [account, usd]), false);
  const reviewed = { ...blocked, duplicateResolutions: { 0: "skip", 2: "keep" } };
  assert.equal(unresolvedDuplicateCount(reviewed), 0);
  assert.equal(readyToSave(reviewed, [account, usd]), true);
});

test("duplicate decisions and the actual import expose distinct visible states", () => {
  assert.match(wizardSource, /✓ Kein Duplikat – wird importiert/);
  assert.match(wizardSource, /✓ Als Duplikat erkannt – wird übersprungen/);
  assert.match(wizardSource, /Import läuft …/);
  assert.match(wizardSource, /duplicate-comparison-row/);
  assert.match(wizardSource, /t\("Vergleichsbuchung"\)/);
  assert.match(applicationCss, /\.duplicate-review-actions[^}]+\[aria-pressed="true"\][^{]*\{[^}]*color:\s*var\(--text-on-action\)/s);
  assert.match(applicationCss, /tr\.duplicate-comparison-row[^}]+background:\s*var\(--bg-surface\)/s);
  assert.doesNotMatch(applicationCss, /--text-inverse/);
});

test("position snapshots use the shared picker, drag-and-drop list and automatic preview", () => {
  assert.equal((wizardSource.match(/className=\{`drop-zone batch-drop/g) ?? []).length, 1);
  assert.match(wizardSource, /onDragDropEvent/);
  assert.match(wizardSource, /await analyzePositionSnapshots\(item\)/);
  assert.match(wizardSource, /preview_position_snapshot/);
  assert.match(wizardSource, /PositionSnapshotReview/);
  assert.match(swissquoteReviewSource, /Positionsbestand/);
  assert.doesNotMatch(wizardSource + swissquoteReviewSource, /swissquote/i);
  assert.doesNotMatch(appSource, /<PositionSnapshotImport/);
  assert.doesNotMatch(swissquoteReviewSource, /plugin-dialog/);
});

test("overlapping card exports expose provisional transaction updates", () => {
  assert.match(wizardSource, /check\.updatableTransactions === 0/);
  assert.match(wizardSource, /vorläufige Kreditkartenbuchungen werden mit den endgültigen Abrechnungsdaten aktualisiert/);
  assert.match(wizardSource, /vorläufige Kartenbuchungen aktualisiert/);
  assert.match(wizardSource, /updatedTransactions/);
});

test("file warnings stay compact and the batch list avoids a second horizontal scroller", () => {
  assert.match(wizardSource, /className="warning-chip"/);
  assert.match(wizardSource, /className="batch-warning-status"/);
  assert.doesNotMatch(wizardSource, /current\.parsed\.warnings\.map/);
  assert.match(applicationCss, /@media \(max-width: 1280px\)[\s\S]+\.import-batch-card \.batch-table \{ overflow-x: hidden; \}/);
  assert.match(wizardSource, /preview-description-column/);
  assert.match(wizardSource, /preview-duplicate-column/);
  assert.match(applicationCss, /\.transaction-preview table[^}]+min-width:\s*920px;[^}]+table-layout:\s*fixed;/s);
  assert.match(applicationCss, /\.import-batch-card \.batch-table > table \{[^}]+table-layout:\s*fixed;/s);
});

test("failed PDF rows can open their exact source document", () => {
  assert.doesNotMatch(wizardSource, /import \{ openPath \} from "@tauri-apps\/plugin-opener"/);
  assert.match(wizardSource, /item\.error && item\.file\.extension === "pdf"/);
  assert.match(wizardSource, /invoke<void>\("open_import_pdf", \{ path: item\.file\.path \}\)/);
  assert.match(wizardSource, /t\("PDF anzeigen"\)/);
  assert.match(wizardSource, /sourceOpenError/);
  assert.ok(!desktopCapability.permissions.includes("opener:allow-open-path"));
});

test("an open PDF preview exposes its source beside the review status", () => {
  assert.match(wizardSource, /className="review-header-actions"/);
  assert.match(wizardSource, /current\.file\.extension === "pdf"/);
  assert.match(wizardSource, /onClick=\{\(\) => void openSource\(current\)\}/);
  assert.match(applicationCss, /\.review-header-actions[^}]+flex-wrap:\s*wrap/);
});

test("preview places unresolved and resolved duplicate candidates before clear rows", () => {
  const ordered = {
    ...item,
    parsed: { ...parsed, transactions: [{ currency: "CHF" }, { currency: "CHF" }, { currency: "CHF" }, { currency: "CHF" }] },
    duplicateCheck: { suspectedTransactions: [{ transactionIndex: 3 }, { transactionIndex: 1 }] },
    duplicateResolutions: { 1: "keep" },
  };
  assert.deepEqual(orderedPreviewTransactionIndices(ordered), [3, 1, 0, 2]);
});

test("batch list places failures and unresolved duplicate reviews first", () => {
  const clear = { ...item, file: { path: "clear" } };
  const duplicate = { ...item, file: { path: "duplicate" }, duplicateCheck: { suspectedTransactions: [{ transactionIndex: 0 }] }, duplicateResolutions: {} };
  const failed = { ...item, file: { path: "failed" }, error: "Invalid PDF" };
  const resolved = { ...duplicate, file: { path: "resolved" }, duplicateResolutions: { 0: "keep" } };
  assert.deepEqual(orderedBatchItems([clear, duplicate, failed, resolved]).map(candidate => candidate.file.path), ["failed", "duplicate", "clear", "resolved"]);
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

test("provider statements can select an account by canonical account reference", () => {
  const statement = { ...parsed, provider: "zkb", accountReference: "CH83 0070 0113 1000 1234 5", currencyBalances: [] };
  const matching = { ...account, provider: "ZKB", providerKey: "zkb", externalReference: "CH8300700113100012345" };
  const other = { ...matching, id: 3, externalReference: "CH8300700113100099999" };
  assert.deepEqual(suggestAccounts([other, matching], statement), { CHF: 1 });
  assert.equal(hasAccounts({ ...item, parsed: statement, accountIds: { CHF: 1 } }, [matching]), true);
  assert.equal(hasAccounts({ ...item, parsed: statement, accountIds: { CHF: 3 } }, [other]), false);
  assert.equal(hasAccountReferenceMismatch([other], statement), true);
  assert.equal(hasAccountReferenceMismatch([matching], statement), false);
  assert.match(wizardSource, /kein aktives Konto mit derselben hinterlegten IBAN oder Kontoreferenz vorhanden/);
});

test("generic camt selects by IBAN across providers and supports balance-only files", () => {
  const statement = { ...parsed, format: "CAMT053", provider: "unknown", accountType: null,
    accountReference: "CH00 0000 0000 0000 1000 1", transactions: [], currencyBalances: [{ currency: "CHF" }] };
  const matching = { ...account, providerKey: "postfinance", externalReference: "CH0000000000000010001" };
  assert.deepEqual(suggestAccounts([matching, { ...account, id: 4 }], statement), { CHF: 1 });
  assert.equal(hasAccounts({ ...item, parsed: statement, accountIds: { CHF: 1 } }, [matching]), true);
  assert.deepEqual(suggestAccounts([{ ...matching, currency: "EUR" }], statement), {});
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
