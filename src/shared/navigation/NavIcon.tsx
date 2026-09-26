// Stellt die einheitlichen Piktogramme der Hauptnavigation für Navigation und erklärende UI-Vorschauen bereit.

import type { ReactNode } from "react";

export type NavIconName = "chat" | "help" | "home" | "bank" | "chart" | "tax" | "transactions" | "tag" | "import" | "history" | "settings" | "logout";

export function NavIcon({ name }: { name: NavIconName }) {
  const paths: Record<NavIconName, ReactNode> = {
    chat: <><path d="M4 4h16v13H9l-5 4V4Z"/><path d="M8 8h8M8 12h5"/></>,
    help: <><circle cx="12" cy="12" r="9"/><path d="M9.7 9a2.5 2.5 0 1 1 3.7 2.2c-.9.5-1.4 1-1.4 2.1M12 17h.01"/></>,
    home: <><path d="M3 11.5 12 4l9 7.5"/><path d="M5.5 10v10h13V10M9 20v-6h6v6"/></>,
    bank: <><path d="M3 9h18L12 4 3 9Z"/><path d="M5 9v8m4-8v8m6-8v8m4-8v8M3 20h18"/></>,
    chart: <><path d="M4 19V5"/><path d="M4 19h16"/><path d="m7 15 4-5 3 3 5-7"/></>,
    tax: <><path d="M6 3h9l4 4v14H6z"/><path d="M15 3v5h5M9 12h6M9 16h6"/></>,
    transactions: <><rect x="4" y="3" width="16" height="18" rx="2"/><path d="M8 8h.01M11 8h5M8 12h.01M11 12h5M8 16h.01M11 16h5"/></>,
    tag: <><path d="M20 13 13 20l-9-9V4h7l9 9Z"/><circle cx="8.5" cy="8.5" r="1"/></>,
    import: <><path d="M12 3v12m-4-4 4 4 4-4"/><path d="M5 16v4h14v-4"/></>,
    history: <><path d="M4 7h16v13H4zM8 4h8l2 3H6l2-3Z"/><path d="M9 12h6M9 16h4"/></>,
    settings: <><circle cx="12" cy="12" r="3"/><path d="M19.4 15a1.7 1.7 0 0 0 .34 1.88l.06.06-2.83 2.83-.06-.06a1.7 1.7 0 0 0-1.88-.34 1.7 1.7 0 0 0-1.03 1.56V21h-4v-.08A1.7 1.7 0 0 0 8.96 19.4a1.7 1.7 0 0 0-1.88.34l-.06.06-2.83-2.83.06-.06A1.7 1.7 0 0 0 4.6 15 1.7 1.7 0 0 0 3 14H3v-4h.08A1.7 1.7 0 0 0 4.6 9a1.7 1.7 0 0 0-.34-1.88l-.06-.06 2.83-2.83.06.06A1.7 1.7 0 0 0 9 4.6 1.7 1.7 0 0 0 10 3h4v.08A1.7 1.7 0 0 0 15 4.6a1.7 1.7 0 0 0 1.88-.34l.06-.06 2.83 2.83-.06.06A1.7 1.7 0 0 0 19.4 9 1.7 1.7 0 0 0 21 10v4h-.08A1.7 1.7 0 0 0 19.4 15Z"/></>,
    logout: <><path d="M10 4H5v16h5M14 8l4 4-4 4M18 12H9"/></>,
  };
  return <svg className="nav-icon" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="1.7" strokeLinecap="round" strokeLinejoin="round" aria-hidden="true" focusable="false">{paths[name]}</svg>;
}
