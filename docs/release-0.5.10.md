# Finanzblick 0.5.10

## Änderungen

- Helles, dunkles oder systemabhängiges Erscheinungsbild mit gespeicherter Auswahl.
- Interaktive Vermögenscharts mit lokal eingebundenen TradingView Lightweight
  Charts: Zoomen, Verschieben, kompakte Zeitraumauswahl und schwebendes Messwerkzeug.
- Optionale Marktindex-Vergleiche mit SMI und S&P 500, auf Basis 100 normalisiert.
  Die Vergleiche sind keine einzahlungsbereinigte Renditeberechnung.
- Kontexthilfe direkt in der Chart-Toolbar mit Bedienung, Vergleichsgrundlagen
  und Hinweisen zu Datenquelle und Datenschutz.

## Windows-Installer und Prüfung

Windows x64: `Finanzblick_0.5.10_x64-setup.exe`.
Die SHA-256-Prüfsumme liegt unter `installers/`.

49 Frontend-Tests und 115 Rust-Tests bestanden (1 Rust-Test ignoriert).
TypeScript-Prüfung, Frontend-Produktionsbuild und nativer Windows-Release-Build erfolgreich.
Browserprüfungen mit synthetischen Daten decken helle und dunkle Darstellung,
verschiedene Fensterbreiten, Chart-Bedienung und Tastaturbedienung der Kontexthilfe ab.

## Hinweise

- Frühe Testversion: Vor Nutzung mit bestehenden Daten ein Backup erstellen.
- Der Windows-Installer ist nicht signiert; Windows kann eine Sicherheitswarnung anzeigen.
- Eine Installation und ein nativer Starttest des neuen Installers wurden nicht durchgeführt.
- Für diese Version wurde kein macOS-Build erstellt oder getestet.
- Marktindizes werden nur bei Auswahl über Yahoo Finance geladen; dabei werden
  Index und Zeitraum, aber keine persönlichen Finanzbuchungen übertragen.
- Bekannte Build-Warnungen: Vite-Bundlegröße und fehlende OpenSSL-Debugsymbole.
- Der native Build wurde mit einem neutralen `CARGO_TARGET_DIR` neu erstellt,
  damit OpenSSL keinen persönlichen Projektpfad einbettet. Die Prüfung auf
  persönliche Build-Pfade wurde nicht deaktiviert.
- Der Installer wird lokal erstellt; eine öffentliche GitHub-Release-Veröffentlichung
  erfolgt nicht automatisch.
