# Finanzblick – verbindlicher Design-Leitfaden

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
- Tabellen standardmäßig kompakt halten. Keine großen Aktionsblöcke in jeder
  Zeile der Haupt-Transaktionsübersicht; dort das Drei-Punkte-Menü verwenden.
- Dedizierte Einrichtungsansichten dürfen direkte Zeilenaktionen haben.
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

- Vermögensverlauf: lokal gebündelte Lightweight Charts, Emerald-Kurve und
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
- Kurze, konkrete Handlungsaufforderungen statt interner Begriffe wie Flags.
- Logo-Master: `public/finanzblick.svg`. Header, Favicon und Plattform-Icons
  daraus ableiten; keine abweichenden Inline-Logos neu zeichnen.
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

Architektur und Modulverantwortlichkeiten: [architecture.md](architecture.md).
