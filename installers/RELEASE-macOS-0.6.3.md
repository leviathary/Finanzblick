# Finanzblick 0.6.3 für macOS

[DMG für Apple Silicon herunterladen](https://github.com/leviathary/Finanzblick/releases/download/v0.6.3/Finanzblick_0.6.3_aarch64.dmg)
· [SHA-256-Prüfsumme](https://github.com/leviathary/Finanzblick/releases/download/v0.6.3/Finanzblick_0.6.3_aarch64.dmg.sha256)

Enthält die Änderungen von v0.6.3 (da0cf7d), die native Chat-Runtime und das
lokale deutsche Sprachmodell. Zusätzlich korrigiert dieser Mac-Build den
Fensterstart: Palette und erster React-Commit werden vor dem Anzeigen abgeschlossen,
ohne Animationsframes eines noch verborgenen WebViews abzuwarten.
Die Korrektur und ihr Regressionstest sind im zugehörigen Mac-Build-Commit auf main
enthalten; der bestehende Tag v0.6.3 und Windows-Installer bleiben unverändert.

## Installation

DMG öffnen und Finanzblick auf Applications ziehen. Eine laufende ältere App
vorher beenden, danach Finanzblick aus Programme starten. Demo-Passwort: `demo1234`.
Die App ist ad-hoc signiert, nicht von Apple notarisiert. Bei einer Sperre nach
dem Download: Systemeinstellungen → Datenschutz & Sicherheit → Dennoch öffnen.
Nur Apple Silicon; kein Intel-Build.

## Prüfung am 21.09.2026 auf macOS 27.0 (26A428)

- 78 Frontend-Tests und 140 Rust-Tests erfolgreich; ein Demo-Hilfstest absichtlich ignoriert.
- Produktionsbuild über `npm run release:mac`; Lizenzinventar für aarch64-apple-darwin
  erneuert, Prüfung auf persönliche Build-Pfade erfolgreich.
- DMG mit `hdiutil verify` geprüft, schreibgeschützt eingebunden und App daraus kopiert.
- Kopierte App mit strikter, rekursiver Signaturprüfung geprüft; Bundle-Version 0.6.3.
- Sichtbarer App-Start, Demo-Übersicht, Abmeldung und Tastaturfokus auf der Anmeldung geprüft.
- Finanzchat erreicht „Mit ChatGPT anmelden“; Hash der gebündelten Runtime und
  mitgelieferte Lizenzdatei geprüft. Lokaler Runtime-Test mit synthetischem Modellserver besteht.
- Die bereits installierte App wurde nicht ersetzt.

Keine vollständige UI-Abnahme: dunkle Darstellung, schmale Fenster und Zoom wurden
in diesem Build-Durchlauf nicht erneut geprüft. Eine echte ChatGPT-Anmeldung,
Mikrofon/Diktat, Intel-Macs, andere macOS-Versionen und eine frische Installation
mit Gatekeeper-Prüfung nach einem GitHub-Download sind nicht getestet.
