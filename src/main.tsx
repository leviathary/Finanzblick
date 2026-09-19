// Startet die React-Oberfläche im HTML-Einstiegspunkt.

import React from "react";
import ReactDOM from "react-dom/client";
import App from "./App";
import { initializeAppearance, showApp } from "./shared/theme/appearance";

await initializeAppearance();
ReactDOM.createRoot(document.getElementById("root") as HTMLElement).render(
  <React.StrictMode>
    <App />
  </React.StrictMode>,
);
requestAnimationFrame(() => requestAnimationFrame(showApp));
