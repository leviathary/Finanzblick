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

## macOS installieren

[macOS-DMG 0.5.3 herunterladen (Apple Silicon)](https://github.com/leviathary/Finanzblick/raw/refs/heads/main/installers/Finanzblick_0.5.3_aarch64.dmg)

Für Macs mit Apple Silicon (M-Chips). DMG öffnen und **Finanzblick** auf **Applications**
ziehen, danach die App aus dem Programme-Ordner starten.

Diese Testversion ist ad-hoc signiert, aber nicht von Apple notarisiert.
Bei einer Sperre nach dem Download lässt sich der Start unter
**Systemeinstellungen → Datenschutz & Sicherheit → Dennoch öffnen** freigeben.
Eine Intel-Version ist in diesem DMG nicht enthalten.

Build und Start mit sichtbaren Demo-Daten wurden am 14.09.2026 auf macOS 26.6.2
(Apple Silicon) geprüft. Die 13 Frontend- und 62 aktiven Rust-Tests bestehen.
Installation und Funktion wurden zusätzlich vom Anwender bestätigt.
Andere macOS-Versionen und eine frische Installation nach einem GitHub-Download
sind noch nicht getestet.

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
für den OpenSSL-Build. Unter macOS werden die Xcode Command Line Tools
(`xcode-select --install`) und Perl für den OpenSSL-Build benötigt.

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
Für ein natives macOS-DMG auf einem Mac:

```sh
cargo fetch --manifest-path src-tauri/Cargo.toml --locked
CARGO_TARGET_DIR=/tmp/finanzblick-build npm run release:mac
```

Das DMG liegt danach unter `/tmp/finanzblick-build/release/bundle/dmg/`.
Die Architektur entspricht dem Build-Mac: Apple Silicon erzeugt `aarch64`,
Intel erzeugt `x64`. Das DMG kann als Asset eines GitHub-Releases hochgeladen
werden. Der Build veröffentlicht nichts automatisch.

Das plattformneutrale Build-Skript sammelt Lizenzhinweise, neutralisiert lokale
Rust-Build-Pfade und prüft die ausführbare Datei vor der Weitergabe.
Die macOS-Konfiguration verwendet eine Ad-hoc-Signatur. Für eine von Apple
signierte und notarisierte Veröffentlichung sind ein Developer-ID-Zertifikat
und die entsprechende Tauri-Signierungs-/Notarisierungskonfiguration nötig;
siehe [Tauri macOS Code Signing](https://v2.tauri.app/distribute/sign/macos/).
Für native Bibliotheken wie OpenSSL muss auch `CARGO_TARGET_DIR` auf ein neutrales
Build-Verzeichnis ohne persönlichen Benutzernamen zeigen. Ein fehlgeschlagener
Pfadcheck bedeutet, dass der erzeugte Installer nicht veröffentlicht werden darf.
Hinweise zur Veröffentlichung und zum Umgang mit lokalen Daten stehen in
[SECURITY.md](SECURITY.md).
