# Mitgelieferte PDF-Profile

Diese JSON-Dateien beschreiben getestete, versionsgebundene Layouts bekannter
Anbieter. Sie werden mit `include_str!` in die Anwendung eingebettet und von den
gemeinsamen Engines in `formats/pdf_profile.rs` beziehungsweise
`formats/pdf_card_profile.rs` ausgeführt.

Ein Profil enthält ausschließlich deklarative Regeln für Erkennung, Metadaten,
Tabellenbeginn und -ende, reguläre Ausdrücke, Capture-Gruppen, Seitenkopfregeln,
Kontrollsummen und Transaktionsreferenzen. Es darf weder Kundendaten noch
ausführbaren Code enthalten. Neue Layoutversionen erhalten eine neue Datei und
synthetische positive sowie negative Regressionstests.

Die Kontoauszugsengine bietet dafür unter anderem gemeinsame oder getrennte Kontrollsummen,
alternative Zeilenlayouts und die Vorzeichenbestimmung über Saldenabgleich. Ein
Profil kann Anfangs- und Schlusssaldo samt Periodendaten auch aus getrennten
Zusammenfassungszeilen erfassen, wenn die Buchungstabelle diese Angaben nicht
in einer gemeinsamen datierten Zeile ausweist. Ein
Kreditkartenprofil beschreibt zusätzlich Kartenwechsel, mehrzeilige Buchungen,
Seitenüberträge und Kartentotale für die anbieterneutral benannte
Kreditkartenengine. Nicht deklarativ abbildbare Dokumentfamilien bleiben enge
Provider-Hooks.
Das optionale Feld `warning` ist ausschließlich für handlungsrelevante Unsicherheit
bestimmt. Eine erfolgreiche Erkennung oder Kontrollsummenprüfung erzeugt keine
Warnung.

Benutzerprofile aus dem PDF-Zuordnungsdialog sind davon getrennt. Sie bleiben
absichtlich weniger mächtig und werden verschlüsselt im jeweiligen Finanzprofil
gespeichert; mitgelieferte Providerprofile sind dagegen Teil des Programmcodes.
