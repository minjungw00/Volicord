# TASK Template

## Used when

Use `TASK` as the continuity projection for active work. It summarizes the four user-facing gate display groups first: Scope, User Decisions, Evidence, and Close Readiness. It also shows where the work is, current decision context, blocker ownership, Autonomy Boundary, Write Authority Summary, Implementation Micro-Plan, Review Stages, Stewardship Impact, next evidence, residual risk, close summary, kernel gate detail when useful, active Change Unit, pending decisions, evidence, report refs, and projection freshness.

Boundary: projection template only; it does not authorize runtime/server implementation or generated operational outputs. Shared phase and projection rules live in [Template Reference](README.md#used-when).

Implementation tier: required for user-facing MVP as a minimal continuity summary. Only the status, user decision request, evidence summary, and close readiness/blocker portions are early-required; the fuller TASK body is later profile polish.

## Source records

- `state.sqlite` Task and task gates
- active Change Unit and Change Unit dependencies
- current-state display inputs for mode, lifecycle, next action, primary blocker, smallest unblocker, guarantee level, and projection freshness
- display inputs for Scope, User Decisions, Evidence, and Close Readiness groups derived from existing owner records, gates, blockers, and refs
- Write Authorization records and Write Authority Summary display inputs
- Decision Packets and Residual Risks, including schema-owned Decision Packet `judgment_domain` when rendered
- latest Run, Evidence Manifest, Eval, Manual QA record, and approval records
- compact authority source refs for Write Authorization, Decision Packet, Approval, Evidence Manifest, Eval, Manual QA, Acceptance context, Residual Risk, Artifact refs, redaction state, and projection freshness when those claims are displayed
- primary blocker, secondary blocker, and smallest unblocker display summaries
- close summary display inputs, including changed scope, evidence, verification, Manual QA, residual-risk visibility, residual-risk acceptance, final acceptance, waivers, and close reason
- Journey Spine source records
- `domain_terms`, `module_map_items`, `interface_contracts`, and `feedback_loops`
- `tdd_traces` when TDD is selected
- design-quality validator results
- expected evidence needs
- Review Stage display inputs from existing owner records and refs
- artifact refs and projection freshness

Generated gate group summaries, user decision display text, close, waiver, review-stage, stewardship, and projection-freshness entries in `TASK` are display bindings. They should resolve to the owner records, gates, artifacts, and refs named above, or render an explicit absence/blocking state when no such source exists. Rendering schema-owned `judgment_domain` does not create canonical records, gates, `ProjectionKind` values, evidence, QA, verification, final acceptance, residual-risk acceptance, close, or Write Authorization.

## Rendered sections

- Gate Group Summary
- Current Summary
- Where We Are
- User Decision Context
- Authority Source Refs
- Autonomy Boundary
- Write Authority Summary
- Implementation Micro-Plan
- Review Stages
- Next Evidence
- Residual Risk
- Close Summary
- Stewardship Impact
- Goal
- Scope
- Acceptance Criteria
- Active Change Unit
- Pending Decisions
- Evidence And Reports
- User Notes and Proposals

Long-running `work` tasks may also render expanded managed sections for shared design, domain term refs, module/interface refs, Change Unit dependencies, implementation details, and Journey Spine.

## Full template

````md
---
doc_type: task
task_id: TASK-0001
display_state: executing
projection_version: 7
source_state_version: 42
updated_at: 2026-05-06T09:30:15+09:00
---

# TASK-0001 Task Title

> Projection view: rendered from `source_state_version` at `updated_at`. Managed sections are generated display; edits inside them are drift/reconcile candidates, not state changes.

<!-- HARNESS:BEGIN managed -->
## Gate Group Summary
- Scope:
  - what may change:
  - out of bounds:
  - write authority:
  - blocker / smallest unblocker:
  - source refs:
- User Decisions:
  - pending items (one line per decision; do not merge):
  - direction judgments:
    - Product/UX judgment:
    - technical architecture judgment:
    - security/privacy judgment:
    - scope/autonomy judgment:
  - permission:
    - sensitive-action approval:
  - waivers:
    - QA waiver:
    - verification waiver:
  - acceptance:
    - final acceptance:
  - risk acceptance:
    - residual-risk acceptance:
    - named risk being accepted:
  - decision / approval / waiver / acceptance / risk refs:
  - blocker / smallest unblocker:
  - what agent may continue:
- Evidence:
  - evidence status:
  - supporting refs:
  - missing or stale support:
  - artifact redaction or omission state:
  - does not replace: verification, Manual QA, final acceptance, or residual-risk acceptance
  - next evidence action:
- Close Readiness:
  - verification:
  - Manual QA:
  - final acceptance:
  - residual-risk visibility:
  - residual-risk acceptance:
  - waiver status:
  - close blockers / close reason:
  - smallest unblocker:
- note: These are display groups only. Exact gate values, recompute rules, and close semantics are owned by Kernel Reference.

## Current Summary
- mode:
- lifecycle phase:
- result:
- close reason:
- assurance:
- scope summary:
- out of bounds:
- next action:
- checked:
- remaining:
- primary blocker:
- blocker owner:
- smallest unblocker:
- secondary blockers:
- pending decision:
- pending decision type:
- user is deciding:
- risk:
- gate display groups: Scope=; User Decisions=; Evidence=; Close Readiness=
- guarantee level:
- kernel gate detail: scope=; decision=; approval=; design=; evidence=; verification=; Manual QA=; acceptance=
- active change unit:
- write authority summary:
- authority source refs: write=; decision=; approval=; evidence=; eval=; manual_qa=; acceptance=; residual_risk=; artifacts=
- redaction state:
- latest report:
- projection freshness:

## Where We Are
- current position:
- active path:
- checked:
- remaining:
- primary blocker:
- blocker owner:
- smallest unblocker:
- secondary blockers:
- latest meaningful evidence:
- next state transition:

## User Decision Context
- pending decision packets:
- pending decision items:
- decision type:
- decision title:
- judgment_domain:
- display label:
- decision_kind:
- decision_profile:
- why needed now:
- what user is deciding:
- options:
- trade-offs:
- recommendation:
- uncertainty:
- deferral consequence:
- residual risk when relevant:
- named risk being accepted:
- what agent may decide without user:
- what this decision does not settle:
- generic consent handling:
- reversibility:
- affected scope:
- minimum context to judge:
- affected display group:
- affected gate refs:

## Authority Source Refs
- Write Authorization:
- Decision Packet:
- Approval:
- Evidence Manifest:
- Eval:
- Manual QA:
- Acceptance Decision Packet:
- Acceptance context:
- Residual Risk:
- Artifact refs and redaction state:
- Projection freshness:

## Autonomy Boundary
- profile:
- agent may do:
- user decision required:
- AFK stop conditions:
- boundary status:

## Write Authority Summary
- active Change Unit:
- write authorization:
- allowed paths:
- allowed tools:
- allowed commands:
- allowed network targets:
- secret scope:
- sensitive categories:
- approval status:
- baseline:
- guarantee:
- note: Autonomy Boundary is judgment latitude, not write authority.

## Implementation Micro-Plan
- note: execution aid only; active Change Unit scope bounds writes and `prepare_write` creates Write Authorization.
- TDD note: when required, show the selected feedback loop, RED target, GREEN target, and whether non-test implementation is waiting on actual RED evidence or a waiver.

| Step / Slice | Purpose | Active Change Unit Scope / Likely Paths | Feedback Loop / TDD | Expected Evidence | Stop / Ask User When |
|---|---|---|---|---|---|
| 1 | | | | | |

## Review Stages
- note: managed display only; Role Lens/playbook labels do not create gates, records, `ProjectionKind` values, Approval, evidence, verification, QA, final acceptance, residual-risk acceptance, close, or Write Authorization. Same-session review is not detached verification. Route findings to existing owner records, refs, gates, or blockers.

### Spec Compliance Review
- acceptance criteria coverage:
- Change Unit completion conditions:
- scope / Write Authority compatibility:
- Decision Packet compatibility:
- evidence coverage:
- residual-risk visibility:
- routed outcome (existing path/ref only):

### Code Quality / Stewardship Review
- domain language:
- module / interface boundary:
- vertical slice shape:
- feedback loop / TDD:
- codebase stewardship:
- context hygiene:
- follow-up risk:
- routed outcome (existing path/ref only):

## Next Evidence
- next evidence action:
- evidence needed for:
- TDD RED target / plan:
- TDD RED evidence:
- TDD GREEN evidence:
- TDD refactor/check evidence:
- expected artifact refs:
- omitted or blocked artifact impact:
- stale or missing evidence:

## Residual Risk
- close-relevant risk:
- visibility status:
- status value:
- named risk being accepted:
- residual-risk acceptance status:
- accepted residual-risk refs:
- follow-up required:
- close impact:

## Close Summary
- changed scope:
- evidence:
- verification:
- Manual QA:
- residual-risk visibility:
- residual-risk acceptance:
- final acceptance:
- what final acceptance does not replace:
- waiver status:
- authority source refs:
- display state label (plain text, not a schema value):
- self-check refs:
- detached verification Eval ref:
- verification waiver ref:
- QA waiver ref:
- accepted residual-risk refs:
- close reason:
- remaining follow-up:

## Stewardship Impact
- summary shape: StewardshipImpactSummary
- domain_language_impact: none | updated | conflict | unresolved
- module_boundary_impact: none | local | public_boundary | unresolved
- interface_contract_impact: none | compatible | breaking | unresolved
- feedback_loop_status: defined | missing | waived
- future_change_risk: none | visible | accepted | unresolved
- close_impact: none | blocks_close | requires_decision | residual_risk
- refs:
  - domain term refs:
  - module map item refs:
  - interface contract refs:
  - feedback loop refs:
  - TDD trace refs when selected:
  - residual risk:
  - Decision Packets:

## Goal
-

## Scope
### In
-

### Out
-

## Acceptance Criteria
- [ ] AC-01:
- [ ] AC-02:

## Active Change Unit
| ID | Purpose | Status | Slice Type | TDD | Manual QA | Core Verification |
|---|---|---|---|---|---|---|
| CU-01 | | | vertical | trace status: required \| recorded \| waived \| not_required; show RED/GREEN refs | pending | |

## Pending Decisions
| Type | Question | Route / refs | Status | Next action |
|---|---|---|---|---|
| Product/UX judgment \| technical architecture judgment \| security/privacy judgment \| scope/autonomy judgment \| sensitive-action approval \| QA waiver \| verification waiver \| final acceptance \| residual-risk acceptance | | | | |

## Evidence And Reports
- Evidence Manifest:
- Run Summary:
- Eval:
- Direct Result:
- TDD Trace:
- Manual QA:
- Approval:
- Decision:
- Diff:
- Logs:
- Artifact refs with redaction state:
- Projection freshness:
<!-- HARNESS:END managed -->

## User Notes and Proposals
<!-- Human-editable: notes and proposals here are reconcile input and do not change state until accepted through Core. -->
-
````

Expanded TASK sections for long-running `work` tasks:

````md
<!-- HARNESS:BEGIN managed -->
## Shared Design Concept
### Questions Resolved
| ID | Question | User Answer | Decision / Assumption |
|---|---|---|---|

### Remaining Ambiguity
- item / owner / stop condition:

## Domain Term Refs
- Terms in force:
  - Term:

## Module and Interface Refs
- module map item refs:
- interface contract refs:
- rendered projection refs, if shown: MODULE-MAP, INTERFACE-CONTRACT
- DESIGN:

## Change Unit Dependencies
| ID | blocked_by | unblocks | parallelizable_with | merge risk |
|---|---|---|---|---|

## Implementation Micro-Plan Details
- source alignment: current Task, active Change Unit, gates, related refs
- boundary: not canonical state, not scope authority, not approval, not Write Authorization; active Change Unit remains the scope source

### Step Queue
| Step | State Alignment | Scope Alignment / Likely Paths | Feedback Loop / TDD Status | Evidence Target | Stop Condition |
|---|---|---|---|---|---|

## Journey Spine
### Facts in Force
- fact / evidence ref:

### Assumptions in Force
- assumption / expiry condition:

### Decisions in Force
- DEC-0001:

### Domain Terms in Force
- term / meaning / code representation:

### Module / Interface Impacts
- module / impact / interface / test boundary:

### Rejected Options
- option / reason / DEC:

### Watchpoints
- regression:
- security/performance/operations:
- architecture drift:

### Resume Notes
- next session should know:
- primary blocker:
- smallest unblocker:
<!-- HARNESS:END managed -->
````

Change Unit block sub-template:

````md
### CU-01 Title
- purpose:
- non-goals:
- slice type: vertical | enabling | cleanup | horizontal-exception
- horizontal exception reason:
- follow-up vertical CU:
- autonomy profile:
- agent may do:
  - implementation detail:
  - local refactor inside scope:
  - evidence collection:
- user decision required:
  - Product/UX judgment:
  - technical architecture judgment:
  - security/privacy judgment:
  - scope/autonomy judgment:
  - sensitive-action approval:
  - QA waiver:
  - verification waiver:
  - final acceptance:
  - public interface or compatibility commitment:
  - residual-risk acceptance:
- AFK stop conditions:
  - boundary exceeded:
  - evidence cannot be produced:
  - close-relevant risk discovered:
- end-to-end path:
  - trigger / input:
  - domain logic:
  - persistence:
  - API / caller boundary:
  - UI / observable output:
- allowed paths:
  - `src/...`
  - `tests/...`
- allowed tools:
  - read
  - edit
  - shell: `npm test -- ...`
- check profile:
  - changed_paths
  - approval_scope
  - evidence_sufficiency
- ValidatorResult IDs:
  - vertical_slice_shape
  - shared_design_alignment
  - decision_quality_check
  - autonomy_boundary_check
  - feedback_loop_check
  - tdd_trace_required
  - domain_language_consistency
  - module_interface_review
  - codebase_stewardship_check
  - residual_risk_visibility_check
  - manual_qa_required
- sensitive categories:
  - none
- TDD:
  - trace status: required | recorded | waived | not_required
  - requirement/source:
  - RED target / plan:
  - RED evidence (actual):
  - green evidence:
  - non-TDD justification:
- Manual QA:
  - required: yes | no
  - profile: ui_quality | workflow | copy | accessibility | browser_smoke | none
- dependencies:
  - blocked_by:
  - unblocks:
  - parallelizable_with:
  - merge risk:
- completion conditions:
  - [ ]
- evaluator focus:
  - item:
````

## Notes

Stewardship Impact in `TASK` is the `StewardshipImpactSummary` display derived from owner records, validator results, and refs. It does not replace Domain Language, Module Map, Interface Contract, Feedback Loop, TDD Trace, residual-risk, or Decision Packet owner records.

Implementation Micro-Plan in `TASK` is a lightweight execution aid rendered from or aligned with current Task and Change Unit state. It stays within the projection/report boundary in [Document Projection Reference](../document-projection.md#projection-principles) and never replaces `prepare_write` or owner state changes.

Review Stages in `TASK` are managed display sections for Role Lens, playbook, or two-stage review guidance. Their exact non-authority rule is owned by [Design Quality Policies](../design-quality-policies.md#two-stage-review-display) and [Agent Integration](../agent-integration.md#role-lens-behavior). They do not create canonical records, `ProjectionKind` values, Approval, evidence, verification, QA, final acceptance, residual-risk acceptance, close, or Write Authorization; findings must route to existing owner paths.

Generated summaries should use ordinary user-facing language first and exact Harness terms as labels or refs where useful. They should not turn the projection into a command language or imply that display text created state.

Gate Group Summary is the first managed section so readers see the practical blocker story before raw gate detail. Scope, User Decisions, Evidence, and Close Readiness are display groups derived from existing owner records, gates, blockers, and refs. They are not canonical fields, aliases for exact gate values, new gates, recompute inputs, close semantics, or authority paths. User Decisions is structured and must not be rendered as one broad judgment or approval bucket. Exact gate values and recompute rules remain in [Kernel Reference](../kernel.md#gates), and close behavior remains in [`close_task`](../kernel.md#close_task).

Decision Packet display in `TASK` should show the schema-owned `judgment_domain` so users can scan whether they are making a Product / UX, Technical architecture, Security / privacy, QA / acceptance, Residual risk, Scope / autonomy, or Mixed judgment. It must also show the concrete decision type derived from the route and owner refs: Product/UX judgment, technical architecture judgment, security/privacy judgment, scope/autonomy judgment, sensitive-action approval, QA waiver, verification waiver, final acceptance, or residual-risk acceptance. If a decision is cross-cutting, render secondary considerations in trade-offs, affected gates, risk, evidence, or follow-up instead of treating the domain as exclusive. `judgment_domain` is not a gate, status, validator input, close aggregation rule, or authority path, and it must not blur the owner contracts for `decision_kind`, Approval, acceptance, QA, residual-risk acceptance, close, or Write Authorization.

Pending decisions must not be merged into one line. If sensitive-action approval, final acceptance, and residual-risk acceptance are all pending, render three items with three labels. Approval cards should not look like final acceptance, and residual-risk acceptance should name the risk being accepted.

Authority claims in `TASK` must resolve to source refs or explicit absence. Write authority claims point to Write Authorization refs, sensitive-action permission to Approval refs, evidence sufficiency to Evidence Manifest refs, detached verification to Eval refs, Manual QA to Manual QA records or valid waiver refs, final acceptance to Acceptance Decision Packet refs, residual-risk visibility to Residual Risk refs or `ResidualRiskSummary.status=none`, and residual-risk acceptance to accepted Residual Risk refs. Missing refs should render as missing support, not as completed authority.

Residual-risk display must distinguish `status=none` from `not_visible`. `status=none` means no known close-relevant residual risk exists for the requested action. `not_visible` means known close-relevant risk exists but has not been made visible enough for acceptance or close; it should remain a blocker or next action until the risk and refs are shown.

Close and assurance display in `TASK` must keep self-checked work, `detached_verified`, verification waiver, QA waiver, and residual-risk accepted close visibly separate. A residual-risk accepted close should cite accepted Residual Risk refs and any required Decision Packet; a verification waiver should cite `verification_gate=waived_by_user` and its Decision Packet when required; a QA waiver should cite `qa_gate=waived`, the Manual QA record or waiver reason, and the QA waiver Decision Packet when required.

Waiver displays in `TASK` are summaries only. Close-relevant QA or verification waivers should point to the existing record that makes the waiver valid: `manual_qa_records`/`qa_gate=waived` and a QA waiver Decision Packet when required, or `verification_gate=waived_by_user` and its Decision Packet when required. They should also show the policy or gate, Task and Change Unit, skipped check or surface, reason, actor, expiry or residual-risk follow-up when needed, relevant refs, close impact, and any close-relevant residual risk that must be visible or accepted through the residual-risk path when required. A QA waiver does not become Manual QA, and a verification waiver does not create detached verification.

Close Summary in `TASK` is a continuity display summary for active or recently closed `work` tasks. It must not hide gate status or residual risk. When close is successful, blocked, canceled, or residual-risk accepted, the summary should show changed scope, evidence, verification, Manual QA, residual-risk visibility, residual-risk acceptance, final acceptance, waiver status, close reason, and residual-risk follow-up as applicable, with refs back to owner records. Final acceptance and residual-risk acceptance must remain separate lines: final acceptance is the user's result judgment, while residual-risk acceptance must identify the accepted risk and cite accepted Residual Risk refs.

Close Summary must not collapse evidence, verification, Manual QA, final acceptance, residual-risk visibility, and residual-risk acceptance into a single "done" flag. If tests pass but Manual QA, final acceptance, or residual-risk acceptance is still pending, the close display should show that exact category as the blocker.

Direct work uses `DIRECT-RESULT` for its low-ceremony close impact summary, and Journey Card close context is compact status/resume display. `TASK` Close Summary remains a continuity display under the [projection/report boundary](../document-projection.md#projection-principles); close and gate effects still come from owner records.

Artifact refs shown in `TASK`, Journey, evidence, and report sections must preserve redaction state. `secret_omitted` refs may support only visible nonsecret evidence; `blocked` refs show committed metadata-only notices and unavailable input rather than raw content.
