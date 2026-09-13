# Finanzblick

Lokaler Multi-Bank-Aggregator für Schweizer Konten, Depots und Vorsorgevermögen.

## Architektur

- Tauri 2 für die Desktop-Anwendung und den Windows-Installer
- React und TypeScript für die Benutzeroberfläche
- Rust für Import, Normalisierung und lokale Systemfunktionen
- SQLCipher als verschlüsselte lokale SQLite-Datenbank

Die Architektur muss Windows und eine spätere macOS-Version unterstützen.
Neue Funktionen und Abhängigkeiten dürfen keine Windows-spezifischen APIs,
Programme oder fest codierten Pfade voraussetzen. Betriebssystemintegration
erfolgt über plattformübergreifende Tauri- und Rust-Abstraktionen. Aktuell wird
keine Mac-Version umgesetzt; ein macOS-Build und entsprechende Tests stehen aus.

Finanzdaten sollen den Computer nicht verlassen. Banken werden zunächst über
Excel-, CSV- und PDF-Dateien angebunden. Jeder bank-spezifische Importer übersetzt sein
Quellformat in ein gemeinsames internes Modell.

## Lokaler Passwortschutz

Beim ersten Start ein Passwort mit mindestens 7 Zeichen einrichten.
Es gibt keine Anmeldung bei einem Cloud-Dienst und keinen Passwort-Reset.
Es gibt keine Vorgaben für Grossbuchstaben, Ziffern oder Sonderzeichen.
Das Passwort sicher ausserhalb der App aufbewahren; ohne Passwort sind auch
verschlüsselte Sicherungen nicht wiederherstellbar.

Die Datenbank `finanzblick.vault.sqlite3` liegt im von Tauri ermittelten
`app_local_data_dir` (unter Windows normalerweise
`%LOCALAPPDATA%/ch.finanzblick.desktop`), ausserhalb des Repositorys und von
OneDrive. Die App erzeugt keine Cloud-Backups. Quelldateien, manuell angelegte
Kopien und bereits synchronisierte OneDrive-Dateien bleiben separat zu schützen.

SQLCipher verwendet seine Standard-Passwortableitung und authentifizierte
Seitenverschlüsselung. Das Passwort liegt während der Sitzung nur im
Rust-Arbeitsspeicher und wird beim Sperren mit `zeroize` gelöscht; SQLCipher
bereinigt seine Schlüssel beim Schliessen der Verbindungen. Vollständiges
Löschen aller Kopien in WebView, Betriebssystem, Auslagerungsdatei oder
Absturzabbildern wird nicht garantiert. Datenträgerverschlüsselung bleibt sinnvoll.

Nach der eingestellten Zeit ohne Bedienung sperrt das Backend selbstständig (standardmässig 15 Minuten). Die Oberfläche
entfernt dann die Finanzansichten und Importvorschauen. Auch beim Minimieren
wird über den nativen Fensterzustand im Backend gesperrt, unabhängig von Sichtbarkeitssignalen der WebView.
Ein Ereignis entfernt die entsperrten Ansichten sofort; zusätzlich prüft das Backend den Fensterzustand jede Sekunde.
Ein gewöhnlicher Fokuswechsel sperrt nicht. Bereits laufende Datenbankoperationen dürfen fertig werden;
eine abgeschlossene Sperre wartet auf ihre Verbindungen. Ein zweiter Start
der neuen App wird durch eine lokale Dateisperre verhindert.

Die ausgelieferte App benötigt keinen Webserver. Die Content Security Policy
begrenzt WebView-Netzwerkzugriffe auf lokale App-Ressourcen und Tauri-IPC.
Die WebView läuft im privaten Modus. Windows Hello und Passkeys sind nicht
Bestandteil dieser Umsetzung. macOS wurde noch nicht gebaut oder getestet.

Prüfen: `npm test`, `npm run build` und `cargo test --manifest-path src-tauri/Cargo.toml --lib`.
Die Rust-Sicherheitstests verwenden ausschliesslich temporäre synthetische Daten.

### Einstellungen

Unter «Einstellungen» in der Navigation lassen sich die Sperrzeit (1, 5, 10,
15, 30 oder 60 Minuten), die Oberflächensprache (Deutsch, Englisch, Französisch,
Italienisch) und die Standardwährung für neue Konten (CHF, EUR, USD, GBP) ändern.
Änderungen werden mit «Einstellungen speichern» übernommen. Die Standardwährung
ist nur ein Vorschlag beim Anlegen eines Kontos: bestehende Konten, Importdaten
und CHF-Auswertungen werden nicht umgerechnet oder umbenannt.

Die Einstellungen liegen in der verschlüsselten Datenbank. Nur die Sprache wird
zusätzlich als nicht vertraulicher Hinweis in der lokalen Datei `language`
gespeichert, damit bereits die nächste Anmeldemaske die gewählte Sprache verwendet.
`src/translations.json` enthält die Oberflächentexte; importierte Beschreibungen,
Kontonamen und selbst benannte Kategorien bleiben unverändert. Technische
Importer-Diagnosen ohne Übersetzung erscheinen weiterhin in der Originalsprache.

«Passwort ändern» prüft das bisherige Passwort und verschlüsselt die Datenbank
mit SQLCipher neu. Es gelten weiterhin mindestens sieben Zeichen. Laufende
Datenzugriffe werden vor der Änderung abgeschlossen. Vorhandene Sicherungen
behalten ihr altes Passwort. Es werden keine echten Nutzerdaten für Tests verwendet.

Für die Entwicklung die App aus VS Code mit `npm run tauri dev` starten.
Windows-Installer nur bei Bedarf bauen; für normale Änderungen genügen der
Frontend-Build und passende Tests.

### Passwortmanager

Die Eingabefelder haben stabile Namen und die standardisierten Autocomplete-Angaben
`new-password` und `current-password`. Beim Absenden liest die App direkt die
Formularfelder, damit auch Einträge ohne React-Eingabeereignis berücksichtigt werden.
Einfügen und externe Passwortmanager werden nicht blockiert. Eine native Integration
mit Apple Passwörter ist damit nicht verbunden: Unter Windows wird dessen automatisches
Ausfüllen offiziell für Browser angeboten. In der Desktop-App den Eintrag «Finanzblick»
im Passwortmanager manuell speichern, das Passwort kopieren und einfügen.
Siehe [Apple-Anleitung](https://support.apple.com/en-ie/guide/icloud-windows/-icw76039ec0f/icloud).
Der private WebView-Modus bleibt aktiv; automatisches Speichern durch die WebView
wird nicht zugesichert. Die Finanzdaten bleiben lokal, unabhängig davon, ob der
gewählte Passwortmanager das Passwort zwischen Geräten synchronisiert.

Die Anmeldemaske zeigt keine technischen Speicherpfade. AppData bleibt der Ort
für die interne Datenbank; der Windows-Dokumente-Ordner dieses Rechners liegt in
OneDrive und eignet sich deshalb nicht für den gewünschten rein lokalen Betrieb.

## Synthetische Importbeispiele

MT940-Auszüge können als `.mt940` oder `.sta` über Dateiauswahl, Ordner oder
Drag-and-drop importiert werden. Unterstützt sind ein Konto und eine Währung
pro Datei, auch mit fortlaufenden Auszugsseiten. Anfangs- und Schlusssalden
werden mit allen Buchungen abgeglichen; unvollständige Dateien oder
Saldoabweichungen werden abgewiesen. Valuta, Buchungsdatum, Gutschriften,
Belastungen, Stornos und mehrzeilige Buchungstexte werden übernommen.
UBS wird am SWIFT-Absender erkannt; sonst erfolgt die Anbieterzuordnung über
das ausgewählte Konto. Bankenspezifische Erweiterungen können zusätzliche
Anpassungen benötigen. Die lokale Referenz lässt sich mit
`FINANZBLICK_MT940_REFERENCE` testen und bleibt ausserhalb des Repositorys.

Unter `fixtures/bank-statements` liegen Testauszüge für UBS, Migros Bank,
Raiffeisen und Generali, jeweils als Excel- und PDF-Datei. Alle Inhalte sind
frei erfunden und dürfen für Importtests verwendet werden. Generali wird als
Versicherungs-/Vorsorgeanbieter modelliert, nicht als Bank.

Unter `outputs/wealth-history-test` liegt zusätzlich ein synthetischer
UBS-Langzeitauszug mit 599 Buchungen über 80 Monate von Januar 2020 bis August
2026. Beim Import werden die Zeilensalden als historische Stichtage gespeichert,
damit Jahres- und Mehrjahresvergleiche in der Vermögensansicht möglich sind.

PDF-Importe werden nie ungeprüft übernommen: Der Parser liefert erkannte Zeilen
zusammen mit einem Erkennungsstatus an eine Importvorschau. Erst bestätigte
Zeilen werden in der lokalen Datenbank gespeichert.

## Voraussetzungen für die Entwicklung

- Node.js 22 oder neuer
- Rust inklusive Cargo
- Perl für den mitgebauten OpenSSL-Kryptografieanbieter (unter Windows ein natives Perl, z. B. Strawberry Perl, im PATH; Git-Perl genügt nicht)
- Microsoft C++ Build Tools und WebView2 für Tauri unter Windows

Node.js, Git, Rust, die Microsoft C++ Build Tools und WebView2 sind auf diesem
Rechner eingerichtet.

## Befehle

```powershell
npm install
npm run dev
```

`npm run dev` startet nur die Weboberfläche auf 127.0.0.1. Entsperren und Finanzdaten sind ausschliesslich in der Tauri-Desktop-App verfügbar.

Nach Installation der Tauri-Voraussetzungen:

```powershell
npm run tauri dev
npm run tauri build
```

## Fachliche Struktur

- `src/domain`: gemeinsame TypeScript-Datentypen
- `src/features`: UI nach Fachbereichen
- `src-tauri/src/domain`: Geschäftsregeln
- `src-tauri/src/importers`: Adapter für Raiffeisen, ZKB, UBS usw.
- `src-tauri/src/storage`: SQLite-Persistenz und verschlüsselte Datenbanken

## Nächster Schritt

Der Importassistent unterstützt Dateidialog und natives Drag-and-drop für XLSX,
XLS, CSV und PDF. Mehrere Dateien oder Ordner können zu einer gemeinsamen
Importliste hinzugefügt werden, optional mit Unterordnern. Nicht unterstützte,
unlesbare und über 25 MB grosse Dateien werden mit Hinweisen übersprungen;
identische Dateipfade werden nur einmal aufgenommen. Verknüpfungen werden nicht
verfolgt. Neu hinzugefügte Dateien werden automatisch gemeinsam analysiert;
gestoppte oder fehlgeschlagene Analysen können gesammelt erneut gestartet werden.
Dateiliste und Buchungsvorschau wachsen mit der Fenstergrösse. Nach der Analyse
können zugeordnete Dateien mit «Jetzt importieren» direkt importiert werden. Die Importübersicht zeigt
Kontozuordnungen und zusammengefasste Warnungen; Dateien mit Fehlern oder fehlenden
Zuordnungen bleiben ausgenommen. Einzelvorschauen sind optional. Bei eindeutig
zugeordneten Konten wird der Anbieter direkt in der Liste angezeigt.
Zugeordnete Dateien können gemeinsam importiert
werden. Jeder Import bleibt atomar; Fehler stoppen die übrigen Dateien nicht.
Erfolgreiche Importe und Duplikate werden nach dem Durchlauf aus der Arbeitsliste
entfernt. Eine Abschlussmeldung nennt die Anzahl; offene und fehlgeschlagene
Dateien bleiben für eine Wiederholung erhalten. Nach vollständigem Import ist
die Liste wieder leer. Die separate Freigabe entfällt; eine Hauptschaltfläche startet den Import. Ein Stopp wirkt nach der aktuellen Datei.
`npm test` prüft Kontozuordnung, Freigabe, Fehlerfortsetzung und Stoppverhalten.

CSV-Dateien unterstützen UTF-8 (auch mit BOM), UTF-16 mit BOM und als Fallback
Windows-1252/Latin-1 mit einem Hinweis in der Vorschau. Semikolon, Komma, Tabulator,
die Excel-Zeile `sep=;` und zitierte Felder werden verarbeitet. Kreditkartenexporte
mit Einkaufsdatum und Buchung verwenden die abgerechneten Belastungen/Gutschriften
und die Abrechnungswährung. Der Saldovortrag wird nicht als Buchung gezählt.
Ein fehlender CSV-Anbieter wird aus den ausgewählten Konten übernommen; bei
mehreren passenden Konten ist eine manuelle Zuordnung nötig.
Die Parser normalisieren Auszüge von UBS, Migros Bank,
Raiffeisen und Generali in ein gemeinsames Transaktionsmodell und zeigen das
Ergebnis vor dem Import zur Kontrolle an. Parser- und Importtests verwenden
ausschliesslich synthetische Testdaten aus dem Repository.

UBS-Monatsauszüge mit «Ihr Konto auf einen Blick» werden über mehrere Seiten
ausgelesen, einschliesslich Tausenderleerzeichen, Buchungsdatum, Valuta und
mehrzeiligen Empfänger-/Referenzangaben. Die Vorzeichen ergeben sich aus den
laufenden Kontoständen. Jede Saldoänderung, beide Umsatzsummen und der
Schlusssaldo müssen übereinstimmen; bei Abweichungen wird der Import abgebrochen.
Nachgelagerte Konto- und Gebührenübersichten werden nicht als Buchungen erfasst.
Die Datei muss auslesbaren PDF-Text enthalten; Scans benötigen weiterhin OCR.

Tests: `cargo test --manifest-path src-tauri/Cargo.toml`.

Weitere echte PDF-Formate:

- **UBS Mastercard-Abrechnung in CHF:** Einzelbuchungen beider Karten,
  Rückerstattungen, LSV-Zahlungen und Jahresgebühren. Karten- und Seitentotale
  werden mit den Details abgeglichen und nicht nochmals gebucht. Verbindlichkeiten
  erscheinen als negative Salden auf einem Kreditkartenkonto. Fremdwährungskäufe
  verwenden den abgerechneten CHF-Betrag; Originalbetrag, Kaufdatum und Kursdetails
  bleiben im Text. Ein Rundungsunterschied von höchstens fünf Rappen zum
  Rechnungsbetrag wird als sichtbare Ausgleichsbuchung mit Warnung übernommen.
- **Swissquote-Kontoauszug:** mehrseitige Buchungen mit Referenzen, Gebühren,
  Trades und Währungswechseln. Pro Währung werden Anfangs-/Endsaldo und beide
  Umsatzsummen geprüft. CHF, USD, EUR und GBP werden in separaten Konten
  gespeichert, auch bei einem Währungskonto ohne Bewegungen. Der umgerechnete
  Gesamtsaldo auf dem Deckblatt wird nicht zusätzlich als Guthaben verbucht.
  Wertpapierpositionen werden aus diesem Cash-Kontoauszug nicht rekonstruiert.

Die Importvorschau zeigt alle Buchungen und Währungssalden. Die Ausgabenanalyse
und die Vermögenssumme berücksichtigen weiterhin nur CHF; Fremdwährungen
werden nicht ohne Wechselkurs addiert. Die automatische Abstimmung interner
Transfers (etwa Kartenausgleich gegen die Belastung des Bankkontos) ist noch
nicht implementiert.

Bestätigte Importe werden atomar in SQLite gespeichert. Dabei werden Anbieter,
Konten, Importläufe, Transaktionen und Saldo-Snapshots erfasst. Ein
Datei-Fingerabdruck verhindert, dass derselbe Auszug versehentlich doppelt
importiert wird. Der Speicherort wird nach einem erfolgreichen Import in der
App angezeigt.

Beim Import werden bestehende aktive Konten per Dropdown ausgewählt. Neue
Konten werden unter «Banken & Konten» angelegt; eine Namenseingabe im Import
erzeugt keine weiteren Konten. Nach der Analyse werden Anbieter, Währung und
erkannter Kontotyp abgeglichen. Mehrwährungsauszüge benötigen eine Zuordnung
pro Währung. Beim Wechsel zur Kontoverwaltung bleibt der Importentwurf erhalten.
Gespeichert wird über die Konto-ID, damit auch umbenannte Konten wiederverwendet
werden. Ungültige oder archivierte Zielkonten werden vor dem Speichern abgewiesen.

Unter «Importverwaltung» werden Dateiname, Anbieter, alle betroffenen Konten,
Buchungszeitraum, Importdatum und Buchungsanzahl angezeigt. Einzelne oder mehrere
Importe können nach einer Bestätigung gelöscht werden. Dabei werden die
zugehörigen Buchungen und Saldo-Snapshots atomar entfernt, einschliesslich aller
Währungskonten. Konten und Originaldateien bleiben erhalten. Die Übersicht nutzt
danach die verbleibenden Salden; dieselbe Datei kann erneut importiert werden.

Die Importliste lässt sich nach Monat/Jahr, Jahr oder Anbieter gruppieren und
ein- oder ausklappen. Zeitgruppen verwenden das letzte Buchungsdatum des Auszugs
(bei fehlenden Buchungen das Importdatum). Gruppen zeigen Import- und
Buchungsanzahlen und können gemeinsam ausgewählt werden. Alternativ steht die
ungegliederte Liste zur Verfügung.

Die Dateinamensuche filtert die Importliste ohne Beachtung der Gross- und
Kleinschreibung. Sie unterstützt Suchbegriffe, die Platzhalter `*` und `?`
sowie einen optionalen Regex-Modus (z. B. `UBS|Kontoauszug`). Treffergruppen
öffnen sich automatisch. Beim Ändern der Suche wird die Auswahl zurückgesetzt.

Die Startseite liest die aktuellen Saldo-Snapshots aus SQLite und zeigt das
Gesamtvermögen, die Aufteilung nach Anbieter, alle Konten und den letzten
Import. Nach einem erfolgreichen Import kann direkt zur Übersicht oder zu
einem weiteren Import gewechselt werden.

Die Vermögensseite aggregiert die neuesten Salden aller berücksichtigten
CHF-Konten. Sie zeigt den zeitlichen Verlauf über alle importierten Stichtage
sowie Aufteilungen nach Kontotyp und Anbieter. Konten in anderen Währungen
werden erst nach Einführung einer Wechselkurslogik in die CHF-Summe einbezogen.
Bei Mehrjahreszeiträumen wird die Kurve aus Monatsendwerten geglättet; die
Y-Achse zeigt automatisch skalierte CHF-Werte. Schnellfilter und ein frei
wählbares Von-/Bis-Datum grenzen den dargestellten Zeitraum ein.

Die Transaktionsseite analysiert Ausgaben nach Zeitraum, Anbieter und Kategorie.
Ein Klick auf eine Kategorie filtert die Einzelbuchungen; ihre Zuordnung kann
direkt im Drilldown geändert werden. Bestehende und neue Transaktionen erhalten
zunächst eine regelbasierte Kategorie aus ihrem Buchungstext.

Manuelle Kategorieänderungen speichern eine lokale Händlerregel und ordnen
passende vorhandene sowie zukünftige Buchungen zu. Einkaufshinweise, Gross-/
Kleinschreibung und numerische Referenzen werden beim Vergleich normalisiert.
Bürgermeister/Burgermeister/Buergermeister wird über Filialen hinweg erkannt;
bei anderen Händlern werden die verbleibenden Namenswörter exakt verglichen.
Eine erneute Kategorieänderung aktualisiert die Regel für diesen Händler.
Die Kategorie «Internet & Mobilfunk» erkennt Sunrise- und Swisscom-Ausgaben
auch rückwirkend unter «Sonstiges». Händlerregeln haben Vorrang.

Die Kategorie «Digitale Abos» erkennt typische digitale Abrechnungstexte von
Apple, Google/YouTube, Paramount+, Netflix und Disney+. Passende bestehende
Ausgaben unter «Sonstiges» werden ebenfalls zugeordnet. Neue manuelle
Kategorieänderungen sind gegen automatische Neuzuordnung geschützt.
Apple- und Google-Abrechnungen können auch Einzelkäufe enthalten; die Zuordnung
kann im Transaktionsdetail korrigiert werden.

Unter `Banken & Konten` lassen sich Bankbeziehungen und Konten manuell anlegen,
Kontonamen, Typ, Währung und Referenz bearbeiten sowie Konten archivieren. Ein
Konto kann aus der Vermögensberechnung ausgeschlossen werden, ohne seine
Importe oder Transaktionen zu löschen.

Bereits importierte Dateiinhalte werden unmittelbar nach Dateiauswahl oder
Drag-and-drop per SHA-256 geprüft, bevor die Analyse startet. Die Liste zeigt
„Datei bereits vorhanden“ und bietet diese Dateien nicht zur Freigabe an.
Nach der Analyse und nach Änderungen der Kontozuordnung werden auch Buchungen
verglichen: Konto, Buchungsdatum, Betrag, Währung und Buchungstext. Vollständige
Übereinstimmungen sind von der Freigabe ausgeschlossen; teilweise Überschneidungen
erscheinen als Hinweis und die bereits vorhandenen Buchungen werden beim Speichern
automatisch übersprungen. Gleichartige Buchungen werden anhand ihrer Anzahl
verglichen, damit mehrfach vorkommende echte Zahlungen erhalten bleiben. Die
Prüfung wird unmittelbar innerhalb der atomaren Datenbanktransaktion wiederholt.


### Mehrere Datenbanken

Unter Einstellungen können mehrere voneinander unabhängige, verschlüsselte
Datenbanken angelegt und geöffnet werden, beispielsweise für verschiedene
Personen. Jede Datenbank besitzt ihren eigenen Datenbestand und ihr eigenes
Passwort. Die zuletzt ausgewählte Datenbank wird lokal gespeichert und beim
nächsten Start wieder angeboten; die Auswahl ist auch vor dem Entsperren
verfügbar.

Neu angelegte Datenbanken sind leer und werden nach dem Anlegen ausgewählt. Ein
Datenbankwechsel sperrt die bisher geöffnete Datenbank, sodass anschließend das
Passwort der gewählten Datenbank eingegeben werden muss. Bereits vorhandene
Datenbankdateien werden weiterhin automatisch gefunden und unverändert geöffnet.

Die aktuell geöffnete Datenbank kann in den Einstellungen anonymisiert werden.
Dabei können Buchungstexte optional durch neutrale Kennungen ersetzt und Beträge zufällig
auf 10 bis 300 Prozent des ursprünglichen Werts verändert. Jede einzelne Buchung erhält einen eigenen
Zufallswert; zusammengehörige Saldenwerte bleiben dabei konsistent. Die Änderung
ist nicht rückgängig zu machen. Alternativ lassen sich sämtliche Buchungs- und
Saldenbeträge mit einem festen Faktor skalieren, wodurch ihre Größenverhältnisse
erhalten bleiben. Zusätzliche Datenbanken
können nach einer ausdrücklichen Bestätigung dauerhaft gelöscht werden. Die
Hauptdatenbank „Meine Daten“ ist vom Löschen ausgenommen.

Eine geöffnete Datenbank kann außerdem unter einem neuen Namen vollständig
kopiert werden. Die verschlüsselte Kopie übernimmt alle Daten und zunächst das
Passwort der Ausgangsdatenbank. Nach erfolgreicher Integritätsprüfung wird sie
als aktuelle Datenbank geöffnet; ihr Passwort kann danach unabhängig geändert
werden.
