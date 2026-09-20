# camt.053- und camt.054-Import (ISO 20022)

XML-Dateien können über Dateiauswahl oder Ordnerimport eingelesen werden.
Unterstützte Namensräume: camt.053.001.02, .04, .08 und .10 sowie
camt.054.001.02, .04, .08 und .10.
Der Parser verarbeitet UTF-8 und UTF-16 mit BOM, berücksichtigt XML-Namensräume
und lädt keine externen Entitäten oder Schemata.

## Abbildung

| Quelle | Normalisiertes Feld |
| --- | --- |
| Stmt bzw. Ntfctn/CreDtTm, sonst GrpHdr/CreDtTm | documentDate |
| XML-Namensraum | recordDefinitionId |
| Stmt bzw. Ntfctn/Acct/Id/IBAN, sonst Othr/Id | accountReference |
| Stmt bzw. Ntfctn/Acct/Ccy | currency |
| camt.053 Bal OPBD / CLBD | datierte Anfangs-/Schlusssalden |
| Ntry/BookgDt und ValDt | bookingDate / valueDate |
| Ntry/Amt und CdtDbtInd | exakter, vorzeichenbehafteter Betrag |
| Ntry/AcctSvcrRef, sonst NtryRef | externalReference mit passendem Namensraum |
| einzelne TxDtls/Refs/AcctSvcrRef | Ersatz, wenn die Buchung keine Bankreferenz hat |
| TxDtls/RltdPties/Dbtr bzw. Cdtr | counterpartyName |
| RmtInf/Ustrd, Strd/CdtrRefInf/Ref, AddtlRmtInf | remittanceInformation |

Die Bank wird im Import gewählt oder aus dem über die Kontoreferenz gewählten
Konto übernommen. Es gibt keine bankabhängige XML-Parserlogik.

## Buchungs- und Prüfregeln

- Ein Ntry wird genau einmal gebucht. Mehrere TxDtls werden als Zahlungsdetails
  derselben Sammelbuchung angezeigt, nicht zusätzlich als Geldbewegungen.
- BOOK wird importiert, PDNG und INFO werden mit einem Hinweis ausgelassen.
  Fehlende/unbekannte Statuscodes führen zu einem Fehler.
- Ein Storno behält das vom Auszug angegebene Vorzeichen.
- Bei camt.053 muss OPBD plus gebuchte Bewegungen exakt CLBD ergeben.
- Die App speichert Beträge in Hundertsteln. Höhere nicht-null Präzision und
  abweichende Buchungswährungen werden abgewiesen, nicht gerundet/umgerechnet.
- Bei camt.053 wird OPBD am ersten Buchungstag als Anfangsbestand des Vortags gespeichert.
- camt.054 enthält Buchungsbenachrichtigungen ohne vollständigen Saldenrahmen;
  deshalb werden daraus Buchungen, aber keine Kontosalden übernommen.
- Bankreferenzen sind innerhalb des Zielkontos und Referenztyps eindeutig.
  Unterschiedliche Referenzen bleiben auch bei gleichem Text/Betrag erhalten.
  Ohne Referenz gelten die gemeinsamen Fingerprint-Regeln. Eine beliebige
  formatübergreifende Dublettenerkennung ist damit nicht garantiert.

## Grenzen

Pro Datei ist genau ein Stmt- oder Ntfctn-Abschnitt vorgesehen. Mehrere Abschnitte
werden abgewiesen; bei camt.053 gilt dies auch für fehlende OPBD-/CLBD-Salden.
Sammelbuchungen werden noch nicht in einzelne Zahlungen aufgeteilt.
Dies ist ein Parser der unterstützten Felder, keine vollständige XSD-Validierung.
MsgId, Stmt/Id, Ntfctn/Id, BkTxCd und weitere Detailreferenzen sind noch keine separat
gespeicherten Modellfelder; EndToEndId wird im Buchungstext erhalten.
camt.052 ist nicht freigeschaltet.

Die Tests verwenden synthetische XML-Dateien. Echte Exporte der einzelnen Banken
und ein macOS-Build sind damit noch nicht verifiziert.

Referenzen:
- [SIX Payment Standards](https://www.six-group.com/en/products-services/banking-services/payment-standardization/downloads-faq/download-center.html)
- [roxmltree Parseroptionen](https://docs.rs/roxmltree/0.21.1/roxmltree/struct.ParsingOptions.html)
