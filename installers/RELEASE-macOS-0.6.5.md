# Saldonaut 0.6.5 für macOS

[DMG für Apple Silicon herunterladen](https://github.com/leviathary/Finanzblick/releases/download/v0.6.5/Saldonaut_0.6.5_aarch64.dmg)
· [SHA-256-Prüfsumme](https://github.com/leviathary/Finanzblick/releases/download/v0.6.5/Saldonaut_0.6.5_aarch64.dmg.sha256)

Gebaut aus v0.6.5, Commit e92db17fb1791c1e9796fd373eed281592e2b0e8.
Enthält das neue Saldonaut-Branding, die optionale kosmische Darstellung,
die native Chat-Runtime und das lokale deutsche Sprachmodell.
Keine Quellcode-Anpassungen für diesen Mac-Build erforderlich.

## Installation

DMG öffnen, Saldonaut auf Applications ziehen und aus Programme starten.
Eine laufende Vorgängerversion vorher beenden. Demo-Passwort: `demo1234`.
Die App ist ad-hoc signiert, nicht von Apple notarisiert. Bei einer Sperre nach
dem Download: Systemeinstellungen → Datenschutz & Sicherheit → Dennoch öffnen.
Nur Apple Silicon; kein Intel-Build. Die bestehende App-Kennung bleibt erhalten.

## Prüfung am 21.09.2026 auf macOS 27.0 (26A428)

- 80 Frontend-Tests und 140 Rust-Tests erfolgreich; ein Demo-Hilfstest absichtlich ignoriert.
- Lokaler Chat-Runtime-Test mit synthetischem Modellserver erfolgreich.
- Produktionsbuild über `npm run release:mac`; Lizenzinventar für aarch64-apple-darwin
  erneuert, Prüfung auf persönliche Build-Pfade erfolgreich.
- DMG mit `hdiutil verify` geprüft, schreibgeschützt eingebunden und App daraus kopiert.
- Kopierte App mit strikter, rekursiver Signaturprüfung geprüft.
- Bundle-Produktname Saldonaut und Version 0.6.5 bestätigt.
- Sichtbarer Start, Demo-Anmeldung und Demo-Übersicht mit Saldonaut-Branding
  in dunkler Darstellung geprüft.
- Hash der gebündelten Chat-Runtime und mitgelieferte Lizenzdatei geprüft.
- Die bereits installierte App wurde nicht ersetzt.

Die kosmische Darstellung und Chat-Anmeldemaske wurden in diesem Build-Durchlauf
nicht interaktiv geprüft; der separate lokale Chat-Runtime-Test besteht. Eine echte ChatGPT-Anmeldung, Mikrofon/Diktat,
Intel-Macs, andere macOS-Versionen und eine frische Installation mit Gatekeeper-Prüfung
nach einem GitHub-Download sind nicht getestet.
