// Verwaltet die lokale Darstellung vor der Anmeldung und folgt bei Bedarf dem Systemfarbschema.
import { invoke, isTauri } from "@tauri-apps/api/core";
export type Appearance = "light" | "dark" | "cosmic" | "system";
const key = "finanzblick.appearance";
const media = window.matchMedia("(prefers-color-scheme: dark)");
let preference: Appearance = "system";
let started = false;
const valid = (value: unknown): Appearance => value === "light" || value === "dark" || value === "cosmic" ? value : "system";
export const getAppearance = () => preference;
export function subscribeAppearance(listener: () => void) {
  window.addEventListener("appearance-changed", listener);
  return () => window.removeEventListener("appearance-changed", listener);
}
export const isDark = () => preference === "dark" || preference === "cosmic" || (preference === "system" && media.matches);
function apply() {
  document.documentElement.dataset.theme = isDark() ? "dark" : "light";
  document.documentElement.dataset.appearance = preference;
  document.documentElement.style.colorScheme = isDark() ? "dark" : "light";
  window.dispatchEvent(new Event("appearance-changed"));
  if (started && isTauri()) void invoke("show_themed_window", { dark: isDark(), reveal: false }).catch(console.error);
}
export async function initializeAppearance() {
  try { preference = valid(isTauri() ? await invoke("load_appearance") : localStorage.getItem(key)); }
  catch { preference = "system"; }
  apply();
  media.addEventListener("change", () => { if (preference === "system") apply(); });
  window.addEventListener("storage", event => {
    if (!isTauri() && (event.key === key || event.key === null)) { preference = valid(event.newValue); apply(); }
  });
}
export function showApp() {
  started = true;
  if (isTauri()) void invoke("show_themed_window", { dark: isDark(), reveal: true }).catch(console.error);
}
export async function saveAppearance(value: Appearance) {
  if (isTauri()) await invoke("save_appearance", { appearance: value });
  else localStorage.setItem(key, value);
  preference = value;
  apply();
}
