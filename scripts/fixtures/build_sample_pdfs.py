# Erzeugt synthetische PDF-Bankbelege als Import-Testdaten.

from pathlib import Path
from reportlab.lib import colors
from reportlab.lib.pagesizes import A4
from reportlab.lib.styles import getSampleStyleSheet, ParagraphStyle
from reportlab.lib.units import mm
from reportlab.pdfbase.ttfonts import TTFont
from reportlab.pdfbase import pdfmetrics
from reportlab.platypus import SimpleDocTemplate, Paragraph, Spacer, Table, TableStyle, PageBreak

ROOT = Path.cwd()
OUT = ROOT / "fixtures" / "bank-statements" / "pdf"
OUT.mkdir(parents=True, exist_ok=True)

font_path = Path("C:/Windows/Fonts/arial.ttf")
bold_path = Path("C:/Windows/Fonts/arialbd.ttf")
if font_path.exists():
    pdfmetrics.registerFont(TTFont("AppSans", str(font_path)))
    pdfmetrics.registerFont(TTFont("AppSansBold", str(bold_path)))
    BODY, BOLD = "AppSans", "AppSansBold"
else:
    BODY, BOLD = "Helvetica", "Helvetica-Bold"

NAVY = colors.HexColor("#10223F")
GREEN = colors.HexColor("#176B57")
LIGHT = colors.HexColor("#EAF1FB")
GRID = colors.HexColor("#DDE3E8")

samples = [
    {
        "file": "ubs_kontoauszug_2026-08.pdf", "provider": "UBS", "kind": "Kontoauszug",
        "account": "Privatkonto CHF | IBAN CH00 0000 0000 0000 1000 1", "period": "01.08.2026 - 31.08.2026",
        "headers": ["Datum", "Informationen", "Belastung", "Gutschrift", "Valuta", "Kontostand"],
        "rows": [["01.08.26", "Anfangssaldo", "", "", "01.08.26", "7'840.25"], ["03.08.26", "Muster Immobilien AG - Miete", "1'850.00", "", "03.08.26", "5'990.25"], ["05.08.26", "Coop Supermarkt Zürich", "126.45", "", "05.08.26", "5'863.80"], ["07.08.26", "Arbeitgeber Beispiel AG - Lohn", "", "6'850.00", "07.08.26", "12'713.80"], ["09.08.26", "SBB Mobile", "68.00", "", "09.08.26", "12'645.80"], ["12.08.26", "Swisscom Rechnung", "89.90", "", "12.08.26", "12'555.90"], ["16.08.26", "Übertrag an Sparkonto", "1'200.00", "", "16.08.26", "11'355.90"], ["20.08.26", "Restaurant Seeblick", "142.80", "", "20.08.26", "11'213.10"], ["25.08.26", "Krankenkasse Muster", "428.60", "", "25.08.26", "10'784.50"], ["31.08.26", "Schlusssaldo", "", "", "31.08.26", "10'784.50"]],
        "summary": [["Anfangssaldo", "7'840.25"], ["Total Gutschriften", "6'850.00"], ["Total Belastungen", "3'905.75"], ["Schlusssaldo", "10'784.50"]],
    },
    {
        "file": "migros_bank_konto_2026-08.pdf", "provider": "Migros Bank", "kind": "Kontoübersicht",
        "account": "Sparkonto | Konto MB-TEST-2001", "period": "August 2026",
        "headers": ["Datum", "Buchungstext", "Soll CHF", "Haben CHF", "Saldo CHF"],
        "rows": [["01.08.2026", "Saldovortrag", "", "", "28'420.10"], ["04.08.2026", "Kartenzahlung Migros Markt", "94.70", "", "28'325.40"], ["08.08.2026", "Eingang Dauerauftrag", "", "1'200.00", "29'525.40"], ["14.08.2026", "Apotheke am Bahnhof", "47.30", "", "29'478.10"], ["18.08.2026", "Online-Shop Beispiel", "219.00", "", "29'259.10"], ["22.08.2026", "Zinsgutschrift", "", "18.40", "29'277.50"], ["31.08.2026", "Schlusssaldo", "", "", "29'277.50"]],
        "summary": [["Anfangssaldo", "28'420.10"], ["Gutschriften", "1'218.40"], ["Belastungen", "361.00"], ["Schlusssaldo", "29'277.50"]],
    },
    {
        "file": "raiffeisen_transaktionen_2026-08.pdf", "provider": "Raiffeisen", "kind": "Kontoauszug",
        "account": "Privatkonto | IBAN CH00 0000 0000 0000 3000 3", "period": "01.08.2026 - 31.08.2026",
        "headers": ["Buchungstag", "Valuta", "Buchungstext", "Betrag CHF", "Saldo CHF"],
        "rows": [["01.08.2026", "01.08.2026", "Anfangssaldo", "0.00", "15'640.00"], ["02.08.2026", "02.08.2026", "TWINT - Bäckerei Muster", "-34.50", "15'605.50"], ["06.08.2026", "06.08.2026", "Spesenrückerstattung", "+240.00", "15'845.50"], ["11.08.2026", "11.08.2026", "Elektrizitätswerk", "-165.20", "15'680.30"], ["15.08.2026", "15.08.2026", "Tankstelle Beispiel", "-76.80", "15'603.50"], ["21.08.2026", "21.08.2026", "Säule 3a Einzahlung", "-500.00", "15'103.50"], ["27.08.2026", "27.08.2026", "TWINT - Freizeit", "-58.40", "15'045.10"], ["31.08.2026", "31.08.2026", "Schlusssaldo", "0.00", "15'045.10"]],
        "summary": [["Anfangssaldo", "15'640.00"], ["Gutschriften", "240.00"], ["Belastungen", "834.90"], ["Schlusssaldo", "15'045.10"]],
    },
    {
        "file": "generali_vorsorge_2026.pdf", "provider": "Generali", "kind": "Vorsorgeübersicht",
        "account": "Gebundene Vorsorge 3a | Police GE-TEST-4001", "period": "Stand 31.08.2026",
        "headers": ["Datum", "Vorgang", "Belastung", "Gutschrift", "Vertragswert CHF"],
        "rows": [["01.01.2026", "Anfangswert", "", "", "36'580.00"], ["31.01.2026", "Prämienzahlung", "", "500.00", "37'080.00"], ["28.02.2026", "Prämienzahlung", "", "500.00", "37'580.00"], ["31.03.2026", "Risikoprämie", "42.00", "", "38'038.00"], ["30.04.2026", "Prämienzahlung", "", "500.00", "38'538.00"], ["31.05.2026", "Prämienzahlung", "", "500.00", "39'038.00"], ["30.06.2026", "Wertentwicklung", "", "614.20", "39'652.20"], ["31.07.2026", "Prämienzahlung", "", "500.00", "40'152.20"], ["31.08.2026", "Vertragswert", "", "", "40'152.20"]],
        "summary": [["Vertragswert 01.01.2026", "36'580.00"], ["Prämien und Entwicklung", "3'614.20"], ["Kosten", "42.00"], ["Vertragswert 31.08.2026", "40'152.20"]],
    },
]

styles = getSampleStyleSheet()
title_style = ParagraphStyle("Title", parent=styles["Title"], fontName=BOLD, textColor=NAVY, fontSize=18, leading=22, alignment=0)
small = ParagraphStyle("Small", parent=styles["BodyText"], fontName=BODY, textColor=colors.HexColor("#56667A"), fontSize=8.5, leading=11)
body = ParagraphStyle("Body", parent=styles["BodyText"], fontName=BODY, fontSize=9, leading=12)

def footer(canvas, doc):
    canvas.saveState()
    canvas.setFont(BODY, 7.5)
    canvas.setFillColor(colors.HexColor("#748297"))
    canvas.drawString(18 * mm, 12 * mm, "Ausschliesslich synthetische Testdaten - keine echte Bankverbindung")
    canvas.drawRightString(192 * mm, 12 * mm, f"Seite {doc.page}")
    canvas.restoreState()

for sample in samples:
    path = OUT / sample["file"]
    doc = SimpleDocTemplate(str(path), pagesize=A4, rightMargin=18*mm, leftMargin=18*mm, topMargin=18*mm, bottomMargin=20*mm, title=sample["kind"], author="Saldonaut Test Fixtures")
    story = [
        Paragraph(sample["provider"], title_style),
        Paragraph(sample["kind"], ParagraphStyle("Kind", parent=title_style, fontSize=12, textColor=GREEN)),
        Spacer(1, 5*mm),
        Paragraph("TESTDOKUMENT - frei erfundene Angaben", ParagraphStyle("Warning", parent=body, fontName=BOLD, textColor=colors.HexColor("#B42318"))),
        Spacer(1, 4*mm),
        Paragraph(sample["account"], body),
        Paragraph(sample["period"], small),
        Spacer(1, 6*mm),
    ]
    summary = Table(sample["summary"], colWidths=[60*mm, 35*mm], hAlign="LEFT")
    summary.setStyle(TableStyle([("FONTNAME", (0,0), (-1,-1), BODY), ("FONTNAME", (0,-1), (-1,-1), BOLD), ("ALIGN", (1,0), (1,-1), "RIGHT"), ("BACKGROUND", (0,0), (-1,-1), LIGHT), ("GRID", (0,0), (-1,-1), 0.5, GRID), ("PADDING", (0,0), (-1,-1), 6)]))
    story += [summary, Spacer(1, 8*mm)]
    data = [[Paragraph(str(c), ParagraphStyle("Head", parent=small, fontName=BOLD, textColor=colors.white)) for c in sample["headers"]]]
    for row in sample["rows"]:
        data.append([Paragraph(str(c), small) for c in row])
    widths_by_provider = {
        "UBS": [22, 55, 22, 22, 22, 31],
        "Migros Bank": [26, 64, 25, 25, 34],
        "Raiffeisen": [26, 26, 62, 28, 32],
        "Generali": [26, 64, 24, 24, 36],
    }
    widths = [value * mm for value in widths_by_provider[sample["provider"]]]
    table = Table(data, colWidths=widths, repeatRows=1)
    table.setStyle(TableStyle([("BACKGROUND", (0,0), (-1,0), NAVY), ("FONTNAME", (0,0), (-1,0), BOLD), ("GRID", (0,0), (-1,-1), 0.35, GRID), ("VALIGN", (0,0), (-1,-1), "TOP"), ("ALIGN", (2,1), (-1,-1), "RIGHT"), ("ROWBACKGROUNDS", (0,1), (-1,-1), [colors.white, colors.HexColor("#F7F9FB")]), ("PADDING", (0,0), (-1,-1), 5)]))
    story += [table, Spacer(1, 8*mm), Paragraph("Dieses Dokument wurde für automatisierte Importtests von Saldonaut erzeugt. Sämtliche Personen, Konten, Referenzen und Beträge sind synthetisch.", small)]
    doc.build(story, onFirstPage=footer, onLaterPages=footer)
    print(path)
