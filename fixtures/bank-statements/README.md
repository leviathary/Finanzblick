# Synthetische Bankauszüge

Diese Dateien dienen ausschliesslich der Entwicklung und automatisierten Tests.
Alle Personen, Kontonummern, IBANs, Policen, Buchungstexte und Beträge sind frei
erfunden. Es wurden keine Werte aus dem bereitgestellten UBS-Dokument übernommen.

## Abgedeckte Formate

| Anbieter | Excel-Logik | PDF-Logik | Besonderheit |
| --- | --- | --- | --- |
| UBS | Belastung und Gutschrift getrennt | laufender Kontostand | zusätzliche Saldoübersichten sind möglich |
| Migros Bank | Soll und Haben getrennt | kompakte Kontoübersicht | bankeigene Referenz pro Buchung |
| Raiffeisen | ein vorzeichenbehafteter Betrag | Transaktionsliste | positive und negative Beträge in einer Spalte |
| Generali | Vertragsbewegungen | Vorsorgeübersicht | Anbieter/Police statt Bank/Konto |

## Erwartetes internes Modell

Die Importer normalisieren alle Quellformate auf:

- `provider`
- `provider_type` (`bank`, `insurance`, `broker` oder `pension`)
- `account_id`
- `booking_date`
- `value_date`
- `description`
- `amount_minor` (Gutschrift positiv, Belastung negativ)
- `currency`
- `balance_minor`
- `source_file`
- `source_row` oder `source_page`
- `fingerprint` zur Duplikaterkennung

PDF-Importe benötigen zusätzlich einen Erkennungsstatus und sollen unklare Zeilen
vor dem Speichern in einer Vorschau anzeigen.
