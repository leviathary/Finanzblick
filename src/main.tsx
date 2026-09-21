// Startet die React-Oberfläche im HTML-Einstiegspunkt.

import React from "react";
import ReactDOM from "react-dom/client";
import { flushSync } from "react-dom";
import App from "./App";
import { initializeAppearance, showApp } from "./shared/theme/appearance";

await initializeAppearance();
flushSync(() => ReactDOM.createRoot(document.getElementById("root") as HTMLElement).render(
  <React.StrictMode>
    <App />
  </React.StrictMode>,
));
// Verborgene WebViews liefern nicht auf jeder Plattform Animationsframes.
// Palette und erster React-Commit sind fertig, bevor das Fenster sichtbar wird.
showApp();
