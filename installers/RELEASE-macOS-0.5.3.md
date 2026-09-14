# Finanzblick 0.5.3 für macOS

## Dateien für GitHub Releases

- `Finanzblick_0.5.3_aarch64.dmg` – App für Apple Silicon (M-Chips)
- `Finanzblick_0.5.3_aarch64.dmg.sha256` – SHA-256-Prüfsumme

Beide Dateien liegen im Repository unter `installers` und können zusätzlich
als Assets an ein GitHub-Release angehängt werden.

## Installation

DMG öffnen, den MIT-Lizenzhinweis bestätigen und Finanzblick auf Applications
ziehen. Anschliessend aus dem Programme-Ordner starten. Die Demo ist enthalten;
das Passwort für das Demo-Profil lautet `demo1234`.

Die App ist ad-hoc signiert und nicht von Apple notarisiert. Falls macOS den
Start nach dem Download blockiert, unter Systemeinstellungen → Datenschutz &
Sicherheit → Dennoch öffnen freigeben. Eine Developer-ID-Signatur mit
Notarisierung ist in dieser Testversion nicht enthalten.

## Prüfung am 14.09.2026

- Nativer Release-Build auf macOS 26.6.2, Apple Silicon erfolgreich.
- Installation und Funktion auf dem Mac zusätzlich vom Anwender bestätigt.
- 13 Frontend-Tests und 62 Rust-Tests bestanden; ein Demo-Export-Hilfstest
  ist absichtlich ignoriert.
- DMG-Integrität mit `hdiutil verify` erfolgreich geprüft.
- App aus dem DMG kopiert; `codesign --verify --deep --strict` erfolgreich.
- Laufende App im Programme-Ordner mit sichtbarer Demo-Vermögensansicht geprüft;
  ihre ausführbare Datei ist SHA-256-identisch mit der Datei im DMG.
- Lizenzhinweise für das macOS-Ziel enthalten; Prüfung der ausführbaren Datei
  auf konfigurierte persönliche Build-Pfade erfolgreich.

Intel-Macs, ältere macOS-Versionen und der Gatekeeper-Ablauf einer frischen
Installation nach einem GitHub-Download sind nicht getestet. Dieser Starttest
ist keine vollständige Prüfung aller App-Funktionen.
