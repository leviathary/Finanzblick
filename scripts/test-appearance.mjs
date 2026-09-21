// Prüft Theme-Start, Systemwechsel und Persistenzfehler ohne Zugriff auf echte Finanzprofile.
import test from 'node:test';
import assert from 'node:assert/strict';
import fs from 'node:fs';
import vm from 'node:vm';
import ts from 'typescript';
const source = fs.readFileSync(new URL('../src/shared/theme/appearance.ts', import.meta.url), 'utf8');
const compiled = ts.transpileModule(source, { compilerOptions: { module: ts.ModuleKind.CommonJS, target: ts.ScriptTarget.ES2020 } }).outputText;
function setup({ native = true, stored = 'system', dark = true, fail = false } = {}) {
  const calls = [], listeners = {}, events = [];
  const media = { matches: dark, addEventListener: (_, fn) => { listeners.system = fn; } };
  const root = { dataset: {}, style: {} };
  const sandbox = { exports: {}, console, Event: class { constructor(type) { this.type = type; } },
    document: { documentElement: root },
    window: { matchMedia: () => media, addEventListener: (name, fn) => { listeners[name] = fn; }, dispatchEvent: event => events.push(event.type) },
    localStorage: { getItem: () => stored, setItem: (_, value) => { if (fail) throw Error('storage unavailable'); stored = value; } },
    require: () => ({ isTauri: () => native, invoke: async (command, args) => {
      calls.push({ command, args });
      if (command === 'load_appearance') return stored;
      if (command === 'save_appearance') { if (fail) throw Error('disk full'); stored = args.appearance; }
    } }),
  };
  vm.runInNewContext(compiled, sandbox);
  return { api: sandbox.exports, root, media, listeners, events, calls };
}
test('startup respects explicit saved mode before showing native window', async () => {
  const s = setup({ stored: 'light' });
  await s.api.initializeAppearance();
  assert.equal(s.root.dataset.theme, 'light');
  assert.equal(s.root.dataset.appearance, 'light');
  assert.equal(s.root.style.colorScheme, 'light');
  assert.equal(s.calls.length, 1);
  s.api.showApp();
  assert.equal(s.calls.at(-1).command, 'show_themed_window');
  assert.equal(s.calls.at(-1).args.dark, false);
});
test('cosmic mode uses the dark palette and keeps its distinct background preference', async () => {
  const s = setup({ stored: 'cosmic', dark: false });
  await s.api.initializeAppearance();
  assert.equal(s.api.getAppearance(), 'cosmic');
  assert.equal(s.root.dataset.theme, 'dark');
  assert.equal(s.root.dataset.appearance, 'cosmic');
  s.api.showApp();
  assert.equal(s.calls.at(-1).args.dark, true);
});
test('hidden webview reveals after theme and committed render without animation frames', async () => {
  const entry = fs.readFileSync(new URL('../src/main.tsx', import.meta.url), 'utf8');
  const js = ts.transpileModule(entry, { compilerOptions: { module: ts.ModuleKind.CommonJS, target: ts.ScriptTarget.ES2020, jsx: ts.JsxEmit.React, esModuleInterop: false } }).outputText;
  const calls = [];
  let pendingRender;
  await vm.runInNewContext(`(async () => { ${js} })()`, {
    exports: {}, document: { getElementById: () => ({}) },
    requestAnimationFrame: () => {}, // A hidden WebView never delivers a frame.
    require: name => {
      if (name === 'react') return { default: { createElement: () => ({}), StrictMode: {} } };
      if (name === 'react-dom/client') return { default: { createRoot: () => ({ render: () => { pendingRender = () => calls.push('commit'); } }) } };
      if (name === 'react-dom') return { flushSync: callback => { callback(); pendingRender(); } };
      if (name === './App') return { default: {} };
      return { initializeAppearance: async () => { await Promise.resolve(); calls.push('theme'); }, showApp: () => calls.push('show') };
    },
  });
  assert.deepEqual(calls, ['theme', 'commit', 'show']);
});
test('system changes follow OS only when selected', async () => {
  const s = setup(); await s.api.initializeAppearance();
  assert.equal(s.root.dataset.theme, 'dark');
  s.media.matches = false; s.listeners.system();
  assert.equal(s.root.dataset.theme, 'light');
  await s.api.saveAppearance('dark'); s.listeners.system();
  assert.equal(s.root.dataset.theme, 'dark');
  await s.api.saveAppearance('system');
  assert.equal(s.root.dataset.theme, 'light');
});
test('failed persistence keeps previous preference and reports an error', async () => {
  for (const native of [true, false]) {
    const s = setup({ stored: 'light', native, fail: true }); await s.api.initializeAppearance();
    await assert.rejects(s.api.saveAppearance('dark'));
    assert.equal(s.api.getAppearance(), 'light');
    assert.equal(s.root.dataset.theme, 'light');
  }
});
test('invalid preference falls back to system; browser storage updates synchronize', async () => {
  const s = setup({ stored: 'invalid', native: false }); await s.api.initializeAppearance();
  assert.equal(s.api.getAppearance(), 'system');
  s.listeners.storage({ key: 'finanzblick.appearance', newValue: 'light' });
  assert.equal(s.root.dataset.theme, 'light');
  s.listeners.storage({ key: 'finanzblick.appearance', newValue: 'cosmic' });
  assert.equal(s.root.dataset.theme, 'dark');
  assert.equal(s.root.dataset.appearance, 'cosmic');
  s.listeners.storage({ key: null, newValue: null });
  assert.equal(s.root.dataset.theme, 'dark');
});

test('desktop window opens wide enough for the complete monthly comparison', () => {
  const config = JSON.parse(fs.readFileSync(new URL('../src-tauri/tauri.conf.json', import.meta.url), 'utf8'));
  const mainWindow = config.app.windows[0];
  assert.equal(mainWindow.width, 1520);
  assert.equal(mainWindow.height, 900);
  assert.ok(mainWindow.minWidth <= 920);
});
test('appearance strings have all translations', () => {
  const messages = JSON.parse(fs.readFileSync(new URL('../src/translations.json', import.meta.url), 'utf8'));
  const ui = fs.readFileSync(new URL('../src/shared/theme/AppearanceSettings.tsx', import.meta.url), 'utf8');
  for (const match of ui.matchAll(/\bt\("([^"]+)"\)/g)) assert.equal(messages[match[1]]?.length, 3, match[1]);
});
test('cosmic appearance is exposed in the selector and styled with the bundled motif', () => {
  const ui = fs.readFileSync(new URL('../src/shared/theme/AppearanceSettings.tsx', import.meta.url), 'utf8');
  const overview = fs.readFileSync(new URL('../src/features/overview/Overview.tsx', import.meta.url), 'utf8');
  const css = fs.readFileSync(new URL('../src/styles/theme.css', import.meta.url), 'utf8');
  assert.match(ui, /<option value="cosmic">/);
  assert.match(overview, /appearance === "cosmic" \? "Dein Vermögen – astronomisch"/);
  assert.match(css, /data-appearance="cosmic"/);
  assert.match(css, /url\("\/saldonaut-login\.png"\)/);
});
