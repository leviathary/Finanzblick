// Prüft isolierte Flugphysik, sichere Landungen, Treibstoffgrenzen und Übersetzungen des Ostereis.
import assert from "node:assert/strict";
import { readFile } from "node:fs/promises";
import test from "node:test";
import ts from "typescript";
const root = new URL("../src/features/moon-lander/", import.meta.url);
const code = await readFile(new URL("model.ts", root), "utf8");
const js = ts.transpileModule(code, { compilerOptions: { module: ts.ModuleKind.ESNext, target: ts.ScriptTarget.ES2022 } }).outputText;
const { newFlight, stepFlight, PAD, FEET } = await import(`data:text/javascript;base64,${Buffer.from(js).toString("base64")}`);
const idle = { thrust: false, left: false, right: false };
const flying = () => ({ ...newFlight(), status: "flying" });
test("inactive flights never move and simulation does not mutate its input", () => {
  for (const status of ["ready", "paused", "landed", "crashed"]) {
    const state = { ...newFlight(), status };
    assert.equal(stepFlight(state, idle, .01), state);
  }
  const original = flying(), copy = { ...original };
  const next = stepFlight(original, idle, 1 / 120);
  assert.deepEqual(original, copy);
  assert(next.vy > 0);
  assert.equal(stepFlight(original, idle, -1), original);
  assert.equal(stepFlight(original, idle, NaN), original);
  assert.deepEqual(stepFlight(original, idle, 10), stepFlight(original, idle, 1 / 30));
});
test("thrust brakes descent, steering works, fuel never goes negative", () => {
  assert(stepFlight(flying(), { ...idle, thrust: true }, .01).vy < 0);
  assert(stepFlight(flying(), { ...idle, right: true }, .01).vx > flying().vx);
  assert(stepFlight(flying(), { ...idle, left: true }, .01).vx < flying().vx);
  const empty = { ...flying(), fuel: 0 };
  assert.deepEqual(stepFlight(empty, { thrust: true, left: true, right: false }, .01), stepFlight(empty, idle, .01));
  assert.equal(stepFlight({ ...flying(), fuel: .001 }, { ...idle, thrust: true }, .01).fuel, 0);
});
test("landing requires the whole craft over the pad and both safe speeds", () => {
  const approach = { ...flying(), x: 670, y: PAD.top - FEET - .1, vx: 0, vy: 20 };
  assert.equal(stepFlight(approach, idle, .01).status, "landed");
  for (const change of [{ vx: 21 }, { vy: 31 }, { x: PAD.left + 5 }, { y: PAD.top - FEET + 2 }]) {
    assert.equal(stepFlight({ ...approach, ...change }, idle, .01).status, "crashed");
  }
  assert.equal(stepFlight({ ...approach, x: 100, y: 356 }, idle, .01).status, "crashed");
  assert.equal(stepFlight({ ...flying(), x: 25 }, idle, .01).status, "crashed");
});
test("a controlled descent can win with available fuel; free fall crashes", () => {
  let state = { ...flying(), x: 670, vx: 0 };
  for (let i = 0; i < 6000 && state.status === "flying"; i++) {
    state = stepFlight(state, { ...idle, thrust: state.vy > 22 }, 1 / 120);
  }
  assert.equal(state.status, "landed");
  assert(state.fuel > 0);
  state = flying();
  for (let i = 0; i < 6000 && state.status === "flying"; i++) state = stepFlight(state, idle, 1 / 120);
  assert.equal(state.status, "crashed");
});
test("game UI is translated and has no finance, network or persistence dependencies", async () => {
  const messages = JSON.parse(await readFile(new URL("../src/translations.json", import.meta.url), "utf8"));
  for (const name of ["MoonLander.tsx", "MoonLanderLauncher.tsx", "model.ts"]) {
    const source = await readFile(new URL(name, root), "utf8");
    for (const match of source.matchAll(/\bt\("([^"\\]+)"\)/g)) assert.equal(messages[match[1]]?.length, 3, match[1]);
    assert.doesNotMatch(source, /@tauri|\binvoke\(|\bfetch\(|localStorage|sessionStorage/);
  }
});
