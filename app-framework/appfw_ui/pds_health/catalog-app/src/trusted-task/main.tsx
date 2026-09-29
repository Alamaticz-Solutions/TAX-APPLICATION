import { StrictMode } from "react";
import { createRoot } from "react-dom/client";
import "@appfw/pds-health-components/styles.css";
import "./trusted-task.css";
import { TrustedTaskExperience } from "./TrustedTaskExperience";
import {
  isKnownResumeToken,
  isKnownWorkToken,
  isTrustedTaskState
} from "./contract";
import { trustedTaskWork } from "./fixtures";

const params = new URLSearchParams(window.location.search);
const requestedState = params.get("state");
const requestedTheme = params.get("theme");
const requestedGrammar = params.get("grammar");
const requestedWorkToken = params.get("work");
const requestedResumeToken = params.get("resume");
const work = trustedTaskWork();

const state = isTrustedTaskState(requestedState) ? requestedState : "permitted-preview";
const theme = requestedTheme === "dark" ? "dark" : "light";
const grammar = requestedGrammar === "material-like" ? "material-like" : "apple-like";
const workToken = isKnownWorkToken(requestedWorkToken, work) ? requestedWorkToken : null;
const resumeToken = isKnownResumeToken(requestedResumeToken, work) ? requestedResumeToken : null;
const invalidInputs = [
  requestedState && !isTrustedTaskState(requestedState) ? "state" : null,
  requestedTheme && requestedTheme !== "light" && requestedTheme !== "dark" ? "theme" : null,
  requestedGrammar && requestedGrammar !== "apple-like" && requestedGrammar !== "material-like" ? "grammar" : null,
  requestedWorkToken && !workToken ? "work" : null,
  requestedResumeToken && !resumeToken ? "resume" : null
].filter((value): value is string => Boolean(value));

document.documentElement.style.colorScheme = theme;
document.documentElement.dataset.theme = theme;
document.documentElement.dataset.visualTheme = grammar;

const root = document.getElementById("trusted-task-root");
if (!root) throw new Error("Trusted Task root was not found.");

createRoot(root).render(
  <StrictMode>
    <TrustedTaskExperience
      initialState={state}
      deepLinkWorkToken={workToken}
      resumeToken={resumeToken}
      invalidInputs={invalidInputs}
    />
  </StrictMode>
);
