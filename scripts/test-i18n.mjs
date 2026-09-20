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

test('unified import navigation and completion messages have translations', () => {
  for (const key of ['Dateien importieren', 'Importierte Dateien', 'Dateien des letzten Importvorgangs', 'Alle Importe anzeigen', 'Die importierten Dateien findest du im Reiter „Importierte Dateien“.']) {
    assert.equal(messages[key]?.length, 3, key);
    assert.ok(messages[key].every(value => value.length > 0), key);
  }
  const app = fs.readFileSync(new URL('../src/App.tsx', import.meta.url), 'utf8');
  assert.ok(!app.includes('t("Importverwaltung")'));
  assert.ok(app.includes('page === "imports" || page === "import-history"'));
  assert.ok(app.includes('"#import-history/tax" : "#import-history"'));
  const wizard = fs.readFileSync(new URL('../src/features/imports/ImportWizard.tsx', import.meta.url), 'utf8');
  assert.ok(wizard.includes('summary.ids.push(change.result.importId)'));
  assert.ok(wizard.includes('#import-history?ids='));
});

test('transfer management labels and actions have translations', () => {
  setLanguage('en');
  for (const file of ['transactions/TransactionActions.tsx', 'transactions/TransferManagement.tsx', 'transactions/SettlementRuleDialog.tsx', 'transactions/TransferRules.tsx', 'transactions/TransferRuleEditor.tsx', 'cards/CreditCards.tsx', 'cards/CardAccountStatus.tsx', 'cards/setup/CardSetupWizard.tsx', 'cards/setup/PatternTable.tsx']) {
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

test('duplicate removal is confirmed, reversible and fully translated', () => {
  const actions = fs.readFileSync(new URL('../src/features/transactions/TransactionActions.tsx', import.meta.url), 'utf8');
  const dialog = fs.readFileSync(new URL('../src/features/transactions/DuplicateRemovalDialog.tsx', import.meta.url), 'utf8');
  const transactions = fs.readFileSync(new URL('../src/features/transactions/Transactions.tsx', import.meta.url), 'utf8');
  const history = fs.readFileSync(new URL('../src/features/imports/ImportHistory.tsx', import.meta.url), 'utf8');
  const audit = fs.readFileSync(new URL('../src/features/imports/DuplicateAudit.tsx', import.meta.url), 'utf8');
  const wizard = fs.readFileSync(new URL('../src/features/imports/ImportWizard.tsx', import.meta.url), 'utf8');
  for (const match of (actions + dialog + transactions + history + audit + wizard).matchAll(/\bt\("([^"\\]+)"\)/g)) {
    assert.equal(messages[match[1]]?.length, 3, match[1]);
  }
  assert.match(actions, /Als Duplikat entfernen …/);
  assert.match(dialog, /Die ursprüngliche Importspur bleibt erhalten/);
  assert.match(transactions, /invoke\("ignore_duplicate_transaction"/);
  assert.match(history, /invoke<number>\("restore_import_duplicates"/);
  assert.match(history, /ignoredDuplicateCount/);
  assert.match(audit, /audit_duplicate_transactions/);
  assert.match(audit, /dismiss_duplicate_candidate_group/);
  assert.match(wizard, /unresolvedDuplicateCount/);
  assert.match(wizard, /Als Duplikat überspringen/);
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

test('chat description-sharing notices are translated and visible in the preview', () => {
  for (const file of ['FinanceChat.tsx', 'SourcePreview.tsx']) {
    const code = fs.readFileSync(new URL('../src/features/chat/' + file, import.meta.url), 'utf8');
    for (const match of code.matchAll(/\bt\("([^"]+)"\)/g)) {
      assert.ok(messages[match[1]], match[1]);
    }
    assert.match(code, /Beschreibungen können persönliche Angaben enthalten/);
    assert.doesNotMatch(code, /Auch im Fearless-Modus werden keine Buchungstexte|Keine Buchungstexte oder Kontodaten/);
  }
});

test('anonymized-copy dialog strings are translated and profile actions do not mutate the original', () => {
  const dialog = fs.readFileSync(new URL('../src/features/settings/AnonymizedCopyDialog.tsx', import.meta.url), 'utf8');
  const picker = fs.readFileSync(new URL('../src/features/settings/DatabasePicker.tsx', import.meta.url), 'utf8');
  for (const match of (dialog + picker).matchAll(/\bt\("([^"]+)"\)/g)) {
    assert.ok(messages[match[1]], match[1]);
    assert.equal(messages[match[1]].length, 3);
  }
  assert.match(dialog, /invoke\("create_anonymized_copy"/);
  assert.match(dialog, /invoke\("copy_database"/);
  assert.doesNotMatch(picker, /role="tablist"|Aktuelles Finanzprofil verwalten/);
  assert.doesNotMatch(picker, /invoke\("anonymize_database/);
});

test('category inline actions are translated and do not scroll to a distant editor', () => {
  const code = fs.readFileSync(new URL('../src/features/categories/Categories.tsx', import.meta.url), 'utf8');
  const styles = fs.readFileSync(new URL('../src/styles/application.css', import.meta.url), 'utf8');
  const categoryStyles = fs.readFileSync(new URL('../src/features/categories/categories.css', import.meta.url), 'utf8');
  for (const match of code.matchAll(/\bt\("([^"]+)"\)/g)) {
    assert.ok(messages[match[1]], match[1]);
  }
  assert.doesNotMatch(code, /window.scrollTo/);
  assert.match(code, /edit===item.key && editor/);
  assert.match(code, /targetKey:target/);
  assert.match(code, /className="category-create-action"><button type="button"[^>]*className="primary-button"/);
  assert.match(code, /className="intro category-page-intro"/);
  assert.match(categoryStyles, /\.category-page-intro\s*\{\s*max-width: none;/);
  assert.match(styles, /\.managed-category\s*\{[\s\S]*gap: 12px;[\s\S]*padding: 8px 0;/);
  assert.match(styles, /\.managed-category \.category-color\s*\{[\s\S]*width: 9px;[\s\S]*height: 9px;[\s\S]*border-radius: 50%/);
  assert.match(styles, /\.transactions-page > \.industry-rules > label\s*\{[\s\S]*grid-template-columns:[\s\S]*padding: 8px 0;/);
  assert.match(code, /className="industry-rules-header"[\s\S]*t\("Kreditkarten-Kategorie"\)[\s\S]*t\("Finanzblick-Kategorie"\)/);
  assert.match(code, /aria-label=\{`\$\{t\("Finanzblick-Kategorie"\)\}: \$\{rule\.industry\}`\}/);
  assert.match(styles, /\.transactions-page > \.industry-rules > label select\s*\{[\s\S]*min-height: 44px/);
  assert.match(styles, /@media \(max-width: 700px\)[\s\S]*\.industry-rules-header\s*\{[\s\S]*display: none/);
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

test('tax history summary and chart labels are translated', () => {
  const code = fs.readFileSync(new URL('../src/features/tax-history/TaxHistory.tsx', import.meta.url), 'utf8');
  const staticKeys = [
    ...Array.from(code.matchAll(/\bt\("([^"]+)"\)/g), match => match[1]),
    ...Array.from(code.matchAll(/label: "([^"]+)"/g), match => match[1]),
  ];
  for (const key of staticKeys) {
    assert.equal(messages[key]?.length, 3, key);
  }
  for (const [language, year, years, debt] of [
    ['en', 'Tax year', 'Tax years', 'Debt'],
    ['fr', 'Année fiscale', 'Années fiscales', 'Dettes'],
    ['it', 'Anno fiscale', 'Anni fiscali', 'Debiti'],
  ]) {
    setLanguage(language);
    assert.equal(t('Steuerjahr'), year);
    assert.equal(t('Steuerjahre'), years);
    assert.equal(t('Schulden'), debt);
  }
  setLanguage('de');
});

test('manual positions use a localized return action', () => {
  for (const [language, label] of [
    ['en', 'Back to banks & accounts'],
    ['fr', 'Retour aux banques et comptes'],
    ['it', 'Torna a banche e conti'],
  ]) {
    setLanguage(language);
    assert.equal(t('Zurück zu Banken & Konten'), label);
  }
  setLanguage('de');
});

test('managed account status badges are localized', () => {
  for (const [language, excluded, archived] of [
    ['en', 'Excluded from total assets', 'Archived'],
    ['fr', 'Exclu du patrimoine total', 'Archivé'],
    ['it', 'Escluso dal patrimonio totale', 'Archiviato'],
  ]) {
    setLanguage(language);
    assert.equal(t('Nicht im Gesamtvermögen'), excluded);
    assert.equal(t('Archiviert'), archived);
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

test('local help is routed, searchable and fully translated', () => {
  const app = fs.readFileSync(new URL('../src/App.tsx', import.meta.url), 'utf8');
  const help = fs.readFileSync(new URL('../src/features/help/Help.tsx', import.meta.url), 'utf8');
  const content = fs.readFileSync(new URL('../src/features/help/helpContent.ts', import.meta.url), 'utf8');
  assert.match(app, /window\.location\.hash\.startsWith\("#help"\)/);
  assert.match(app, /href="#help\/start"/);
  assert.match(app, /page === "help" \? <Help/);
  assert.match(help, /type="search"/);
  assert.match(help, /aria-current=\{activeArticle\.id === article\.id \? "page"/);
  assert.match(help, /headingRef\.current\?\.focus\(\)/);
  assert.match(content, /t\("Datenschutz beim Import"\)/);
  assert.match(content, /Finanzblick kopiert die Quelldokumente nicht in dein Finanzprofil/);
  assert.match(content, /Gespeichert werden nur die von dir bestätigten Finanzdaten, der Dateiname und ein technischer Fingerabdruck/);
  for (const match of (help + content).matchAll(/\bt\("([^"\\]+)"\)/g)) {
    assert.equal(messages[match[1]]?.length, 3, match[1]);
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
