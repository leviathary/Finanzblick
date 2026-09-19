# Kreditkarten einrichten

Unter Transaktionen → Kartentransaktionen → Karte einrichten führt der Assistent
durch vier Schritte. Die Desktop-App verwendet die Hash-Route
`#transactions/cards/setup`.

1. Aktives Kreditkartenkonto auswählen.
2. Eine Gutschrift zum Rechnungsausgleich auswählen, Textanfang und Treffer
   kontrollieren. Andere Gutschriften können als Händlererstattung mit Kategorie
   bestätigt oder bewusst ungeklärt gelassen werden.
3. Optional das zahlende Bankkonto und eine Beispielabbuchung auswählen. Die
   Vorschau ist getrennt nach Konto, Währung und Zahlungsrichtung.
4. Beide Regeln und Einzelfallentscheidungen bestätigen. Historische Anwendung
   und zukünftige Imports sind separat wählbar. Alle Änderungen werden gemeinsam
   in einer Transaktion gespeichert; ein Fehler übernimmt keine Teiländerung.

Bereits getroffene manuelle Entscheidungen anderer Buchungen bleiben geschützt.
Bei geänderten Treffermengen muss die Vorschau neu geprüft werden. Regeln, die
eine separat als Erstattung/ungeklärt eingestufte Buchung erfassen, werden nicht
übernommen: Der Textanfang muss zunächst präzisiert werden.

## Typisierung

- Kartenkauf: normale Ausgabe mit Kategorie.
- Kartenausgleich: importiert und saldowirksam, aber neutral in Berichten.
- Händlererstattung: mindert nur die zugewiesene Ausgabenkategorie.
- Ungeklärte Gutschrift: weder Einnahme noch Erstattung. Sichtbarer Prüffall;
  Summen sind bis zur Prüfung unvollständig.

Manuelle Kreditentscheidungen liegen in einer eigenen 1:1-Tabelle
`card_credit_decisions`. Es gibt keine historische Datenmigration.

Der UBS-Provider erkennt bei neuen Kartenimports die exakten positiven
Belegzeilen `2002 LSV-ZAHLUNG` und `LSV-Zahlung` (ggf. mit getrennten Zusatzdetails).
Andere Texte, Anbieter und gewöhnliche Banklastschriften werden daraus nicht
abgeleitet. Historische Treffer werden nur vorgeschlagen, nicht automatisch
umklassifiziert. Andere Provider bleiben ohne sichere Erkennung ungeklärt.

Die Kartentransaktionsansicht ist ein Kontofilter, keine zusätzliche
Ausgabenkategorie. Sie bietet jederzeit die Prüfung offener Gutschriften.
Gespeicherte Regeln werden weiterhin unter Umbuchungen & Ausgleiche verwaltet.
Es wird keine Vollständigkeit behauptet und kein Einkauf einer Rechnung zugeordnet.
