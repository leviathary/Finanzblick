// Prüft Karten-Command-Verträge und die Abhängigkeitsgrenzen des Architekturumbaus.
import assert from "node:assert/strict";
import fs from "node:fs";
import path from "node:path";
import test from "node:test";
import vm from "node:vm";
import ts from "typescript";

const root = new URL("../", import.meta.url);
const read = file => fs.readFileSync(new URL(file, root), "utf8");
test("transfer sorting toggles headers, compares magnitude and preserves source rows", () => {
  const sandbox = { exports: {}, Intl };
  vm.runInNewContext(ts.transpileModule(read("src/features/transactions/transferSorting.ts"), {
    compilerOptions: { module: ts.ModuleKind.CommonJS, target: ts.ScriptTarget.ES2020 },
  }).outputText, sandbox);
  const { sortTransfers, nextTransferSort } = sandbox.exports;
  const rows = [
    { id: 1, bookingDate: "2026-09-01", amountMinor: -400000, accountName: "B", description: "Beta", transferType: "INTERNAL_TRANSFER" },
    { id: 2, bookingDate: "2026-09-03", amountMinor: 20000, accountName: "A", description: "Alpha", transferType: "CREDIT_CARD_SETTLEMENT" },
    { id: 3, bookingDate: "2026-09-02", amountMinor: -20000, accountName: "A", description: "Gamma", transferType: "INTERNAL_TRANSFER" },
  ];
  const labels = type => type === "INTERNAL_TRANSFER" ? "Umbuchung" : "Kartenausgleich";
  const ids = (key, descending) => Array.from(sortTransfers(rows, { key, descending }, "de-CH", labels), row => row.id);
  assert.deepEqual(ids("amountMinor", true), [1, 2, 3]);
  assert.deepEqual(ids("amountMinor", false), [2, 3, 1]);
  assert.deepEqual(ids("bookingDate", true), [2, 3, 1]);
  assert.deepEqual(ids("bookingDate", false), [1, 3, 2]);
  assert.deepEqual(ids("accountName", false), [2, 3, 1]);
  assert.deepEqual(ids("description", false), [2, 1, 3]);
  assert.deepEqual(ids("transferType", false), [2, 3, 1]);
  assert.equal(nextTransferSort({ key: "bookingDate", descending: true }, "amountMinor").descending, true);
  assert.equal(nextTransferSort({ key: "amountMinor", descending: true }, "amountMinor").descending, false);
  assert.equal(nextTransferSort({ key: "amountMinor", descending: true }, "description").descending, false);
  assert.deepEqual(rows.map(row => row.id), [1, 2, 3]);
  assert.equal(sortTransfers([], { key: "amountMinor", descending: true }, "en", labels).length, 0);
});
test("unresolved credit badge is an accessible shortcut to the existing account-scoped filter", () => {
  const source = read("src/features/cards/CreditCards.tsx");
  assert.match(source, /<button type="button" className="cards-unresolved-count"/);
  assert.match(source, /aria-controls="card-transactions-table"/);
  assert.match(source, /onClick=\{showUnresolved\}/);
  assert.match(source, /setFilter\("UNKNOWN"\);setPeriod\("all"\);setSearch\(""\)/);
  assert.match(source, /id="card-transactions-table"/);
});
test("card history filters use local calendar years, inclusive dates and account-scoped search", () => {
  const sandbox = { exports: {} };
  vm.runInNewContext(ts.transpileModule(read("src/features/cards/overviewModel.ts"), {
    compilerOptions: { module: ts.ModuleKind.CommonJS, target: ts.ScriptTarget.ES2020 },
  }).outputText, sandbox);
  const { cardPeriodRange, selectCardRows, sortCardRows } = sandbox.exports;
  const rows = [
    { id:1, accountId:1, accountName:"Visa", kind:"PURCHASE", bookingDate:"2026-01-01", description:"Bolt ride", amountMinor:-200 },
    { id:2, accountId:1, accountName:"Visa", kind:"UNKNOWN", bookingDate:"2025-12-31", description:"Credit", amountMinor:500 },
    { id:3, accountId:2, accountName:"Mastercard", kind:"PURCHASE", bookingDate:"2026-12-31", description:"BOLT trip", amountMinor:-1000 },
    { id:4, accountId:2, accountName:"Mastercard", kind:"UNKNOWN", bookingDate:"2024-06-01", description:"Old credit", amountMinor:100 },
  ];
  const range = period => cardPeriodRange(period, "2026-01-01", "2026-01-01", new Date(2026,0,1));
  const ids = (account, kind, filters) => Array.from(selectCardRows(rows, account, kind, filters), r=>r.id);
  assert.deepEqual(ids("ALL", "ALL", range("current")), [3,1]);
  assert.deepEqual(ids("ALL", "ALL", range("previous")), [2]);
  assert.deepEqual(ids("ALL", "ALL", range("custom")), [1]);
  assert.deepEqual(ids("1", "UNKNOWN", range("all")), [2]);
  assert.deepEqual(ids("ALL", "UNKNOWN", range("all")), [2,4]);
  assert.deepEqual(ids("ALL", "ALL", {...range("current"),search:" bolt "}), [3,1]);
  assert.deepEqual(ids("1", "ALL", {...range("current"),search:"bolt"}), [1]);
  assert.deepEqual(ids("ALL", "ALL", {from:"2026-12-31",to:"2026-01-01"}), []);
  assert.deepEqual(Array.from(sortCardRows(rows,{key:"amountMinor",descending:true},"de"),r=>r.id),[3,2,1,4]);
  assert.deepEqual(rows.map(r=>r.id),[1,2,3,4]);
});
test("bank payment search filters descriptions case-insensitively and excludes credits", () => {
  const sandbox = { exports: {} };
  const compiled = ts.transpileModule(read("src/features/cards/setup/model.ts"), {
    compilerOptions: { module: ts.ModuleKind.CommonJS, target: ts.ScriptTarget.ES2020 },
  });
  vm.runInNewContext(compiled.outputText, sandbox);
  const filterPayments = sandbox.exports.filterBankPayments;
  sandbox.exports.filterBankPayments = (rows, query, sort) => filterPayments(rows, query, sort, new Date(2026, 8, 19));
  const rows = [
    { id: 1, description: "LSV Kartenrechnung", amountMinor: -100, bookingDate: "2026-09-02" },
    { id: 2, description: "TWINT Einkauf", amountMinor: -50, bookingDate: "2026-09-01" },
    { id: 3, description: "LSV Gutschrift", amountMinor: 100, bookingDate: "2026-09-03" },
  ];
  const ids = query => Array.from(sandbox.exports.filterBankPayments(rows, query), row => row.id);
  assert.deepEqual(ids(" lsv "), [1]);
  assert.deepEqual(ids(""), [1, 2]);
  assert.deepEqual(ids("missing"), []);
  assert.deepEqual(Array.from(sandbox.exports.filterBankPayments(rows, "", "oldest"), row => row.id), [2, 1]);
  assert.deepEqual(Array.from(sandbox.exports.filterBankPayments(rows, "", "smallest"), row => row.id), [2, 1]);
  assert.deepEqual(Array.from(sandbox.exports.filterBankPayments(rows, "", "largest"), row => row.id), [1, 2]);
  assert.deepEqual(Array.from(sandbox.exports.filterBankPayments(rows, "lsv", "largest"), row => row.id), [1]);
  assert.deepEqual(rows.map(row => row.id), [1, 2, 3]);
  assert.equal(rows.length, 3);
  assert.equal(filterPayments([{...rows[0], bookingDate: "2026-06-01"}], "lsv", "largest", new Date(2026,8,19), 4).length, 1);
  assert.equal(filterPayments([{...rows[0], bookingDate: "2026-06-01"}], "lsv", "largest", new Date(2026,8,19), 2).length, 0);
  const dated = dates => dates.map((bookingDate, id) => ({ id, bookingDate, description: "Payment", amountMinor: -100 }));
  assert.deepEqual(Array.from(filterPayments(dated(["2024-10-15", "2026-07-18", "2026-07-19", "2026-09-19", "2026-09-20"]), "", "oldest", new Date(2026, 8, 19)), row => row.bookingDate), ["2026-07-19", "2026-09-19"]);
  assert.deepEqual(Array.from(filterPayments(dated(["2026-02-27", "2026-02-28", "2026-04-30"]), "", "oldest", new Date(2026, 3, 30)), row => row.bookingDate), ["2026-02-28", "2026-04-30"]);
  assert.equal(filterPayments(dated(["2024-01-01"]), "", "newest", new Date(2026, 8, 19)).length, 0);
});
function files(directory, extension) {
  const base = new URL(directory, root);
  return fs.readdirSync(base, { recursive: true })
    .filter(file => file.endsWith(extension))
    .map(file => directory + "/" + file.split(path.sep).join("/"));
}

test("card API preserves command names, payloads and error propagation", async () => {
  const calls = [];
  let failure;
  const sandbox = {
    exports: {},
    require: name => {
      assert.equal(name, "@tauri-apps/api/core");
      return { invoke: (command, args) => {
        calls.push({ command, args });
        return failure ? Promise.reject(failure) : Promise.resolve([]);
      } };
    },
  };
  const compiled = ts.transpileModule(read("src/features/cards/api.ts"), {
    compilerOptions: { module: ts.ModuleKind.CommonJS, target: ts.ScriptTarget.ES2020 },
  });
  vm.runInNewContext(compiled.outputText, sandbox);
  const api = sandbox.exports.cardsApi;
  const decision = { transactionId: 7, kind: "REFUND", categoryKey: "groceries" };
  const request = { cardId: 2, cardRule: null, bankRule: null, decisions: [decision], past: true, future: false };
  await api.accounts();
  await api.rules();
  await api.categories();
  await api.rows(2);
  await api.preview(7, "Payment");
  await api.decide(decision);
  await api.transfer(7, "NONE");
  await api.confirm(request);
  assert.deepEqual(JSON.parse(JSON.stringify(calls)), [
    { command: "list_accounts" },
    { command: "list_settlement_rules" },
    { command: "list_categories" },
    { command: "list_card_setup_transactions", args: { accountId: 2 } },
    { command: "preview_settlement_rule", args: { transactionId: 7, prefix: "Payment" } },
    { command: "set_card_credit_decision", args: { decision } },
    { command: "set_transaction_transfers", args: { transactionIds: [7], transferType: "NONE" } },
    { command: "confirm_card_setup", args: { request } },
  ]);
  failure = new Error("locked");
  await assert.rejects(api.confirm(request), error => error === failure);
});

test("domain and importers stay independent of persistence and Tauri state", () => {
  for (const file of [...files("src-tauri/src/domain", ".rs"), ...files("src-tauri/src/importers", ".rs")]) {
    assert.doesNotMatch(read(file), /crate::storage|rusqlite|tauri::State/, file);
  }
});

test("setup previews preserve optional sides, limits and confirmed IDs", () => {
  const compiled = ts.transpileModule(read("src/features/cards/setup/model.ts"), {
    compilerOptions: { module: ts.ModuleKind.CommonJS, target: ts.ScriptTarget.ES2020 },
  });
  const sandbox = { exports: {} };
  vm.runInNewContext(compiled.outputText, sandbox);
  const { validPreview, draft } = sandbox.exports;
  const row = { id: 7 };
  const data = { prefix: "Payment", matches: [{ id: 7 }, { id: 8 }] };
  assert.equal(validPreview(undefined, { loading: false, data: null }), true);
  assert.equal(validPreview(row, { loading: true, data }), false);
  assert.equal(validPreview(row, { loading: false, data: null }), false);
  assert.equal(validPreview(row, { loading: false, data: { matches: [] } }), false);
  assert.equal(validPreview(row, { loading: false, data: { matches: Array(5000).fill(row) } }), true);
  assert.equal(validPreview(row, { loading: false, data: { matches: Array(5001).fill(row) } }), false);
  assert.equal(draft(undefined, data), null);
  assert.equal(draft(row, null), null);
  assert.deepEqual(JSON.parse(JSON.stringify(draft(row, data))), {
    transactionId: 7, prefix: "Payment", expectedIds: [7, 8],
  });
});

test("new banking commands and card workflow do not contain SQL", () => {
  for (const file of [...files("src-tauri/src/commands", ".rs"), "src-tauri/src/application/cards.rs"]) {
    assert.doesNotMatch(read(file), /SELECT\s|INSERT\s+INTO|UPDATE\s+\w+\s+SET|DELETE\s+FROM/, file);
  }
  for (const file of files("src-tauri/src/storage", ".rs")) {
    assert.doesNotMatch(read(file), /#\[tauri::command\]|tauri::State/, file);
  }
});

test("card overview and wizard use separate components and the typed API", () => {
  const overview = read("src/features/cards/CreditCards.tsx");
  const wizard = read("src/features/cards/setup/CardSetupWizard.tsx");
  for (const code of [overview, wizard]) assert.doesNotMatch(code, /@tauri-apps|\binvoke\s*[<(]/);
  assert.doesNotMatch(overview, /setStep|setCardSeed|setBankSeed|CardSetupWizard/);
  assert.doesNotMatch(wizard, /TransactionActions|SettlementRuleDialog/);
  assert.match(read("src/App.tsx"), /<CardSetupWizard\s*\/>/);
});

test("storage entry points only wire modules and reporting separates reads from writes", () => {
  for (const file of ["storage/mod.rs", "storage/database/mod.rs", "storage/imports/mod.rs", "storage/reporting/mod.rs"]) {
    assert.doesNotMatch(read("src-tauri/src/" + file), /\bfn\s+\w+|\bimpl\s+|SELECT\s|CREATE\s+TABLE/, file);
  }
  const flags = read("src-tauri/src/storage/banking/reporting_flags.rs").split("#[cfg(test)]")[0];
  assert.doesNotMatch(flags, /CREATE\s+(TABLE|TEMP\s+VIEW)/);
  const projection = read("src-tauri/src/storage/reporting/consumption.rs");
  assert.match(projection, /CREATE TEMP VIEW/);
  assert.doesNotMatch(projection, /INSERT\s+INTO|UPDATE\s+\w+\s+SET|DELETE\s+FROM/);
  assert.match(read("src-tauri/src/storage/database/schema.rs"), /fn initialize_reporting_flags/);
  assert.doesNotMatch(read("src-tauri/src/storage/imports/identity.rs"), /\.execute|\.prepare|\.query_row/);
});

test("market and tax workflows keep SQL and external data access behind their boundaries", () => {
  for (const file of ["application/market_data.rs", "application/taxes.rs", "application/assistant/context.rs"]) {
    const production = read("src-tauri/src/" + file).split(/#\[cfg\(test\)\]\s*mod tests/)[0];
    assert.doesNotMatch(production, /SELECT\s|INSERT\s+INTO|UPDATE\s+\w+\s+SET|DELETE\s+FROM/, file);
  }
  for (const file of ["storage/securities/market_data.rs", "storage/taxes/snapshots.rs", "storage/assistant/context.rs"]) {
    assert.doesNotMatch(read("src-tauri/src/" + file), /reqwest|tauri::|pdf_extract/, file);
  }
  assert.doesNotMatch(read("src-tauri/src/infrastructure/market_data.rs"), /rusqlite|\.execute\(|\.query_row\(/);
  assert.doesNotMatch(read("src-tauri/src/storage/database/schema.rs"), /fn apply_categories/);
  assert.match(read("src-tauri/src/storage/rules/categorization.rs"), /fn apply_categories/);
});

test("positions use a separate component and preserve typed command payloads", async () => {
  const accounts = read("src/features/accounts/Accounts.tsx");
  assert.doesNotMatch(accounts, /setValuation|save_manual_valuation|list_manual_positions/);
  assert.match(accounts, /<ManualPositions\s/);
  assert.doesNotMatch(read("src/features/positions/ManualPositions.tsx"), /@tauri-apps|\binvoke\s*[<(]/);
  const calls = [];
  const sandbox = {
    exports: {},
    require: name => {
      assert.equal(name, "@tauri-apps/api/core");
      return { invoke: (command, args) => { calls.push({ command, args }); return Promise.resolve(); } };
    },
  };
  vm.runInNewContext(ts.transpileModule(read("src/features/positions/api.ts"), {
    compilerOptions: { module: ts.ModuleKind.CommonJS, target: ts.ScriptTarget.ES2020 },
  }).outputText, sandbox);
  const api = sandbox.exports.positionsApi;
  const request = { id: null, accountId: 4, amountMinor: 1200 };
  await api.list(4);
  await api.save(request);
  await api.remove(9);
  await api.refresh();
  assert.deepEqual(JSON.parse(JSON.stringify(calls)), [
    { command: "list_manual_positions", args: { accountId: 4 } },
    { command: "save_manual_valuation", args: { request } },
    { command: "delete_manual_position", args: { positionId: 9 } },
    { command: "refresh_market_data", args: { force: true } },
  ]);
});

test("auth and reusable charts belong to their own modules", () => {
  assert.match(read("src/App.tsx"), /features\/auth\/VaultGate/);
  assert.match(read("src/features/transactions/Transactions.tsx"), /shared\/charts\/TimelineChart/);
  for (const file of ["src/VaultGate.tsx", "src/features/assets/TimelineChart.tsx", "src-tauri/src/storage/models.rs"]) {
    assert.equal(fs.existsSync(new URL(file, root)), false, file);
  }
});

test("card setup starts without a duplicate account picker on the overview", () => {
  const overview = read("src/features/cards/CreditCards.tsx");
  assert.match(overview, /href="#transactions\/cards\/setup"/);
  assert.doesNotMatch(overview, /Bitte Konto wählen/);
  assert.match(overview, /cards-account-select/);
  assert.match(overview, /Alle Kartenkonten/);
  assert.match(overview, /<CardAccountStatus/);
  assert.match(overview, /cards-review-notice/);
  assert.match(read("src/features/cards/setup/CardSetupWizard.tsx"), /Bitte Konto wählen/);
  assert.match(read("src/App.css"), /features\/cards\/cards.css/);
});

test("multi-card status uses account ID, currency and credit direction; filters preserve ownership", () => {
  const sandbox = { exports: {} };
  vm.runInNewContext(ts.transpileModule(read("src/features/cards/overviewModel.ts"), {
    compilerOptions: { module: ts.ModuleKind.CommonJS, target: ts.ScriptTarget.ES2020 },
  }).outputText, sandbox);
  const { hasSettlementRule, selectCardRows, ALL_CARDS } = sandbox.exports;
  const a = { id: 1, name: "Family card", currency: "CHF" };
  const b = { id: 2, name: "Family card", currency: "CHF" };
  const rule = { id: 8, accountId: 1, accountName: "Family card", currency: "CHF", direction: 1 };
  assert.equal(hasSettlementRule(a, [rule, rule]), true);
  assert.equal(hasSettlementRule(b, [rule]), false);
  assert.equal(hasSettlementRule(a, [{ ...rule, direction: -1 }]), false);
  assert.equal(hasSettlementRule(a, [{ ...rule, currency: "EUR" }]), false);
  assert.equal(hasSettlementRule(a, []), false);
  const rows = [
    { id: 3, accountId: 1, kind: "UNKNOWN", bookingDate: "2026-01-01", currency: "CHF" },
    { id: 4, accountId: 2, kind: "UNKNOWN", bookingDate: "2026-02-01", currency: "EUR" },
    { id: 5, accountId: 1, kind: "PURCHASE", bookingDate: "2026-03-01", currency: "CHF" },
    { id: 6, accountId: 2, kind: "REFUND", bookingDate: "2026-03-02", currency: "EUR" },
  ];
  const ids = result => JSON.parse(JSON.stringify(result.map(row => row.id)));
  assert.deepEqual(ids(selectCardRows(rows, ALL_CARDS)), [6, 5, 4, 3]);
  assert.deepEqual(ids(selectCardRows(rows, "1", "UNKNOWN")), [3]);
  assert.deepEqual(ids(selectCardRows(rows, "2", "UNKNOWN")), [4]);
  assert.deepEqual(ids(selectCardRows(rows, ALL_CARDS, "UNKNOWN")), [4, 3]);
  assert.deepEqual(ids(selectCardRows(rows, "1", "REFUND")), []);
  assert.equal(rows[0].id, 3);
});

test("wizard uses direct pattern selection and explicit skip navigation", () => {
  const wizard = read("src/features/cards/setup/CardSetupWizard.tsx");
  assert.doesNotMatch(wizard, /Unverändert lassen|Keine Beispielzahlung auswählen|Aktion wählen/);
  assert.match(wizard, /<PatternTable rows=\{credits\}/);
  assert.match(wizard, /<PatternTable rows=\{filterBankPayments\(bankRows,bankSearch,bankSort,new Date\(\),bankMonths\)\}/);
  assert.match(wizard, /window.confirm/);
  assert.match(wizard, /if\(step===2\)\{chooseCard\(""\);\}/);
  assert.doesNotMatch(wizard, /Weitere Einordnung|extraActions|chooseCategory|setDecisions/);
  assert.match(wizard, /step===2&&\(!cardRow\|\|!validPreview\(cardRow,cardPreview\)\)/);
  assert.match(wizard, /state.key===requestKey/);
  assert.match(wizard, /if\(step===3\)chooseBank\(""\)/);
  assert.match(wizard, /<footer className="wizard-footer"/);
});

test("pattern table has one action per row and preserves radio-style selection", () => {
  const jsx = (type, props) => ({ type, props });
  const sandbox = { exports: {}, require: name => {
    if (name === "react/jsx-runtime") return { jsx, jsxs: jsx };
    if (name === "../../../i18n") return { t: text => text };
    if (name === "../presentation") return { date: x => x, money: x => String(x), kindLabels: { UNKNOWN: "Ungeklärte Gutschrift" } };
    throw new Error(name);
  } };
  vm.runInNewContext(ts.transpileModule(read("src/features/cards/setup/PatternTable.tsx"), {
    compilerOptions: { module: ts.ModuleKind.CommonJS, target: ts.ScriptTarget.ES2020, jsx: ts.JsxEmit.ReactJSX },
  }).outputText, sandbox);
  const calls = [];
  const rows = [7, 8].map(id => ({ id, kind: "UNKNOWN", suggestedSettlement: true, description: "Payment", bookingDate: "2026-01-01", amountMinor: 100, currency: "CHF" }));
  function nodes(node) {
    if (Array.isArray(node)) return node.flatMap(nodes);
    if (!node || typeof node !== "object") return [];
    return [node, ...nodes(node.props?.children)];
  }
  const render = selectedId => nodes(sandbox.exports.PatternTable({ rows, selectedId, disabled: false, onSelect: id => calls.push(id) }));
  const initial = render("");
  assert.deepEqual(calls, []);
  assert.equal(initial.filter(n => n.type === "button").length, rows.length);
  assert.equal(initial.filter(n => n.type === "details" || n.type === "select").length, 0);
  assert.ok(initial.some(n => n.props.className === "pattern-recommendation"));
  const button = initial.find(n => n.type === "button");
  assert.equal(button.props["aria-pressed"], false);
  button.props.onClick();
  assert.deepEqual(calls, ["7"]);
  const selected = render("7").find(n => n.type === "button");
  assert.equal(selected.props["aria-pressed"], true);
  selected.props.onClick();
  assert.deepEqual(calls, ["7", "7"]);
  render("7").filter(n => n.type === "button")[1].props.onClick();
  assert.deepEqual(calls, ["7", "7", "8"]);
  assert.equal(render("8").filter(n => n.type === "button" && n.props["aria-pressed"]).length, 1);
});
