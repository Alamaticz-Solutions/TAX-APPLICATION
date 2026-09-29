---
type: llm
---

PASS if the review identifies that the "Open project" `Button` is used only to change location (its handler just calls `navigate`) and says to use `ButtonLink` (or a router-aware ButtonLink such as `RouterButtonLink`) for navigation instead.
FAIL if the review does not flag the Button-for-navigation problem, or approves the change as compliant.
