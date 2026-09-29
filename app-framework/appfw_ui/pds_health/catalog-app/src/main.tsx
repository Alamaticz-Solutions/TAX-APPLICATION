import { StrictMode } from "react";
import { createRoot } from "react-dom/client";
import "@appfw/pds-health/tokens/pdsTokens.css";
import "@appfw/pds-health-components/styles.css";
import "./app.css";
import App from "./App";
import {
  applyTheme,
  applyVisualTheme,
  readStoredTheme,
  readStoredVisualTheme
} from "./lib/theme";

applyTheme(readStoredTheme());
applyVisualTheme(readStoredVisualTheme());

const container = document.getElementById("root");
if (!container) {
  throw new Error("PDS design-system root element #root was not found.");
}

createRoot(container).render(
  <StrictMode>
    <App />
  </StrictMode>
);
