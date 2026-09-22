// Zeigt den gemeinsamen Mengenabgleich und die Stichtagsauswahl für Depotbestände aller Provider.

import { t, tr, locale } from "../../i18n";
import type { ImportAccount, SavePositionSnapshotResult, PositionSnapshotChange, PositionSnapshotPreview } from "./importTypes";

interface Props {
  preview: PositionSnapshotPreview;
  accountId: number | null;
  accounts: ImportAccount[];
  busy: boolean;
  result?: SavePositionSnapshotResult;
  dateEditable: boolean;
  onDateChange: (value: string) => void;
  onAccountChange: (accountId: number | null) => void;
}

export function PositionSnapshotReview({ preview, accountId, accounts, busy, result, onAccountChange, dateEditable, onDateChange }: Props) {
  const matchingAccounts = positionSnapshotAccounts(accounts, preview);
  const changes = preview.changes;
  const rows = accountId && changes.length ? changes : preview.positions;
  const changeCount = changes.filter(change => change.action !== "unchanged").length;

  return <>
    {result && <div className="mapping-ready-notice" role="status"><strong>{result.duplicate ? t("Dieser Positionsbestand wurde bereits importiert.") : t("Positionsabgleich gespeichert")}</strong><p>{tr`${result.createdPositions} neue Positionen · ${result.updatedPositions} aktualisiert · ${result.zeroedPositions} auf null gesetzt · ${result.unchangedPositions} unverändert`}</p></div>}
    <div className="position-snapshot-meta"><div><span>{t("Dokumenttyp")}</span><strong>{t("Positionsbestand")}</strong></div><div><span>{t("Stichtag")}</span><strong>{formatDate(preview.snapshotDate)}</strong></div><div><span>{t("Kontoreferenz")}</span><strong>{preview.accountReference ?? "–"}</strong></div><div><span>{t("Positionen")}</span><strong>{preview.positions.length}</strong></div></div>
    {dateEditable && <label className="position-snapshot-account-field">{t("Stichtag")}<input type="date" required disabled={busy || Boolean(result)} value={preview.snapshotDate ?? ""} onChange={event => onDateChange(event.target.value)} /></label>}
    {preview.warnings.filter(warning => !preview.snapshotDate || warning !== "Bitte den Stichtag des Positionsbestands angeben.").map((warning, index) => <p className="warning-message" role="status" key={index}>{t(warning)}</p>)}
    {preview.alreadyImported && !result && <p className="mapping-ready-notice" role="status">{t("Dieser Positionsbestand wurde bereits importiert.")}</p>}
    <label className="position-snapshot-account-field">{t("Depot")}<select disabled={busy || Boolean(result)} value={accountId ?? ""} onChange={event => onAccountChange(event.target.value ? Number(event.target.value) : null)}><option value="">{matchingAccounts.length ? t("Konto auswählen") : t("Kein passendes Konto vorhanden")}</option>{matchingAccounts.map(account => <option value={account.id} key={account.id}>{account.name} · {account.currency}</option>)}</select></label>
    {!matchingAccounts.length && <p className="warning-message" role="alert">{preview.accountReference ? t("Kein passendes aktives Anlagekonto mit derselben Kontoreferenz gefunden. Prüfe Banken & Konten.") : t("Lege zuerst ein Anlagekonto beim Anbieter des Dokuments an.")} <a href="#banks">{t("Banken & Konten verwalten")}</a></p>}
    <div className="transaction-preview position-snapshot-preview"><table><thead><tr><th>{t("Symbol")}</th><th>{t("Kurssymbol")}</th><th>{t("Vorher")}</th><th>{t("Nachher")}</th><th>{t("Änderung")}</th></tr></thead><tbody>{rows.map(row => {
      const change = "action" in row ? row : null;
      return <tr key={row.marketSymbol}><td><strong>{row.symbol}</strong></td><td>{row.marketSymbol}</td><td>{change?.previousQuantity !== null && change?.previousQuantity !== undefined ? formatQuantity(change.previousQuantity) : "–"}</td><td>{formatQuantity(row.quantity)}</td><td>{change ? <span className={change.action === "zero" ? "warning-chip" : change.action === "unchanged" ? "privacy-chip" : "success-chip"}>{actionLabel(change.action)}</span> : "–"}</td></tr>;
    })}</tbody></table></div>
    <div className="batch-position-summary"><strong>{accountId && preview.snapshotDate ? tr`${changeCount} Mengenänderungen zum ${formatDate(preview.snapshotDate)}` : t("Wähle das zugehörige Depot und den Stichtag.")}</strong><small>{preview.scope === "fullPortfolio" ? t("Vollständiger Depotbestand: Fehlende Positionen werden am Stichtag auf null gesetzt.") : t("Teilbestand: Nicht enthaltene Positionen bleiben unverändert.")}</small></div>
  </>;
}

export function positionSnapshotAccounts(accounts: ImportAccount[], preview: PositionSnapshotPreview) {
  return accounts.filter(account => account.isActive && preview.eligibleAccountIds.includes(account.id));
}

function actionLabel(action: PositionSnapshotChange["action"]) {
  return t(action === "new" ? "Neu" : action === "update" ? "Aktualisieren" : action === "zero" ? "Auf null setzen" : "Unverändert");
}

function formatDate(value: string | null) {
  return value ? new Intl.DateTimeFormat(locale(), { timeZone: "UTC" }).format(new Date(value + "T00:00:00Z")) : "–";
}

function formatQuantity(value: string) {
  const [whole, fraction] = value.split(".");
  const formatter = new Intl.NumberFormat(locale());
  const grouped = formatter.format(BigInt(whole));
  const separator = formatter.formatToParts(1.1).find(part => part.type === "decimal")?.value ?? ".";
  return fraction ? grouped + separator + fraction : grouped;
}
