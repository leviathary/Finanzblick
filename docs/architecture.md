# Modularchitektur

Verbindliche UI-Vorgaben: [Design-Leitfaden](design-system.md).

## Leitlinien

- Ein Rust-Crate und ein Frontend-Package mit fachlichen Modulgrenzen. Separate
  Cargo-/npm-Pakete erst bei einer tatsächlich unabhängig nutzbaren Schnittstelle.
- Refactorings verändern weder bestehende Daten noch Command-Namen oder Finanzregeln.
- Keine Bestandsdatenmigrationen für persönliche Konten oder Entwicklungsdaten.
- Plattformintegration bleibt Windows/macOS-neutral. Ein macOS-Build ist separat zu prüfen.
- Neue Abläufe gehören nicht in Repository- oder Basiskomponenten.
- Produktionsmodule importieren ihre Abhängigkeiten explizit; Unit-Tests dürfen
  über `use super::*` auf die geprüfte Implementierung zugreifen.

## Backend

```text
src-tauri/src/
  commands/                Dünne Tauri-Eingänge, stabile Command-Namen
    accounts.rs            Kontenverwaltung
    appearance.rs          Gerätebezogenes Farbschema und native Fensterdarstellung
    imports.rs             Importhistorie, Freigabe, Mappingprofile
    transactions.rs        Buchungen und manuelle Auswertungsmarkierungen
    positions.rs           Manuelle Positionen
    position_history.rs    Historische Bewertungen
    reporting.rs           Übersichten und Vermögensprojektionen
    database.rs            Profile, Sperren, Einstellungen, Backups, Demo
    cards.rs               Karten-Setup
    categories.rs          Kategorien und Branchenregeln
    rules.rs               Ausgleichsregeln
    taxes.rs               Steuerhistorie
    market_data.rs         Kursaktualisierung
    assistant.rs           Finanzassistent
  application/
    cards.rs, cards/       Atomarer Karten-Setup-Workflow
    imports.rs             Freigabe normalisierter Importe
    taxes.rs               Steuerimport und manuelle Jahreswerte
    market_data.rs         Kursaktualisierung und Caches
    benchmarks.rs          Expliziter Indexabruf ohne Bewertungs- oder Kontenschreibzugriffe
    assistant/             Anmeldung, Freigabe und Chat-Workflow
  domain/
    banking/               Kontenregeln, Kartenklassifizierung, Transfertypen
    securities/quotes.rs   Kurswerte und historische Abdeckungsregeln
    taxes.rs               Steuer-Datentypen und Plausibilitätsregeln
    assistant.rs           Anfragevalidierung und Datenschutzregeln
  storage/
    banking/               Konten, Transaktionen, Karten und Auswertungsflags
    imports/               Historie, Mappingprofile, Identitäten, Dubletten, Speicherung
    securities/            Positionen, Bewertungsverläufe und gespeicherte Marktpreise
    taxes/snapshots.rs     Jahressteuerwerte und Vermögensaufteilung
    rules/                 Händlerregeln, Kategorisierung und Ausgleichsregeln
    reporting/             SQL-Projektionen für Übersicht, Konsum und Vermögen
    assistant/             Begrenzte Finanzprojektionen und Bereinigung von Zugangsdaten
    database/              Verbindung, Sitzung, Profile, Backups, Schema und Fehler
    categories.rs          Kategorienpersistenz
  importers/
    formats/               CSV, Excel, PDF, MT940, camt.053/.054, Tabellenmapping
    providers/             Bankabhängige Interpretation
    taxes/zurich.rs        Parser für Zürcher Steuererklärungen
    pipeline.rs            Datei lesen und normalisieren, ohne DB-Zugriff
  infrastructure/
    appearance.rs          Lokale Darstellung, unabhängig von Finanzprofilen
    chat_runtime.rs        Plattformneutraler Prozess- und Protokolladapter
    market_data.rs         Externe Kursanbieter und Wechselkurse
```

### Verantwortlichkeiten

Commands nehmen Parameter entgegen und delegieren. Mehrstufige Abläufe liegen
in `application`; einfache Lese-/Schreibvorgänge können unmittelbar an ein
Repository delegiert werden. In `storage` gibt es keine Tauri-Commands mehr.
Die bestehenden 66 registrierten Command-Namen bleiben unverändert.

Karten-Setup: Command → Anwendungsschicht → Repositories.
Die Anwendungsschicht hält eine gemeinsame Datenbanktransaktion für Regeln
beider Kontoseiten und explizite Gutschriftentscheidungen. Fehler rollen
sämtliche Änderungen zurück.

Importer liefern normalisierte DTOs und kennen keine Datenbank. Der Abgleich
gegen bereits gespeicherte Referenzen gehört zur Importpersistenz, nicht zum
Formatparser. `identity.rs` erzeugt Fingerabdrücke; `deduplication.rs`
gleicht sie gegen gespeicherte Buchungen ab.

Die nachträgliche Bestandsprüfung liegt im Banking-Repository `duplicates.rs`.
Sie liest Buchungen und Importquellen, gruppiert Verdachtsfälle und persistiert
nur explizite Prüfentscheidungen. Buchungen werden dabei nicht gelöscht:
bestätigte Dubletten werden reversibel aus Auswertungen ausgeblendet, bestätigte
Mehrfachzahlungen als geprüfte Transaktionspaare gespeichert.

Repository-Anfragen und Ergebnisprojektionen liegen beim jeweiligen Fachbereich
in `models.rs`, nicht mehr in einer zentralen Sammeldatei.
Die Domäne bleibt unabhängig von SQL und Tauri-Zustand.

`storage/banking/reporting_flags.rs` schreibt manuelle Markierungen.
`storage/reporting/consumption.rs` stellt die Konsumprojektion bereit.
Kategorisierungsabfragen liegen in `storage/rules/categorization.rs`;
die bisherige Initialisierung ruft diese unverändert auf. Das Schema enthält
nicht mehr deren Implementierung.

Steuerwerte bilden einen eigenen Fachbereich: Auch Immobilien und Schulden
gehören dazu, nicht nur Wertschriftendepots. Steuer-PDF-Erkennung, Validierung,
Workflow und SQL sind getrennt. Vorhandene Parser- und Speicherungstests bleiben erhalten.

### Datenbank und Sicherheit

`database/mod.rs` verbindet Module und Exporte, implementiert aber keine
Datenbankoperationen. Der Bereich enthält:

- `handle.rs`: gemeinsamer Storage-Handle.
- `connection.rs`: SQLCipher-Verbindung, Öffnen und Zugriffsschutz.
- `session.rs`: flüchtiges Passwort, Ablauf und Sitzungszustand.
- `settings.rs`: Lesen und Validieren der Einstellungen.
- `profiles.rs`: Finanzprofile, Kopien und Anonymisierung.
- `backups.rs`: verschlüsselte Sicherung und Wiederherstellung.
- `demo.rs`: synthetische Demodaten.
- `schema.rs`: bestehende Schema-Initialisierung.
- `queries.rs`: gemeinsame Zählabfragen.
- `errors.rs`: gemeinsame Fehlerübersetzung.

Der Sitzungs-Lesezugriff bleibt während jeder geöffneten Verbindung erhalten:
Sperren wartet weiterhin auf laufende Zugriffe. Sitzungskennung und Generation
können von der Anwendungsschicht geprüft werden; Passwort und interner
Sitzungszustand werden dadurch nicht öffentlich.

## Frontend

- `features/auth/`: Anmeldung und zugehörige Styles.
- `features/accounts/`: Kontenübersicht und eigenständiger Konteneditor.
- `features/positions/`: Positionsverwaltung mit eigenem Zustand, DTOs,
  Anzeigehelfern und typisierter Backend-API.
- `features/cards/`: Kartenansicht, typisierte API und Präsentation.
- `features/cards/setup/`: vierstufiger Workflow und Entwurfsvalidierung.
- `features/assets/`: Vermögensansicht und deren Diagrammkomponenten.
- `shared/charts/TimelineChart.tsx`: gemeinsam nutzbares Zeitverlaufsdiagramm.
- `domain/finance.ts`: gemeinsamer Kontotyp; ungenutzte, abweichende
  Parallelmodelle mit String-IDs wurden entfernt.

Featurebezogene `types.ts`, `api.ts` und `presentation.ts` trennen
Datenverträge, Backend-Aufrufe und Anzeigehelfer von den Komponenten.
Die Kontenübersicht besitzt nicht mehr den Positionsformular-Zustand.
Die Positionsansicht wird pro Konto neu montiert und entfernt ihre
Event-Listener beim Verlassen.

## Tests und Erweiterungsregeln

- Rust-Unit-Tests bleiben beim verantwortlichen Modul.
- Karten-Workflowtests liegen in `application/cards/tests.rs`.
- Gemeinsame Storage-Regressionstests liegen in `storage/tests.rs`.
- Externe Rust-Integrationstests gehören nach `src-tauri/tests/`.
- `scripts/test-card-architecture.mjs` prüft Command-Payloads,
  Modulgrenzen, DB-freie Parser/Domäne und getrennte Frontend-Workflows.
- Keine Migration ist für diese reine Code-Neuordnung erforderlich.

## Bewusst verbleibende Grenzen

Dies ist eine schrittweise Modularisierung, keine Neuschreibung.
Einige bestehende Repository-Operationen enthalten weiterhin lokale Validierung
und atomare Schreibabläufe. Diese werden bei fachlichen Änderungen weiter in
Domänenregeln und Anwendungsschritte getrennt, nicht nur künstlich weitergereicht.

Der Chat-Workflow verwendet für Ereignisse, Zustandsverwaltung und Hintergrundjobs
weiterhin Tauri. Das ist in `application` gekapselt, nicht in der Domäne oder
den SQL-Projektionen. `import_files.rs` bleibt der bestehende Datei-/Dialogadapter.

`App.css` erhält die bisherige Kaskadenreihenfolge. Die verbleibenden gemeinsamen
Styles in `styles/application.css` und `styles/management.css` sind noch nicht
vollständig featureweise aufgeteilt. Auch große Darstellungs-Komponenten können
bei kommenden Änderungen weiter zerlegt werden; neue Workflows dürfen daraus
keine zusätzlichen Sammelmodule machen.
