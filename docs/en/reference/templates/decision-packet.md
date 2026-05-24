# DEC Template

## Used when

Use `DEC` when standalone Decision Packet projection is enabled for product judgment, approval-shaped judgment, waiver, acceptance, residual-risk acceptance, or reconcile decisions.

## Source records

- `state.sqlite.decision_packets`
- related Task and Change Unit refs
- related `decision_gate` state and decision events
- approval records for approval-shaped decisions
- related reconcile records, if applicable
- residual risk refs
- evidence and artifact refs
- projection freshness inputs

## Rendered sections

- Why Now
- Current State
- Approval-Shaped Context, If Applicable
- What User Is Deciding
- What Agent May Decide Without User
- Autonomy Boundary Impact, If Any
- Options
- Recommendation
- Consequence Of Deferring
- Minimum Context To Judge
- User Decision And Accepted Risk
- Follow-Up
- References

## Example content cues

Use the same rendered sections for these common Decision Packet shapes. These cues are not extra template sections.

- Product/UX trade-off: failed-login feedback as inline message, toast, or modal/layer. Put flow, interruption, accessibility, copy, and product-risk differences under Options and Recommendation.
- Technical choice: session cookie, JWT, or social login. Put revocation, CSRF/XSS exposure, client compatibility, implementation cost, and migration impact under Options and Minimum Context To Judge.
- Security-sensitive approval: put the approval boundary under Approval-Shaped Context. If roles, exported fields, redaction, audit logging, retention, rollback, or user notice remain undecided, name them as unresolved product/security judgments and route them to separate compatible Decision Packets. Do not treat the approval packet as resolving those judgments.
- QA or verification waiver: put the skipped check, accepted user/product/technical risk, and smallest credible follow-up under User Decision And Accepted Risk and Follow-Up.
- Residual-risk acceptance before close: put the visible limitation, existing evidence, risk refs the user is being asked to accept, and remaining follow-up under Current State, Minimum Context To Judge, User Decision And Accepted Risk, and Follow-Up.

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
- product judgment requiring separate Decision Packet:
- approval boundary:
- write authorization boundary:

## What User Is Deciding
- decision:
- affected scope:
- affected acceptance criteria:
- affected gates:

## What Agent May Decide Without User
- implementation detail:
- code organization inside approved scope:
- evidence collection:
- follow-up proposal:

## Autonomy Boundary Impact, If Any
- current boundary impact:
- proposed boundary update:
- user judgment required:

## Options
### Option A
- choice:
- benefits:
- costs:
- risks:
- reversibility: reversible | partially_reversible | irreversible | unknown
- confidence: low | medium | high
- evidence refs:

### Option B
- choice:
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
