# DEC Template

## Used when

Use `DEC` when standalone Decision Packet projection is enabled for a specific user-owned decision: Product/UX judgment, technical architecture judgment, security/privacy judgment, scope/autonomy judgment, sensitive-action approval, QA waiver, verification waiver, final acceptance, residual-risk acceptance, or reconcile decision.

Boundary: projection template only; it does not authorize runtime/server implementation or generated operational outputs. Shared phase and projection rules live in [Template Reference](README.md#used-when).

Implementation tier: User-facing MVP summaries for the Decision Packet display/card shape for user decision requests, not as the standalone `DEC` `ProjectionKind`. A standalone persisted `DEC` Markdown projection remains optional unless the standalone Decision Packet projection feature is enabled; the required prompt can appear through status/next, `TASK`, or decision resources.

## Source records

- `state.sqlite.decision_packets`
- related Task and Change Unit refs
- `decision_kind`, schema-owned `decision_profile`, and schema-owned `judgment_domain`
- display decision type derived from `decision_kind`, `decision_profile`, `judgment_domain`, and related owner records
- related `decision_gate` state and decision events
- approval records for approval-shaped decisions
- related reconcile records, if applicable
- residual risk refs
- evidence and artifact refs
- Write Authorization, Approval, Evidence Manifest, Eval, Manual QA, final acceptance context, Artifact refs, redaction state, and projection freshness when displayed as related authority context
- affected scope display inputs: product areas, screens or flows, modules, interfaces, paths, acceptance criteria, gates, and sensitive categories
- projection freshness inputs

Approval-shaped display bullets such as "what this approval covers," "what this approval does not cover," and "secret exposure boundary" are derived display summaries from linked Approval records, approval scope, related Decision Packet refs, and current write or close context. They explain the boundary only; they do not grant Approval or settle separate user-owned judgment. Approval-shaped displays must be labeled as sensitive-action approval and must not look like final acceptance.

A resolved Decision Packet is not sensitive-action Approval unless it is the approval-shaped Decision Packet linked to an Approval record. Other Decision Packet resolutions may settle user-owned decisions, waivers, residual-risk acceptance, final acceptance, or reconcile choices, but they do not grant sensitive-action Approval.

`judgment_domain` is the schema-owned user-visible judgment grouping. Render it with a friendly label, but keep `decision_kind` as the lifecycle and gate route and render a concrete decision type. Render affected gates from `affected_gates` and related owner refs, not from the domain label. `judgment_domain` does not directly change close gate aggregation, sensitive-action Approval, waiver behavior, or residual-risk acceptance unless a separate owner rule says so.

`decision_profile` is the schema-owned prompt-depth and validation profile. Render `minimal_decision` as a concise explicit judgment, not as an incomplete full trade-off packet. Render full profiles such as `product_ux_tradeoff`, `architecture_tradeoff`, `approval_shaped`, `waiver`, `acceptance`, `residual_risk_acceptance`, `reconcile`, and `mixed` with the additional context that profile requires. The profile does not change authority by itself and must not merge separate approval, acceptance, waiver, residual-risk acceptance, and product/technical decisions into one answer.

## Rendered sections

- Why Now
- Current State
- Decision Profile, Type, And Route
- Approval-Shaped Context, If Applicable
- What User Is Deciding
- What Agent May Decide Without User
- Autonomy Boundary Impact, If Any
- Affected Scope And Boundaries
- Options
- Recommendation
- Consequence Of Deferring
- Minimum Context To Judge
- User Decision
- Residual-Risk Acceptance, If Applicable
- Follow-Up
- References

A sufficient rendered Decision Packet uses these sections to answer one user-owned decision, not to ask for broad permission. The exact public request and response fields are owned by [`harness.request_user_decision`](../mcp-api-and-schemas.md#harnessrequest_user_decision), and the canonical authority rules are owned by [Decision Packet](../kernel.md#decision-packet) and [Decision Gate](../kernel.md#decision-gate). This template may summarize the existing fields, including `judgment_domain`, but it must not add additional schema fields, gates, or alternate authority.

Profile-specific rendering follows the selected MCP `profile_payload` branch: common fields remain visible, while branch-specific sections may be omitted when the selected profile does not require them. A `minimal_decision` card should still show the question, route, domain, scope, concise options or selected outcome, related refs, and what the answer does not settle, but it does not need full pros/cons, recommendation, uncertainty, and deferral analysis unless those are material. Full profiles should render the detailed sections needed for the user to judge risk, trade-offs, approval scope, waiver impact, acceptance basis, residual-risk consequence, or reconcile target.

The user-facing question should ask for the decision directly: choose an option, defer it with the stated consequence, reject the path, waive the named check, accept the named risk, accept the result, or reconcile the named drift. Use "approve" only for the approval-shaped context linked to Approval. For other packet kinds, ask what choice should be recorded and what remains outside that choice. If several decisions are pending, render separate prompts or separate lines; do not combine approval, acceptance, and risk acceptance into one answer.

**Example content cues:**

Use the same rendered sections for these common Decision Packet shapes. These cues are not extra template sections.

- Tiny unblocker (`decision_profile=minimal_decision`): e.g., choose whether a button label should say "Save" or "Update" inside an already scoped settings copy change. Put the concise choice, scope, refs, and non-effects under What User Is Deciding and References. Do not force a full architecture-tradeoff layout.
- Product/UX trade-off (`judgment_domain=product_ux`): failed-login feedback as inline layer, toast, or modal. Put flow, interruption, accessibility, copy, and product-risk differences under Options and Recommendation.
- Product/copy trade-off: failed-login wording as generic, specific, or hybrid. Put account-enumeration risk, recovery usefulness, support burden, clarity, and product tone under Options and Minimum Context To Judge.
- Technical architecture choice (`decision_profile=architecture_tradeoff`, `judgment_domain=technical_architecture`): session cookie, bearer/JWT token, OAuth/OIDC provider, or social-login provider integration. Put revocation, CSRF/XSS exposure, client compatibility, implementation cost, identity-provider boundaries, and migration impact under Options and Minimum Context To Judge.
- Dependency approval versus dependency decision: if the user is approving an install command or dependency-file edit, put that sensitive-action boundary under Approval-Shaped Context. If the user is choosing whether the dependency is the right architecture direction, put the technical choice under What User Is Deciding and Options.
- Schema/data-model decision: put additive migration, compatibility shim, breaking cleanup, data backfill, migration evidence, rollback risk, and test boundary under Options and Minimum Context To Judge.
- Scope or Autonomy Boundary expansion: put the proposed additional surface, why current scope or latitude is insufficient, what remains out of bounds, and whether a smaller Change Unit can continue under Consequence Of Deferring.
- Security/privacy decision (`judgment_domain=security_privacy`): for PII logging, exported fields, redaction, audit logging, retention, rollback, user notice, or role exposure, compare privacy exposure, debugging value, proof needed, and follow-up. If a sensitive action is also needed, put that Approval boundary under Approval-Shaped Context and do not treat the Approval packet as resolving the security/privacy judgment.
- Public API/interface decision: put caller compatibility, migration path, documentation promise, and rollback risk under Options and Minimum Context To Judge. Do not treat a resolved API decision as merge authority, deployment authority, or Write Authorization.
- QA or acceptance decision (`judgment_domain=qa_acceptance`): for Manual QA, verification waiver, or final acceptance, put the skipped check or accepted result, accepted user/product/technical risk, relevant refs, close impact, and smallest credible follow-up under User Decision, Residual-Risk Acceptance when applicable, and Follow-Up.
- Residual-risk acceptance before close (`judgment_domain=residual_risk`): put the visible limitation, existing evidence, risk refs the user is being asked to accept, and remaining follow-up under Current State, Minimum Context To Judge, Residual-Risk Acceptance, and Follow-Up.
- Final acceptance: put the final result, evidence status, Manual QA and verification status, and close-relevant residual-risk visibility under Current State and Minimum Context To Judge. Do not treat final acceptance as approval for new sensitive actions, additional writes, deployment, or merge.
- Broad "go ahead" answers: show why the packet asks for this specific route and option. A generic consent phrase does not resolve product trade-off, architecture choice, QA waiver, verification risk, final acceptance, or residual-risk acceptance unless this packet records that exact judgment.

**Rendered example: minimal decision**

```text
Decision: Settings label wording
Profile: concise decision (`minimal_decision`)
Route/domain: product trade-off (`decision_kind=product_tradeoff`), Product / UX (`product_ux`)
Question: Should this scoped settings label say "Save" or "Update"?
Scope/refs: settings form copy in CU-04; source ref TASK-012/CU-04; no sensitive action or close-risk ref.
Choice to record: Save | Update
Does not settle: broader settings flow behavior, localization strategy, final acceptance, residual-risk acceptance, or write authority.
```

**Rendered example: full architecture trade-off**

```text
Decision: Login session architecture
Profile: detailed architecture trade-off (`architecture_tradeoff`)
Route/domain: architecture choice (`decision_kind=architecture_choice`), Technical architecture (`technical_architecture`)
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
decision_kind: product_tradeoff
decision_profile: product_ux_tradeoff
judgment_domain: product_ux
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

## Decision Profile, Type, And Route
- decision_profile: minimal_decision | product_ux_tradeoff | architecture_tradeoff | approval_shaped | waiver | acceptance | residual_risk_acceptance | reconcile | mixed
- profile display: concise decision | detailed trade-off | sensitive-action approval | waiver | final acceptance | residual-risk acceptance | reconcile | mixed
- profile-required detail: common fields plus selected profile_payload branch; minimal decisions may omit non-material detailed fields
- decision type: Product/UX judgment | technical architecture judgment | security/privacy judgment | scope/autonomy judgment | sensitive-action approval | QA waiver | verification waiver | final acceptance | residual-risk acceptance | reconcile
- decision_kind:
- judgment_domain:
- display label:
- route verb: choose | defer | reject | approve | waive | accept result | accept named risk | reconcile
- what this route can record:
- what this route cannot record:
- generic consent handling:

## Approval-Shaped Context, If Applicable
- card label: Sensitive-action approval
- decision_kind=approval scope:
- linked approval record:
- sensitive categories:
- what this approval covers:
- what this approval does not cover:
- must not be rendered as: final acceptance or residual-risk acceptance
- user-owned decision requiring separate Decision Packet:
- approval boundary:
- write authorization boundary:
- secret exposure boundary:

## What User Is Deciding
- decision type:
- judgment_domain:
- display label:
- decision_kind:
- decision_profile:
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
- user decision required:

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

## User Decision
- status: proposed | pending_user | resolved | deferred | rejected | blocked | superseded
- selected option:
- user decision:
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

This template is a rendered shape, not canonical state. Decision Packet visibility required by the active stage/profile still comes through `TASK` projections, status/next responses, judgment-context resources, and decision-packet resources unless standalone `DEC` projection is enabled.

Decision Packet projections should keep authority context refs compact and explicit. Displaying a Write Authorization, Approval, Evidence Manifest, Eval, Manual QA, final acceptance, residual-risk visibility, residual-risk acceptance, artifact, redaction, or freshness ref in this template does not make the packet prose the authority for that record.

Decision Packet cards should display one decision type at a time. Approval cards use sensitive-action approval language, acceptance prompts use final acceptance language, and residual-risk acceptance prompts name the specific risk being accepted.

Repeat option subsections as needed. Some product choices have more than two realistic options.
