# Finanzblick

Finanzblick ist eine lokale Desktop-App zur Auswertung von Konten, Depots,
Vorsorgevermögen und Transaktionen. Bankauszüge werden importiert, vereinheitlicht
und in einer verschlüsselten Datenbank gespeichert.

## Funktionen

- Konten und selbst verwaltete Vermögenswerte gemeinsam darstellen
- Excel-, CSV-, PDF- und MT940-Dateien per Dateidialog oder Drag-and-drop importieren
- mehrere Dateien gesammelt prüfen und importieren
- Salden, Vermögensentwicklung und kumulierten Geldfluss visualisieren
- Transaktionen durchsuchen und kategorisieren
- Kreditkartenbuchungen als Detail behandeln, ohne Zahlungen doppelt zu zählen
- Aktien und Kryptowährungen mit historischen Marktpreisen bewerten
- mehrere unabhängige Datenbanken verwalten, kopieren und anonymisieren
- Deutsch, Englisch, Französisch und Italienisch sowie regionale Zahlen- und
  Datumsformate verwenden

## Datenschutz

Konten, Buchungen und Auswertungen bleiben auf dem eigenen Gerät. Die lokale
SQLite-Datenbank wird mit SQLCipher verschlüsselt und durch ein selbst gewähltes
Passwort geschützt. Es gibt weder Benutzerkonto noch Cloud-Synchronisierung oder
Passwort-Reset.

Für automatische Bewertungen fragt die App ausschließlich Wertpapierkennungen,
Zeiträume und Währungspaare bei den konfigurierten Marktdatenanbietern ab. Bankauszüge,
Buchungstexte und Kontostände werden nicht übertragen. Optionale API-Schlüssel
liegen verschlüsselt in der Datenbank.

Die App sperrt sich nach der eingestellten Inaktivitätszeit und beim Minimieren.
Ohne Passwort kann eine verschlüsselte Datenbank nicht wiederhergestellt werden;
eine zusätzliche Datenträgerverschlüsselung und eigene Sicherungskopien bleiben
empfohlen.

## Unterstützte Importe

Der Import vereinheitlicht unterschiedliche Bankformate und zeigt die erkannten
Buchungen vor dem Speichern zur Kontrolle an. Salden und Summen werden soweit im
Quellformat vorhanden gegengeprüft. Datei-Fingerabdrücke und ein zusätzlicher
Buchungsvergleich verhindern doppelte Importe bei überlappenden Auszügen.

Aktuell bestehen Parser unter anderem für:

- UBS-Kontoauszüge und UBS-Mastercard-Abrechnungen
- Swissquote-Kontoauszüge
- Migros Bank
- Raiffeisen
- Generali
- MT940 sowie allgemeine CSV- und Excel-Formate

Unter [fixtures/bank-statements](fixtures/bank-statements) liegen ausschließlich
frei erfundene Testauszüge. Echte Finanzdaten gehören nicht in das Repository.

## Technik

- [Tauri 2](https://tauri.app/) für die Desktop-Anwendung
- React und TypeScript für die Benutzeroberfläche
- Rust für Import, Geschäftslogik und lokale Systemfunktionen
- SQLCipher für die verschlüsselte SQLite-Datenbank

Die Anwendung wird derzeit unter Windows entwickelt. Die Architektur ist für
Windows und macOS ausgelegt; macOS-Kompatibilität ist noch nicht durch einen
Build und praktische Tests bestätigt.

## Entwicklung

Vorausgesetzt werden Node.js 22 oder neuer, Rust mit Cargo sowie die für Tauri
benötigten Systemabhängigkeiten. Unter Windows werden zusätzlich WebView2, die
Microsoft C++ Build Tools und ein natives Perl für den OpenSSL-Build benötigt.

```sh
npm install
npm run tauri dev
```

`npm run dev` startet nur die Weboberfläche. Datenbankzugriff und native Funktionen
stehen in der vollständigen Tauri-App zur Verfügung.

### Tests und Build

```sh
npm test
npm run build
cargo test --manifest-path src-tauri/Cargo.toml --lib
```

Alle eingecheckten Tests verwenden synthetische Daten.

## Projektstruktur

```text
src/                         React-Oberfläche
src/features/                Fachbereiche der Oberfläche
src-tauri/src/importers/     Parser und Importadapter
src-tauri/src/storage/       verschlüsselte Persistenz und Auswertungen
fixtures/bank-statements/    synthetische Testauszüge
```
