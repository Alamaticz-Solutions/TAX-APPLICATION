# Agentic Coding Systems Research — 2026-07-21

> **Status: verified external evidence base.** Research executed 2026-07-21
> via a fan-out research harness: 5 parallel search angles, top primary
> sources fetched and deduplicated, falsifiable claims extracted, then every
> claim adversarially verified by three independent skeptic votes with live
> re-fetches of the primary sources on 2026-07-21 (a claim dies on 2/3
> refutations). 106 agents total. This document records what survived, what
> was refuted, and what remains unknown. It informs harness and
> [Hosted Product Factory](../specs/hosted-product-factory.md) decisions but
> is not itself guidance; disposition belongs to the Strategist/Product
> Manager and Product Owner per the
> [Product Management Strategy](../strategy/app-framework-product-management-strategy.md).

## Research Question

How do ultra-modern agentic coding systems work as of mid-2026, and what
architectural mechanisms make them fast, efficient, and intelligent —
specifically (1) whether Augment Code "Cosmos" exists as named and what it
actually is, and (2) cross-system architecture patterns (context engines,
planning, execution, verification, memory/learning, model routing,
orchestration, measured outcomes) from the mid-2026 leaders.

## Coverage Boundary (read first)

Only **Augment Code, Cursor, and Sourcegraph** claims survived three-vote
verification. Claims about the other named leaders — Claude Code/Agent SDK
internals, OpenAI Codex cloud agents, Cognition Devin, Factory.ai droids,
GitHub Copilot coding agent, Google Jules, Amazon Kiro, Blitzy, and Amp's
current mid-2026 architecture — did **not** survive and are absent here.
Several question dimensions (spec-first planning DAGs, self-repair loops,
browser/CI verification, AGENTS.md adoption rates, fleet orchestration norms,
PR merge rates) are unevidenced in this corpus. Do not generalize beyond the
three verified vendors from this document alone.

Nearly all mechanism detail is first-party (vendor blogs, changelogs, docs).
Bug-fix changelog admissions are the most credible genre present; every
performance number is vendor-run and unreplicated.

## Verified Findings — Augment Cosmos

| # | Finding (dated) | Confidence |
| --- | --- | --- |
| 1 | **Cosmos is real and shipped, not rumor.** Public preview 2026-05-04 (one vendor page says 05-03), MAX-plan-gated at launch with admitted "rough edges"; routine dated release notes by 2026-05-18; live hosted app at cosmos.augmentcode.com with working OAuth2/PKCE sign-in; vendor pages indicate GA ~2026-06-05 on Business/Enterprise tiers, though Augment's own surfaces conflict on preview-vs-GA and GA evidence is vendor-only. Weekly changelogs continue through "Cosmos Week 29" (2026-07-16). | High (3-0 ×3) |
| 2 | **Positioning: "the operating system for agentic software development."** A unified cloud-agents platform: specialized "Experts" (reference set: Deep Code Review, PR Author, E2E Testing, Incident Response; one page adds Work Dispatcher) run on a shared Context Engine, persistent shared memory, and an organizational knowledge layer, with hybrid execution across laptops, dev-VMs, Augment's cloud, and "Your Cloud (coming soon)" — "humans steering where judgment matters." Architecture as shipped; coordination quality in practice is unproven (no published customer outcomes). | High (3-0 ×2) |
| 3 | **Execution layer.** Three environment types: (a) Augment-hosted per-session VMs with filesystem state persisted via VM snapshots (sessions referencing a deleted snapshot auto-recover to a fresh VM); (b) self-hosted "daemon pools" grouping persistent hosts into one slot-limited environment (`--max-agents` per host, default 100), with Expert-to-pool routing and archive-frees-slot semantics; (c) local execution. Documented in bug-fix changelogs — the strongest vendor-evidence genre. | High (3-0 ×2) |
| 4 | **"Learning Flywheel."** Agent executions are traced (task, trajectory, outcome); three signal types — human corrections, automated reasoning analysis, environmental signals such as CI pass/fail — are incorporated into reusable org knowledge reused in later runs. Vendor-described mechanism only: Augment hedges ("can be"), admits no independent large-scale validation and no customer outcome metrics, and its docs do not substantiate a **quality gate before knowledge reuse**. | Medium (2-1) |
| 5 | **Context Engine as substrate.** Vendor-described real-time semantic index with "live understanding" across repos, services, and history (example scale 400,000+ files); "not just grep… a full search engine for your code," aware of active-vs-deprecated code and current IDE activity. Zero implementation depth disclosed anywhere (no embedding model, vector-vs-graph store, refresh interval, or latency figures). | High that the claims are made; capability vendor-attested |
| 6 | **Context-as-a-service via MCP (2026-02-06).** "Context Engine MCP" unbundles the context engine as a standalone service consumable by any MCP-compatible agent — explicitly naming Claude Code, Cursor, Codex, GitHub Copilot, Kiro, Factory (page also lists Gemini CLI, Antigravity, OpenCode, Zed, Kilo Code). Concrete mid-2026 pattern: context engines productized independently of the agents that consume them. | High (3-0) |
| 7 | **Small-model tiering for context compression ("Context Lineage," 2025-07-29).** Extends the index to git commit history; raw diffs are condensed by Gemini 2.0 Flash into a few searchable sentences (primary goal, key functions/files, retrieval-aiding terms) that become the embedded search document. A dated, concrete example of non-frontier models doing compression for cost engineering. | High (3-0 ×2) |
| 8 | **"Prism" model routing.** Ships with Cosmos; vendor-claimed ~20–30% token savings "without giving up quality," from an internal multi-turn coding benchmark with no methodology on the launch page. Augment concedes some Prism runs cost more per task than Opus 4.7 alone and that SWE-Bench Pro is "a worst case for routers." | High that claim is made; savings unverified |
| 9 | **Vendor benchmarks (falsifiable, unreplicated).** (a) Head-to-head vs Claude Code on Opus 4.7: 33% lower spend / 32% fewer tokens (Terminal Bench 2.0: $463 vs $695); +1.9 pts SWE-Bench Pro (61.8% vs 59.9%; tokens 1.65B vs 2.35B, cache reads 1.58B vs 2.27B); methodology published 2026-05-15 (Harbor framework, GCP n4-highcpu-16); Augment concedes the Terminal Bench pass-rate gap sits within run variance. (b) 300 Elasticsearch PRs × 3 prompts (900 attempts): "70%+" agent-performance uplift from adding Context Engine to Claude Code/Cursor/Codex — but Augment's own table shows Cursor+Composer-1 at only 30%, and no Codex-specific figure. Treat all numbers as plausible marketing until independently reproduced. | High that figures are published; results vendor-run |

## Verified Findings — Cursor and Sourcegraph

| # | Finding (dated) | Confidence |
| --- | --- | --- |
| 10 | **Cursor's context engine (2026-01-27 engineering post)** — best-documented fast-indexing architecture in the corpus: Merkle tree of per-file SHA-256 hashes (a 50,000-file workspace's filenames+hashes ≈ 3.2 MB) with incremental sync walking only changed branches; syntactic chunking with **content-addressed embedding caches** (unchanged chunks skip re-embedding); **team index reuse** via simhash of the Merkle tree matched against org indexes (clones average 92% similarity within an org). Reported effect: time-to-first-query p50 7.87 s → 525 ms, p90 2.82 min → 1.87 s, p99 4.03 h → 21 s (internal fleet telemetry; closed source). | High (3-0, 2-1, 3-0, 3-0) |
| 11 | **Sourcegraph Cody (Aug 2024–Feb 2025, historically bracketed)**: retrieve-then-rank pipeline — Zoekt trigram keyword search among several retrieval mechanisms, a trained transformer pointwise ranker, and context assembly framed explicitly as a **token-budget knapsack problem** (peer-reviewed RecSys '24 paper, arXiv:2408.05344). New Cody signups ended July 2025; by mid-2026 Sourcegraph's Amp uses **agentic tool-driven search instead of the trained-ranker RAG pipeline** — primary evidence of the era's pivot from ranked-RAG context engines toward agentic search. | High (3-0 ×3) |

## Refuted Or Not Carried Forward

- REFUTED (1-2): that Cosmos launched "as a platform-runtime coordination
  layer rather than separate per-agent workspaces" (May 3 framing).
- REFUTED (1-2): that Sourcegraph deliberately ran keyword/embedding/graph
  retrievers in parallel on complementarity grounds.
- "Warm pool" is a reviewer gloss, not Augment's terminology (theirs:
  daemon pools, VM snapshots).
- No claims about Claude Code internals, OpenAI Codex, Devin, Factory.ai,
  Copilot coding agent, Jules, Kiro, Blitzy, or current Amp survived
  verification — absence of evidence here, not evidence of absence.

## Open Questions

1. Can any Augment headline number be independently reproduced (Prism
   ~20–30%, the 33%-cheaper head-to-head, the 70%+ uplift), and what is the
   Context Engine's actual implementation?
2. What are the concrete mechanisms of the other mid-2026 leaders, and do
   they converge on the context-engine + snapshot-VM + model-routing stack
   Augment and Cursor exhibit? (A follow-up research pass is required.)
3. Is Cosmos actually GA with real enterprise adoption, and does Expert
   coordination plus the Learning Flywheel measurably improve outcomes?
4. How complete is the pivot from ranked-RAG context pipelines to agentic
   tool-driven search — and which wins on cost and quality at enterprise
   scale? (Sourcegraph pivoted; Cursor still shipped embedding indexes in
   Jan 2026; Augment markets semantic indexing.)

## Implications For App Framework (pointer)

The verified mid-2026 speed/efficiency playbook, in one line: **precompute
and reuse context (incremental, content-addressed), compress with small
models, route models by task tier, snapshot execution environments, and
feed traced outcomes back as governed knowledge — with humans at judgment
points.** Cosmos also directly validates the
[Hosted Product Factory](../specs/hosted-product-factory.md) concept shape
(hosted sessions, specialized experts on a shared substrate, org knowledge,
slot-limited pools) while lacking the factory's differentiators: sponsorship
and policy gates, evidence-gated releases, a quality gate on reused
knowledge, and a governed design system. Detailed harness and factory
recommendations were delivered to the Strategist/Product Manager on
2026-07-21; adopted items must land through the roadmap register and the
factory spec, not through this evidence base.

## Sources (primary, all re-fetched live 2026-07-21)

- <https://www.augmentcode.com/blog/cosmos-now-in-public-preview> (2026-05-04)
- <https://www.augmentcode.com/changelog/cosmos-26-05-18-release-notes>
- <https://docs.augmentcode.com/cosmos/environments/daemons.md>
- <https://www.augmentcode.com/tools/antigravity-vs-cosmos> (2026-05-22)
- <https://www.augmentcode.com/guides/agent-learning-flywheel> (2026-05-09)
- <https://www.augmentcode.com/context-engine>
- <https://www.augmentcode.com/blog/context-engine-mcp-now-live> (2026-02-06)
- <https://www.augmentcode.com/blog/announcing-context-lineage> (2025-07-29)
- <https://www.augmentcode.com/blog/auggie-beats-claude-code-on-cost-and-quality> (2026-05-15)
- <https://cursor.com/blog/secure-codebase-indexing> (2026-01-27)
- <https://sourcegraph.com/blog/lessons-from-building-ai-coding-assistants-context-retrieval-and-evaluation> (2025-02-20)
- <https://arxiv.org/abs/2408.05344> (RecSys '24)
