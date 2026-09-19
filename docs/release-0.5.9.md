# Finanzblick 0.5.9

## Änderungen

- Einheitlichere Designsprache: dunkelblaue Primäraktionen und Navigation,
  konsistente Button-Typografie, Formularfelder und sichtbarer Tastaturfokus.
- Überarbeitete Anmeldung und separate Backup-Wiederherstellung sowie neues App-Icon.
- Kreditkartenassistent, mehrere Kartenkonten, persistente Ausgleichsregeln
  und separate Verwaltung neutraler Umbuchungen.
- Ungeklärte Kartengutschriften direkt filtern; eingerichtete Karten werden
  namentlich angezeigt. Kartenkäufe bleiben in den Konsumausgaben enthalten.
- Modularisierte Frontend- und Backend-Struktur sowie generischer camt.053-
  und ZKB-Import.

## Download und Prüfung

Windows x64: `Finanzblick_0.5.9_x64-setup.exe`.
Die zugehörige SHA-256-Prüfsumme wird als separate Datei veröffentlicht.

33 Frontend-Tests und 108 Rust-Tests bestanden (1 Rust-Test ignoriert).
TypeScript-Prüfung und Windows-Release-Build erfolgreich.
Die Browser-Designprüfung umfasst 13 Hauptansichten bei zwei Fensterbreiten
mit synthetischen Daten; Details stehen in `docs/design-audit-0.5.9.md`.

## Hinweise

- Frühe Testversion: Vor Nutzung mit bestehenden Daten ein Backup erstellen.
- Der Installer ist nicht signiert; Windows kann eine Sicherheitswarnung anzeigen.
- Eine frische Installation auf einem separaten Windows-System wurde nicht getestet.
- Diese Veröffentlichung enthält keinen neuen macOS-Build.
- Es wird keine automatische Migration bestehender Datenbanken mitgeliefert.
  Ältere Profile können wegen des geänderten Schemas inkompatibel sein;
  nötigenfalls ein neues Profil anlegen und Belege erneut importieren.
- Bekannte Build-Warnungen: Vite-Bundlegröße und fehlende OpenSSL-Debugsymbole.
