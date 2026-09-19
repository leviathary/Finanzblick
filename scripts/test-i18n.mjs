// Prüft Übersetzungen, Platzhalter und sprachabhängige Formatierung.

import test from 'node:test';
import assert from 'node:assert/strict';
import fs from 'node:fs';
import vm from 'node:vm';
import ts from 'typescript';

const messages = JSON.parse(fs.readFileSync(new URL('../src/translations.json', import.meta.url), 'utf8'));
const source = fs.readFileSync(new URL('../src/i18n.ts', import.meta.url), 'utf8');
const compiled = ts.transpileModule(source, { compilerOptions: { module: ts.ModuleKind.CommonJS, target: ts.ScriptTarget.ES2020, esModuleInterop: true } });
const sandbox = { exports: {}, require: () => messages };
vm.runInNewContext(compiled.outputText, sandbox);
const { t, tr, setLanguage, setRegion, categoryName, locale } = sandbox.exports;

test('transfer management labels and actions have translations', () => {
  setLanguage('en');
  for (const file of ['transactions/TransactionActions.tsx', 'transactions/TransferManagement.tsx', 'transactions/SettlementRuleDialog.tsx', 'transactions/SettlementRules.tsx', 'cards/CreditCards.tsx', 'cards/CardAccountStatus.tsx', 'cards/setup/CardSetupWizard.tsx', 'cards/setup/PatternTable.tsx']) {
    const code = fs.readFileSync(new URL('../src/features/' + file, import.meta.url), 'utf8');
    for (const match of code.matchAll(/\bt\("([^"]+)"\)/g)) {
      assert.ok(messages[match[1]], match[1]);
    }
  }
  assert.equal(t('Alle Transaktionen'), 'All transactions');
  assert.equal(t('Wiederherstellen'), 'Restore');
  assert.equal(t('Umbuchungen & Ausgleiche'), 'Transfers & settlements');
  setLanguage('de');
});

test('settlement actions and notices are available in English', () => {
  setLanguage('en');
  assert.equal(t('Als Rechnungsausgleich markieren'), 'Mark as statement settlement');
  assert.equal(t('Rechnungsausgleich aufheben'), 'Unmark statement settlement');
  for (const key of Object.keys(messages).filter(key => /Rechnungsausgleich|Rechnungsausgleiche/.test(key))) {
    assert.notEqual(t(key), key);
  }
  setLanguage('de');
});

test('unlock controls and security messages are translated', () => {
  const keys = ['Anmelden', 'Persönliche Finanzen', 'Lokale Daten prüfen …', 'Passwort anzeigen', 'Passwort verbergen',
    'Caps Lock ist aktiviert', 'Aus Backup wiederherstellen', 'Zurück zur Anmeldung', 'Sicherungsdatei auswählen …', 'Wechseln'];
  for (const language of ['en', 'fr', 'it']) {
    setLanguage(language);
    for (const key of keys) assert.notEqual(t(key), key);
  }
  setLanguage('de');
});

test('financial profile labels are localized consistently', () => {
  for (const [language, singular, plural, create] of [
    ['de', 'Finanzprofil', 'Finanzprofile', 'Neues Finanzprofil'],
    ['en', 'Financial profile', 'Financial profiles', 'New financial profile'],
    ['fr', 'Profil financier', 'Profils financiers', 'Nouveau profil financier'],
    ['it', 'Profilo finanziario', 'Profili finanziari', 'Nuovo profilo finanziario'],
  ]) {
    setLanguage(language);
    assert.equal(t('Finanzprofil'), singular);
    assert.equal(t('Finanzprofile'), plural);
    assert.equal(t('Neues Finanzprofil'), create);
    assert.equal(t('Mein persönliches Profil'), 'Mein persönliches Profil');
  }
  setLanguage('de');
});

test('translations retain every interpolation and provide all three languages', () => {
  const placeholders = text => [...text.matchAll(/\{\d+\}/g)].map(match => match[0]).sort();
  for (const [key, values] of Object.entries(messages)) {
    assert.equal(values.length, 3, key);
    for (const value of values) {
      assert.ok(value.trim().length, key);
      assert.deepEqual(placeholders(value), placeholders(key), key);
    }
  }
});

test('language and region independently affect messages and locale without translating personal labels', () => {
  setRegion('CH');
  for (const [language, heading, expectedLocale] of [['de','Einstellungen','de-CH'], ['en','Settings','en-CH'], ['fr','Paramètres','fr-CH'], ['it','Impostazioni','it-CH']]) {
    setLanguage(language);
    assert.equal(t('Einstellungen'), heading);
    assert.equal(locale(), expectedLocale);
    assert.equal(categoryName('custom_42', 'Wohnen'), 'Wohnen');
    assert.equal(categoryName('housing', 'Meine Wohnung'), 'Meine Wohnung');
  }
  setRegion('DE');
  assert.equal(locale(), 'it-DE');
  setLanguage('en');
  assert.equal(categoryName('housing', 'Wohnen'), 'Housing');
  assert.equal(tr`${3} von ${5} Importen`, '3 of 5 imports');
  assert.equal(t('A personal bank description'), 'A personal bank description');
});
