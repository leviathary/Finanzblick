# Designprüfung 0.5.9

Stand: 19. September 2026. Grundlage: [Designsystem](design-system.md).

## Vereinheitlicht

- Primäraktionen, aktive Navigation und Tastaturfokus verwenden Slate/Dunkelblau.
- Gemeinsame Buttons verwenden 14 px, Schriftgewicht 600, mindestens 44 px Höhe
  und zentrierte Beschriftungen; kompakte Tabellenaktionen bleiben gesondert gestaltet.
- Formularfelder haben konsistente Rundungen und Fokuszustände.
- Grün bleibt für positive Finanzwerte, Diagramme und informative Statusanzeigen erhalten.
- Die Filterleiste für Umbuchungen bricht bei schmaleren Fenstern um,
  statt horizontal über den Seitenrand hinauszuragen.

## Prüfung

13 Hauptansichten wurden mit synthetischen Daten und simulierten Tauri-Aufrufen
in Chrome bei 1440 und 920 px Fensterbreite geöffnet: Übersicht, Banken,
Vermögen, Steuerhistorie, Transaktionen, Umbuchungen, Karten, Kategorien,
Import, Importverwaltung, Finanzchat, Daten & Sicherheit und Einstellungen.
Alle 26 Aufrufe blieben ohne JavaScript-Fehler und ohne horizontalen
Seitenüberlauf. Screenshots wurden für alle Ansichten erstellt; alle breiten
Hauptansichten und ausgewählte schmale Ansichten wurden visuell kontrolliert.

Zusätzlich geprüft: Passwort-Autofokus, Anmeldung per Enter, separate
Wiederherstellung, Tastaturaktivierung des Hinweises auf ungeklärte Gutschriften,
Kartenassistent und Sortierung über Tabellenüberschriften.

TypeScript-Prüfung erfolgreich; 33 Frontend-Tests erfolgreich;
108 Rust-Tests erfolgreich, 1 Test ignoriert.

## Grenzen

Dies ist keine vollständige Prüfung aller Dialogzustände, Zoomstufen oder
Screenreader-Kombinationen. Die Browserprüfung ersetzt keinen nativen
Windows-Installationstest. macOS wurde für diese Version nicht gebaut oder
getestet. Bestehende feature-spezifische CSS-Dateien bleiben erhalten;
weitere Komponenten-Konsolidierung kann schrittweise erfolgen.
