import { StrictMode } from "react";
import { createRoot } from "react-dom/client";
import "@appfw/pds-health-components/styles.css";
import { RepresentativeInteractionsFixture } from "./RepresentativeInteractionsFixture";

const params = new URLSearchParams(window.location.search);
const requestedTheme = params.get("theme");
const requestedGrammar = params.get("grammar");
const theme = requestedTheme === "dark" ? "dark" : "light";
const grammar = requestedGrammar === "material-like" ? "material-like" : "apple-like";

document.documentElement.style.colorScheme = theme;
document.documentElement.dataset.theme = theme;
document.documentElement.dataset.visualTheme = grammar;

const root = document.getElementById("root");
if (!root) throw new Error("F1 fixture root was not found.");

document.body.style.margin = "0";
document.body.style.background = "var(--pds-color-surface-page)";
document.body.style.color = "var(--pds-color-text-default)";
document.body.style.fontFamily = "var(--pds-font-sans, system-ui, sans-serif)";

createRoot(root).render(
  <StrictMode>
    <RepresentativeInteractionsFixture />
  </StrictMode>
);
