# Finanzchat mit ChatGPT-Konto

Stand: 14. September 2026.

## Bedienung

Finanzchat öffnen, „Mit ChatGPT anmelden“ wählen und die Anmeldung im Browser
abschliessen. Die Ansicht zeigt einen zentrierten Nachrichtenstream mit einer
ständig sichtbaren, mitwachsenden Eingabeleiste. Enter oder der runde Sendepfeil
öffnet standardmässig die Datenfreigabe als Dialog; Shift+Enter fügt eine neue Zeile ein.
„Vor dem Senden prüfen“ lässt sich unter „Datenschutz & Modell-Info“ ausschalten.
Dann senden Enter und Sendepfeil die gewählten Daten direkt, ohne weitere Bestätigung.
Für neue Daten sind mit aktiver Prüfung Freigabekästchen und „Freigeben & an ChatGPT senden“
erforderlich. Escape oder „Frage bearbeiten“ kehrt zur Eingabe zurück.

Der Zeitraum steht unter „Datenschutz & Modell-Info“, standardmässig laufendes
Kalenderjahr bis heute. Dort gibt es zwei unabhängige Schalter:

- Fearless-Modus: standardmässig aus. Aktiviert zusätzlich Detailtransaktionen mit
  Datum, Betrag, Währung und Kategorie sowie technischen Berechnungskennzeichen.
  Buchungstexte, Konto- und Banknamen, IBAN, Kontonummern, Inhaber- und Adressfelder
  werden auch im Direktmodus nicht übertragen. Persönliche Angaben in eigenen
  Kategorienamen oder Fragen müssen Nutzer selbst weglassen.
- Vor dem Senden prüfen: standardmässig an. Ausschalten erlaubt direkten Versand
  sowohl für Zusammenfassungen als auch für Detailtransaktionen.

Jede Änderung des Modus, Zeitraums oder Prüfverhaltens leert den Verlauf, damit zuvor
geteilte Details nicht unbemerkt im Folgechat weitergesendet werden. Diese Optionen
werden nicht dauerhaft gespeichert; nach Verlassen/Neuladen oder Kontoabmeldung
startet die Ansicht wieder mit Zusammenfassungen und Prüfung. Der aktive Modus und
das Versandverhalten sind im Header sichtbar. Dort befinden sich auch die
Datenschutzhinweise und die Kontoabmeldung. „Neuer Chat“ leert den Verlauf. Assistentenantworten unterstützen
Markdown mit Listen, Fettungen und Tabellen; HTML und externe Bilder werden nicht
aktiviert, Modelllinks nur als Text angezeigt.

Ein unterstützter ChatGPT-Zugang mit Codex-Freischaltung ist erforderlich. Es gelten
dessen Nutzungslimits. API-Schlüssel und separates API-Guthaben werden nicht verwendet.
Gemini und Anthropic sind in dieser Variante nicht enthalten.

## Anmeldung und Datenfluss

- Mitgelieferte, fest gebundene Laufzeit: OpenAI Codex 0.154.0; Modell gpt-5.6-sol.
- Rust startet den App-Server über private stdio-Pipes, ohne öffentliches Socket.
  Executable und SHA-256 stammen aus dem beim Build erzeugten Manifest. Der
  WebView kann weder Programm, Dateipfade, RPC-Methoden noch Modell frei wählen.
- Ein eigenes, pro Finanzprofil getrenntes CODEX_HOME und eine bereinigte Prozessumgebung verhindern
  die Übernahme bestehender Codex-Anmeldungen, API-Schlüssel oder Benutzerkonfiguration.
- Anmeldedaten werden im Betriebssystem-Anmeldespeicher behalten
  (`cli_auth_credentials_store = "keyring"`, ohne Klartext-Fallback). Ein stabiler,
  aus der Profil-ID abgeleiteter Pfad im Tauri-App-Datenverzeichnis trennt die Konten.
  Sperren, Profilwechsel und Schliessen beenden den Prozess, löschen aber nicht die
  Anmeldung. Beim Öffnen wird sie wieder geladen. „ChatGPT abmelden“ führt
  `account/logout` aus und entfernt sie. Die Anmeldung anderer Apps bleibt unverändert.
- Kontoabfragen und Anmeldung können OpenAI kontaktieren. Finanzdaten werden erst
  nach dem gewählten Versandverhalten übergeben: einzeln geprüft oder per Enter/Sendepfeil
  direkt, wenn der Nutzer die Prüfung bewusst ausgeschaltet hat.
- Auch im Direktmodus wird zuerst ein unveränderliches Datenpaket lokal vorbereitet,
  das die UI sofort über dessen ID sendet. Die Vorschau wird im Backend gespeichert: maximal zehn Minuten, genau einmal
  verwendbar, gebunden an Profil, Entsperrsitzung und Verbindungsstand. Das Frontend
  sendet nur die Vorschau-ID, keinen ersetzbaren Payload. Abbruch invalidiert auch
  eine noch laufende Vorbereitung.
- Während der Übertragung der freigegebenen Anfrage an die lokale Laufzeit schützt
  eine kurze Tresorsperre die Sitzung. Das Warten auf die Modellantwort hält keine
  Tresorsperre. Ein Hintergrundwächter beendet die Laufzeit nach Sperren/Profilwechsel;
  verspätete Antworten werden zusätzlich gegen die Sitzung geprüft und verworfen.
- Die erste Frage eröffnet einen flüchtigen Thread mit dem freigegebenen Datenpaket.
  Folgefragen verwenden denselben Thread und übergeben nur die neue Frage; die App
  fügt weder Transaktionen noch den bisherigen Chat erneut ein. Ein lokaler SHA-256-
  Vergleich von Datenpaket und Sprache prüft, ob der Kontext unverändert ist.
  Geänderte Daten, Zeiträume, Kontenauswahl oder Modi erfordern einen neuen Thread
  und bei aktiver Prüfung eine neue Freigabe. Folgefragen werden direkt gesendet.
  Neuer Chat, Verlassen, Sperren und Abbruch verwerfen den Kontext. Nach einem
  Laufzeitfehler wird keine Frage stillschweigend mit einem neuen Datenpaket gesendet.
  Chatverläufe werden nicht dauerhaft gespeichert. Der Anbieter kann den bestehenden
  Kontext intern erneut verarbeiten und auf Nutzungslimits anrechnen.
- Ein Abbruch während einer Antwort beendet die Verbindung; danach wird die
  gespeicherte Anmeldung erneut geladen. Bereits übertragene Inhalte lassen sich damit nicht zurückholen.
- Anbieterfehler werden auf feste Meldungen abgebildet, ohne Anbieterantworten oder
  Anmeldedaten in Logs oder Fehleranzeigen zu übernehmen. Nutzungslimits verweisen
  auf das ChatGPT-Konto, nicht auf API-Guthaben.

## Begrenzung der Modellfähigkeiten

Der mitgelieferte Modellkatalog deaktiviert Shell und Apply Patch und enthält keine
experimentellen Werkzeuge. Die feste Konfiguration deaktiviert zusätzlich Bildzugriff,
Websuche, Browser, Apps, MCP-Installation, Plugins, Code-Ausführung, Unteragenten,
Skillsuche, Hooks und Erinnerungen. Benutzerkonfiguration wird nicht übernommen.
Die effektive Konfiguration wird vor der Anmeldung geprüft; abweichende
Systemvorgaben für Werkzeuge, Endpunkte, MCP oder Speicherung führen zum Abbruch.
Der Thread nutzt ausserdem einen nur lesenden Sandboxmodus. Unbekannte
serverseitige Anfragen werden abgelehnt.

Diese Grenze beruht auf der geprüften Laufzeit plus Konfiguration, nicht allein auf
Prompt-Instruktionen. Versionsänderungen müssen erneut geprüft werden.

## Fachliche Grundlage

Die Zusammenfassungen entsprechen den bestehenden lokalen CHF-Auswertungen.
Kartenkäufe zählen zu Ausgabenkategorien, erkannte Kartenabrechnungen dort nicht erneut.
Geldfluss enthält Bankabrechnungen ohne zusätzliche Kreditkartenbuchungen. Interne
Überträge sind nicht generell bereinigt. Zuflüsse sind nicht automatisch Einkommen,
Vermögensänderungen keine Rendite. Fehlende Vermögensstände werden nicht erfunden.
Banknamen, Kontonummern, Buchungstexte und eigene Kategorienamen fehlen in den
Aggregaten. Fragen können dennoch persönliche Angaben enthalten.

Im Fearless-Modus umfassen die Details alle Buchungen mit Betrag ungleich null von
aktiven Konten im ausgewählten Zeitraum, auch Fremdwährungen. Diese werden nicht in
CHF umgerechnet. Pro Zeile kennzeichnen Ausschlussflags die Zugehörigkeit zu den
bestehenden CHF-Ausgaben- und Geldflusssummen. Eigene Kategorienamen bleiben in den
Detailzeilen erhalten; in den anonymisierten Summen bleiben sie zusammengefasst.
Es werden höchstens 2000 Detailzeilen und 400000 Bytes Anfrageinhalt zugelassen.
Überschreitungen führen zum Fehler mit Bitte um kürzeren Zeitraum, nicht zu stiller
Kürzung. Ohne Fearless gilt weiterhin das Limit von 120000 Bytes. Der Zugriff bleibt
rein lesend, ohne Modellwerkzeuge oder Änderungen an Buchungen/Kategorien.

## Entwicklung und Paketierung

`npm install` installiert die für das Buildsystem passende native Laufzeit über das
fest gebundene npm-Paket. `npm run chat:stage` kopiert sie in das ignorierte Verzeichnis
`src-tauri/chat-runtime/bin` und erzeugt das Manifest. Dieser Schritt läuft automatisch
vor `npm run dev` und `npm run build`; vor direkten Cargo-Aufrufen muss er einmal
manuell laufen. Tauri bündelt die Laufzeit als Ressource. Endnutzer benötigen kein
Node.js und keine eigene Codex-Installation. Apache-Lizenz und Upstream-NOTICE werden
mitgeliefert und in die Lizenzinventur aufgenommen.

Windows und macOS haben plattformübergreifende Pfade und native npm-Pakete.
Auf macOS bestehen seit dem 15.09.2026 die Tests der nativen Apple-Silicon-Runtime,
einschliesslich Schlüsselbund-Speicherung, Profiltrennung und Abmeldung mit
synthetischen Zugangsdaten. Dafür bleibt das Betriebssystem-Benutzerverzeichnis
erhalten: Ein temporäres `HOME` verhindert auf macOS das Finden des Standardschlüsselbunds.
`CODEX_HOME` und Arbeitsverzeichnis bleiben Finanzblick-eigen, die Suche nach
Host-Skills bleibt deaktiviert.

## Validierung und Grenzen

- Windows-Testbuild und Frontend-Build erfolgreich; 13 JavaScript-Tests und
  75 Rust-Tests erfolgreich, ein bestehender Rust-Test ignoriert.
- Rust-Tests zu lokalen Aggregaten, Schlüsselbereinigung, Kontoadressen,
  Fehlertexten, Freigabeabbruch und Sperren/erneutem Entsperren.
- `node scripts/test-chat-runtime.mjs`: echte gebündelte Laufzeit gegen lokalen
  synthetischen Modellserver; keine Kontoanmeldung und keine Finanzdaten nach aussen.
  Prüft leere Werkzeugliste, Ablehnung erzwungener Shell-, Dateiänderungs- und
  Bildzugriffsaufrufe, Modellantwort, flüchtigen Thread, sofortiges Entladen und
  fehlende gespeicherte Authentifizierung.
- `node scripts/probe-chatgpt-login.mjs src-tauri/chat-runtime/bin/codex.exe`:
  Anmeldestart und Abbruch mit der gebündelten Windows-Laufzeit erfolgreich.
- Browserprüfung mit synthetischer Tauri-Schnittstelle: Anmeldung, Zurücknavigation,
  Freigabe vor Senden, Folgefragen, Zeitraumwechsel, Abmeldung und 920-Pixel-Layout.

Eine vollständige persönliche Browseranmeldung und eine echte Antwort über das
Abonnement können nur mit dem jeweiligen Nutzerkonto geprüft werden. Diese Prüfung
wurde nicht durch Zugriff auf eine vorhandene Codex-Anmeldung ersetzt. Die neue
Integration ist daher als erste Testversion zu behandeln.

Offizielle Grundlagen:
[App-Server](https://learn.chatgpt.com/docs/app-server),
[Authentifizierung](https://learn.chatgpt.com/docs/auth),
[Konfiguration](https://learn.chatgpt.com/docs/config-file/config-reference).

Zusätzlich geprüft: alle vier Kombinationen aus Datenumfang und Prüfverhalten,
Zeitraumfilter inklusive Randdaten, Fremdwährungen, unveränderte CHF-Summen,
Opt-in-Standardwerte, Detailmengengrenze ohne Kürzung und Verlaufslöschung beim Wechsel.

## Lokale Spracheingabe

Der Mikrofonknopf diktiert deutsche Fragen ins Eingabefeld. Er sendet nicht selbst.
Vosk Browser (Lichess-Fork 0.0.3) und das deutsche Modell small-de-0.15 laufen lokal in einem
WebWorker. Audio wird nicht gespeichert oder hochgeladen. Das Modell (ca. 46 MB)
kommt mit der App; das plattformneutrale Buildskript prüft dessen SHA-256.
Das Modell ist deutschsprachig; Dialekt kann Korrekturen am erkannten Text erfordern.
Mikrofonberechtigung wird erst beim Klick angefragt. Stop, Ansichtswechsel,
Profilwechsel und Sperren beenden den Audiostream; maximal zwei Minuten pro Aufnahme.
Während der Aufnahme ist Absenden gesperrt. macOS enthält den Mikrofon-Nutzungstext
in Info.plist. Das Sprachmodell wird auch im macOS-DMG mitgeliefert;
ein praktischer Mikrofon-/Diktiertest auf macOS steht weiterhin aus.

Die Detailabfrage nutzt eine explizite Feldfreigabe; Beschreibung, Kontoname und
Kontoreferenz werden nicht selektiert. Ein Regressionstest prüft den vollständigen
Snapshot auf synthetische Namen, Adressen, IBAN und Kontonummern und erzwingt
die erlaubten JSON-Felder für jede Detailtransaktion.

## Auf Kreditkarten begrenzter Kontext

Die lokale Kontenauswahl steht standardmässig auf „Automatisch nach Frage“.
Erwähnungen von Kreditkarten (auch englisch, französisch oder italienisch) begrenzen
Zusammenfassungen und Details auf aktive Konten vom Typ `credit_card`. Es werden
keine Bankumsätze, globalen Geldflusssummen oder Vermögensdaten beigefügt. Eine eigene
SQL-Abfrage erstellt Kartenbelastungen und Gutschriften pro Monat im gewählten
Zeitraum. CHF-Summen vermischen keine Fremdwährungen; Kartengutschriften sind kein
Einkommen. Es gelten weiterhin die Feldfreigabe und die Grenze von 2000 Detailzeilen.

Die Vorschau und der Chat-Kopf zeigen „Nur Kreditkarten“. Die Auswahl bleibt für
Folgefragen erhalten. Beim automatischen Wechsel wird alter globaler Chatkontext
auch im Backend weggelassen. Unter „Kontenauswahl“ kann die Einschränkung ausdrücklich
gewählt oder mit „Alle aktiven Konten“ aufgehoben werden; Änderungen starten einen
neuen Chat. Die Erkennung verwendet lokale Schlüsselwörter, keine externe Modellabfrage.
Der Zeitraum wird weiterhin über Von/Bis festgelegt.
