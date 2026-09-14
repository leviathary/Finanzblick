# Finanzblick

Deine Finanzen auf einen Blick: eine lokale Desktop-App für Konten, Depots,
Vermögen und Ausgaben. Ohne Benutzerkonto oder Cloud-Synchronisierung.

## Windows installieren

[Windows-Installer 0.5.3 herunterladen (64 Bit)](https://github.com/leviathary/Finanzblick/raw/refs/heads/main/installers/Finanzblick_0.5.3_x64-setup.exe)

Die heruntergeladene Datei ausführen. Ein Windows-Entwicklermodus ist nicht nötig.
Der Installer ist nicht signiert; Windows kann deshalb eine Sicherheitswarnung anzeigen.
Frühe Testversion: vor der Nutzung mit echten Daten ein Backup erstellen.

Die Demo ist bereits enthalten: **Finanzprofil → Demo-Daten**, Passwort **`demo1234`**.
Sie wird beim ersten Öffnen lokal erzeugt; es ist kein zusätzlicher Download nötig.
Falls WebView2 fehlt, benötigt dessen Einrichtung eine Internetverbindung.

## Einblick

Die Screenshots zeigen ausschliesslich fiktive Demo-Daten.

![Vermögensübersicht mit Konten und Anbietern](docs/screenshots/uebersicht.png)

![Vermögensentwicklung über acht Jahre](docs/screenshots/vermoegensentwicklung.png)

## Funktionen

- Bankauszüge aus Excel, CSV, PDF und MT940 importieren und vor dem Speichern prüfen
- Konten, Aktien, ETFs, Kryptowährungen und Vorsorgevermögen gemeinsam auswerten
- Vermögensentwicklung, Geldfluss und Steuerhistorie visualisieren
- Depotpositionen einzeln oder gemeinsam im Verlauf anzeigen; bei einer Position zwischen Wert und Kurs wechseln
- Transaktionen kategorisieren und Kreditkartenzahlungen ohne Doppelzählung erfassen
- Unabhängige Finanzprofile erstellen, kopieren, anonymisieren und sichern
- Deutsch, Englisch, Französisch und Italienisch mit regionalen Zahlenformaten

Importvorlagen gibt es unter anderem für UBS inklusive Mastercard, Swissquote,
Migros Bank, Raiffeisen und Generali. [Testauszüge](fixtures/bank-statements)
enthalten nur erfundene Daten.

## Einfach ausprobieren

In der Auswahl **Finanzprofil → Demo-Daten** findest du Banken, Aktien und ETFs,
kategorisierte Kreditkartenbuchungen und acht Jahre simulierter Historie.
Die Demo funktioniert offline und wird beim nächsten Öffnen wiederverwendet.

**Passwort für das Demo-Profil: `demo1234`** (alles kleingeschrieben, ohne Leerzeichen).

Für persönliche Daten wählst du **Neues Finanzprofil** und ein eigenes Passwort.
Demo-Kurse sind keine historischen Börsendaten oder Anlageempfehlungen.

## Datenschutz und Backup

Finanzdaten bleiben auf deinem Gerät und sind mit SQLCipher verschlüsselt.
Für automatische Bewertungen werden Wertpapierkennungen, Zeiträume und
Währungspaare an Marktdatenanbieter übermittelt – keine Kontostände oder Buchungen.

Unter **Daten & Sicherheit** kannst du dein Passwort ändern und verschlüsselte
Backups per Dateidialog erstellen oder wiederherstellen. Eine Wiederherstellung
legt ein zusätzliches Finanzprofil an; bestehende Daten bleiben erhalten.

**Wichtig:** Es gibt keinen Passwort-Reset. Backups benötigen das Passwort vom
Zeitpunkt der Sicherung. Bewahre sie möglichst auf einem anderen Datenträger auf.

## Lizenz

Der eigene Quellcode steht unter der [MIT-Lizenz](LICENSE).
[Abhängigkeiten](DEPENDENCIES.md) behalten ihre eigenen Lizenzen; deren Texte und
Hinweise stehen in [THIRD_PARTY_LICENSES.txt](THIRD_PARTY_LICENSES.txt) und werden
im Installer mitgeliefert. Banklogos werden nicht mitgeliefert; stattdessen
erscheinen Kürzel. Eigene Logos lassen sich lokal im Finanzprofil hochladen.

## Entwicklung

Tauri 2 · React · TypeScript · Rust · SQLCipher

Voraussetzungen: Node.js 22+, Rust/Cargo und die Tauri-Systemabhängigkeiten.
Unter Windows zusätzlich WebView2, Microsoft C++ Build Tools und natives Perl
für den OpenSSL-Build. macOS ist architektonisch vorgesehen, aber noch nicht
durch Build und praktische Tests verifiziert.

```sh
npm install
npm run tauri dev
```

`npm run dev` startet nur die Weboberfläche ohne native Funktionen.

Tests und Build:

```sh
npm test
npm run build
cargo test --manifest-path src-tauri/Cargo.toml --lib
```

Alle eingecheckten Tests verwenden synthetische Daten. Echte Finanzdaten gehören
nicht ins Repository.

Veröffentlichungs-Build: `npm run release -- --bundles nsis` (Windows).
Das plattformneutrale Build-Skript sammelt Lizenzhinweise, neutralisiert lokale
Rust-Build-Pfade und prüft die EXE vor der Weitergabe. Bei anderen Zielplattformen
die passenden Tauri-Bundle-Argumente verwenden; macOS ist noch nicht getestet.
Für native Bibliotheken wie OpenSSL muss auch `CARGO_TARGET_DIR` auf ein neutrales
Build-Verzeichnis ohne persönlichen Benutzernamen zeigen. Ein fehlgeschlagener
Pfadcheck bedeutet, dass der erzeugte Installer nicht veröffentlicht werden darf.
Historische Installer und Logos bleiben bis zu einer gesondert freigegebenen
Bereinigung in älteren Git-Commits erreichbar.
