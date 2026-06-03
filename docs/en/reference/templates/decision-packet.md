# DEC Template

## Used when

Use `DEC` when standalone Decision Packet projection is enabled for a specific user-owned judgment: Product/UX judgment, technical architecture judgment, security/privacy judgment, scope/autonomy judgment, sensitive-action approval, QA/verification waiver, work acceptance, residual-risk acceptance, or reconcile judgment.

Boundary: projection template only; it does not authorize runtime/server implementation or generated operational outputs. Shared phase and projection rules live in [Template Reference](README.md#used-when).

Implementation tier: User judgment prompt shape. Use it for the Decision Packet display/card shape when a user-owned judgment is pending, not as the v0.2 First User-Value Slice projection and not as the standalone `DEC` `ProjectionKind`. A standalone persisted `DEC` Markdown projection remains optional unless the standalone Decision Packet projection feature is enabled; the required prompt can appear through the compact status card, status/next, or decision resources.

## Source records

- `state.sqlite.decision_packets`
- related Task and Change Unit refs
- `judgment_category`, `judgment_route`, and `display_depth`
- display judgment type derived from `judgment_category`, `judgment_route`, `display_depth`, and related owner records
- related `decision_gate` state and decision events
- approval records for approval-shaped decisions
- related reconcile records, if applicable
- residual risk refs
- evidence and artifact refs
- Write Authorization, Approval, Evidence Manifest, Eval, Manual QA, work-acceptance context, Artifact refs, redaction state, and projection freshness when displayed as related authority context
- affected scope display inputs: product areas, screens or flows, modules, interfaces, paths, acceptance criteria, gates, and sensitive categories
- projection freshness inputs

Approval-shaped display bullets such as "what this approval covers," "what this approval does not cover," and "secret exposure boundary" are derived display summaries from linked Approval records, approval scope, related Decision Packet refs, and current write or close context. They explain the boundary only; they do not grant Approval or settle separate user-owned judgment. Approval-shaped displays must be labeled as sensitive-action approval and must not look like work acceptance.

A resolved Decision Packet is not sensitive-action Approval unless it is the approval-shaped Decision Packet linked to an Approval record. Other Decision Packet resolutions may settle user-owned judgments, waivers, work acceptance, residual-risk acceptance, or reconcile choices, but they do not grant sensitive-action Approval.

`judgment_category` is the user-facing category for grouping and display. Render it with a friendly label, but keep `judgment_route` visible as the route that controls the owner path and recorded-answer rules. Render affected gates from `affected_gates` and related owner refs, not from the category label. `judgment_category` does not directly change close gate aggregation, sensitive-action Approval, waiver behavior, work acceptance, or residual-risk acceptance unless a separate owner rule says so.

`display_depth` is the prompt depth for the rendered packet. Render `simple` as a concise explicit judgment, not as an incomplete full trade-off packet. Render `tradeoff`, `high-risk`, and `close-affecting` with the additional context that the route and risk level require. Display depth does not change authority by itself and must not merge separate approval, work acceptance, waiver, residual-risk acceptance, and product/technical judgments into one answer.

## Rendered sections

- Why Now
- Current State
- Judgment Category, Route, And Display Depth
- Approval-Shaped Context, If Applicable
- What User Is Deciding
- What Agent May Decide Without User
- Autonomy Boundary Impact, If Any
- Affected Scope And Boundaries
- Options
- Recommendation
- Consequence Of Deferring
- Minimum Context To Judge
- User Judgment
- Residual-Risk Acceptance, If Applicable
- Follow-Up
- References

A sufficient rendered Decision Packet uses these sections to answer one user-owned judgment, not to ask for broad permission. The exact public request and response fields are owned by [`harness.request_user_judgment`](../mcp-api-and-schemas.md#harnessrequest_user_judgment), and the canonical authority rules are owned by [Decision Packet](../kernel.md#decision-packet) and [Decision Gate](../kernel.md#decision-gate). This template may summarize the existing fields, including `judgment_category`, `judgment_route`, and `display_depth`, but it must not add additional schema fields, gates, or alternate authority.

Route-specific rendering follows the selected MCP `judgment_payload` and `judgment_route`: common fields remain visible, while route-specific sections may be omitted when the selected route and display depth do not require them. The final recorded answer is separate: `judgment_route` selects the user-judgment route and `RecordUserJudgmentPayload` value rules. A `display_depth=simple` card should still show the question, route, category, scope, concise options or selected outcome, related refs, and what the answer does not settle, but it does not need full pros/cons, recommendation, uncertainty, and deferral analysis unless those are material. Higher-depth prompts should render the detailed sections needed for the user to judge risk, trade-offs, approval scope, waiver impact, acceptance basis, residual-risk consequence, or reconcile target.

The user-facing question should ask for the decision directly: choose an option, defer it with the stated consequence, reject the path, waive the named check, accept the named risk, accept the result, or reconcile the named drift. Use "approve" only for the approval-shaped context linked to Approval. For other packet kinds, ask what choice should be recorded and what remains outside that choice. If several decisions are pending, render separate prompts or separate lines; do not combine approval, acceptance, and risk acceptance into one answer.

**Example content cues:**

Use the same rendered sections for these common Decision Packet shapes. These cues are not extra template sections.

- Tiny unblocker (`display_depth=simple`, `judgment_route=choose`): e.g., choose whether a button label should say "Save" or "Update" inside an already scoped settings copy change. Put the concise choice, scope, refs, and non-effects under What User Is Deciding and References. Do not force a full architecture-tradeoff layout.
- Product/UX trade-off (`judgment_category=product_ux`, `display_depth=tradeoff`): failed-login feedback as inline layer, toast, or modal. Put flow, interruption, accessibility, copy, and product-risk differences under Options and Recommendation.
- Product/copy trade-off: failed-login wording as generic, specific, or hybrid. Put account-enumeration risk, recovery usefulness, support burden, clarity, and product tone under Options and Minimum Context To Judge.
- Technical architecture choice (`judgment_category=technical_architecture`, `display_depth=tradeoff`): session cookie, bearer/JWT token, OAuth/OIDC provider, or social-login provider integration. Put revocation, CSRF/XSS exposure, client compatibility, implementation cost, identity-provider boundaries, and migration impact under Options and Minimum Context To Judge.
- Dependency approval versus dependency decision: if the user is approving an install command or dependency-file edit, put that sensitive-action boundary under Approval-Shaped Context. If the user is choosing whether the dependency is the right architecture direction, put the technical choice under What User Is Deciding and Options.
- Schema/data-model decision: put additive migration, compatibility shim, breaking cleanup, data backfill, migration evidence, rollback risk, and test boundary under Options and Minimum Context To Judge.
- Scope or Autonomy Boundary expansion: put the proposed additional surface, why current scope or latitude is insufficient, what remains out of bounds, and whether a smaller Change Unit can continue under Consequence Of Deferring.
- Security/privacy judgment (`judgment_category=security_privacy`): for PII logging, exported fields, redaction, audit logging, retention, rollback, user notice, or role exposure, compare privacy exposure, debugging value, proof needed, and follow-up. If a sensitive action is also needed, put that Approval boundary under Approval-Shaped Context and do not treat the Approval packet as resolving the security/privacy judgment.
- Public API/interface decision: put caller compatibility, migration path, documentation promise, and rollback risk under Options and Minimum Context To Judge. Do not treat a resolved API decision as merge authority, deployment authority, or Write Authorization.
- QA/verification waiver (`judgment_category=qa_verification`, `judgment_route=waive`): put the skipped check, accepted user/product/technical risk, relevant refs, close impact, and smallest credible follow-up under User Judgment, Residual-Risk Acceptance when applicable, and Follow-Up.
- Work acceptance (`judgment_category=work_acceptance`, `judgment_route=accept-result`): put the final result, evidence status, Manual QA and verification status, and close-relevant residual-risk visibility under Current State and Minimum Context To Judge. Do not treat work acceptance as approval for new sensitive actions, additional writes, deployment, or merge.
- Residual-risk acceptance before close (`judgment_category=residual_risk`, `judgment_route=accept-risk`): put the visible limitation, existing evidence, risk refs the user is being asked to accept, and remaining follow-up under Current State, Minimum Context To Judge, Residual-Risk Acceptance, and Follow-Up.
- Broad "go ahead" answers: show why the packet asks for this specific route and option. A generic consent phrase does not resolve product trade-off, architecture choice, QA waiver, verification risk, work acceptance, or residual-risk acceptance unless this packet records that exact judgment.

**Rendered example: minimal decision**

```text
Decision: Settings label wording
Display depth: simple
Route/category: choose, Product / UX (`product_ux`)
Question: Should this scoped settings label say "Save" or "Update"?
Scope/refs: settings form copy in CU-04; source ref TASK-012/CU-04; no sensitive action or close-risk ref.
Choice to record: Save | Update
Does not settle: broader settings flow behavior, localization strategy, work acceptance, residual-risk acceptance, or write authority.
```

**Rendered example: approval-shaped decision**

```text
Decision: Dependency install approval
Display depth: high-risk
Route/category: approve-sensitive-action, Security / privacy (`security_privacy`)
Question: Do you approve the named dependency install/update action for this task?
Approval scope: named install command or dependency-file update; named manifest/lockfile paths; current task and approval window only.
Covers: the scoped sensitive action.
Does not cover: resolving whether the dependency is the right architecture direction, future installs, unrelated product writes, QA or verification waiver, work acceptance, or residual-risk acceptance.
Separate judgments required: use a `judgment_category=technical_architecture`, `display_depth=tradeoff` packet if the dependency choice itself is still user-owned judgment.
Refs: approval scope refs, prepare-write candidate refs, dependency comparison refs, and affected file refs when available.
```

**Rendered example: full architecture trade-off**

```text
Decision: Login session architecture
Display depth: tradeoff
Route/category: choose, Technical architecture (`technical_architecture`)
Question: Which session model should this login work use?
Options: server-side session cookie; client-held bearer/JWT; OAuth/OIDC provider plus local session strategy; social-login provider integration.
Recommendation: server-side session cookie for a first-party web app unless current requirements need third-party identity, non-browser clients, or social sign-in now.
Uncertainty: existing session middleware, revocation requirements, SSO requirement, CSRF posture, and migration constraints.
Deferral consequence: read-only inspection and UI scaffolding can continue only if they do not commit to storage, token lifetime, provider, or middleware behavior.
Refs: auth model refs, affected acceptance criteria, security evidence refs when available, and any residual-risk or migration refs.
```

## Full template

````md
---
doc_type: decision_packet
projection_kind: DEC
projection_id: DEC-PROJ-0001
decision_packet_id: DEC-0001
task_id: TASK-0001
change_unit_id: CU-01
judgment_category: product_ux
judgment_route: choose
display_depth: tradeoff
status: pending_user
source_state_version: 42
updated_at: 2026-05-06T09:30:15+09:00
---

# DEC-0001 Decision Packet Title

> Projection view: rendered from `source_state_version` at `updated_at`; displays `decision_packet_id` and related refs from state. Editing this Markdown does not resolve the Decision Packet; decisions are recorded through the decision path.

## Why Now
- trigger:
- blocker:
- affected operation:
- why this cannot proceed under current state:

## Current State
- task state:
- active change unit:
- current gates:
- latest evidence:
- residual risk:
- source refs: decision={decision_packet_id}; write={write_authorization_ref|none}; approval={approval_refs|none}; evidence={evidence_manifest_ref|none}; eval={eval_ref|none}; manual_qa={manual_qa_ref|none}; acceptance={acceptance_context_ref|none}; residual_risk={residual_risk_refs|none}; artifacts={artifact_refs|none}; redaction={redaction_availability_summary|none}; freshness={projection_freshness}

## Judgment Category, Route, And Display Depth
- judgment_category: product_ux | technical_architecture | security_privacy | scope_autonomy | qa_verification | work_acceptance | residual_risk | mixed
- judgment_route: choose | defer | approve-sensitive-action | waive | accept-result | accept-risk | reconcile
- display_depth: simple | tradeoff | high-risk | close-affecting
- display type: Product/UX judgment | technical architecture judgment | security/privacy judgment | scope/autonomy judgment | sensitive-action approval | QA/verification waiver | work acceptance | residual-risk acceptance | reconcile
- route-specific detail: common fields plus the selected `judgment_payload` branch validated by `judgment_route`; simple prompts may omit non-material detailed fields or render them as null where the schema allows null
- final recorded answer: `judgment_route` chooses the user-judgment route and `RecordUserJudgmentPayload` value rules
- display label:
- route verb: choose | defer | reject | approve | waive | accept result | accept named risk | reconcile
- what this route can record:
- what this route cannot record:
- generic consent handling:

## Approval-Shaped Context, If Applicable
- card label: Sensitive-action approval
- judgment_route=approve-sensitive-action scope:
- linked approval record:
- sensitive categories:
- what this approval covers:
- what this approval does not cover:
- must not be rendered as: work acceptance or residual-risk acceptance
- user-owned judgment requiring separate Decision Packet:
- approval boundary:
- write authorization boundary:
- secret exposure boundary:

## What User Is Judging
- judgment type:
- judgment_category:
- display label:
- judgment_route:
- display_depth:
- user-facing question:
- decision:
- what this decision settles:
- what this decision does not settle:
- why broad approval is insufficient:

## What Agent May Decide Without User
- implementation detail:
- code organization inside granted scope:
- evidence collection:
- follow-up proposal:

## Autonomy Boundary Impact, If Any
- current boundary impact:
- proposed boundary update:
- user judgment required:

## Affected Scope And Boundaries
- in scope:
- out of bounds:
- affected product areas:
- affected screens or flows:
- affected modules/interfaces/paths:
- affected acceptance criteria:
- affected gates:
- sensitive categories:

## Options
### Option A
- choice:
- trade-offs:
- benefits:
- costs:
- risks:
- reversibility: reversible | partially_reversible | irreversible | unknown
- confidence: low | medium | high
- evidence refs:

### Option B
- choice:
- trade-offs:
- benefits:
- costs:
- risks:
- reversibility: reversible | partially_reversible | irreversible | unknown
- confidence: low | medium | high
- evidence refs:

## Recommendation
- recommendation:
- reason:
- uncertainty:

## Consequence Of Deferring
- consequence:
- can continue if deferred:
- must stop until decided:
- operation impact:
- close impact:
- residual risk or follow-up visibility:

## Minimum Context To Judge
- relevant facts:
- assumptions:
- constraints:
- evidence refs:
- residual risk refs:
- related decisions:

## User Judgment
- status: proposed | pending_user | resolved | deferred | rejected | blocked | superseded
- selected option:
- user judgment:
- decision note:
- broad approval handling:
- decided by:
- decided at:

## Residual-Risk Acceptance, If Applicable
- named risk being accepted:
- residual-risk visibility status:
- accepted residual-risk summary:
- accepted residual-risk refs:
- accepted consequence:
- what risk acceptance does not replace:

## Follow-Up
- [ ]

## References
- TASK:
- Change Unit:
- Write Authorization:
- DESIGN:
- APR:
- EVIDENCE-MANIFEST:
- EVAL:
- MANUAL-QA:
- Acceptance context:
- Residual Risk:
- artifacts:
- redaction state:
- projection freshness:
````

## Notes

This template is a rendered shape, not canonical state. Decision Packet visibility required by the active stage/profile can come through the compact status card, status/next responses, judgment-context resources, decision-packet resources, or a dedicated prompt. `TASK` may also show it when a later continuity profile is active. Standalone `DEC` projection remains optional.

Decision Packet projections should keep authority context refs compact and explicit. Displaying a Write Authorization, Approval, Evidence Manifest, Eval, Manual QA, work acceptance, residual-risk visibility, residual-risk acceptance, artifact, redaction, or freshness ref in this template does not make the packet prose the authority for that record.

Decision Packet cards should display one judgment type at a time. Approval cards use sensitive-action approval language, work-acceptance prompts use work-acceptance language, and residual-risk acceptance prompts name the specific risk being accepted.

Repeat option subsections as needed. Some product choices have more than two realistic options.
