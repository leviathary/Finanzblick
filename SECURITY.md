# Sicherheit und Veröffentlichung

- Keine echten Finanzdaten, Finanzprofile, Backups, Zugangsdaten oder privaten
  Schlüssel einchecken. Die mitgelieferten Demo- und Testdaten sind fiktiv.
- Eigene Banklogos werden nur lokal im Finanzprofil gespeichert und gehören
  nicht ins Repository. Die Oberfläche verwendet standardmässig Kürzel.
- Installer nur über `npm run release` bauen. `CARGO_TARGET_DIR` muss ein
  neutrales Verzeichnis ohne persönlichen Benutzernamen sein. Ein fehlgeschlagener
  Pfadcheck sperrt die Veröffentlichung des erzeugten Installers.
- Vor einer Veröffentlichung `npm test`, `npm run build`, Lizenzhinweise und
  den tatsächlichen Installer prüfen. Ein erfolgreicher Build ersetzt keinen
  Installationstest auf einem frischen Rechner.
- Die Git-Historie wurde im September 2026 bereinigt. Ältere Klone neu klonen;
  alte Branches nicht zurückmergen oder zurückpushen, da sonst entfernte Dateien
  wieder in die Historie gelangen können.

Eine Historienbereinigung entfernt keine fremden Klone oder serverseitigen
Caches. Für verbleibende sensible GitHub-Referenzen ist gegebenenfalls der
[GitHub-Support](https://docs.github.com/en/authentication/keeping-your-account-and-data-secure/removing-sensitive-data-from-a-repository)
zuständig. Diese Prüfung ist keine Garantie für Fehlerfreiheit oder vollständige
rechtliche Freigabe aller Verwendungsarten.
