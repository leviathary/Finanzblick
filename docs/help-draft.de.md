# Entwurf: Integrierte Hilfe

Status: Inhalts- und Strukturentwurf für eine lokale, in Saldonaut integrierte
Hilfe. Die Texte verwenden ausschliesslich fiktive Beispiele. Screenshots werden
später mit dem Demo-Profil erstellt.

## 1. Aufbau der Hilfe

Die globale Navigation erhält im unteren Bereich oberhalb von **Daten & Sicherheit**
den Eintrag **Hilfe**. Er öffnet eine eigenständige Hilfeseite unter `#help`.

Auf breiten Fenstern besteht die Hilfeseite aus:

- einer lokalen Themennavigation links,
- dem aktuellen Hilfeartikel rechts,
- einer Suche über Titel, Kurzbeschreibungen und Artikeltexte.

Auf schmalen Fenstern steht die Themennavigation als Auswahl oberhalb des Artikels.
Alle Inhalte werden mit der App ausgeliefert und funktionieren offline. Links aus
Kontexthilfen öffnen direkt den passenden Abschnitt, zum Beispiel
`#help/assets/chart`.

### Themennavigation

1. Erste Schritte
2. Bankauszüge importieren
3. Banken & Konten
4. Übersicht und Vermögen
5. Transaktionen und Kreditkarten
6. Kategorien und Regeln
7. Steuerhistorie
8. Finanzchat
9. Daten, Backup und Sicherheit
10. Einstellungen
11. Häufige Fragen

## 2. Startseite der Hilfe

### Hilfe & erste Schritte

Saldonaut bündelt Konten, Depots und weitere Finanzquellen in einer gemeinsamen
lokalen Sicht. Deine Finanzdaten werden verschlüsselt auf diesem Gerät gespeichert.
Ein Benutzerkonto oder eine Cloud-Synchronisierung ist nicht erforderlich.

**Was möchtest du tun?**

- **Saldonaut ausprobieren** – Demo-Profil öffnen und die wichtigsten Bereiche
  kennenlernen.
- **Eigene Daten einrichten** – Finanzprofil, Konten und ersten Import anlegen.
- **Vermögen auswerten** – Salden, Positionen und Entwicklung verstehen.
- **Ausgaben analysieren** – Transaktionen, Kategorien und Kreditkarten prüfen.
- **Daten sichern** – ein verschlüsseltes Backup erstellen oder wiederherstellen.

> **Wichtig:** Es gibt keinen Passwort-Reset. Bewahre das Passwort deines
> Finanzprofils sicher auf und erstelle regelmässig ein Backup.

**Screenshot:** `help-start.png` – Hilfe-Startseite mit Suche und Themenkarten.

## 3. Erste Schritte

### Saldonaut mit Demo-Daten ausprobieren

Das Demo-Profil enthält erfundene Banken, Konten, Wertpapierpositionen,
Transaktionen und mehrjährige Verläufe. Es eignet sich, um alle Ansichten ohne
eigene Finanzdaten kennenzulernen.

1. Wähle auf der Anmeldung unter **Finanzprofil** den Eintrag **Demo-Daten**.
2. Gib das Passwort `demo1234` ein.
3. Wähle **Anmelden**.
4. Öffne nacheinander **Übersicht**, **Vermögen**, **Transaktionen** und
   **Banken & Konten**.

Demo-Kurse sind synthetische Beispieldaten und keine historischen Börsenkurse
oder Anlageempfehlungen.

**Screenshot:** `demo-login.png` – Anmeldung mit ausgewähltem Demo-Profil.

### Ein eigenes Finanzprofil anlegen

Ein Finanzprofil ist ein eigenständiger, verschlüsselter Datenbestand. Mehrere
Profile bleiben voneinander getrennt und können unterschiedliche Passwörter haben.

1. Wähle auf der Anmeldung unter **Finanzprofil** die Aktion zum Erstellen eines
   neuen Profils.
2. Vergib einen eindeutigen Namen.
3. Lege ein starkes Passwort fest und bestätige es.
4. Melde dich mit dem neuen Profil an.
5. Lege unter **Banken & Konten** deine Konten an oder beginne unter **Import**
   mit einem Bankauszug.

> **Passwort merken:** Ohne das aktuelle Passwort kann das Profil nicht geöffnet
> und ein Backup nicht wiederhergestellt werden.

### Sprache bereits bei der Anmeldung ändern

Die Sprache lässt sich oben rechts auf der Anmeldeseite wählen. Deutsch, Englisch,
Französisch und Italienisch stehen zur Verfügung. Nach der Anmeldung kannst du die
Sprache unter **Einstellungen → Allgemein** erneut ändern.

## 4. Bankauszüge importieren

### Unterstützte Dateien

Saldonaut verarbeitet Excel-, CSV-, PDF- und MT940-Dateien sowie camt.053 und
camt.054 nach ISO 20022 lokal. Vor dem Speichern siehst du eine Vorschau und die
vorgesehene Kontozuordnung. Vorlagen existieren unter anderem für UBS inklusive
Mastercard, Swissquote, Migros Bank, Raiffeisen und Generali. Andere tabellarische
Dateien lassen sich über eine Spaltenzuordnung einlesen.

Enthält ein Auszug eine IBAN oder Kontoreferenz, ordnet Saldonaut ihn nur einem
aktiven Konto mit derselben hinterlegten Kennung zu. Bei einer fehlenden oder
abweichenden Kennung bleibt der Import gesperrt, bis die Kontodaten unter
**Banken & Konten** korrigiert wurden. Nur Auszüge ohne Kontokennung werden über
Anbieter, Währung und Kontotyp zugeordnet.

### Datenschutz beim Import

Bankauszüge, Kreditkartenabrechnungen und Steuererklärungen werden ausschliesslich
lokal auf deinem Gerät verarbeitet. Die Importdateien werden weder hochgeladen
noch an Saldonaut oder Dritte übertragen.

Saldonaut kopiert die Quelldokumente nicht in dein Finanzprofil. Nach der Analyse
werden die eingelesenen Dateiinhalte aus dem Arbeitsspeicher verworfen; die App
benötigt die Dokumente danach nicht mehr. Deine Originaldateien bleiben an ihrem
bisherigen Speicherort und werden weder verändert noch gelöscht.

Gespeichert werden nur die von dir bestätigten Finanzdaten, der Dateiname und ein
technischer Fingerabdruck zur Duplikaterkennung. Diese Informationen liegen
verschlüsselt in deinem lokalen Finanzprofil.

> Nur wenn du im Finanzchat ausdrücklich ein Datenpaket freigibst, werden die darin
> angezeigten Informationen an den gewählten Chat-Dienst übertragen. Die
> ursprünglichen Importdokumente gehören nicht zu diesem Datenpaket.

### Einen oder mehrere Auszüge importieren

1. Öffne **Import → Dateien importieren → Bankauszüge**.
2. Wähle einzelne Dateien oder einen ganzen Ordner.
3. Lass die Dateien analysieren.
4. Prüfe bei jeder Datei Anbieter, Zielkonto, Zeitraum und Buchungsvorschau.
5. Ordne unbekannte Spalten bei Bedarf manuell zu.
6. Wähle **Importieren**, sobald alle gewünschten Dateien bereit sind.
7. Öffne danach **Importierte Dateien**, um das Ergebnis nachzuvollziehen.

Bereits importierte Dateien und vorhandene Buchungen werden erkannt. Sind nur
einzelne Buchungen eindeutig bereits vorhanden, überspringt Saldonaut diese beim Import
und weist in der Vorschau darauf hin.

Mögliche Duplikate mit gleichem Konto, Betrag, ähnlicher Beschreibung und nahem
Buchungsdatum werden in der Vorschau markiert. Der Import bleibt gesperrt, bis du
jeden Treffer als neue Buchung bestätigst oder als Duplikat überspringst.

Unter **Importierte Dateien** kann der bestehende Buchungsbestand manuell geprüft
werden. Die Prüfung gruppiert eindeutige und mögliche Dubletten, zeigt ihre
Importquellen und verändert keine Buchung automatisch. Bestätigte Mehrfachzahlungen
werden bei späteren Prüfungen nicht erneut angezeigt; entfernte Dubletten bleiben
über ihre Importspur wiederherstellbar.

> **Vor dem Import prüfen:** Achte besonders auf Zielkonto, Währung, Vorzeichen
> und Datumsformat. Ein erfolgreicher Import ersetzt keine fachliche Kontrolle.

### Offene Kreditkartenmonate

Noch nicht abgerechnete Kartenkäufe in der Kontowährung können bereits importiert
werden. Saldonaut verwendet dafür vorläufig das Einkaufsdatum und den
Originalbetrag und zeigt in der Vorschau einen Warnhinweis. Der spätere
Abrechnungsbetrag oder das endgültige Buchungsdatum kann davon abweichen.

Offene Fremdwährungsbuchungen ohne abgerechneten Betrag werden nicht automatisch
umgerechnet. Importiere sie erst, wenn der Kartenanbieter den endgültigen Betrag
in der Kontowährung ausweist. Spätere vollständige Jahres- oder Monatsauszüge
dürfen sich überlappen: Saldonaut erkennt zuvor offene Kartenkäufe wieder und
ersetzt Einkaufsdatum und Originalbetrag automatisch durch die endgültigen
Abrechnungsdaten, statt eine zweite Buchung anzulegen.

**Screenshot:** `import-review.png` – Importliste mit geöffneter Vorschau und
Kontozuordnung.

### Eine unbekannte Excel- oder CSV-Struktur zuordnen

Öffne für die Datei **Spalten zuordnen** und weise mindestens Datum, Beschreibung
und Betrag zu. Prüfe die Vorschau, bevor du die Zuordnung speicherst. Eine
gespeicherte Zuordnung kann für gleich aufgebaute spätere Dateien wiederverwendet
werden.

### Importierte Dateien verwalten

Unter **Import → Importierte Dateien** findest du die Importhistorie. Von hier aus
kannst du nachvollziehen, welche Quelle wann welchem Konto zugeordnet wurde.
Entferne einen Import nur, wenn du seine Auswirkungen auf Buchungen und Salden
verstanden hast; der Bestätigungsdialog beschreibt die konkrete Wirkung.

## 5. Banken & Konten

### Eine Bank und ihre Konten hinzufügen

1. Öffne **Banken & Konten**.
2. Wähle **Bank oder Anbieter hinzufügen** und erfasse Name sowie Anbietertyp.
3. Öffne anschließend das Drei-Punkte-Menü der neuen Anbietergruppe.
4. Wähle **Konto hinzufügen** und erfasse Kontoname, Kontotyp und Währung.
5. Lege fest, ob das Konto zum Gesamtvermögen zählt, und speichere es.

Name, Typ und Logo werden zentral über das Drei-Punkte-Menü im Kopf der
Anbietergruppe bearbeitet. Die Änderung gilt für alle zugehörigen Konten. Über
dieses Menü kannst du außerdem direkt ein weiteres Konto beim Anbieter anlegen.

Konten desselben Anbieters werden gemeinsam gruppiert. Ein Konto mit dem Hinweis
**Nicht im Gesamtvermögen** bleibt sichtbar, wird aber in den Vermögenssummen nicht
berücksichtigt.

**Screenshot:** `accounts-overview.png` – kompakte Anbietergruppen mit Konten,
Salden und Statushinweisen.

### Manuelle Positionen verwalten

Manuell bewertete Konten eignen sich beispielsweise für Mitarbeiteraktien,
Depots ohne importierbaren Auszug oder andere Vermögenspositionen.

1. Öffne bei einem manuellen Konto das Drei-Punkte-Menü.
2. Wähle die Verwaltung der Positionen.
3. Lege eine neue Position mit Name, Datum, Menge und Bewertung an.
4. Hinterlege bei Wertpapieren nach Möglichkeit eine passende Kennung und Börse,
   damit Marktpreise automatisch ermittelt werden können.
5. Speichere regelmässig neue Stichtage, wenn du die Entwicklung im Zeitverlauf
   abbilden möchtest.

Bei Mitarbeiteraktien kann beispielsweise für jedes Zuteilungsjahr eine eigene
Position geführt werden. So bleiben Menge, Bewertungsdatum und Entwicklung je
Jahr nachvollziehbar.

**Screenshot:** `manual-positions.png` – Konto „Mitarbeiteraktien“ mit mehreren
jährlichen Apple-Positionen.

### Konten archivieren oder ausschliessen

- **Vom Gesamtvermögen ausschliessen:** Das Konto bleibt aktiv und sichtbar,
  fliesst aber nicht in Vermögenssummen ein.
- **Archivieren:** Das Konto bleibt mit seiner Historie erhalten, erscheint aber
  nicht mehr als aktives Konto.
- **Löschen:** Ist nur für geeignete leere Konten vorgesehen und wird bestätigt.

## 6. Übersicht und Vermögen

### Übersicht lesen

Die Übersicht fasst das aktuelle Gesamtvermögen, die berücksichtigten Konten und
Anbieter zusammen. Der Donut **Vermögen nach Anbieter** zeigt die positiven
Anbietersalden. Negative Salden bleiben in der Gesamtsumme berücksichtigt und
werden in der Liste separat ausgewiesen.

Unter **Aktuelle Salden** siehst du die neuesten Werte aller einbezogenen Konten.
Alle Beträge verwenden tabellarische Ziffern und die jeweils ausgewiesene
Bewertungswährung.

**Screenshot:** `overview.png` – Kennzahlen, Anbieter-Donut und aktuelle Salden.

### Vermögensentwicklung untersuchen

Unter **Vermögen** zeigt der Saldoverlauf die Entwicklung der importierten und
manuell gepflegten Werte.

- Wähle einen Zeitraum wie **1M**, **6M**, **YTD**, **1J**, **3J**, **5J** oder
  **Max**.
- Zoome mit dem Mausrad in den Verlauf.
- Verschiebe den sichtbaren Zeitraum durch Ziehen.
- Aktiviere das Messwerkzeug, um die Veränderung zwischen zwei Punkten zu sehen.
- Setze die Ansicht mit dem Zurücksetzen-Symbol zurück.
- Nutze **Vergleichen**, um einen verfügbaren Index einzublenden.

Ein eigener Bank- oder Kontofilter begrenzt ausschliesslich den Saldochart. Die
Aufteilungen darunter verwenden ihre jeweils ausgewiesene Auswahl.

**Screenshot:** `assets-chart.png` – Vermögensentwicklung mit YTD-Auswahl,
Werkzeugleiste und Vergleichsfunktion.

### Aufteilungen verstehen

**Nach Kategorie** und **Nach Anbieter** zeigen die Zusammensetzung des aktuellen
Vermögens. Im Kategorien-Donut werden positive Summen dargestellt; negative Werte
bleiben in der Liste und in der Gesamtsumme sichtbar. Kleine Kategorien können als
**Weitere Kategorien** gebündelt und aufgeklappt werden.

## 7. Transaktionen und Kreditkarten

### Einnahmen und Ausgaben auswerten

Unter **Transaktionen → Alle Transaktionen** findest du den Saldoverlauf sowie die
Auswertung von Einnahmen und Ausgaben.

1. Begrenze bei Bedarf Bank und Konto.
2. Wähle **Ausgaben** oder **Einnahmen**.
3. Wechsle zwischen der Darstellung **nach Kategorie** und **nach Monat**.
4. Klicke auf eine Kategorie oder einen Monatswert, um die zugehörigen Buchungen
   zu öffnen.
5. Ziehe in der Monatsmatrix mit gedrückter linker Maustaste über mehrere Zellen,
   um die Summe der ausgewählten Beträge anzuzeigen. Halte **Strg** (Windows)
   oder **Cmd** (macOS) gedrückt, um weitere Zellen und Bereiche hinzuzufügen.
6. Suche oder sortiere im Drilldown nach Datum, Beschreibung, Konto oder Betrag.

**Weitere Kategorien** bündelt kleinere Kategorien im Donut. Öffne die Zeile, um
Einzelwerte wie **Digitale Abos** zu sehen und auszuwählen.

**Screenshot:** `transactions-analysis.png` – Einnahmen-/Ausgabenübersicht mit
Kategorie-Donut und geöffneter Restgruppe.

### Eine Buchung kategorisieren

Öffne das Drei-Punkte-Menü einer Buchung und wähle **Kategorie ändern …**. Die
Änderung kann auch auf passende Buchungen desselben Händlers und zukünftige
Importe wirken. Prüfe deshalb den Hinweis im Editor, bevor du speicherst.

### Umbuchungen und Ausgleiche

Über das Drei-Punkte-Menü kannst du eine Buchung als Kartenausgleich oder als
Übertrag zwischen eigenen Konten markieren. Solche Bewegungen werden neutral
behandelt, damit interne Verschiebungen Einnahmen und Ausgaben nicht doppelt
verfälschen. Unter **Umbuchungen & Ausgleiche** lassen sich Markierungen prüfen und
wiederherstellen.

### Kreditkarten einrichten

Unter **Kartentransaktionen** wird für jedes Kartenkonto eine Regel benötigt, mit
der Gutschriften beziehungsweise Ausgleichsbuchungen erkannt werden.

1. Wähle das Kartenkonto.
2. Starte **Karte einrichten**.
3. Wähle eine passende Beispielbuchung als Muster.
4. Prüfe Richtung, Währung und Vorschau der Treffer.
5. Entscheide, ob die Regel auch für zukünftige Importe gelten soll.
6. Speichere die Einrichtung.

Ungeklärte Kartengutschriften werden als Warnung angezeigt. Prüfe sie, damit
Ausgaben und Kontostände nicht doppelt gezählt werden.

## 8. Kategorien und Regeln

### Kategorien verwalten

Unter **Kategorien** kannst du Namen und Farben an deine Auswertungen anpassen.
Eigene Kategorien stehen auch bei der Zuordnung von Transaktionen zur Verfügung.

- **Neue Kategorie:** legt eine zusätzliche Kategorie an.
- **Bearbeiten:** ändert Name und Farbe direkt in der Zeile.
- **Zusammenführen:** verschiebt Buchungen und Regeln kontrolliert in eine
  Zielkategorie.
- **Löschen:** ist nur nach Bestätigung und Klärung der betroffenen Zuordnungen
  möglich.

### Kreditkarten-Kategorien zuordnen

Im Bereich **Kartenkäufe automatisch kategorisieren** steht links die vom
Kartenanbieter gelieferte **Kreditkarten-Kategorie** und rechts die zugehörige
**Saldonaut-Kategorie**. Die Auswahl gilt für bestehende und künftig importierte
Kartenkäufe. Manuell gesetzte Kategorien und speziellere Händlerregeln bleiben
erhalten.

**Screenshot:** `category-mapping.png` – Zweispaltenliste mit Quellkategorie und
Saldonaut-Kategorie.

## 9. Steuerhistorie

Die Steuerhistorie verwaltet jährliche Vermögenswerte unabhängig von laufenden
Bankimporten. Du kannst unterstützte Steuererklärungen importieren und die
ermittelten Jahreswerte vor dem Speichern prüfen.

1. Öffne **Import → Dateien importieren → Steuererklärungen**.
2. Wähle die Datei und prüfe die Vorschau.
3. Übernimm die erkannten Werte.
4. Öffne **Steuerhistorie**, um Entwicklung, Wertschriften, Immobilien, übrige
   Werte und Schulden nach Steuerjahr zu vergleichen.

Die Steuerhistorie ist eine separate Jahresbetrachtung. Ihre Werte werden nicht
automatisch mit den laufenden Bank- und Depotständen vermischt.

**Screenshot:** `tax-history.png` – Timeline mit englisch oder deutsch vollständig
lokalisierten Kennzahlen und Legende.

## 10. Finanzchat

Der Finanzchat beantwortet Fragen zu ausgewählten Daten aus deinem Finanzprofil.
Er ist rein lesend und verändert keine Konten, Buchungen oder Kategorien.

### ChatGPT verbinden

1. Öffne **Finanzchat**.
2. Wähle **Mit ChatGPT anmelden**.
3. Schliesse die Anmeldung im geöffneten Fenster ab.
4. Prüfe vor dem Senden das vorbereitete Datenpaket.

Standardmässig zeigt Saldonaut vor der Übermittlung, welche zusammengefassten
Daten gesendet werden. Bei reinen Kreditkartenfragen werden allgemeines Einkommen
und Vermögen nicht automatisch mitgegeben.

### Datenschutz und Fearless-Modus

Im optionalen Fearless-Modus können für einen gewählten Zeitraum auch
Buchungsbeschreibungen übertragen werden. Darin können persönliche Informationen
enthalten sein. Separate Konto- und Banknamen, IBAN, Kontonummern, Inhaber- und
Adressfelder bleiben ausgeschlossen. Prüfe dennoch Kategorienamen, Frage und
Buchungstexte sorgfältig.

Die lokale Spracherkennung für deutsche Fragen lädt kein Audio hoch. **Neuer Chat**,
Abmelden oder Sperren beendet den flüchtigen Gesprächskontext.

## 11. Daten, Backup und Sicherheit

### Ein Backup erstellen

1. Öffne **Daten & Sicherheit → Backup**.
2. Wähle **Backup erstellen**.
3. Speichere die Datei mit der Endung `.saldonaut-backup` an einem sicheren Ort.
4. Bewahre mindestens eine Kopie auf einem anderen Datenträger auf.

Backups bleiben verschlüsselt. Für die Wiederherstellung benötigst du das Passwort,
das zum Zeitpunkt der Sicherung für das Profil galt.

### Ein Backup wiederherstellen

1. Öffne auf der Anmeldung **Aus Backup wiederherstellen** oder innerhalb der App
   **Daten & Sicherheit → Backup**.
2. Wähle die Sicherungsdatei.
3. Vergib einen Namen für das wiederhergestellte Profil.
4. Gib das Passwort des Backups ein.
5. Wähle **Backup importieren**.

Die Wiederherstellung legt ein zusätzliches Finanzprofil an. Bestehende Profile
und Daten bleiben unverändert.

### Passwort ändern

Unter **Daten & Sicherheit → Passwort ändern** kannst du das Passwort des aktuell
geöffneten Profils ersetzen. Erstelle danach ein neues Backup; ältere Backups
benötigen weiterhin das zum Sicherungszeitpunkt gültige Passwort.

### Profile kopieren oder anonymisieren

Finanzprofile lassen sich getrennt verwalten. Eine anonymisierte Kopie kann Beträge
und erkennbare Inhalte verändern, garantiert aber keine vollständige Anonymität.
Prüfe eine solche Kopie immer selbst, bevor du sie weitergibst.

## 12. Einstellungen

### Darstellung

Wähle unter **Einstellungen → Darstellung** zwischen **Hell**, **Dunkel** und
**Systemeinstellung**. Die Auswahl gilt bereits für die Anmeldung und reagiert bei
der Systemeinstellung auf Änderungen des Betriebssystems.

### Allgemein

- **Automatisch sperren nach:** legt fest, nach welcher Inaktivität Saldonaut
  gesperrt wird. Auch beim Minimieren wird gesperrt.
- **Sprache:** Deutsch, Englisch, Französisch oder Italienisch.
- **Ländereinstellungen:** steuern die Darstellung von Datum und Zahlen, ohne
  gespeicherte Werte zu ändern.
- **Standardwährung:** wird für neue Konten vorgeschlagen. Sie rechnet bestehende
  Beträge nicht um.

### Marktpreise

Saldonaut kann Tageskurse und Wechselkurse automatisch beziehen. Optionale
Marketstack- und Alpha-Vantage-Schlüssel werden verschlüsselt im aktuellen Profil
gespeichert. Ohne Schlüssel dient Yahoo Finance als Rückfall. Dabei werden
Wertpapierkennungen, Zeiträume und Währungspaare übertragen, jedoch keine
Kontostände oder Buchungen.

## 13. Häufige Fragen

### Warum stimmt mein Gesamtvermögen nicht mit der Summe aller sichtbaren Konten überein?

Prüfe, ob Konten als **Nicht im Gesamtvermögen** markiert sind, ob archivierte
Konten beteiligt sind und welche Bewertungswährung angezeigt wird. Negative
Kreditkartensalden werden in der Gesamtsumme berücksichtigt, erscheinen aber nicht
als positives Donut-Segment.

### Warum erscheint eine Buchung doppelt?

Öffne **Importierte Dateien** und prüfe, ob derselbe Zeitraum aus mehreren Dateien
oder demselben Auszug importiert wurde. Saldonaut erkennt identische Dateien und
bereits vorhandene Transaktionen; abweichende Beschreibungen, Daten oder
Quellformate können eine eindeutige Erkennung jedoch erschweren. Lösche nicht
vorschnell einzelne Datensätze, sondern kläre zuerst den betroffenen Import.

### Warum wird eine Fremdwährung als anderer CHF-Wert angezeigt?

Konten und Positionen können eine eigene Währung besitzen. Auswertungen verwenden
den gelieferten Wert in der ausgewiesenen Bewertungswährung. Prüfe Kontowährung,
Bewertungswährung, Kursdatum und verfügbare Wechselkurse.

### Warum fehlen aktuelle Marktwerte?

Prüfe Wertpapierkennung, Börse, Internetverbindung und den Bereich
**Einstellungen → Marktpreise**. Mit **Kurse jetzt aktualisieren** kannst du einen
neuen Abruf starten und konkrete Fehlermeldungen sehen.

### Ich habe mein Passwort vergessen. Was kann ich tun?

Es gibt keinen Passwort-Reset. Ein Profil und seine Backups können nur mit dem
jeweils gültigen Passwort geöffnet werden.

### Wo liegen meine Daten?

Saldonaut speichert Finanzprofile lokal und verschlüsselt im von Tauri für die
Anwendung vorgesehenen Datenverzeichnis des Betriebssystems. Verwende für Zugriff
und Sicherung die Funktionen in **Daten & Sicherheit**, statt interne Dateien
manuell zu verändern.

## 14. Vorgeschlagene Kontextlinks

| Ausgangspunkt | Linkziel |
| --- | --- |
| Anmeldung | `#help/getting-started` |
| Backup-Wiederherstellung | `#help/security/restore` |
| Importvorschau | `#help/import/review` |
| Vermögenschart | `#help/assets/chart` |
| Saldochart | `#help/transactions/balance-chart` |
| Kreditkarten-Warnung | `#help/transactions/cards` |
| Kategorie-Zuordnung | `#help/categories/mapping` |
| Marktpreise | `#help/settings/market-data` |
| Finanzchat-Datenschutz | `#help/chat/privacy` |

## 15. Redaktionelle und technische Leitlinien

- Kurze Absätze und konkrete Handlungsanweisungen verwenden.
- Begriffe exakt wie in der Oberfläche schreiben.
- Pro Artikel höchstens einen zentralen Screenshot einsetzen; zusätzliche Bilder
  nur dort, wo sie einen Schritt sichtbar besser erklären.
- Screenshots ausschliesslich mit dem synthetischen Demo-Profil erstellen.
- Bildunterschriften beschreiben die relevante Stelle, nicht den gesamten Screen.
- Warnungen nur bei tatsächlichem Risiko verwenden.
- Hilfetexte nicht an Versionsnummern koppeln, sofern der Ablauf unverändert bleibt.
- Suchbegriffe um gebräuchliche Varianten ergänzen, etwa „Bankauszug“, „Import“,
  „CSV“, „Kreditkarte“, „Depot“, „Backup“ und „Sicherung“.
- Alle Funktionen per Tastatur erreichbar halten; bei einem Sprung in einen Artikel
  erhält dessen Überschrift den Fokus.
- Inhalte für Deutsch, Englisch, Französisch und Italienisch über dieselbe
  Artikelstruktur pflegen. Fehlende Übersetzungen dürfen nicht unbemerkt gemischt
  in der Oberfläche erscheinen.
