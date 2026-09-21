# Saldonaut – verbindlicher Design-Leitfaden

## Geltung und Arbeitsweise

Dieser Leitfaden gilt für neue und geänderte Oberflächen. Vor UI-Arbeiten vollständig
lesen und die betroffenen bestehenden Komponenten sowie CSS-Regeln prüfen.
Ausdrückliche neue Designentscheidungen des Nutzers haben Vorrang; anschließend
den Leitfaden entsprechend aktualisieren.

Ziel: eine ruhige, gut lesbare Finanzanwendung mit eindeutigen Aktionen.
Keine eigene Farbwelt oder neue Komponentenvariante für jedes Feature erfinden.
Bestehende gemeinsame Klassen und Designvariablen wiederverwenden.

Dies ist der Soll-Standard, keine Behauptung, dass der gesamte Bestand bereits
konform ist. Abweichungen sind unten dokumentiert. Ein Refactoring des Designs
darf keine Buchungsregeln, Salden oder gespeicherten Entscheidungen verändern.

## 1. Farben und ihre Bedeutung

Die gemeinsamen Farbvariablen stehen in `src/styles/theme.css`. Layouts stehen
in `src/styles/application.css`; Feature-Styles verwenden dieselben Farbrollen.

### Helle und dunkle Darstellung

- Unter Einstellungen → Darstellung stehen Hell, Dunkel und Systemeinstellung zur
  Verfügung. Standard ist Systemeinstellung; Systemwechsel wirken sofort.
- Die Auswahl wird gerätebezogen außerhalb der verschlüsselten Finanzprofile
  gespeichert und gilt bereits für Anmeldung und Wiederherstellung. Die native
  App zeigt ihr Fenster erst nach dem Anwenden der Palette; der private WebView
  bleibt erhalten. Keine Finanzdaten in Darstellungseinstellungen speichern.
- Dunkle Palette: Seite `#0b1220`, Cards `#141e2e`, abgesetzte Flächen `#1c293c`,
  Haupttext `#f1f5f9`, Sekundärtext `#afbdd0`, Konturen `#394960`.
- Hauptaktionen bleiben Slate: `--color-primary` als Fläche mit
  `--text-on-action`, Hover `--bg-action-hover`. Im dunklen Modus Slate 700/600.
  Links, aktive Unterstriche und Fokus nutzen getrennt `--interactive`, im
  dunklen Modus helles Slate/Blaugrau. Keine weiße Buttonbeschriftung auf hellen Flächen.
- Flächen verwenden `--bg-surface`, `--bg-muted`, `--bg-hover`; Hinweise je
  `--bg-positive/warning/negative`, `--text-positive/warning/negative` und
  `--border-positive/warning/negative`. Finanzfarben behalten ihre Bedeutung.
- Kein Invertieren der Oberfläche oder Logos. Kategorie- und Markenfarben bleiben
  Datenkennzeichnungen. Native Formularcontrols erhalten das passende `color-scheme`.
- Canvas-Charts aktualisieren ihre Palette ohne Neuanlage, Zoom-Reset oder Verlust
  einer Messung. SVG-Charts verwenden dieselben Variablen.
- Neue Oberflächen immer in beiden Paletten prüfen, inklusive Fokus, Hover,
  Fehlern, Auswahl und Dialogen. Normaler Text mindestens 4,5:1 Kontrast.

| Rolle | Variable / Wert | Verwendung |
| --- | --- | --- |
| Primärfarbe | `--color-primary`: `#0f172a` (hell), `#334155` (dunkel) | Flächen von Hauptaktionen, aktiven Pills und Wizard-Schritten |
| Interaktion | `--interactive`: `#0f172a` (hell), `#dbeafe` (dunkel) | Links, aktive Unterstriche und Fokus |
| Primär-Hover | `--bg-action-hover` | Hover auf Slate-Hauptaktionen |
| Seitenfläche | `--bg-page`: `#f8fafc` | Regulärer Seitenhintergrund |
| Kartenfläche | `--bg-surface`: `#ffffff` | Cards, Tabellen, Formularfelder |
| Kontur | `--border-subtle`: `#e2e8f0` | Dezente Rahmen und Trennlinien |
| Haupttext | `--text-primary`: `#0f172a` | Überschriften, wichtige Inhalte und Aktionen |
| Sekundärtext | `--text-secondary`: `#64748b` | Hilfetexte, Beschriftungen, Metadaten |
| Positiver Akzent | `--accent-positive`: `#10b981` | Diagramme, positive Indikatoren und Branding |
| Positiver Wert | `--accent-positive-strong`: `#059669` | Positive Finanzwerte; Kontrast auf der konkreten Fläche prüfen |
| Negativer Akzent | `--accent-negative`: `#ef4444` | Negative Indikatoren; für kleine Texte ausreichend dunkle Variante wählen |
| Hinweis | Fläche `#ecfdf5`, Text `#022c22`, Rahmen `#a7f3d0` | Dezente Erklärung der Buchungslogik |
| Warnung | Fläche `#fffbeb`, Text `#92400e` | Ungeklärte Buchungen, unvollständige Auswertungen |

- Dunkelblau ist die Interaktionsfarbe, nicht Smaragdgrün.
- Keine flächig grünen Weiter-/Speichern-Buttons oder aktiven Stepper-Pills.
- Grün darf positive Werte, Erfolg und Empfehlung kennzeichnen, aber niemals
  allein ausdrücken, dass eine vorgeschlagene Entscheidung bereits gespeichert ist.
- Warnungen und Zustände immer zusätzlich mit Text oder Symbol kennzeichnen.
- Kategorie-Farben sind Datenkennzeichnungen, keine neue Palette für Buttons.
- Verteilungsdiagramme verwenden die zentralen, gedeckten Rollen
  `--chart-distribution-1` bis `--chart-distribution-8`; keine neonartigen
  Einzelfarben in Komponenten ergänzen.
- Vorhandene Variablen verwenden; keine gleichwertigen Hex-Farben je Feature kopieren.
  Neue wiederkehrende Rollen zentral definieren und hier dokumentieren.

## 2. Schrift und Zahlen

- Gemeinsamer Font-Stack: `Inter, ui-sans-serif, system-ui, -apple-system,
  BlinkMacSystemFont, "Segoe UI", sans-serif`. Keine neue Schrift laden.
  Inter ist nur verfügbar, wenn installiert; System-Fallbacks sind beabsichtigt.
- Formulare und Buttons erben die Schriftfamilie (`font: inherit`).
- Seitentitel: 30–34 px, Gewicht 700, leicht negative Laufweite.
- Card-/Abschnittstitel: 19–22 px, Gewicht 600–700.
- Fließtext und Formulare: 14–16 px, Gewicht 400, Zeilenhöhe 1.5–1.7.
- Labels: 13–14 px, Gewicht 500–600. Hilfetexte: 12–13 px.
- Haupt- und Nebenaktionen: 14 px, Gewicht 600; keine dünnen Abbrechen-Links.
- Die Anmeldung ist eine kompakte Ausnahme: Titel 20 px, Markenname 16 px.
- Geldbeträge tabellarisch ausrichten; gemeinsame Locale-Formatierung nutzen.
  Keine fest eingebauten Tausendertrennzeichen oder Datumsformate.
- Lange Namen und Übersetzungen müssen umbrechen können. Bei Kürzung bleibt
  der vollständige Inhalt zugänglich. Wesentliche Informationen nicht nur im Tooltip.

## 3. Flächen, Abstände und Ausrichtung

- Weiß auf hellem Slate; dezente Rahmen und Schatten statt vieler farbiger Kästen.
- Das Desktopfenster startet mit 1520 × 900 px, damit die vollständige
  Monatsvergleichstabelle einschließlich „Ø / Monat“ sichtbar ist. Die unterstützte
  Mindestbreite bleibt 920 px; schmalere Layouts dürfen horizontal scrollbare
  Datentabellen verwenden.
- Standard-Card: `dashboard-card`, Radius `--radius-card` (12 px),
  Schatten `--shadow-card`, Innenabstand normalerweise 24 px.
- Anmeldung: maximal 420 px breit, Radius 16 px, Innenabstand 32 px.
- Backup-Wiederherstellung auf der Anmeldung nur als dezenter, zentrierter Link.
  Der Link öffnet eine separate Ansicht statt eines zusätzlichen Formularblocks.
  Diese bietet eine Rückkehr zur Anmeldung; während der Wiederherstellung ist
  die Rückkehr gesperrt. Beim Ansichtswechsel den Tastaturfokus passend setzen.
  Im Restore-Screen 8–16 px zwischen Titel und Erklärung sowie jeweils 24 px
  zwischen Erklärung, Dateiauswahl-Button und Rückkehr-Link vorsehen.
  Der Dateiauswahl-Button heißt „Sicherungsdatei auswählen …“.
- Abstände aus 4, 8, 12, 16, 24 und 32 px verwenden. Verwandte Elemente eng,
  eigenständige Abschnitte mit 24–32 px Abstand gruppieren.
- Labels stehen über dem Feld. In kompakten Filterleisten dürfen Label und Feld
  nebeneinander stehen, mit mindestens 8 px Abstand.
- Aktionen innerhalb einer Zeile mit Flexbox und `align-items: center` ausrichten.
  Keine manuell verschobenen Texte oder Leerzeichen zur Ausrichtung.

## 4. Buttons und Links

| Art | Gestaltung | Beispiele |
| --- | --- | --- |
| Primär | Dunkelblau, weiße Schrift, Hover Slate 800 | Weiter, Speichern, Karte einrichten |
| Sekundär | Weiß, Slate-Kontur, dunkle Schrift | Zurück, Als Muster wählen |
| Tertiär | Transparent, dunkle Schrift, Gewicht 600 | Abbrechen, optionale Nebenaktion |
| Destruktiv | Klar rot gekennzeichnet, explizite Beschriftung | Löschen |

- Gemeinsame Klassen `primary-button` und `secondary-button` verwenden.
- Aktionshöhe mindestens 44 px; horizontales Padding 16–17 px, Radius 9 px.
  Die bestehenden Anmeldecontrols verwenden einheitlich 12 px Radius.
- Anmeldung und Backup-Dateiauswahl auf dem Restore-Screen nutzen denselben
  vollbreiten Hauptbutton: 44 px Höhe, 14 px Schrift, Gewicht 500, 12 px Radius.
- Alle Aktionen einer Footer-Zeile haben dieselbe Höhe, Schriftgröße und
  vertikale Zentrierung. Das gilt auch für Links, die wie Buttons aussehen.
- In der Regel eine visuell dominante Hauptaktion pro Handlungsbereich.
- Navigation ist ein `<a href>`, eine Zustandsänderung ein `<button type="button">`.
- Links in Hinweisen sind deutlich erkennbar: unterstrichen oder als Button.
  Kein unauffälliger Link im identischen Stil wie der umgebende Fließtext.
- Fokus sichtbar: 2 px `--interactive`, 2–3 px Abstand. Nicht nur Farbe beim Hover ändern.
- Während des Speicherns Aktionen sperren und Fortschritt verständlich anzeigen.
  `aria-disabled` allein sperrt einen Link nicht; Navigation zusätzlich verhindern.

## 5. Formularfelder, Tabellen und Zustände

- Inputs und Selects: 44 px Höhe, weiß, Slate-Rahmen, Radius 10–12 px;
  innerhalb eines Formulars eine einheitliche Variante verwenden.
- Sichtbare Labels; bei kompakten Tabellencontrols zusätzlich passende zugängliche Namen.
- Fehler am betroffenen Bereich erklären. Eingaben nicht stillschweigend verwerfen.
- Noch nicht abgerechnete Kreditkartenbuchungen in Kontowährung dürfen in der
  Importvorschau vorläufig mit Einkaufsdatum und Originalbetrag erscheinen. Ein
  Warnhinweis erklärt die Vorläufigkeit. Fremdwährungen ohne endgültigen Betrag
  in Kontowährung werden nicht geschätzt oder stillschweigend umgerechnet. Bei
  später überlappend importierten Jahres- oder Monatsauszügen werden ausschließlich
  stabil identifizierte vorläufige Kartenbuchungen atomar mit den endgültigen
  Abrechnungsdaten aktualisiert. Die Importvorschau und Erfolgszusammenfassung
  nennen die Anzahl dieser Aktualisierungen.
- Tabellen standardmäßig kompakt halten. Keine großen Aktionsblöcke in jeder
  Zeile der Haupt-Transaktionsübersicht; dort das Drei-Punkte-Menü verwenden.
- Dedizierte Einrichtungsansichten dürfen direkte Zeilenaktionen haben.
- Buchungs-Drilldown zeigt Kategorien als Text mit Farbpunkt, Herkunft als Tooltip.
  „Kategorie ändern …“ im Drei-Punkte-Menü öffnet die Inline-Auswahl mit Speichern
  und Abbrechen. Erst dort auf die Wirkung für passende Händlerbuchungen und
  zukünftige Importe hinweisen. Auswahl allein speichert nicht; Fehler im Editor
  anzeigen, Escape bricht ab und gibt den Fokus an die Zeilenaktion zurück.
  Kein permanenter Sortierhilfetext über der Drilldown-Tabelle; Richtungspfeile
  bleiben sichtbar. Trefferzahl und gefilterte Summe nur bei aktiven Detailfiltern.
  Kein zusätzlicher „Grösste Beträge zuerst“-Button; Betragssortierung ausschließlich
  über den Spaltenkopf.
  Im Buchungsmenü keine Zwischenüberschrift für Umbuchungen: Aktionen direkt als
  „Als Kartenausgleich markieren“ und „Als Übertrag zwischen eigenen Konten markieren“
  benennen. Das Drei-Punkte-Menü enthält nur Aktionen für die jeweilige Buchung;
  kein Navigationslink zu „Umbuchungen & Ausgleiche“ (über den Reiter erreichbar).
- „Als Duplikat entfernen …“ ist eine destruktiv dargestellte, aber reversible
  Zeilenaktion. Ein Bestätigungsdialog zeigt Beschreibung, Datum, Konto und Betrag
  und erklärt die Auswirkung. Die Buchung bleibt mit ihrer Importspur gespeichert,
  wird aber aus Salden und Auswertungen ausgeschlossen; ein erneuter Import aktiviert
  sie nicht wieder. Unter „Importierte Dateien“ werden entfernte Dubletten je Import
  ausgewiesen und wiederhergestellt. Einen vollständig falschen Import entfernt
  weiterhin die Importverwaltung.
- Unter „Importierte Dateien“ steht eine manuell ausgelöste Bestandsprüfung für
  ältere Importe. Sie gruppiert eindeutige und mögliche Dubletten mit Importquelle,
  verändert keine Buchung automatisch und verwendet für eine Entfernung denselben
  reversiblen Bestätigungsdialog. „Kein Duplikat“ speichert die geprüfte Gruppe
  dauerhaft, damit legitime Mehrfachzahlungen bei späteren Läufen nicht erneut
  erscheinen. Neue oder veränderte Kandidaten bleiben weiterhin sichtbar.
- Tabellensortierung direkt an den Spaltenüberschriften mit `expense-sort`,
  Richtungspfeilen und Tastaturbedienung anbieten, nicht als separates Dropdown.
  Dies ist der Standard für datenreiche Tabellen, insbesondere mit Geldbeträgen:
  Datum und Betrag sowie sinnvolle Textspalten auf-/absteigend sortierbar machen.
  Aktive Richtung mit `aria-sort` auszeichnen; Checkbox- und Aktionsspalten nicht
  sortierbar machen. Beträge wie in der Transaktionsübersicht nach absoluter
  Betragshöhe sortieren, ohne Vorzeichen, Währung oder gespeicherte Daten zu ändern.
- Ausnahme für die Mustertabelle im Kreditkarten-Wizard: genau ein Auswahlbutton
  pro Zeile, 36 px Höhe, Schriftgewicht 500, vertikal mittig. Keine Zeilen-Akkordeons
  oder zusätzlichen Einordnungsbuttons. Erstattungen werden in der Kartenübersicht
  bearbeitet. Auswahl verhält sich wie eine Radio-Auswahl; erneutes Klicken löscht
  sie nicht. „Weiter“ wird erst nach gültiger Mustervorschau freigegeben.
- „Empfohlener Ausgleich“ erscheint als nicht interaktives Pill-Badge mit heller
  Emerald-Fläche, dezenter Kontur und dunkler grüner Schrift, nicht als Textlink.
- Einfachauswahl über Radio-Button oder eindeutig beschrifteten Button mit
  `aria-pressed`. Aktuelle Auswahl zusätzlich textlich markieren: „✓ Ausgewählt“.
- Empfehlung, gespeicherter Zustand und vorgemerkte Änderung sind drei verschiedene
  Zustände. Vorschläge nicht als bereits erledigt darstellen.
- Immer Lade-, Leer-, Fehler-, ausgewählten und deaktivierten Zustand berücksichtigen.
- Kartenübersicht: ein Kontoselektor mit „Alle Kartenkonten“, Status je Konto und
  kontospezifischem Setup-Link. Grün bedeutet eine gespeicherte Gutschriftregel
  für die Konto-ID und Kontowährung, nicht nur einen passenden Namen oder eine
  manuelle Einzelmarkierung. Fehlende Regeln gelb anzeigen; Ladefehler niemals
  als „nicht eingerichtet“ interpretieren. Den Onboarding-Banner ab der ersten
  konfigurierten Karte kompakt darstellen. Ungeklärte Gutschriften nur innerhalb
  der gewählten Kontenauswahl zählen; verschiedene Währungen nicht zusammenrechnen.
  Im oberen Banner die eingerichteten Karten zusätzlich mit Namen und Währung
  auflisten; diese Gesamtliste bleibt unabhängig vom Tabellen-Kontofilter.
  Kartenhistorie standardmäßig auf das aktuelle Kalenderjahr und 50 sichtbare
  Buchungen begrenzen, mit Vorjahr, eigenem Zeitraum, gesamter Historie und Suche.
  „Weitere laden“ zeigt jeweils 50 zusätzliche Treffer. Filterwechsel setzt die
  Anzeigegrenze zurück. Ungeklärte Gutschriften kontobezogen über alle Jahre zählen;
  ihr Hinweis öffnet die gesamte Historie und setzt die Textsuche zurück.
- Horizontales Scrollen auf breite Tabellen begrenzen, nicht auf die gesamte Seite.
- Automatische Umbuchungsregeln mit Name, Konto, Währung, Richtung, Textanfang
  und Wirkung darstellen. Regeln bearbeiten und deaktivieren können; die
  Bedingungen sind kein Tabellenfilter. Vor dem Anwenden Treffer zeigen.
  Zukünftige Importe und historische Buchungen getrennt bestätigen lassen;
  manuelle Entscheidungen und bisherige Markierungen außerhalb der Treffer erhalten.

## 6. Navigation und Einrichtungsassistenten

- Die Hauptnavigation der Seitenleiste ist ohne sichtbare Gruppenüberschriften in
  drei Bereiche gegliedert: Übersicht, Vermögen und Transaktionen; danach
  Steuerhistorie und Finanzchat; danach Import, Banken & Konten und Kategorien.
  Dezente Trennlinien mit kompaktem Abstand kennzeichnen die Gruppen. Für
  assistive Technologien bleiben die Bereiche als benannte Gruppen erkennbar.
- Die globale „Hilfe“ steht im unteren Navigationsbereich direkt oberhalb von
  „Daten & Sicherheit“. Sie öffnet eine vollständig lokal verfügbare Hilfeseite
  mit Suche, eigener benannter Themennavigation und direkten Artikelankern. Auf
  breiten Ansichten stehen Themenindex und Artikel nebeneinander, auf schmalen
  Ansichten untereinander. Ein Direktlink setzt den Fokus auf die Artikelüberschrift.
  Screenshots verwenden ausschließlich synthetische Demo-Daten. Kontexthilfen
  dürfen direkt in den passenden Hilfeartikel verlinken.
  Der Import-Hilfeartikel enthält ein klares Privacy Statement: Quelldokumente
  werden lokal verarbeitet, nicht hochgeladen oder in das Finanzprofil kopiert
  und nach der Analyse aus dem Arbeitsspeicher verworfen. Der Text unterscheidet
  dies ausdrücklich von den bestätigten Finanzdaten, dem Dateinamen und dem
  technischen Fingerabdruck, die verschlüsselt im Profil gespeichert werden.
- Banken & Konten: Zeilenaktionen im Drei-Punkte-Menü (Bearbeiten, Einbezug ins
  Gesamtvermögen, Archivieren/Aktivieren bzw. Löschen bei leeren Konten).
  Archivieren/Löschen absetzen und bestätigen lassen. Ausschluss vom Vermögen
  und Archivstatus bleiben direkt in der Zeile sichtbar. „Bank oder Anbieter
  hinzufügen“ ist die sichtbare Hauptaktion der Seite und legt zunächst nur den
  Anbieter an. Ein Konto wird anschließend über „Konto hinzufügen“ im Menü des
  jeweiligen Anbieterkopfs ergänzt. Menü mit Tastatur, Escape/Fokusrückkehr
  und Schließen bei Außenklick bedienen können.
  Anbieter sind eigenständige Stammdaten: Die Kontoanlage wählt einen bestehenden
  Anbieter oder legt ausdrücklich einen neuen an; kein freies Anbietertextfeld für
  jedes weitere Konto. Name, Typ und Logo werden ausschließlich im Drei-Punkte-Menü
  des Anbieterkopfs bearbeitet und gelten für alle zugehörigen Konten. Bei der
  erstmaligen Anlage kann das zentrale Logo bereits optional ausgewählt werden; es
  wird zusammen mit dem Anbieter gespeichert. Dort steht
  außerdem „Konto hinzufügen“ mit bereits vorausgewähltem Anbieter bereit.
  Keine dauerhafte Erklärung zu Archivierung und Löschung im Seitenkopf;
  solche allgemeinen Erläuterungen gehören in eine gemeinsame Hilfe. Konkrete
  Folgen werden weiterhin unmittelbar im jeweiligen Bestätigungsdialog erklärt.
  Kontenzeilen formatieren den aktuellen Wert mit dessen gelieferter
  Bewertungswährung (`balanceCurrency`), nicht pauschal mit der Kontowährung.
  Ein fehlender oder exakter Nullsaldo erscheint konsistent als `CHF 0.00` in
  Sekundärtextfarbe. Der rechtsbündige Betragsblock hat feste Breite und
  tabellarische Ziffern. Ausgeschlossene Konten zeigen „Nicht im Gesamtvermögen“,
  archivierte Konten „Archiviert“ als dezentes Status-Badge direkt in der Zeile.
  Die Standardansicht bleibt vertikal kompakt: Institutszeilen sind 56 px hoch
  mit 36-px-Logo und inline gesetzter Kontenanzahl, Kontenzeilen mindestens 64 px.
  Kontotyp, Referenz und Positions-/Importanzahl stehen in einer gemeinsamen
  Metadatenzeile. Zwischen Institutskarten liegen 10 px; die 44-px-Aktionsfläche
  und ein kontrollierter Umbruch langer Metadaten bleiben erhalten.
  Die Verwaltung manueller Positionen ist eine eigene Detailansicht. Oberhalb
  ihrer Card steht ein lokalisierter sekundärer Button „← Zurück zu Banken &
  Konten“; kein „Abbrechen“ im Card-Header. Beim Öffnen erhält die Rückkehraktion
  den Fokus. Formular-Abbrechen bleibt unten neben der Speicheraktion; Escape
  verwirft keinen begonnenen Positionsentwurf.

- „Vermögen nach Anbieter“: gemeinsamer Donut links, Liste mit Logos, Farbpunkt,
  Betrag und Anteil rechts; auf schmalen Fenstern untereinander. Keine zusätzlichen
  Balken. Alle Anbieter einzeln zeigen, Details bei Hover und Tastaturfokus.
  Ein Mausklick auf ein nicht auswählbares Ringsegment zeigt keinen Browser-
  Standardrahmen; der eigene sichtbare Tastaturfokus für die Detailanzeige bleibt.
  Im Ringzentrum Label, Betrag und Anteil niemals innerhalb eines Worts oder Betrags
  umbrechen. Prozentanteile einheitlich mit genau einer Nachkommastelle darstellen.
  Ring und Prozente basieren auf positiven Anbietersalden; negative Salden in der
  Liste benennen und im Gesamtvermögen der Ringmitte weiterhin berücksichtigen.
  Bei fehlenden positiven Salden neutralen leeren Ring mit Hinweis zeigen.

- „Vermögenspositionen“ bleibt eine kompakte Liste ohne zusätzliche Tabellenleiste
  und ist standardmäßig nach absoluter Betragshöhe absteigend sortiert. Beträge
  stehen rechtsbündig mit tabellarischen Ziffern; ein fehlender oder exakter
  Nullsaldo erscheint konsistent als `CHF 0.00` in Sekundärtextfarbe. Kontotypen
  werden lokalisiert und niemals als interne Schlüssel angezeigt. Ein Stichtag
  wird nur dargestellt, wenn ein Saldo- oder Bewertungsdatum vorliegt.

- Kategorien: Drei-Punkte-Aktionen pro Zeile; Name und Farbe direkt inline
  bearbeiten, jeweils nur eine Kategorie. Speichern/Abbrechen bleiben in der
  betroffenen Zeile. Oben nur „Neue Kategorie“ als dunkelblaue Hauptaktion.
  Der kurze Einleitungssatz nutzt auf breiten Ansichten die verfügbare Zeile und
  wird nicht durch die allgemeine 700-px-Textbreite künstlich umgebrochen; auf
  schmalen Ansichten bleibt natürlicher Umbruch erlaubt.
  Kategoriezeilen verwenden einen 8–10 px großen Farbpunkt statt eines Balkens,
  rund 8 px vertikales Padding und weiterhin eine 44-px-Aktionsfläche.
  Zusammenführen und Löschen
  öffnen einen Bestätigungsdialog mit Zielkategorie, Buchungs-/Regelanzahlen
  und Erklärung der Wirkung auf zukünftige Importe. Keine stillen Datenverluste.
  Branchenzuordnungen als kompakte Zweispaltenliste: Branche mit Buchungsanzahl
  links, Kategorieauswahl rechts; 8 px vertikaler Zeilenabstand und weiterhin
  mindestens 44 px hohe Auswahlfelder. Die Spalten heißen „Kreditkarten-Kategorie“
  und „Saldonaut-Kategorie“, damit Quellwert und eigene Zuordnung eindeutig sind.
  Auf schmalen Ansichten stehen die Zeilen untereinander und die Zielbeschriftung
  wird direkt über dem Auswahlfeld wiederholt.

- Kategorie-Mehrfachauswahl ohne Checkbox-Kästchen: Nur ausgewählte Zeilen zeigen
  ein Häkchen zusätzlich zum farbigen Hintergrund. Zeilen bleiben ausgerichtet
  und als Buttons mit aria-pressed per Tastatur bedienbar.
  Daneben ein Donut mit den acht größten positiven Kategoriesummen; Rest als
  „Weitere Kategorien“. Die danebenstehende Hauptliste verwendet dieselbe
  Gruppierung; „Weitere Kategorien“ ist eine mit Pfeil und `aria-expanded`
  gekennzeichnete Aufklappzeile. Aufgeklappt erscheinen die gebündelten Kategorien
  einzeln mit Betrag und Anteil und bleiben für den Buchungs-Drilldown auswählbar.
  Im Ring bleibt die Restgruppe unabhängig vom Aufklappzustand ein einziges,
  zusammenhängendes Slate-Segment und übernimmt dessen `aria-expanded`-Zustand.
  Unterkategorien verwenden 6-px-Farbpunkte; Beträge und Anteile sind tabellarisch
  ausgerichtet und der Aufklapppfeil hat 8 px Abstand zum nachfolgenden Inhalt.
  Die kompakten, mindestens 44 px hohen Zeilen
  zeigen Kategorie und Buchungsanzahl inline mit einem 8-px-Farbpunkt. Kein
  permanenter Anleitungstext für Klick-, Zieh- oder Shift-Auswahl.
  Farben und Mehrfachauswahl mit der Liste synchronisieren.
  Negative Nettokategorien nicht als positive Segmente darstellen, sondern erläutern.
  Auf schmalen Fenstern Diagramm über der Liste. Bank und Konto der Auswertung
  sind Mehrfachauswahlen (leere Auswahl = alle); Banken und Konten werden kombiniert.
  Der Saldochart behält seine unabhängigen Einfachfilter.
  „Einnahmen und Ausgaben“ fasst Auswertungsfilter, die drei rahmenlosen Kennzahlen
  und die Kategorie-/Monatsaufteilung in einer gemeinsamen Card zusammen.
  Einfache Chartfilter und Mehrfachfilter der Auswertung verwenden dieselbe
  44-px-Feldhöhe, 10-px-Radius und Select-Anmutung; die unterschiedliche Auswahlart
  bleibt über Beschriftung und Verhalten eindeutig. In der Aufteilung steht zuerst
  der dominante Typ „Ausgaben/Einnahmen“, danach die dezentere Darstellung
  „nach Kategorie/nach Monat“. Die beiden Darstellungsoptionen belegen gleich
  breite Spalten und haben dieselbe Mindesthöhe, unabhängig von Beschriftung und
  aktivem Zustand. „Typ“ und „Darstellung“ sowie ihre jeweils 44 px hohen Controls
  beginnen auf derselben horizontalen Linie; unterschiedliche Control-Höhen dürfen
  die Labels nicht gegeneinander verschieben. Die Differenz ist bei positivem Wert grün und bei
  negativem Wert rot. Summen- und Auswahlzeilen bleiben neutral statt mintgrün.
  Der Chart reserviert oben ausreichend Skalenraum für Werte- und Fokuslabels.
  Aufteilung mit dezenter Trennlinie absetzen. Zeitraum und Monatsdurchschnitt
  sind kompakte Zusatzinformationen statt weiterer Cards; keine doppelte Gesamtsumme.
  Der auswahlabhängige Durchschnitt steht bei der Aufteilung, die Auswahlsumme
  bleibt in deren Statusleiste. In der Monatsdarstellung stehen Währungshinweis und
  Jahreswahl rechts in derselben Zeile wie die Aufteilungscontrols; kein zusätzlicher
  Einleitungstext. Die Vergleichstabelle verwendet 6–8 px vertikales Zellpadding,
  kompakte Monatsspalten und kleine Kategorie-Farbpunkte. Leere Monate treten
  optisch zurück. Hohe Ausgaben, Hover und Auswahl werden neutral in Slate statt
  positivem Grün gekennzeichnet. In der Monatsmatrix markiert Ziehen mit der linken
  Maustaste einen rechteckigen Zellbereich; eine neutrale Statuszeile zeigt Anzahl
  und Summe der darin enthaltenen Beträge. Strg-Ziehen oder Strg-Klick ergänzt unter
  Windows weitere getrennte Bereiche beziehungsweise Zellen; auf macOS übernimmt
  Cmd dieselbe Funktion. Überlappende Bereiche zählen Werte nur einmal. Ein normaler
  Werteklick öffnet weiterhin den Monats-Drilldown, Escape hebt die Bereichsauswahl
  auf. Saldochart und Buchungsdetails bleiben separat.

- Unter „Alle Transaktionen“ keinen allgemeinen Banner zu neutralisierten
  Kartenabrechnungen anzeigen; konkrete Hinweise auf ungeklärte Gutschriften bleiben.
  Den Kategorie-Klickhinweis nur an der Kategorieübersicht zeigen, nicht im Seitenkopf.
  Saldoverlauf mit derselben interaktiven TradingView-Komponente wie Vermögen,
  kompakten Zeiträumen (Standard YTD), Lineal, Reset und eigener Kontexthilfe.
  Im Saldoverlauf beginnt die Zeitraumleiste links ohne vorgelagerten Trennstrich
  oder zusätzlichen Einzug; die Instrument-Trennung im Vermögenschart bleibt erhalten.
  Bank-/Kontofilter erhalten, keine Benchmarks, Datenbasis- oder Tageswertezähler.
  Zeitraumwahl filtert die Auswertung; Canvas-Zoom und Verschieben ändern sie nicht.
  Saldochart ausschließlich für aktive Privat-/Sparkonten nach gespeichertem
  Kontotyp (cash/savings), auch bei expliziter Einzelauswahl. Eigene Bank-/Kontofilter
  für den Chart und die Auswertung darunter; Kreditkarten bleiben in der Auswertung.
  Keine namensbasierte Erkennung oder automatische Umklassifizierung bestehender Konten.

- Daten & Sicherheit, Einstellungen und Transaktionen verwenden dieselbe
  Unterstrich-Navigation unter dem Seitentitel. Der aktive Reiter bleibt dauerhaft
  markiert; keine Sprungknöpfe zu gleichzeitig sichtbaren Abschnitten.
  Lokale Reiter verwenden `SectionTabs`/`SectionPanel` mit `aria-selected`,
  zugeordneten Panels und Pfeiltasten/Home/End. Inaktive Panels bleiben verborgen
  montiert, damit Entwürfe beim Reiterwechsel erhalten bleiben. Routennavigation
  bleibt ein Link mit `aria-current="page"` und erhält bestehende Direktlinks.
- Profilverwaltung: Keine zweite Reiterleiste für Aktionen. Unter der Profilauswahl
  stehen „Neues Profil“ und „Kopie erstellen …“ als sekundäre Buttons. Der
  Kopierdialog bietet unveränderte und anonymisierte Kopien an. Löschen steht
  räumlich getrennt darunter; beim Hauptprofil nur den Hinweis statt eines
  deaktivierten Löschen-Buttons anzeigen.
  Bei anonymisierten Kopien Methode, Faktor, Buchungstexte, Name und eigenes Passwort erst im
  Dialog zeigen. Original und aktive Auswahl bleiben unverändert. Die Kopie erst
  nach erfolgreicher Bearbeitung und Integritätsprüfung verfügbar machen; bei
  Fehlern keine halbfertige Kopie anbieten. Keine permanente Warnung im normalen
  Verwaltungsbereich. Im Dialog die Grenzen der Anonymisierung erklären, besonders
  erkennbare Verhältnisse bei festem Faktor und verbleibende persönliche Angaben.
  Escape/Abbrechen mit Fokus-Rückkehr; während der Erstellung schließen sperren.
- Vermögensverlauf: lokal gebündelte Lightweight Charts, Emerald-Kurve und
  standardmäßig YTD (laufendes Kalenderjahr). Ein expliziter Berichtszeitraum
  hat weiterhin Vorrang. Die übrigen Zeiträume bleiben auswählbar.
  Der Chart verwendet
  eine integrierte Toolbar direkt über dem Canvas (48 px Mindesthöhe). Links
  die durchsuchbare Konto-/Depotauswahl und „Vergleichen“, mittig Zeiträume,
  rechts Lineal-, Reset- und Hilfe-Icons mit zugänglichen Namen. Bei schmalen Fenstern
  kontrolliert umbrechen; keine horizontale Seitenüberbreite. Keine zusätzliche
  Chart-Überschrift oder wiederholte Zeitraum-Metadaten über dieser Leiste.
  Benchmarks SMI / S&P 500 werden erst nach Auswahl aus der vorhandenen Yahoo-
  Anbindung geladen. Beide Kurven starten am ersten gemeinsamen Datum bei 100,
  nur bei positivem Vermögensstart. Kein FX-/Dividenden- oder Cashflow-bereinigter
  Renditevergleich; dies in der Chart-Kontexthilfe erklären. Legende und Achsen verwenden
  Indexpunkte, die Kennzahlen darüber bleiben unverändert. Ohne gemeinsame Daten
  bleibt der normale Vermögenschart sichtbar, mit verständlichem Hinweis.
  Kein Finanzdatenversand an die Kursquelle, keine Konten-/Bewertungsänderung.
  Vergleichsfehler und Wiederholen im Auswahlmenü anbieten.
  Das Vergleichsmenü enthält keine dauerhaften Erklärtextblöcke. Das Fragezeichen
  rechts öffnet einen Dialog zu Auswahl, Zeiträumen, Navigation, Messen/Tastatur,
  Benchmark-Grenzen und Datenquelle/Datenschutz. Escape und Schließen führen den
  Fokus zum Hilfe-Button zurück; die Chart-Auswahl und Messung bleiben erhalten.
  Der Chart verwendet weiterhin
  Slate-Controls. Mausrad/Pinch zoomt; Ziehen verschiebt nur den Ausschnitt.
  Kennzahlen und Auswertungszeitraum bleiben dabei unverändert. Explizite
  Zeitraumknöpfe steuern weiterhin die Auswertung. Kompakte 32-px-Pills mit
  1M, 6M, YTD, 1J, 3J, 5J und Max; eigener Zeitraum unter „…“. Keine Zoom-/Pfeil-
  Buttons und keine statischen Datenzähler oder Hilfetextblöcke. Reset als 16-px-
  Icon rechts, Mess-Toggle mit Lineal daneben (bewusste Ausnahme: aktiv hellgrün).
  Messung per Shift-Ziehen oder zwei Punkten (Pfeile/Enter), Ergebnis als dunkles,
  am Endpunkt begrenztes Floating-Tooltip mit Betrag, Prozent und Kalendertagen.
  Tastatur-Zoom mit +/− erhalten; Bedienhinweise für Screenreader verfügbar halten.
  AreaSeries mit geraden Verbindungen, Emerald-500-Linie und Verlauf 28% bis 0%;
  CrosshairMode.Normal bedeutet frei bewegliches Fadenkreuz, nicht Magnetmodus.
  Kompakte rechtsbündige Attribution mit TradingView-Link, Copyright im aufklappbaren
  Lizenzhinweis und in den ausgelieferten Lizenzen; keine externen Datenfeeds.
- Import und Importverwaltung bilden einen Seitenleistenpunkt „Import“ mit den
  Reitern „Dateien importieren“ und „Importierte Dateien“. Darunter bleibt die
  Auswahl Bankauszüge/Steuererklärungen beim Reiterwechsel erhalten. Bestehende
  Direktlinks bleiben erreichbar; erfolgreiche Bankimporte verlinken ihre Einträge.
- Aktiver Tab: Text und Unterstrich Dunkelblau, Gewicht 600, `aria-current="page"`.
- Aktueller Schritt: Dunkelblau mit Weiß, `aria-current="step"`.
- Vergangene Schritte: dezentes Slate 200; zukünftige Schritte: Slate 100.
  Übersprungene Schritte nicht mit einem Häkchen als fachlich erledigt ausweisen.
- Schrittbenennungen beschreiben die Aufgabe. Beim Karten-Setup ausdrücklich
  „Zahlungseingang auf der Karte“ (+) und „Abbuchung vom Bankkonto“ (−)
  unterscheiden; Konto sichtbar benennen und die jeweilige Kontoseite erklären.
- Keine doppelte Auswahl derselben Buchung in Dropdown und Tabelle.
- Wizard-Footer innerhalb der Card: Zurück/Abbrechen links, Überspringen/Weiter
  rechts. Bei langen Ansichten sticky, mit weißem Hintergrund und oberer Kontur.
- Überspringen nur bei optionalen Schritten; bei vorhandenen Änderungen vor dem
  Verwerfen nachfragen. Bereits gespeicherte Daten bleiben unberührt.
- Vorschau und abschließende Bestätigung vor persistenten Massenänderungen.
- Footer darf keine Inhalte oder Tastaturfokus verdecken; bei schmalen Fenstern
  Aktionen umbrechen und ausreichend Scroll-Abstand vorsehen.

## 7. Sprache, Branding und technische Umsetzung

- Alle UI-Texte mit `t`/`tr` und Übersetzungen für Deutsch, Englisch, Französisch
  und Italienisch. Labels auch mit längeren Übersetzungen prüfen.
- Anmeldung und Wiederherstellung bieten oben rechts eine kompakte Sprachauswahl
  mit den Eigennamen Deutsch, English, Français und Italiano. Die Auswahl wirkt
  sofort, wird gerätebezogen außerhalb der Finanzprofile gespeichert und nach dem
  Entsperren als Sprache des gewählten Profils übernommen.
- Kurze, konkrete Handlungsaufforderungen statt interner Begriffe wie Flags.
- Logo-Master: `public/saldonaut.svg`. Header, Favicon und Plattform-Icons
  daraus ableiten; keine abweichenden Inline-Logos neu zeichnen.
- Die Anmeldung verwendet das ruhige Orbitmotiv `public/saldonaut-login.png` als
  vollflächigen Hintergrund. Auf breiten Fenstern bleibt die Formularkarte rechts
  und das Motiv links sichtbar; auf schmalen Fenstern wird die Karte zentriert.
  Das Motiv enthält keinen Text und keine fremden Science-Fiction-Elemente.
- Die README verwendet `docs/screenshots/saldonaut-readme-hero.png` als textfreies
  Markenmotiv in derselben dunkelblau-smaragdgrünen Orbitwelt. Alte Screenshots
  mit überholtem Branding werden nicht als aktuelle Produktansichten gezeigt.
- Technische Altkennungen für Bundle-ID, Darstellungseinstellung, Datendatei und
  bestehende Datenbankschemata bleiben aus Kompatibilitätsgründen unverändert und
  sind nicht Teil der sichtbaren Marke. Neue Sicherungen verwenden
  `.saldonaut-backup`; der Dateidialog akzeptiert weiterhin bestehende
  `.finanzblick-backup`-Dateien zur Wiederherstellung.
- `npm run icons` generiert Plattform-Assets. Asset-Erzeugung ist kein Nachweis
  eines getesteten Windows-/macOS-Pakets.
- Gemeinsame Komponenten und Klassen zuerst prüfen, Feature-CSS auf das Feature
  begrenzen. Keine globalen Element-Overrides aus Feature-Dateien hinzufügen.
- Gemeinsame Designänderungen gehören in gemeinsame Styles; die Importreihenfolge
  in `src/App.css` und CSS-Spezifität berücksichtigen. Kein `!important` als Standardlösung.
- Wiederkehrende neue Controls als gemeinsame Komponente/Klasse umsetzen,
  statt nur deren Farben und Abstände in mehrere Dateien zu kopieren.

## 8. Bekannte Abweichungen im Bestand

- Gemeinsame Navigation, Textbuttons, Sortieraktionen und Fokusrahmen wurden
  für 0.5.9 auf Slate vereinheitlicht; Finanzwerte und Statusfarben bleiben erhalten.
- Gemeinsame Aktionsbuttons verwenden 14 px / Gewicht 600 / mindestens 44 px;
  dokumentierte kompakte Varianten wie Wizard-Musterauswahl bleiben ausgenommen.
- Radius, Input-Höhen und Fokusfarben sind noch nicht überall vereinheitlicht.
- Gemeinsame Tab-Farben liegen in `application.css`, nicht im Karten-Feature.
- `styles/application.css` und `styles/management.css` sind noch große Sammeldateien.

Diese Liste begründet keine stillschweigende globale Neugestaltung. Bei Arbeiten
am jeweiligen Bereich die Abweichung kontrolliert beheben und Regressionen prüfen.

## 9. Abnahmecheck für jede UI-Änderung

- [ ] Farben, Typografie, Höhen und Radien entsprechen diesem Leitfaden.
- [ ] Bestehende Klassen/Komponenten wiederverwendet; keine unnötigen Varianten.
- [ ] Primäre Aktion erkennbar; Links sichtbar; Abbrechen und Weiter korrekt ausgerichtet.
- [ ] Lade-, Leer-, Fehler-, Auswahl- und deaktivierte Zustände berücksichtigt.
- [ ] Tab-Reihenfolge, Enter/Space, sichtbarer Fokus und zugängliche Labels geprüft.
- [ ] Vorschläge, Entwürfe und gespeicherte Entscheidungen eindeutig unterschieden.
- [ ] Übersetzungen vollständig; lange Kontonamen und mehrere Karten berücksichtigt.
- [ ] Normales und schmales unterstütztes Fenster sowie vergrößerte Darstellung geprüft.
- [ ] TypeScript, relevante Tests und Frontend-Build erfolgreich.
- [ ] Screenshot/visuelle Prüfung der betroffenen Ansicht, einschließlich Fokuszustand.
      Falls nicht möglich: ausdrücklich als ungeprüft nennen; ein Build ersetzt diese Prüfung nicht.
- [ ] Plattformkompatibilität nur dann als verifiziert bezeichnet, wenn tatsächlich getestet.

### Importliste als gemeinsamer Arbeitsbereich

- Importübersicht, Dateiliste, Inline-Vorschau und Importaktion bilden eine gemeinsame Card.
- Enthält ein Dokument eine IBAN oder Kontoreferenz, werden nur aktive Konten mit
  exakt derselben normalisierten Kennung angeboten. Eine fehlende oder abweichende
  hinterlegte Kennung blockiert den Import mit einem konkreten Warnhinweis und
  einem Link zur Kontoverwaltung. Nur Dokumente ohne Kontokennung dürfen auf die
  Zuordnung über Anbieter, Währung und Kontotyp zurückfallen.
- Im Kopf stehen Dateistatus, Buchungsanzahl und Kontozuordnungen kompakt; keine wiederholte Bereitschaftsmeldung oder allgemeine Bedienerklärung.
- Die Vorschau öffnet direkt unter der zugehörigen Datei. Warnungen und Duplikathinweise bleiben der Datei zugeordnet.
- Eine geöffnete PDF-Vorschau bietet im Kopf neben ihren Statusangaben direkt
  „PDF anzeigen“ an. So bleibt das Quelldokument auch während der
  Duplikatentscheidung ohne Rücksprung in die Dateizeile erreichbar.
- In der Dateiliste stehen fehlgeschlagene Analysen zuerst, danach Dateien mit
  ungeklärten Duplikatverdachtsfällen und anschließend alle übrigen Dateien.
  Innerhalb derselben Statusgruppe bleibt die Reihenfolge der Dateiauswahl erhalten.
  Die geöffnete Vorschau bleibt stets mit ihrer Dateizeile verbunden.
- Die Duplikatprüfung unterscheidet eindeutige bereits vorhandene Buchungen von
  möglichen Dubletten. Eindeutige Treffer werden automatisch übersprungen.
  Mögliche Dubletten werden in der Transaktionsvorschau mit Vergleichsbuchung
  markiert und blockieren den Import, bis jede einzeln als neue Buchung bestätigt
  oder als Duplikat übersprungen wurde. Als Verdachtsfall gilt derselbe Betrag
  in derselben Währung auf demselben Konto und dasselbe vollständige Kommentarfeld
  innerhalb eines Toleranzfensters von zwei Tagen. Beim Vergleich werden nur
  technisch bedingte Unterschiede bei Leerzeichen vereinheitlicht; Referenzen oder
  andere Bestandteile werden nicht aus dem Kommentar herausgelöst. Fehlt eines der
  beiden Kommentarfelder, bleibt der Fall aus Sicherheitsgründen ein manueller
  Verdachtsfall. Nur zwei vorhandene, eindeutig unterschiedliche vollständige
  Kommentare schließen den Verdacht aus. Der
  Backend-Speichervorgang wiederholt die Prüfung atomar und verweigert Importe mit
  ungeklärten Treffern. Gleicher Tag und Betrag allein erzeugen keinen Verdacht:
  Das normalisierte Kommentarfeld muss vollständig übereinstimmen; unterschiedliche
  Kommentare bleiben eigenständige Buchungen. Gewählte Entscheidungen
  zeigen ihren Zustand mit Häkchen und sichtbarer Beschriftung, die Zusammenfassung
  nennt die verbleibende Anzahl. Während des eigentlichen Speicherns benennt die
  Hauptaktion den laufenden Import unmittelbar. In der Vorschautabelle stehen
  ungeklärte Verdachtsfälle zuerst, danach bereits entschiedene Verdachtsfälle und
  zuletzt unauffällige Buchungen; diese Sortierung verändert weder Quellzeilen noch
  die gespeicherte Buchungsreihenfolge. Der kompakte Restzähler muss auch in
  schmalen Ansichten vollständig sichtbar bleiben. Unterschiedliche Kommentarfelder
  schließen einen Verdachtsfall aus. Bei einem echten Verdachtsfall
  steht die vollständige Vergleichsbuchung als eigene helle Tabellenzeile unmittelbar
  unter der zu prüfenden Buchung; Datum, Beschreibung und Betrag verwenden dieselben
  Spalten, damit Abweichungen ohne gekürzten Hilfetext direkt vergleichbar sind.
- Nicht interaktiv lösbare Dateiwarnungen wie eine erkannte ältere Zeichenkodierung
  erscheinen kompakt als Warnstatus an Datei und Vorschau, nicht als dauerhaftes
  gelbes Textband. Der technische Wortlaut bleibt als zugängliche Zusatzinformation
  erhalten. Die Importliste wechselt bei begrenzter Inhaltsbreite rechtzeitig in
  ein zweispaltiges und auf schmalen Fenstern einspaltiges Zeilenlayout; sie erzeugt
  keine zweite horizontale Scrollleiste außerhalb der eigentlichen Buchungsvorschau.
  In der Buchungsvorschau erhält die Beschreibung ausreichend, aber nicht den gesamten
  Restplatz. Branche, Beträge, Erkennung und Duplikatentscheidung behalten definierte
  Anteile, sodass die Aktionsspalte innerhalb der sichtbaren Card liegt. Erst unterhalb
  der kompakten Mindestbreite scrollt ausschließlich diese innere Buchungstabelle.
- Der gemeinsame Footer ist nur durch eine Linie getrennt; „Importieren“ bleibt die dunkelblaue Hauptaktion. Kontoprüfungen und deaktivierte Zustände bleiben erhalten.
- Fehlgeschlagene PDF-Analysen bieten direkt in ihrer Dateizeile „PDF anzeigen“
  an. Die Aktion öffnet exakt die gewählte Quelldatei mit der auf dem System
  hinterlegten PDF-Anwendung; ein Öffnungsfehler wird an derselben Datei angezeigt
  und ersetzt nicht die eigentliche Importfehlermeldung.

Architektur und Modulverantwortlichkeiten: [architecture.md](architecture.md).
