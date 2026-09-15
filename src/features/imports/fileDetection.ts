import { t } from "../../i18n";
export const supportedExtensions = ["xlsx", "xls", "csv", "pdf", "mt940", "sta"] as const;

export type SupportedExtension = (typeof supportedExtensions)[number];
// Importers may introduce additional provider keys without requiring a new
// frontend union member. Built-ins below provide labels and filename hints.
export type ProviderId = string;
export const CUSTOM_EXCEL_PROVIDER = "custom-excel";

export interface SelectedStatement {
  name: string;
  path?: string;
  size?: number;
  extension: SupportedExtension;
  provider: ProviderId;
}

export const providers: Array<{ id: ProviderId; label: string; type: string }> = [
  { id: "ubs", label: "UBS", type: "Bank" },
  { id: "swissquote", label: "Swissquote", type: "Bank / Trading" },
  { id: "migros", label: "Migros Bank", type: "Bank" },
  { id: "raiffeisen", label: "Raiffeisen", type: "Bank" },
  { id: "generali", label: "Generali", type: "Versicherung / Vorsorge" },
  { id: CUSTOM_EXCEL_PROVIDER, label: "Eigene Excel-Datei", type: "Benutzerdefinierte Datei" },
  { id: "unknown", label: "Anderer Anbieter", type: "Manuelle Zuordnung" },
];

export function detectProvider(fileName: string): ProviderId {
  const normalized = fileName.toLocaleLowerCase("de-CH");
  if (normalized.includes("ubs")) return "ubs";
  if (normalized.includes("swissquote")) return "swissquote";
  if (normalized.includes("migros")) return "migros";
  if (normalized.includes("raiffeisen")) return "raiffeisen";
  if (normalized.includes("generali") || normalized.includes("vorsorge")) return "generali";
  return "unknown";
}

export function getSupportedExtension(fileName: string): SupportedExtension | null {
  const extension = fileName.split(".").pop()?.toLocaleLowerCase();
  return supportedExtensions.find((candidate) => candidate === extension) ?? null;
}

export function fileNameFromPath(filePath: string): string {
  return filePath.split(/[\\/]/).pop() ?? filePath;
}

export function formatFileSize(size?: number): string {
  if (size === undefined) return t("Größe wird beim Einlesen ermittelt");
  if (size < 1024) return `${size} B`;
  if (size < 1024 * 1024) return `${(size / 1024).toFixed(1)} KB`;
  return `${(size / (1024 * 1024)).toFixed(1)} MB`;
}
