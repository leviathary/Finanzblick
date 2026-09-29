# Saldonaut 0.6.12

## Highlights

- Erfasst Käufe und Verkäufe einzelner Aktien-, ETF- und Fondspositionen mit
  datiertem Bestand, ohne andere Konten oder Positionen neu zu berechnen.
- Zeigt den Verlauf von Anfangsbestand, Käufen, Verkäufen und sonstigen
  Bestandsanpassungen direkt unter dem Chart der ausgewählten Position.
- Ergänzt und korrigiert manuelle Stichtagsbewertungen in einer eigenen,
  aufgeräumten Verlaufsansicht mit Diagramm und fokussierten Dialogen.
- Macht Kategoriezuordnungen verständlicher: Einzelbuchung oder Händlerregel,
  sichtbare Zuordnungsregeln sowie deren kontrolliertes Löschen.
- Vereinheitlicht Positionslisten und Eingabedialoge auf die Bezeichnung
  „Anzahl“ und ergänzt eine kompakte deutschsprachige Kurzanleitung.

## Windows-Download

`Saldonaut_0.6.12_x64-setup.exe` und die zugehörige SHA-256-Prüfsumme können
von diesem Release heruntergeladen werden. Der Installer ist nicht signiert;
Windows kann deshalb eine Sicherheitswarnung anzeigen. Vor einem Update mit
echten Daten sollte ein verschlüsseltes Backup erstellt werden.

## Prüfung

- 107 Frontendtests bestanden.
- 186 Rust-Tests bestanden; ein Test wurde absichtlich ignoriert.
- Frontend-Produktionsbuild, Rust Clippy und Formatprüfung bestanden.
- Nativer Windows-x64-Release-Build und NSIS-Paketierung mit vollständigem
  Strawberry Perl im neutralen Build-Verzeichnis erfolgreich.
- Prüfung auf persönliche Build-Pfade bestanden.
- Produktname `Saldonaut`, Version `0.6.12`, SHA-256
  `945652dec618ad6c1519a278f1f7429965f98f0fc4340b7b2a9f025a8b2785d5` und
  nicht signierter Authenticode-Status kontrolliert.
- Das stille Update der lokalen Installation von 0.6.11 auf 0.6.12 wurde
  erfolgreich durchgeführt und die installierte Programmversion geprüft.
- Eine vollständige visuelle und tastaturgestützte Regression der paketierten
  Anwendung wurde nicht durchgeführt.
- Dieses Release enthält Windows x64. macOS 0.6.12 wurde nicht gebaut oder getestet.
