# Branch Disposition Ledger

This is the durable, terminal-only record for App Framework branch value
disposition and carrier retirement. Follow
[Branch Disposition And Scavenging](../specs/branch-disposition-and-scavenging.md).

Do not add planning, active CI state, tentative successor mapping, or routine
poll history here. Those remain in current convergence reports. Add a branch
only after it satisfies the `MERGED`, `SCAVENGED`, or `DEPRECATED` terminal
gate. Later update its carrier state to `RETIRED` only with exact cleanup and
recovery evidence.

## Terminal Records

No terminal scavenging records have been added under this contract yet.

<!--
Append records using this shape:

## `<source-branch>@<full-source-tip>`

- Disposition: `MERGED | SCAVENGED | DEPRECATED`
- Carrier state: `LIVE | RETIRED`
- Recorded at: `<UTC timestamp>`
- Successor PRs and merge SHAs: `<references or not_applicable>`
- Kept payload: `<coverage summary and retained evidence link>`
- Non-kept payload: `<equivalent, superseded, product-exported, or discarded
  outcomes with evidence>`
- Coverage evidence: `<path and SHA-256>`
- Recovery evidence: `<path and SHA-256>`
- Chief Architect observation: `OBSERVED | CHALLENGE` — `<short rationale>`
- Integration verification: `VERIFIED` — `<short rationale>`
- Retired at: `<UTC timestamp or not_yet>`
- Retirement evidence: `<path and SHA-256 or not_yet>`
- Amendments: `<later correction references or none>`
-->
