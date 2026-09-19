# ADR: Konsumbasiertes Ausgabenmodell

Status: Akzeptiert. Ersetzt die reine Bankbuchungs-Sicht.

Kreditkartenkäufe zählen mit ihrem Buchungsdatum und ihrer Kategorie zu den
allgemeinen Konsumausgaben. Nur bestätigte oder eindeutig durch einen Provider
typisierte Händlererstattungen mindern die Ausgaben ihrer Kategorie, auch wenn
die Nettosumme negativ wird. Ungeklärte Kartengutschriften sind weder Einkommen
noch Erstattung; sie bleiben aus den Summen ausgeschlossen und werden als offene
Prüffälle ausgewiesen. Betroffene Berichte sind bis zur Prüfung unvollständig.
Die vorhandenen Importdaten liefern nicht immer ein separates Kaufdatum;
ein abweichendes Kaufdatum wird nicht aus dem Belegtext erfunden.

Die Bankbelastung und die Gutschrift auf dem Kartenkonto können unabhängig
voneinander als Rechnungsausgleich markiert werden. Beide zählen dann weder
als Konsumausgabe noch als Einnahme. Es gibt keine automatische Verknüpfung
der beiden Kontoseiten, keine Bank-Textmuster in der gemeinsamen Berechnung und keinen
Abgleich mit Summen einzelner Kartenkäufe. Nutzerbestätigte Importregeln
sind in der Erweiterung unten beschrieben.

## Persistenz ohne Bestandsmigration

Die logischen Transaktionsattribute exclude_from_cashflow, is_settlement und
is_manually_overridden liegen physisch in transaction_reporting_flags, einer
1:1-Erweiterung von transactions mit Fremdschlüssel und kaskadierender Löschung.
Fehlt der Eintrag, gelten alle Flags als false. Bestehende Buchungen werden
nicht umklassifiziert. Keine ALTER-/UPDATE-Migration und keine persönlichen
Kontoregeln werden ausgeliefert.

is_settlement erzwingt exclude_from_cashflow per Datenbank-Constraint. Auch
das Aufheben setzt is_manually_overridden=true. Neue Importe dürfen eindeutige
Provider-Kennzeichnungen und bestätigte Regeln übernehmen, aber keine manuellen
Entscheidungen überschreiben. Ein deduplizierter Wiederimport erhält diese ebenfalls.
Wird der Ursprungsimport explizit gelöscht, werden auch seine Markierungen
gelöscht. Ein anschliessender Neuimport ist keine Wiederherstellung dieser Daten.

## Gemeinsame Berechnung

Die sprachneutrale View reporting_transactions liefert Einnahmen und signierte
Konsumausgaben für alle Berichte und den Finanzchat. Positive Kartenbeträge sind
keine Einnahmen. Rückerstattungen müssen der passenden Kategorie zugeordnet sein;
ohne Kaufverknüpfung wird die ursprüngliche Kategorie nicht automatisch erraten.

Kontosalden, Buchungshistorien, Bewertungen und gespeicherte Geldbewegungen bleiben
unverändert. Das Flag betrifft Konsum-/Budgetauswertungen und die bereinigte
Geldflussauswertung (nicht den tatsächlichen Bankumsatz). Die bestehende
CHF-Auswertung summiert keine fremden Währungen und erfindet keine Wechselkurse.

## Oberfläche und Grenzen

Die Buchungsliste erlaubt Markieren und Aufheben über ein kompaktes Drei-Punkte-Menü.
Neutrale Buchungen bleiben mit Badge sichtbar, ohne zu Summen beizutragen.
Der Reiter Umbuchungen & Ausgleiche (Hash-Route #transactions/transfers)
bietet Konto-, Datums- und Textfilter sowie atomare Sammelwiederherstellung.
Ein Hinweis
erinnert bei markierten Ausgleichen an den separaten Import der Kartenkäufe.
Es wird keine Vollständigkeitsprüfung behauptet.

Nicht markierte Abrechnungen können weiterhin Doppelzählungen verursachen.
Eine unmarkierte, nicht bestätigte Gutschrift bleibt ein offener Prüffall.
Beide Seiten müssen bewusst markiert werden. Alle Aktionen und Hinweise sind
übersetzt; die Berechnung hängt weder von der UI- noch von der Belegsprache ab.

## Bestätigte Regeln für wiederkehrende Kartenausgleiche

Beim Markieren wird zuerst gefragt: nur diese Buchung oder alle Vorschautreffer?
Der Nutzer prüft einen editierbaren Textanfang und die Treffer über die gesamte
Historie. Konto, Währung und Zahlungsrichtung müssen übereinstimmen. Gross-/
Kleinschreibung und Leerraum werden vereinheitlicht; Zahlen werden nicht entfernt.
Es gibt keine bankspezifischen Begriffe und kein Fuzzy-Matching.

Nur nach zusätzlichem Opt-in wird eine Regel für künftige Importe gespeichert.
Bestehende Vorschautreffer werden atomar als manuell bestätigt markiert;
spätere automatische Treffer bleiben manuell übersteuerbar. Alle anderen
manuellen Entscheidungen, insbesondere Wiederherstellungen, sind geschützt.
Ändert sich die Treffermenge seit der Vorschau, wird die Bestätigung abgelehnt.

Unter Umbuchungen & Ausgleiche lassen sich gespeicherte Regeln deaktivieren.
Bestehende Markierungen bleiben dabei unverändert. Die Gegenkontoseite braucht
eine eigene Entscheidung. Regeln werden nur bei neuen Imports angewendet,
nicht beim Start auf historische Daten. Bei Anonymisierung der Beschreibungen
werden die Regeln in der anonymisierten Kopie entfernt.

## Erweiterung: Eigene Kontoüberträge

Der sprachneutrale API-Typ transfer_type unterscheidet NONE,
CREDIT_CARD_SETTLEMENT und INTERNAL_TRANSFER. Physisch wird er ohne Migration
aus den vorhandenen Flags abgebildet: nicht ausgeschlossen = NONE,
ausgeschlossen und is_settlement = CREDIT_CARD_SETTLEMENT, sonst INTERNAL_TRANSFER.
Manuelle Änderungen und Wiederherstellungen setzen stets is_manually_overridden.
Kartenkäufe und Händlererstattungen werden nicht als Umbuchung markiert.
