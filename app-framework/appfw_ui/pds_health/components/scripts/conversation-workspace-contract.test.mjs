import assert from "node:assert/strict";
import fs from "node:fs";
import test from "node:test";
import { createElement } from "react";
import { renderToStaticMarkup } from "react-dom/server";
import { ConversationWorkspace } from "../dist/conversation-workspace.js";

const styles = fs.readFileSync(new URL("../dist/styles.css", import.meta.url), "utf8");

const messages = [
  {
    id: "message-1",
    role: "assistant",
    author: "Assistant",
    content: "How can I help?"
  }
];

test("conversation workspace composes labelled regions and a disabled busy state", () => {
  const html = renderToStaticMarkup(
    createElement(ConversationWorkspace, {
      title: "Guidance",
      description: "Review the available context before acting.",
      messages,
      isBusy: true,
      busyContent: "Reviewing context…",
      contextRail: createElement("p", null, "Current record"),
      suggestedPrompts: [
        { id: "explain", value: "Explain this record" },
        { id: "next", value: "What happens next?", disabled: true }
      ],
      actionRegion: createElement("button", { type: "button" }, "Review"),
      composerProps: {
        value: "Draft question",
        onValueChange() {},
        onSubmitMessage() {}
      }
    })
  );

  assert.match(
    html,
    /<section[^>]*class="pds-conversation-workspace"[^>]*data-busy="true"[^>]*data-has-header="true"/
  );
  assert.match(html, /aria-labelledby="[^"]+-title"/);
  assert.match(html, /aria-describedby="[^"]+-description"/);
  assert.match(
    html,
    /class="pds-conversation-workspace__thread-region" role="log" aria-label="Conversation messages" aria-live="polite"/
  );
  assert.match(html, /aria-relevant="additions text"/);
  assert.match(
    html,
    /class="pds-conversation-workspace__context-rail" aria-label="Conversation context"/
  );
  assert.match(
    html,
    /class="pds-conversation-workspace__suggestions" role="group" aria-label="Suggested prompts"/
  );
  assert.match(
    html,
    /class="pds-conversation-workspace__action-region" aria-label="Conversation actions"/
  );
  assert.match(html, /Reviewing context…/);
  assert.match(html, /data-streaming="true"/);
  assert.equal((html.match(/disabled=""/g) ?? []).length, 4);
});

test("conversation workspace omits optional regions and uses its fallback label", () => {
  const html = renderToStaticMarkup(
    createElement(ConversationWorkspace, {
      messages
    })
  );

  assert.match(html, /aria-label="Conversation workspace"/);
  assert.match(html, /aria-busy="false"/);
  assert.doesNotMatch(html, /data-has-header/);
  assert.doesNotMatch(html, /pds-conversation-workspace__header/);
  assert.doesNotMatch(html, /pds-conversation-workspace__context-rail/);
  assert.doesNotMatch(html, /pds-conversation-workspace__suggestions/);
  assert.doesNotMatch(html, /pds-conversation-workspace__action-region/);
  assert.doesNotMatch(html, /pds-conversation-workspace__composer/);
});

test("streaming text communicates live work without forcing motion", () => {
  assert.match(
    styles,
    /\.pds-streaming-text__cursor\s*\{[^}]*animation:\s*pds-streaming-cursor-pulse/s
  );
  assert.match(styles, /@keyframes pds-streaming-cursor-pulse/);
  assert.match(
    styles,
    /@media \(prefers-reduced-motion: reduce\)[\s\S]*?\.pds-streaming-text__cursor\s*\{[^}]*animation:\s*none/s
  );
});

test("conversation workspace keeps its header content-sized above the flexible body", () => {
  assert.match(
    styles,
    /\.pds-conversation-workspace\[data-has-header="true"\]\s*\{[^}]*grid-template-rows:\s*auto minmax\(0, 1fr\)/s
  );
  assert.match(
    styles,
    /\.pds-conversation-workspace:not\(\[data-has-header="true"\]\)\s*\{[^}]*grid-template-rows:\s*minmax\(0, 1fr\)/s
  );
  assert.match(
    styles,
    /\.pds-message-thread__list\s*\{[^}]*align-content:\s*start/s
  );
});
