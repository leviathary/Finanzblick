# Designprüfung vom 26.09.2026

## Ergebnis und Geltungsbereich

Geprüft wurde der aktuelle Arbeitsstand von Saldonaut 0.6.8 gegen
`docs/design-system.md`, nicht nur ein früherer Screenshot oder Build.
Die visuelle Basis ist konsistent. Die ursprünglich gefundenen fünf
Bedienungs- und Textprobleme sind unten samt Korrektur dokumentiert.
Eine uneingeschränkte native Designabnahme wird nicht erteilt.

## Stand nach der Korrektur am 26.09.2026

Die fünf unten beschriebenen Befunde sind im aktuellen Arbeitsstand behoben:

| Befund | Korrektur und Nachprüfung |
| --- | --- |
| D1 | Importarten nutzen `SectionTabs` und `SectionPanel`. Pfeil links/rechts, Home/End, Auswahl, Fokus und Panel-Sichtbarkeit im Browser geprüft. |
| D2 | Jede aktive Seitenleistenroute setzt genau einmal `aria-current="page"`; elf Haupt- und Unterrouten mit synthetischen Daten geprüft. |
| D3 | Französisches „Se déconnecter“ korrigiert und in der Oberfläche geprüft. |
| D4 | Unvollständiger manueller Steuerentwurf zeigt einen neutralen Hinweis; widersprüchliche vollständige Werte zeigen weiter einen Fehler, korrekte Werte Erfolg. |
| D5 | Das globale `min-width: 920px` ist entfernt. Bei 760, 920 und 1520 CSS-Pixeln, hell und dunkel, kein globaler Überlauf auf den fünf betroffenen Seiten. Bei 200 % CSS-Vergrößerung im 1520-px-Fenster bricht die Navigation um; Kartenwerte bleiben sichtbar und es entsteht kein globaler Überlauf. |

`npm test` (96/96), `npm run build` und `git diff --check` waren nach der
Korrektur erfolgreich. Der Build meldet weiterhin die bestehende Warnung zu
großen JavaScript-Chunks. Die nativen und in „Nicht vollständig abgedeckt“
genannten Prüfgrenzen gelten weiterhin.

Die Prüfung verwendet den echten React-/CSS-Code in Chrome unter Windows mit
synthetischen Tauri-Antworten. Es wurden keine persönlichen Finanzprofile geöffnet
oder verändert. Sie ersetzt keinen Test des paketierten Tauri-WebViews und keinen
macOS-Test. Die vollständige native Abnahme bleibt offen.

## Ursprüngliche Befunde

### D1 – P2: Import-Reiter haben keine vollständige Tastatur-/Tab-Semantik

Ort: `src/App.tsx`, `ImportKindTabs`, Zeilen 35–39 und zugehörige Inhaltsbereiche.
Auf „Bankauszüge“ fokussieren und Pfeil rechts drücken: Fokus und Auswahl bleiben
auf Bankauszüge. Beide Reiter bleiben normale Tabstopps; Panel-Verknüpfungen fehlen.
Die Einstellungsreiter wechseln dagegen korrekt zu „Allgemein“.
Das erschwert eine vorhersehbare Bedienung mit Tastatur und assistiven Hilfsmitteln.

Empfehlung: bestehende `SectionTabs`/`SectionPanel` verwenden; Pfeile, Home/End,
aktiven Tabstopp und Panel-Zuordnung gemeinsam testen. Importentwürfe beim
Wechsel weiterhin erhalten. Keine Änderung an der Importlogik erforderlich.

### D2 – P2: Aktuelle Seite wird in der Navigation nicht einheitlich ausgezeichnet

Ort: `src/App.tsx`, Sidebar ab Zeile 123.
Nur Konten/Depots, Steuern und Import setzen `aria-current`. Übersicht, Vermögen,
Transaktionen, Finanzchat, Kontenverwaltung, Kategorien, Hilfe, Sicherheit und
Einstellungen besitzen zwar eine sichtbare aktive Klasse, aber keine entsprechende
programmatische Kennzeichnung. Das ist ein bestätigter Quellcodebefund;
die tatsächliche Ansage eines Screenreaders wurde nicht getestet.

Empfehlung: aktive Haupt-/Unterseiten einheitlich mit `aria-current="page"`
kennzeichnen und mit Screenreader nachprüfen.

### D3 – P3: Beschädigte französische Abmeldebeschriftung

Ort: `src/translations.json`, Zeile 1112.
Die französische Navigation zeigt „Se dÃ©connecter“ statt „Se déconnecter“.
In der französischen Browseransicht reproduziert und im Übersetzungskatalog bestätigt.

Empfehlung: Text korrigieren und Übersetzungstests um typische Kodierungsartefakte
ergänzen. Vollständige Schlüssel allein beweisen keine sprachlich korrekten Texte.

### D4 – P3: Neues Steuerformular startet bereits mit einer Fehlermeldung

Ort: `src/features/tax-history/TaxHistory.tsx`, Meldung um Zeile 458;
Initialwerte und Statusberechnung in `taxHistorySupport.tsx`.
Direkt nach „Steuerjahr erfassen“, ohne Eingabe, erscheint rot
„Bitte Werte korrigieren: Vermögenswerte minus Schulden müssen dem steuerbaren
Vermögen entsprechen.“ Tatsächlich fehlen zunächst Pflichtwerte; eine falsche
Rechnung wurde noch gar nicht eingegeben. Das macht den Einstieg unnötig alarmierend.

Empfehlung: unvollständigen Entwurf neutral erklären; eine konkrete Rechenabweichung
erst bei auswertbaren Eingaben beziehungsweise nach Interaktion anzeigen.
Die vorhandene Speichersperre bei ungültigen Werten beibehalten.

### D5 – P2, Ergonomiegrenze: Vergrößerung erzeugt globalen Seitenüberlauf

Ort: `src/styles/application.css`, Zeile 23: `body { min-width: 920px; }`.
Bei 1520 px Fensterbreite und CSS-Vergrößerung auf 200 % laufen Übersicht,
Einstellungen, Import, Kartentransaktionen und Sicherheit horizontal über.
Bei 150 % trat in diesen fünf Ansichten kein Seitenüberlauf auf.

Einordnung: Der Leitfaden unterstützt mindestens 920 CSS-Pixel. Bei 200 % bleiben
effektiv nur 760 übrig; daher kein Nachweis einer Regression innerhalb der
festgelegten Mindestbreite. Es bleibt aber eine relevante Einschränkung für
Menschen, die größere Schrift benötigen. CSS-Vergrößerung ist nicht identisch
mit Betriebssystemskalierung oder nativem WebView-Zoom.

Empfehlung: einen expliziten Zoom-/Reflow-Standard festlegen; Navigation bei wenig
Platz reduzieren und möglichst nur Datentabellen intern scrollen lassen.
Eine globale Layoutänderung ist nicht Bestandteil dieses Reviews.

## Durchgeführte Prüfungen

| Bereich | Prüfung und Ergebnis |
| --- | --- |
| Hauptansichten | 18 Routen × Hell/Dunkel × 1520/920 px, jeweils 1000 px hoch: 72 Aufnahmen; keine JavaScript-Ausnahme, kein globaler horizontaler Überlauf. |
| Visuelle Hierarchie | Screenshots der Hauptansichten und ausgewählter Detailzustände betrachtet: konsistente Slate-Hauptaktionen, Status-/Finanzfarben getrennt, erkennbare Überschriften, Cards und Tabellen. |
| Navigation | Login per Enter; Import-Pfeiltastenproblem D1 reproduziert; Einstellungsreiter mit Pfeil rechts erfolgreich. |
| Umbuchungsdialog | Initialfokus im nativen Dialog, Tab-Folge, sichtbarer Fokus, Escape und Rückkehr zum Auslöser geprüft. Ein Tab-Zyklus zur Browseroberfläche ist kein Beleg für eine fokussierbare Hintergrundaktion. |
| Positionsverwaltung | Mit synthetischem Depot: Einstieg per Enter, Fokus auf Zurück, Position anlegen/ändern/löschen, Erfolgsmeldung, hell und dunkel/schmal erfolgreich. Isolierte Feature-Testseite, keine echte Persistenz. |
| Kategorien | Branchen-Tabfolge, Inline-Bearbeitung/Speichern/Abbrechen, Zusammenführen, Zielpflicht, Escape ohne Änderung, hell/dunkel und Vergrößerung erfolgreich. |
| Profilkopie | Anonymisierte Variante: Fokus/Rückkehr, Passwort-/Faktorvalidierung, Busy-Zustand, simulierter Fehler/Wiederholung, Löschen der Passworteingaben nach Schließen sowie beide Methoden erfolgreich. Normale Kopie nicht end-to-end ausgeführt. |
| Import | Synthetische Datei, Kontozuordnung, Bereitschaft und Inline-Vorschau per Enter geprüft; langer Buchungstext bricht um, Oberfläche zeigt Dateinamen statt Quellpfad. Kein echter Import ausgeführt. |
| Steuern | Leere Historie/Jahresverwaltung, Import-Einstieg und manuelles Formular betrachtet; D4 beim Öffnen reproduziert. |
| Darstellung | Hell/Dunkel über simulierte Systemeinstellung; Kosmisch auf Übersicht visuell geprüft. Fehlgeschlagener Theme-Speichervorgang zeigt verständlichen Alert und behält die vorige Palette. |
| Sprachen | EN/FR/IT auf Steuerimport, Karten, Sicherheit und Einstellungen bei 1520 px ohne Seitenüberlauf; FR/IT-Steuerimport zusätzlich bei 920 px. D3 sichtbar. Keine vollständige sprachliche Abnahme aller Texte. |
| Vergrößerung | Je fünf Seiten bei 150 % und 200 % CSS-Zoom, siehe D5. |
| Automatisierte Basis | `npm test`: 96/96 erfolgreich. `npm run build`: erfolgreich; bestehende Warnung zu großen JavaScript-Chunks bleibt. `git diff --check`: erfolgreich. |

Die 18 Routen umfassen Übersicht, Vermögen, Konten/Depots, Kontenverwaltung,
Transaktionen, Karten, Karten-Setup (Einstieg), Umbuchungen, Steuerhistorie,
Steuerjahre, Bankimport, Steuerimport, Importhistorie, Kategorien, Finanzchat,
Hilfe, Daten/Sicherheit und Einstellungen.

## Nicht vollständig abgedeckt

- Native Windows-Dateidialoge, Betriebssystemskalierung, WebView-spezifischer Fokus,
  Screenreader, macOS-Build und macOS-Bedienung.
- Jeder Lade-/Fehlerzustand jeder Seite; sämtliche Wizard-Schritte und jede Kombination
  aus mehreren Konten, großen Datenbeständen, langen Namen und Sprachen.
- Gefüllte Steuerdiagramme, alle Drilldowns, vollständige Dublettenentscheidung,
  Excel-Mapping, echte Backup-Wiederherstellung und laufender authentifizierter
  Finanzchat einschließlich Spracherkennung.
- Vollständige gemessene Kontrastmatrix sämtlicher Texte, Hover-/Fokuszustände,
  Diagramme und semantischer Farben. Sichtprüfung allein ist keine Kontrastzertifizierung.

Leere Auswertungen beruhen teilweise bewusst auf leeren Mock-Antworten und sind
kein Nachweis für die korrekte Darstellung aller gefüllten Varianten.
Die oben genannten Lücken müssen für eine vollständige Produktabnahme geschlossen
werden; Build und Unit-Tests ersetzen das nicht.

## Nachweise und Umsetzung

Lokale Screenshots und Messdaten: `tmp/design-review-2026-09-26/`, einschließlich
`results.json`, `states.json`, `import-preview.png`, `tax-editor.png`,
`appearance-error.png`, `zoom-2-settings.png` und Sprachaufnahmen.
Zusätzliche Feature-Aufnahmen liegen in `tmp/ui-audit/screenshots/` und
`tmp/depot-light.png` / `tmp/depot-dark.png`. Diese lokalen Prüfartefakte sind
nicht Teil des ausgelieferten Produkts und nicht als dauerhaft versionierte
CI-Suite zu verstehen.

Ältere Prüfscripte benötigten Anpassungen an Onboarding, Profilkopien-Auswahl und
Mock-Verträgen. Ihre anfänglichen Abbrüche wurden nicht als Produktfehler gewertet.
Die Importvorschau wurde anschließend mit aktuellem Mock-Vertrag separat geprüft.

Die nachfolgende Korrektur hat D1–D5 umgesetzt. Die ursprünglichen Befundtexte
bleiben als nachvollziehbare Ausgangslage erhalten.
