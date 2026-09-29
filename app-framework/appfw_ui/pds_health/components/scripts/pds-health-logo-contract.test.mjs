import assert from "node:assert/strict";
import test from "node:test";
import { createElement } from "react";
import { renderToStaticMarkup } from "react-dom/server";
import { PdsHealthLogo } from "../dist/foundation.js";

test("PDS Health logo exposes one meaningful accessible identity", () => {
  const html = renderToStaticMarkup(createElement(PdsHealthLogo));

  assert.match(html, /class="pds-health-logo"/);
  assert.match(html, /data-variant="wordmark"/);
  assert.match(html, /role="img"/);
  assert.match(html, /aria-label="PDS Health"/);
  assert.match(html, /viewBox="0 0 266\.83 49\.56"/);
});

test("compact decorative logo preserves artwork without duplicating a name", () => {
  const html = renderToStaticMarkup(
    createElement(PdsHealthLogo, { variant: "mark", decorative: true })
  );

  assert.match(html, /data-variant="mark"/);
  assert.match(html, /aria-hidden="true"/);
  assert.match(html, /viewBox="0 0 49\.56 49\.56"/);
  assert.doesNotMatch(html, /role="img"/);
  assert.doesNotMatch(html, /aria-label=/);
});
