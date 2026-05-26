# DEC Template

## Used when

Use `DEC` when standalone Decision Packet projection is enabled for user-owned product or material technical judgment, approval-shaped judgment, waiver, acceptance, residual-risk acceptance, or reconcile decisions.

## Source records

- `state.sqlite.decision_packets`
- related Task and Change Unit refs
- related `decision_gate` state and decision events
- approval records for approval-shaped decisions
- related reconcile records, if applicable
- residual risk refs
- evidence and artifact refs
- affected scope display inputs: product areas, screens or flows, modules, interfaces, paths, acceptance criteria, gates, and sensitive categories
- projection freshness inputs

Approval-shaped display bullets such as "what this approval covers," "what this approval does not cover," and "secret exposure boundary" are derived display summaries from linked Approval records, approval scope, related Decision Packet refs, and current write or close context. They explain the boundary only; they do not grant Approval or settle separate user-owned judgment.

A resolved Decision Packet is not sensitive Approval unless it is the approval-shaped Decision Packet linked to an Approval record. Other Decision Packet resolutions may settle user-owned judgment, waivers, residual-risk acceptance, final acceptance, or reconcile choices, but they do not grant sensitive-action Approval.

## Rendered sections

- Why Now
- Current State
- Approval-Shaped Context, If Applicable
- What User Is Deciding
- What Agent May Decide Without User
- Autonomy Boundary Impact, If Any
- Affected Scope And Boundaries
- Options
- Recommendation
- Consequence Of Deferring
- Minimum Context To Judge
- User Decision And Accepted Risk
- Follow-Up
- References

### Example content cues

Use the same rendered sections for these common Decision Packet shapes. These cues are not extra template sections.

- Product/UX trade-off: failed-login feedback as inline message, toast, or modal/layer. Put flow, interruption, accessibility, copy, and product-risk differences under Options and Recommendation.
- Product/copy trade-off: failed-login wording as generic, specific, or hybrid. Put account-enumeration risk, recovery usefulness, support burden, clarity, and product tone under Options and Minimum Context To Judge.
- Technical choice: session cookie, JWT, or social login. Put revocation, CSRF/XSS exposure, client compatibility, implementation cost, and migration impact under Options and Minimum Context To Judge.
- Dependency approval versus dependency decision: if the user is approving an install command or dependency-file edit, put that sensitive-action boundary under Approval-Shaped Context. If the user is choosing whether the dependency is the right architecture direction, put the technical choice under What User Is Deciding and Options.
- Schema/data-model decision: put additive migration, compatibility shim, breaking cleanup, data backfill, migration evidence, rollback risk, and test boundary under Options and Minimum Context To Judge.
- Security-sensitive approval: put the approval boundary under Approval-Shaped Context. If roles, exported fields, redaction, audit logging, retention, rollback, or user notice remain undecided, name them as unresolved product/security judgments and route them to separate compatible Decision Packets. Do not treat the approval packet as resolving those judgments.
- Public API/interface decision: put caller compatibility, migration path, documentation promise, and rollback risk under Options and Minimum Context To Judge. Do not treat a resolved API decision as merge authority, deployment authority, or Write Authorization.
- QA or verification waiver: put the skipped check or surface, accepted user/product/technical risk, relevant refs, close impact, and smallest credible follow-up under User Decision And Accepted Risk and Follow-Up.
- Residual-risk acceptance before close: put the visible limitation, existing evidence, risk refs the user is being asked to accept, and remaining follow-up under Current State, Minimum Context To Judge, User Decision And Accepted Risk, and Follow-Up.
- Final acceptance: put the final result, evidence status, Manual QA and verification status, and close-relevant residual-risk visibility under Current State and Minimum Context To Judge. Do not treat final acceptance as approval for new sensitive actions, additional writes, deployment, or merge.
- Broad "go ahead" answers: show why the packet asks for this specific route and option. A generic approval phrase does not resolve product trade-off, architecture choice, QA waiver, verification risk, final acceptance, or residual-risk acceptance unless this packet records that exact judgment.

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
- source refs:

## Approval-Shaped Context, If Applicable
- decision_kind=approval scope:
- linked approval record:
- sensitive categories:
- what this approval covers:
- what this approval does not cover:
- user-owned judgment requiring separate Decision Packet:
- approval boundary:
- write authorization boundary:
- secret exposure boundary:

## What User Is Deciding
- decision category:
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

## User Decision And Accepted Risk
- status: proposed | pending_user | resolved | deferred | rejected | blocked | superseded
- selected option:
- user decision:
- decision note:
- broad approval handling:
- accepted residual-risk summary:
- accepted residual-risk refs:
- accepted consequence:
- decided by:
- decided at:

## Follow-Up
- [ ]

## References
- TASK:
- Change Unit:
- DESIGN:
- APR:
- EVIDENCE-MANIFEST:
- EVAL:
- MANUAL-QA:
- Residual Risk:
- artifacts:
````

## Notes

This template is a rendered shape, not canonical state. MVP Decision Packet visibility still comes through `TASK` projections, status/next responses, judgment-context resources, and decision-packet resources unless standalone `DEC` projection is enabled.

Repeat option subsections as needed. Some product choices have more than two realistic options.
