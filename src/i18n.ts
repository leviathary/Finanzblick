import messages from "./translations.json";
import type { Language, Region } from "./settings";

const catalog: Record<string, string[]> = messages;
let language: Language = "de";
let region: Region = "CH";
export function setLanguage(value: Language) {
  language = value;
  applyDocumentLocale();
}
export function setRegion(value: Region) {
  region = value;
  applyDocumentLocale();
}
export function locale() { return `${language}-${region}`; }
function applyDocumentLocale() {
  if (typeof document !== "undefined") document.documentElement.lang = locale();
}
export function t(message: string): string {
  if (language === "de") return message.replace(/&amp;/g, "&");
  return catalog[message]?.[{ en: 0, fr: 1, it: 2 }[language]] ?? message;
}
export function tr(parts: TemplateStringsArray, ...values: unknown[]): string {
  const key = parts.reduce((key, part, index) => key + (index ? `{${index - 1}}` : "") + part, "");
  return t(key).replace(/\{(\d+)\}/g, (_, index: string) => String(values[Number(index)]));
}

// Only translate unchanged built-in category names, never user-defined labels.
const defaultCategories: Record<string, string> = {
 housing: "Wohnen", furnishing: "Möbel & Einrichtung", electronics: "Elektronik", groceries: "Lebensmittel & Haushalt",
 health: "Gesundheit", leisure: "Freizeit & Sport", restaurants: "Restaurants", telecom: "Internet & Mobilfunk", digital_subscriptions: "Digitale Abos",
 transport: "Mobilität", travel: "Reisen", taxes: "Steuern", alimony: "Alimente", saving: "Sparen & Vorsorge", income: "Einkommen", other: "Sonstiges",
};
export function categoryName(key: string, label: string) { return defaultCategories[key] === label ? t(label) : label; }
