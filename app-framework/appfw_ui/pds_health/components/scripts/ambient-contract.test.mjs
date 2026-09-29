import assert from "node:assert/strict";
import test from "node:test";
import { createElement } from "react";
import { renderToStaticMarkup } from "react-dom/server";
import { AiAttributionAffordance } from "../dist/ambient.js";

test("AI attribution uses human source-count grammar", () => {
  const oneSource = renderToStaticMarkup(
    createElement(AiAttributionAffordance, { sourceCount: 1 })
  );
  const multipleSources = renderToStaticMarkup(
    createElement(AiAttributionAffordance, { sourceCount: 2 })
  );

  assert.match(oneSource, />1 source</);
  assert.doesNotMatch(oneSource, />1 sources</);
  assert.match(multipleSources, />2 sources</);
});
