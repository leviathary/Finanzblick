# Architekturvorgaben

- Die App soll später auch auf macOS laufen. Aktuell ist keine Umsetzung,
  Build-Pipeline oder Veröffentlichung für macOS beauftragt.
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
