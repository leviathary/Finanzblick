# Features

Die Benutzeroberfläche wird fachlich nach Features organisiert:

- `overview`: aggregierte Übersicht
- `accounts`: Banken, Konten und Konteneditor
- `transactions`: Transaktionen, Umbuchungen und Ausgleichsregeln
- `cards`: Kartenansicht, typisierte API und separater Setup-Workflow
- `categories`: Kategorienverwaltung
- `imports`: Dateiimporte, Mapping und Importhistorie
- `assets`: Vermögensentwicklung, Diagramme und Aufteilungen
- `tax-history`: Steuerhistorie
- `settings`: Einstellungen, Profile und Backups
- `chat`: Finanzassistent
- `help`: lokale, durchsuchbare Hilfe und Themenartikel
- `auth`: Anmeldung und Anmeldestyles
- `positions`: Positionsverwaltung, typisierte API und Bewertungsformular

Gemeinsame Diagramme liegen in `src/shared/charts`, der Kontotyp in `src/domain/finance.ts`.

Gemeinsame Feature-Typen liegen in `types.ts`, reine Anzeigehelfer in
`presentation.ts`. Neue Workflows erhalten einen eigenen Unterbereich
und werden nicht per Modus-Flag in Übersichtsseiten eingebaut.

Backend-Schichten, Testgrenzen und verbleibende Übergänge:
[Architekturleitfaden](../../docs/architecture.md).
