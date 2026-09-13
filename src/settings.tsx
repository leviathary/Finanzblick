import { createContext, useContext } from "react";

export type Language = "de" | "en" | "fr" | "it";
export type Region = "CH" | "DE" | "AT" | "FR" | "IT" | "GB" | "US";
export type AppSettings = {
  autoLockMinutes: number;
  language: Language;
  region: Region;
  defaultCurrency: string;
  marketstackApiKey: string;
  alphaVantageApiKey: string;
};
export const defaultSettings: AppSettings = {
  autoLockMinutes: 15,
  language: "de",
  region: "CH",
  defaultCurrency: "CHF",
  marketstackApiKey: "",
  alphaVantageApiKey: "",
};
export const SettingsContext = createContext<{
  settings: AppSettings;
  saveSettings: (value: AppSettings) => Promise<void>;
}>({
  settings: defaultSettings,
  saveSettings: async () => {
    throw new Error("Finanzblick ist gesperrt.");
  },
});
export const useSettings = () => useContext(SettingsContext);
