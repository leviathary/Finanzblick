# Finanzblick 0.5.7 für macOS

[DMG für Apple Silicon herunterladen](https://github.com/leviathary/Finanzblick/releases/download/v0.5.7/Finanzblick_0.5.7_aarch64.dmg)

Das DMG enthält Finanzblick, die native Chat-Runtime und das lokale deutsche
Sprachmodell. Wegen seiner Grösse liegt es als Asset beim GitHub-Release v0.5.7.
Die Datei `Finanzblick_0.5.7_aarch64.dmg.sha256` enthält die SHA-256-Prüfsumme.

## Installation

DMG öffnen, den MIT-Lizenzhinweis bestätigen und Finanzblick auf Applications
ziehen. Eine laufende ältere App vorher beenden. Danach Finanzblick aus dem
Programme-Ordner starten. Demo-Passwort: `demo1234`.

Die App ist ad-hoc signiert und nicht von Apple notarisiert. Falls macOS den
Start nach dem Download blockiert, unter **Systemeinstellungen → Datenschutz &
Sicherheit → Dennoch öffnen** freigeben.

## Änderungen gegenüber dem vorherigen Mac-Build

- Änderungen der Windows-Version 0.5.7 einschliesslich Finanzchat,
  Kreditkarten-Auswahl, Folgefragen und lokaler deutscher Spracheingabe.
- Mac-Korrektur für die Chat-Anmeldung: Das reguläre Betriebssystem-
  Benutzerverzeichnis bleibt erhalten, damit der Standardschlüsselbund gefunden
  wird. Finanzblick verwendet weiterhin ein eigenes `CODEX_HOME`, getrennte
  Anmeldungen pro Finanzprofil und eine Konfiguration ohne Werkzeuge/Host-Skills.

## Validierung am 15.09.2026

- Nativer Release-Build auf macOS 26.6.2 (Apple Silicon) und Prüfung auf
  persönliche Build-Pfade erfolgreich.
- DMG-Integrität, strikte App-Signaturprüfung und Prüfsumme der enthaltenen
  Chat-Runtime erfolgreich. Version 0.5.7 und Mikrofon-Nutzungstext im Bundle geprüft.
- App aus dem DMG kopiert und gestartet; Demo-Übersicht mit Konten und Vermögen
  sichtbar. Die installierte Vorgängerversion wurde nicht ersetzt.
- Finanzchat in der gebündelten App geöffnet: Die Komponentenprüfung funktioniert
  und zeigt „Mit ChatGPT anmelden“. Test-Apps über ihren tatsächlichen Pfad starten:
  Tauri verweigert auf macOS die Ressourcenauflösung bei einem Start über Symlinks
  (etwa `/tmp` statt `/private/tmp`). Regulär die App nach Programme kopieren.
- 13 Frontend-Tests und 81 Rust-Tests erfolgreich; ein Demo-Export-Hilfstest
  ist absichtlich ignoriert.
- Schlüsselbund-Speicherung, Neustart, Profiltrennung und Abmeldung mit
  synthetischen Zugangsdaten erfolgreich getestet.
- Native Chat-Runtime gegen einen lokalen synthetischen Modellserver geprüft:
  Antworten und Folgefragen funktionieren; Shell-, Datei- und Bildwerkzeuge
  werden abgelehnt; der Test speichert keine Anmeldung.

Eine echte ChatGPT-Anmeldung und der praktische Mikrofon-/Diktiertest sind auf
macOS nicht geprüft. Intel-Macs, ältere macOS-Versionen und der Gatekeeper-Ablauf
nach einem GitHub-Download sind ebenfalls nicht getestet.
