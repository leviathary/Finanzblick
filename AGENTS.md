# Architekturvorgaben

- Die App unterstützt Windows; ein lokaler macOS-Build mit DMG ist ebenfalls
  beauftragt. Eine automatische Veröffentlichung ist nicht beauftragt.
- Neue Funktionen und Abhängigkeiten müssen Windows und macOS unterstützen.
  Keine Windows-spezifischen APIs, Programme oder Shell-Befehle als Voraussetzung
  für die Anwendung oder ihre Geschäftslogik einführen.
- Für Dateidialoge, Dateipfade, Datenverzeichnisse und Betriebssystemintegration
  die plattformübergreifenden Abstraktionen von Tauri und Rust verwenden.
  Keine fest codierten Windows-Pfade oder Pfadtrenner ergänzen.
- Auch neue Hilfsskripte plattformübergreifend gestalten. Bestehende
  Windows-Bezüge bei Änderungen am betroffenen Bereich berücksichtigen.
- Plattformneutrale Begriffe in der Oberfläche verwenden, z. B. „Dateidialog“.
- macOS-Kompatibilität erst nach einem tatsächlichen Build und Test als
  verifiziert bezeichnen.
- Keine einmaligen Bestandsdatenmigrationen für Entwicklungsdaten, persönliche
  Konten oder individuelle Datenkorrekturen in die Anwendung aufnehmen. Solcher
  Migrationscode darf nicht Teil des paketierten oder veröffentlichten Produkts
  sein und insbesondere bestehende Konten nicht automatisch umklassifizieren.
  Falls eine Migration ausnahmsweise nötig ist, dafür ein separates, nicht mit
  der App ausgeliefertes Werkzeug erstellen; persönliche Bestände vorzugsweise
  manuell anpassen.

# Verbindliche UI- und Designvorgaben

- Vor neuen oder geänderten Oberflächen `docs/design-system.md` vollständig lesen
  und die dortige Abnahmecheckliste anwenden. Dies gilt auch für kleine Änderungen
  an Buttons, Formularen, Tabellen und Navigationsleisten.
- Bestehende gemeinsame Komponenten und CSS-Variablen zuerst prüfen und
  wiederverwenden. Dunkelblau ist die Farbe für Hauptaktionen und aktive Navigation;
  Grün dient Finanzwerten, Erfolg und dezenten Hinweisen, nicht primären Buttons.
- Widersprüchliche ältere CSS-Regeln sind keine Designvorlage. Bekannte
  Abweichungen stehen im Leitfaden; Änderungen auf den beauftragten Bereich begrenzen.
- Neue vom Nutzer bestätigte Designentscheidungen im Leitfaden nachführen.
- UI-Änderungen auch visuell und mit Tastatur prüfen. Falls das nicht möglich ist,
  die fehlende Prüfung ausdrücklich benennen; erfolgreiche Builds ersetzen sie nicht.

# Quellcode-Dokumentation

- Jede eigene Quellcode-Datei beginnt mit einem kurzen Kommentar (ein bis drei
  Sätze), der ihren Zweck und ihre zentrale Verantwortung beschreibt. Bei neuen
  Dateien ergänzen und bei Aufgabenänderungen aktualisieren; passende vorhandene
  Dateibeschreibungen beibehalten.
- Erforderliche Shebangs, Compiler-Direktiven und Lizenzhinweise erhalten und
  deren Platzierung beachten. Generierte Dateien, Fremdcode und reine Daten-
  oder Konfigurationsdateien ohne Kommentarsyntax sind ausgenommen.
