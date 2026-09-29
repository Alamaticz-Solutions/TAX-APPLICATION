import { StrictMode } from "react";
import { createRoot } from "react-dom/client";
import "@appfw/pds-health-components/styles.css";
import "./neutral-work.css";
import { NeutralMyWorkExperience } from "./NeutralMyWorkExperience";
import {
  isKnownNeutralWorkToken,
  isNeutralWorkOperationalState
} from "./contract";
import { allKnownNeutralWorkItems } from "./fixtures";

const params = new URLSearchParams(window.location.search);
const requestedState = params.get("state");
const requestedTheme = params.get("theme");
const requestedGrammar = params.get("grammar");
const requestedDirection = params.get("direction");
const requestedToken = params.get("item");

const state = isNeutralWorkOperationalState(requestedState) ? requestedState : "populated";
const theme = requestedTheme === "dark" ? "dark" : "light";
const grammar = requestedGrammar === "material-like" ? "material-like" : "apple-like";
const direction = requestedDirection === "rtl" ? "rtl" : "ltr";
const deepLinkToken = isKnownNeutralWorkToken(requestedToken, allKnownNeutralWorkItems())
  ? requestedToken
  : null;

const invalidInputs = [
  requestedState && !isNeutralWorkOperationalState(requestedState) ? "state" : null,
  requestedTheme && requestedTheme !== "light" && requestedTheme !== "dark" ? "theme" : null,
  requestedGrammar && requestedGrammar !== "apple-like" && requestedGrammar !== "material-like" ? "grammar" : null,
  requestedDirection && requestedDirection !== "ltr" && requestedDirection !== "rtl" ? "direction" : null,
  requestedToken && !deepLinkToken ? "item" : null
].filter((value): value is string => Boolean(value));

document.documentElement.style.colorScheme = theme;
document.documentElement.dataset.theme = theme;
document.documentElement.dataset.visualTheme = grammar;
document.documentElement.dir = direction;

const root = document.getElementById("neutral-work-root");
if (!root) throw new Error("Neutral My Work root was not found.");

createRoot(root).render(
  <StrictMode>
    <NeutralMyWorkExperience
      initialState={state}
      deepLinkToken={deepLinkToken}
      invalidInputs={invalidInputs}
    />
  </StrictMode>
);
