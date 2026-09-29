// Liest Kategorie-Deep-Links und fokussiert nach dem Laden den Buchungs-Drilldown.

import { useEffect, useRef } from "react";

export function readTransactionRoute() {
  const params = new URLSearchParams(window.location.hash.split("?")[1] ?? "");
  return {
    category: params.get("category"),
    all: params.get("period") === "all",
    details: params.get("view") === "details",
    mode: params.get("mode") === "income" ? "income" as const : "expense" as const,
  };
}

export function useTransactionDeepLink(enabled: boolean, loading: boolean, ready: boolean) {
  const handled = useRef(false);
  useEffect(() => {
    if (!enabled || handled.current || loading || !ready) return;
    handled.current = true;
    requestAnimationFrame(() => {
      const details = document.getElementById("transaction-details");
      details?.scrollIntoView({ block: "start" });
      details?.focus({ preventScroll: true });
    });
  }, [enabled, loading, ready]);
}
