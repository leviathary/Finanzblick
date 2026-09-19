# Automatische Umbuchungsregeln

Unter Transaktionen → Umbuchungen & Ausgleiche lassen sich benannte Regeln
anlegen, bearbeiten und deaktivieren. Typen sind eigene Umbuchung und
Kartenausgleich. Jede Regel gilt für genau ein Konto, eine Währung und eine
Zahlungsrichtung. Der normalisierte Buchungstext muss mit dem angegebenen
Text beginnen (mindestens acht Zeichen). Groß-/Kleinschreibung und mehrfache
Leerzeichen sind unerheblich; Datum und Betrag gehören nicht zur Bedingung.

Die Vorschau ist unverbindlich. Speichern prüft die Bedingungen erneut und
wendet historische Änderungen und die Regeländerung in einer DB-Transaktion an.
Bei historischen Anwendungen müssen die Treffer-IDs der bestätigten Vorschau
entsprechen. Mehr als 5000 historische Änderungen werden nicht angewendet.
Die Ansicht zeigt höchstens 200 Vorschautreffer, nennt aber die Gesamtzahl.

Zukünftige Importe sind standardmäßig ausgewählt, historische Änderungen nicht.
Bei Bearbeitung ohne zukünftige Anwendung wird die gespeicherte Regel entfernt;
die Oberfläche weist darauf hin. Deaktivieren verändert keine Buchungsmarkierungen.
Regeländerungen setzen außerhalb der neuen Treffer keine alten Markierungen zurück.
Manuell markierte oder wiederhergestellte Buchungen werden nie überschrieben.
Überlappende Textanfänge mit unterschiedlichen Wirkungen sind nicht zulässig.

Für Privatkonto → Haushaltskonto werden Belastungs- und Gutschriftsseite jeweils
separat eingerichtet. Kein Zwangsmatching, keine Veränderung von Beträgen oder Salden.

## Speicherung ohne Bestandsdatenmigration

Bestehende Kartenregeln verbleiben in `settlement_rules`. Die zusätzliche
Funktionstabelle `transfer_rule_details` speichert Name und Regeltyp, referenziert
über die Regel-ID. Ohne Details bleibt die bisherige Bedeutung Kartenausgleich.
Es gibt keine ALTER-/UPDATE-Migration bestehender Konten, Regeln oder Buchungen.
Die Kartenstatus-API liefert ausschließlich Kartenausgleichsregeln; die allgemeine
Regelverwaltung verwendet eine eigene API mit beiden Typen.
