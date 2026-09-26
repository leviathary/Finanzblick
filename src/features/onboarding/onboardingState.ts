// Verwaltet den gerätebezogenen Versionsstand der Einführung und ihr app-weites Öffnungsereignis.

const storageKey = "saldonaut.onboarding-version";
const currentVersion = 5;
export const onboardingOpenEvent = "saldonaut-open-onboarding";

export function shouldShowOnboarding() {
  try {
    const storedVersion = Number.parseInt(localStorage.getItem(storageKey) ?? "0", 10);
    return !Number.isInteger(storedVersion) || storedVersion < currentVersion;
  } catch {
    return true;
  }
}

export function markOnboardingComplete() {
  try {
    localStorage.setItem(storageKey, String(currentVersion));
  } catch {
    // Eine gesperrte WebView-Speicherung darf das Schliessen der Einführung nicht verhindern.
  }
}

export function openOnboarding() {
  window.dispatchEvent(new Event(onboardingOpenEvent));
}
