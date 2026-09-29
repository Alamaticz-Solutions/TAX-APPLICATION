import React from "react";
import { createRoot } from "react-dom/client";
import { App } from "./App";
import "@appfw/pds-health/tokens/pdsTokens.css";
import "@appfw/pds-health-components/styles.css";
import "./styles.css";

const container = document.getElementById("root");
if (!container) {
  throw new Error("Root element #root is missing from index.html");
}

createRoot(container).render(
  <React.StrictMode>
    <App />
  </React.StrictMode>,
);
