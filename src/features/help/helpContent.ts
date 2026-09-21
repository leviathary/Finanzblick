// Definiert die lokalisierten, durchsuchbaren Artikel der integrierten Hilfe.

import { t } from "../../i18n";

export type HelpSection = {
  title: string;
  paragraphs?: string[];
  steps?: string[];
  bullets?: string[];
  note?: string;
};

export type HelpArticle = {
  id: string;
  title: string;
  summary: string;
  sections: HelpSection[];
};

export function getHelpArticles(): HelpArticle[] {
  return [
    {
      id: "start",
      title: t("Erste Schritte"),
      summary: t("Demo-Daten ausprobieren oder ein eigenes Finanzprofil sicher einrichten."),
      sections: [
        {
          title: t("Saldonaut kennenlernen"),
          paragraphs: [t("Saldonaut bündelt Konten, Depots und weitere Finanzquellen in einer gemeinsamen lokalen Sicht. Deine Finanzdaten werden verschlüsselt auf diesem Gerät gespeichert.")],
          steps: [
            t("Wähle auf der Anmeldung unter Finanzprofil den Eintrag Demo-Daten."),
            t("Gib das Passwort demo1234 ein und wähle Anmelden."),
            t("Öffne nacheinander Übersicht, Vermögen, Transaktionen und Banken & Konten."),
          ],
          note: t("Demo-Kurse sind synthetische Beispieldaten und keine historischen Börsenkurse oder Anlageempfehlungen."),
        },
        {
          title: t("Eigenes Finanzprofil anlegen"),
          paragraphs: [t("Ein Finanzprofil ist ein eigenständiger, verschlüsselter Datenbestand. Mehrere Profile bleiben voneinander getrennt und können unterschiedliche Passwörter haben.")],
          steps: [
            t("Erstelle auf der Anmeldung ein neues Finanzprofil."),
            t("Vergib einen eindeutigen Namen und ein starkes Passwort."),
            t("Lege unter Banken & Konten deine Konten an oder beginne unter Import mit einem Bankauszug."),
          ],
          note: t("Es gibt keinen Passwort-Reset. Bewahre dein Passwort sicher auf und erstelle regelmässig ein Backup."),
        },
      ],
    },
    {
      id: "import",
      title: t("Bankauszüge importieren"),
      summary: t("Dateien prüfen, Konten zuordnen und doppelte Buchungen vermeiden."),
      sections: [
        {
          title: t("Dateien vorbereiten und prüfen"),
          paragraphs: [
            t("Saldonaut verarbeitet Excel-, CSV-, PDF- und MT940-Dateien sowie camt.053 und camt.054 nach ISO 20022 lokal. Vor dem Speichern siehst du eine Vorschau und die vorgesehene Kontozuordnung."),
            t("Enthält der Auszug eine IBAN oder Kontoreferenz, wird er nur einem aktiven Konto mit derselben hinterlegten Kennung zugeordnet. Bei einer Abweichung bleibt der Import gesperrt, bis du die Kontodaten korrigiert hast."),
          ],
          steps: [
            t("Öffne Import, Dateien importieren und danach Bankauszüge."),
            t("Wähle einzelne Dateien oder einen ganzen Ordner und lass sie analysieren."),
            t("Prüfe Anbieter, Zielkonto, Zeitraum, Währung und Buchungsvorschau."),
            t("Ordne unbekannte Spalten bei Bedarf zu und importiere alle bereiten Dateien."),
          ],
        },
        {
          title: t("Datenschutz beim Import"),
          paragraphs: [
            t("Bankauszüge, Kreditkartenabrechnungen und Steuererklärungen werden ausschliesslich lokal auf deinem Gerät verarbeitet. Die Importdateien werden weder hochgeladen noch an Saldonaut oder Dritte übertragen."),
            t("Saldonaut kopiert die Quelldokumente nicht in dein Finanzprofil. Nach der Analyse werden die eingelesenen Dateiinhalte aus dem Arbeitsspeicher verworfen; die App benötigt die Dokumente danach nicht mehr. Deine Originaldateien bleiben an ihrem bisherigen Speicherort und werden weder verändert noch gelöscht."),
            t("Gespeichert werden nur die von dir bestätigten Finanzdaten, der Dateiname und ein technischer Fingerabdruck zur Duplikaterkennung. Diese Informationen liegen verschlüsselt in deinem lokalen Finanzprofil."),
          ],
          note: t("Nur wenn du im Finanzchat ausdrücklich ein Datenpaket freigibst, werden die darin angezeigten Informationen an den gewählten Chat-Dienst übertragen. Die ursprünglichen Importdokumente gehören nicht zu diesem Datenpaket."),
        },
        {
          title: t("Duplikate und Importhistorie"),
          paragraphs: [t("Bereits importierte Dateien und eindeutige Buchungen werden automatisch erkannt und übersprungen. Ähnliche Buchungen werden als mögliche Duplikate markiert und müssen vor dem Import einzeln bestätigt oder übersprungen werden."), t("Unter „Importierte Dateien“ kannst du den bestehenden Buchungsbestand manuell auf Duplikate prüfen. Bestätigte Mehrfachzahlungen werden bei späteren Prüfungen nicht erneut angezeigt."), t("Unter Importierte Dateien kannst du später nachvollziehen, welche Quelle welchem Konto zugeordnet wurde.")],
          note: t("Lösche einen Import erst, nachdem du die im Bestätigungsdialog beschriebene Wirkung geprüft hast."),
        },
        {
          title: t("Offene Kreditkartenmonate"),
          paragraphs: [t("Noch nicht abgerechnete Kartenkäufe in der Kontowährung können bereits importiert werden. Bis zur Abrechnung verwendet Saldonaut dafür vorläufig das Einkaufsdatum und den Originalbetrag; die Vorschau weist darauf hin."), t("Offene Fremdwährungsbuchungen ohne abgerechneten Betrag werden nicht umgerechnet. Importiere sie erst, wenn der Kartenanbieter den endgültigen Betrag in der Kontowährung ausweist.")],
          note: t("Importiere spätere vollständige Jahres- oder Monatsauszüge wie gewohnt. Saldonaut erkennt zuvor offene Kartenkäufe wieder und ersetzt Einkaufsdatum und Originalbetrag automatisch durch die endgültigen Abrechnungsdaten, statt eine zweite Buchung anzulegen."),
        },
      ],
    },
    {
      id: "accounts",
      title: t("Banken & Konten"),
      summary: t("Konten, Anbieter und manuelle Vermögenspositionen verwalten."),
      sections: [
        {
          title: t("Konto hinzufügen"),
          steps: [
            t("Lege über Bank oder Anbieter hinzufügen zuerst die Bankbeziehung an. Ein Konto ergänzt du danach über das Drei-Punkte-Menü der Anbietergruppe."),
            t("Erfasse Kontoname, Kontotyp und Währung."),
            t("Lege fest, ob das Konto zum Gesamtvermögen zählt, und speichere es."),
          ],
          paragraphs: [t("Name, Typ und Logo bearbeitest du zentral über das Drei-Punkte-Menü des Anbieters. Die Änderung gilt für alle zugehörigen Konten."), t("Konten desselben Anbieters werden gemeinsam gruppiert. Ausgeschlossene Konten bleiben sichtbar, fliessen aber nicht in Vermögenssummen ein.")],
        },
        {
          title: t("Manuelle Positionen"),
          paragraphs: [t("Manuell bewertete Konten eignen sich für Mitarbeiteraktien, Depots ohne importierbaren Auszug und andere Vermögenspositionen."), t("Für jedes Zuteilungsjahr kann eine eigene Position geführt werden. So bleiben Menge, Bewertungsdatum und Entwicklung nachvollziehbar.")],
          steps: [
            t("Öffne das Drei-Punkte-Menü des manuellen Kontos und verwalte seine Positionen."),
            t("Erfasse Name, Datum, Menge und Bewertung der Position."),
            t("Hinterlege bei Wertpapieren eine passende Kennung und Börse, wenn Marktpreise automatisch ermittelt werden sollen."),
          ],
        },
      ],
    },
    {
      id: "assets",
      title: t("Übersicht und Vermögen"),
      summary: t("Gesamtvermögen, Salden, Aufteilungen und Entwicklung verstehen."),
      sections: [
        {
          title: t("Übersicht lesen"),
          paragraphs: [t("Die Übersicht fasst das aktuelle Gesamtvermögen, die berücksichtigten Konten und Anbieter zusammen."), t("Der Anbieter-Donut zeigt positive Salden. Negative Salden bleiben in der Gesamtsumme berücksichtigt und werden in der Liste separat ausgewiesen.")],
        },
        {
          title: t("Vermögensentwicklung untersuchen"),
          bullets: [
            t("Wähle einen Zeitraum von 1M bis Max oder einen eigenen Zeitraum."),
            t("Zoome mit dem Mausrad und verschiebe den sichtbaren Zeitraum durch Ziehen."),
            t("Nutze das Lineal, um Betrag, Prozentänderung und Kalendertage zwischen zwei Punkten zu messen."),
            t("Blende über Vergleichen einen verfügbaren Index ein."),
          ],
          note: t("Chartfilter begrenzen nur den jeweiligen Chart. Auswertungen darunter besitzen eigene Filter."),
        },
      ],
    },
    {
      id: "transactions",
      title: t("Transaktionen und Kreditkarten"),
      summary: t("Einnahmen und Ausgaben analysieren sowie interne Bewegungen korrekt behandeln."),
      sections: [
        {
          title: t("Einnahmen und Ausgaben auswerten"),
          steps: [
            t("Begrenze bei Bedarf Bank, Konto und Zeitraum."),
            t("Wähle Ausgaben oder Einnahmen und danach die Darstellung nach Kategorie oder Monat."),
            t("Öffne eine Kategorie oder einen Monatswert, um die zugehörigen Buchungen zu prüfen."),
            t("Ziehe in der Monatsmatrix mit gedrückter linker Maustaste über mehrere Zellen. Mit Strg oder Cmd kannst du weitere Zellen und Bereiche hinzufügen."),
          ],
          paragraphs: [t("Weitere Kategorien bündelt kleinere Werte. Klappe die Zeile auf, um jede enthaltene Kategorie einzeln zu sehen.")],
        },
        {
          title: t("Umbuchungen und Kartenausgleiche"),
          paragraphs: [t("Markiere Überträge zwischen eigenen Konten und Kreditkartenausgleiche über das Drei-Punkte-Menü. Dadurch werden interne Bewegungen nicht als zusätzliche Einnahmen oder Ausgaben gezählt."), t("Unter Umbuchungen & Ausgleiche kannst du Markierungen prüfen und wiederherstellen.")],
        },
        {
          title: t("Kreditkarte einrichten"),
          paragraphs: [t("Für jedes Kartenkonto kann eine Regel eingerichtet werden, die Gutschriften und Ausgleichsbuchungen erkennt.")],
          steps: [
            t("Wähle das Kartenkonto und starte Karte einrichten."),
            t("Wähle eine passende Beispielbuchung und prüfe Richtung, Währung und Treffer."),
            t("Lege fest, ob die Regel für zukünftige Importe gilt, und speichere sie."),
          ],
        },
      ],
    },
    {
      id: "categories",
      title: t("Kategorien und Regeln"),
      summary: t("Auswertungen strukturieren und Kartenkäufe automatisch zuordnen."),
      sections: [
        {
          title: t("Kategorien verwalten"),
          paragraphs: [t("Du kannst Kategorien anlegen sowie Namen und Farben direkt in der Liste bearbeiten. Zusammenführen und Löschen zeigen vor dem Speichern die betroffenen Buchungen und Regeln.")],
        },
        {
          title: t("Kartenkäufe automatisch zuordnen"),
          paragraphs: [t("Links steht die vom Kartenanbieter gelieferte Kreditkarten-Kategorie, rechts deine Saldonaut-Kategorie."), t("Die Auswahl gilt für bestehende und künftig importierte Kartenkäufe. Manuelle Zuordnungen und speziellere Händlerregeln bleiben erhalten.")],
        },
      ],
    },
    {
      id: "tax-history",
      title: t("Steuerhistorie"),
      summary: t("Jährliche Steuerwerte importieren und unabhängig von Bankständen vergleichen."),
      sections: [
        {
          title: t("Steuerwerte übernehmen"),
          steps: [
            t("Öffne Import, Dateien importieren und danach Steuererklärungen."),
            t("Wähle eine unterstützte Datei und prüfe die erkannten Jahreswerte."),
            t("Öffne Steuerhistorie, um Vermögen, Immobilien, übrige Werte und Schulden zu vergleichen."),
          ],
          note: t("Die Steuerhistorie ist eine separate Jahresbetrachtung und wird nicht automatisch mit laufenden Bankständen vermischt."),
        },
      ],
    },
    {
      id: "chat",
      title: t("Finanzchat"),
      summary: t("Fragen zu ausgewählten Finanzdaten stellen und die Freigabe kontrollieren."),
      sections: [
        {
          title: t("ChatGPT verbinden"),
          steps: [
            t("Öffne Finanzchat und wähle Mit ChatGPT anmelden."),
            t("Schliesse die Anmeldung im geöffneten Fenster ab."),
            t("Prüfe vor dem Senden das vorbereitete Datenpaket."),
          ],
          paragraphs: [t("Der Finanzchat ist rein lesend und verändert keine Konten, Buchungen oder Kategorien.")],
        },
        {
          title: t("Datenschutz und Fearless-Modus"),
          paragraphs: [t("Im optionalen Fearless-Modus können auch Buchungsbeschreibungen übertragen werden. Darin können persönliche Angaben enthalten sein."), t("Die lokale Spracherkennung für deutsche Fragen lädt kein Audio hoch. Neuer Chat, Abmelden oder Sperren beendet den flüchtigen Gesprächskontext.")],
        },
      ],
    },
    {
      id: "security",
      title: t("Daten, Backup und Sicherheit"),
      summary: t("Verschlüsselte Profile sichern, wiederherstellen und schützen."),
      sections: [
        {
          title: t("Backup erstellen"),
          steps: [
            t("Öffne Daten & Sicherheit und danach Backup."),
            t("Wähle Backup erstellen und speichere die verschlüsselte Sicherungsdatei an einem sicheren Ort."),
            t("Bewahre mindestens eine Kopie auf einem anderen Datenträger auf."),
          ],
          note: t("Zur Wiederherstellung benötigst du das Passwort, das zum Zeitpunkt der Sicherung galt."),
        },
        {
          title: t("Backup wiederherstellen"),
          paragraphs: [t("Wähle eine Sicherungsdatei, vergib einen Profilnamen und gib das Passwort des Backups ein."), t("Die Wiederherstellung legt ein zusätzliches Finanzprofil an. Bestehende Profile bleiben unverändert.")],
        },
      ],
    },
    {
      id: "settings",
      title: t("Einstellungen"),
      summary: t("Darstellung, Sprache, Region, automatische Sperre und Marktpreise anpassen."),
      sections: [
        {
          title: t("Darstellung und Allgemein"),
          bullets: [
            t("Wähle eine helle, dunkle oder systemabhängige Darstellung."),
            t("Lege Sprache, Ländereinstellungen und die Standardwährung für neue Konten fest."),
            t("Bestimme, nach welcher Inaktivität Saldonaut automatisch gesperrt wird."),
          ],
          note: t("Ländereinstellungen ändern nur die Darstellung. Gespeicherte Werte werden nicht umgerechnet."),
        },
        {
          title: t("Automatische Marktpreise"),
          paragraphs: [t("Optionale API-Schlüssel werden verschlüsselt im aktuellen Profil gespeichert. Ohne Schlüssel verwendet Saldonaut Yahoo Finance als Rückfall."), t("An Kursanbieter werden Wertpapierkennungen, Zeiträume und Währungspaare übertragen, jedoch keine Kontostände oder Buchungen.")],
        },
      ],
    },
    {
      id: "faq",
      title: t("Häufige Fragen"),
      summary: t("Antworten zu Summen, Duplikaten, Währungen, Marktwerten und Passwörtern."),
      sections: [
        {
          title: t("Warum stimmt mein Gesamtvermögen nicht mit allen sichtbaren Konten überein?"),
          paragraphs: [t("Prüfe ausgeschlossene oder archivierte Konten, die angezeigte Bewertungswährung und negative Kreditkartensalden.")],
        },
        {
          title: t("Warum erscheint eine Buchung doppelt?"),
          paragraphs: [t("Prüfe unter Importierte Dateien, ob derselbe Zeitraum aus mehreren Dateien importiert wurde. Abweichende Beschreibungen, Daten oder Quellformate können eine eindeutige Erkennung erschweren.")],
          note: t("Lösche nicht vorschnell einzelne Buchungen, sondern kläre zuerst den betroffenen Import."),
        },
        {
          title: t("Warum fehlen aktuelle Marktwerte?"),
          paragraphs: [t("Prüfe Wertpapierkennung, Börse, Internetverbindung und Einstellungen → Marktpreise. Dort kannst du einen neuen Abruf starten und Fehlermeldungen sehen.")],
        },
        {
          title: t("Ich habe mein Passwort vergessen. Was kann ich tun?"),
          paragraphs: [t("Es gibt keinen Passwort-Reset. Ein Profil und seine Backups können nur mit dem jeweils gültigen Passwort geöffnet werden.")],
        },
      ],
    },
  ];
}
