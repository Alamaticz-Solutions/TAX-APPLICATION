---
type: llm
---

PASS if the review flags the hand-built "Nothing here." paragraph and says to use the PDS `EmptyState` (with a title, an explanation and an action such as clearing the search), AND flags the raw colour `#888` (colours must come from `--pds-*` tokens).
FAIL if it misses either the hand-built empty state or the raw colour, or approves the change as compliant.
