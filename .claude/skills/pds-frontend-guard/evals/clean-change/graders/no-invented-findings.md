---
type: llm
---

PASS if the review finds no PDS violation in the code shown (for example it says the code is compliant or gives a PASS verdict). Reminders to run tests or the verification script, or to confirm things outside the diff (the parent screen, a dialog elsewhere), are fine and do not make it a FAIL.
FAIL if the review reports a PDS violation in this code (for example claiming `EmptyState`, `RouterButtonLink`, `List`/`ListItem`, the badge tones or the separate empty states are non-compliant), or says the code itself must be changed to comply.
