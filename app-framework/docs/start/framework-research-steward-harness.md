# Framework Research Steward Harness

Use this harness when App Framework needs current external research to challenge
and sharpen Strategist/Product Manager and Product Owner guidance. The role exists to
keep strategy alive, evidence-backed, and resistant to stale assumptions or
fashionable but low-value trends.

The obvious name for this role is **App Framework Research Steward**. The
repo-native skill is `framework-research-steward`.

## Invocation

Canonical slash command:

```text
/framework-research-refresh
/framework-research-refresh --comprehensive
/framework-research-refresh --focus agentic-product-development
```

If a tool does not support slash commands, invoke the
`framework-research-steward` skill and follow this harness.

## Role Boundary

The Research Steward challenges the Strategist/Product Manager function and the
Product Owner; it does not replace either role.

Owns:

- external market, industry, product-management, platform, UX, AI, security,
  and delivery-practice research;
- anti-fad analysis;
- evidence-backed challenges to North Star, roadmap, backlog, role, review,
  SRA/CAB, packaging, and harness guidance; and
- Strategist/Product Manager and Product Owner action recommendations.

Does not own:

- final product priority;
- architecture approval;
- accepted risk;
- direct code or doc edits;
- branch pushes;
- rewriting North Star, roadmap, or strategy as truth without Product
  Owner/Strategist/human approval.

## Cadence

Use this cadence unless the human chooses otherwise:

- monthly strategic refresh;
- ad hoc focused refresh before major roadmap or wave planning;
- focused refresh when major AI, security, platform, SaaS, UX, CI/CD,
  regulatory, or change-management signals appear.

Do not spawn a standing thread only because the cadence exists. Use a fresh
slash-command/skill run when research is due, and create a durable adjacent
thread only for a large research effort that needs follow-up.

## Research Inputs

Read current intent first:

- [Product Development North Star](../strategy/product-development-north-star.md)
- [Platform Strategy](../strategy/app-framework-platform-strategy.md)
- [Product Management Strategy](../strategy/app-framework-product-management-strategy.md)
- [Roadmap](../release/roadmap.md)
- [Agentic And Human Operating Model](agentic-human-operating-model.md)
- [Agent Role Cards](agent-role-cards.md)

Then research current external signals. Prefer:

- standards and governance bodies;
- primary vendor product/platform docs;
- empirical research and field studies;
- credible practitioner reports;
- security advisories and control frameworks;
- product/UX evidence from comparable enterprise platforms.

## Output Contract

Use this order:

1. **Research Verdict.** What changed, what matters, and what is noise.
2. **Business Value Implications.** How the signal could improve or threaten PDS
   speed, safety, scale, ROI, team-member experience, security, support load, or
   operating cost.
3. **Challenge To Current Guidance.** Where existing docs may be stale,
   over-scoped, under-ambitious, vague, missing evidence, or chasing the wrong
   thing.
4. **Recommended Product Actions.** Mark each as `accept`, `reject`,
   `investigate`, `add_to_backlog`, `add_to_tech_debt`, `update_docs`, or
   `schedule_proof`, and name whether the action belongs to Strategist/Product
   Manager, Product Owner, XO, Architect, Integration, or another steward.
5. **Evidence Quality.** Source type, recency, confidence, uncertainty,
   citations, and whether the conclusion is direct evidence or inference.
6. **Anti-Fad Filter.** Why the recommendation is durable signal; what would
   prove it is not worth continuing.

## Retained Evidence

When research informs roadmap, strategy, Product Owner, or Strategist/Product
Manager guidance, retain a short artifact under:

```text
target/appfw/research-refresh.md
target/appfw/research-refresh.json
```

These are build artifacts, not source-of-truth docs. Accepted changes should be
applied later to strategy, roadmap, backlog, or role docs by the appropriate
owner.

## Review And Acceptance

Research recommendations become official guidance only when the appropriate
owner accepts them and updates the appropriate source:

- North Star for durable direction;
- Platform Strategy for business "why";
- Product Management Strategy for value themes and trend-vetting;
- Roadmap for execution order and maturity;
- Agent Role Cards or operating model for harness behavior;
- backlog or tech debt register for tracked work.

Do not let research create churn. If a recommendation cannot name a business
value mechanism, target audience, proof slice, risk posture, and falsification
test, it should be marked `investigate` or `reject`, not added to the roadmap.
