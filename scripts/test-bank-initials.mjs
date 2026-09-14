import assert from "node:assert/strict";
import { readFile } from "node:fs/promises";
import test from "node:test";
import ts from "typescript";

const source = await readFile(new URL("../src/features/accounts/bankInitials.ts", import.meta.url), "utf8");
const js = ts.transpileModule(source, { compilerOptions: { module: ts.ModuleKind.ESNext, target: ts.ScriptTarget.ES2020 } }).outputText;
const { bankInitials } = await import(`data:text/javascript;base64,${Buffer.from(js).toString("base64")}`);

test("bank abbreviations use names, normalized aliases and provider fallback", () => {
  assert.equal(bankInitials("Swissquote", "manual"), "SQ");
  assert.equal(bankInitials("Zürcher Kantonalbank", "manual"), "ZKB");
  assert.equal(bankInitials("Julius Bär", "manual"), "JB");
  assert.equal(bankInitials("Privat", "bank-cler"), "BC");
  assert.equal(bankInitials("Mein Anbieter", "manual"), "MA");
  assert.equal(bankInitials("Ledger", "manual"), "LE");
  assert.equal(bankInitials("", "manual"), "?");
  assert.equal(bankInitials("Clerical Services", "manual"), "CS");
});
