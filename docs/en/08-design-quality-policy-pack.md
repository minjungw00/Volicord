# Design Quality Policy Pack

## Document Role

This document owns design-quality policies as policy contracts. These policies guide how AI-assisted work stays aligned with product design, domain language, module boundaries, testing discipline, human QA, and context hygiene.

The policy strategy is agency-preserving: agents should move independently inside clear boundaries, surface meaningful choices as decisions, and stop for user judgment when product direction, risk acceptance, or public commitments are at stake.

Design-quality policies are not additional kernel invariants. The kernel owns lifecycle, gate transitions, close semantics, blocker mechanics, and state transitions. This document tells policy evaluators when `decision_gate`, `design_gate`, `qa_gate`, evidence sufficiency, `prepare_write` blockers, or close blockers may be affected. It does not define kernel transitions.

This document does not define MCP schemas, SQLite DDL, state transition tables, or full templates.

## Policy Contract Shape

Each policy uses the same fields:

| Field | Meaning |
|---|---|
| `name` | Stable policy name. |
| `applies_when` | Conditions that make the policy relevant. |
| `default_requirement` | What should happen by default when it applies. |
| `allowed_waiver` | Who may waive it and what must be recorded. |
| `required_record` | Canonical state record or record family that stores the result. |
| `validator` | Validator that reports compliance, warning, failure, or blocker. |
| `evidence` | Evidence or projection refs expected by the policy. |
| `close_impact` | How unmet requirements affect close or gates. |

Policy validators return the validator result schema owned by the MCP API document.

## Policy Contracts

### Shared Design

| Field | Contract |
|---|---|
| `name` | `shared_design` |
| `applies_when` | Work request is ambiguous, scope/non-scope is unclear, user value needs alignment, public interface/schema/auth/UX/workflow is affected, or a `work` task needs shaping. |
| `default_requirement` | Record goal, scope, non-goals, acceptance criteria, blocking decisions, assumptions, rejected options, domain-language impact, module/interface impact, and first Change Unit shape. Separate agent assumptions from choices that need user judgment, ask the most blocking questions one at a time, and stop when the first safe Change Unit can be proposed. |
| `allowed_waiver` | Allowed for small obvious `direct` work, docs-only edits, or emergency fixes when the user/operator records a reason and a follow-up if design risk remains. |
| `required_record` | Shared Design record, Task shaping fields, decision records, and optionally `DESIGN` or `DEC` projections. |
| `validator` | `shared_design_alignment` |
| `evidence` | Task summary, acceptance criteria, decision refs, rejected option refs, domain/module/interface impact refs. |
| `close_impact` | If required and absent, set or keep `design_gate=pending` or `partial`. If risk is high and no waiver exists, block close. A valid waiver may allow `design_gate=waived`. |

### Decision Quality

| Field | Contract |
|---|---|
| `name` | `decision_quality` |
| `applies_when` | Design choices, product trade-offs, scope expansion, public API/interface changes, architecture choices, horizontal exceptions, verification waiver, QA waiver, or acceptance with known risk. |
| `default_requirement` | Record a Decision Packet before the decision is acted on. The packet must capture context, options considered, trade-offs, recommendation, uncertainty, reversibility, evidence refs, deferral consequence, and residual risk. Keep agent recommendation distinct from user judgment or risk acceptance. For `decision_kind=approval`, evaluate the clarity of the sensitive-change scope and boundary; do not treat approval-shaped context as resolving product judgment. |
| `allowed_waiver` | Allowed only for trivial reversible choices with no public interface, product, architecture, verification, QA, or known-risk impact. Waiver must record why a Decision Packet would not improve judgment. |
| `required_record` | Decision Packet records and optionally `DEC` projection when rendered. |
| `validator` | `decision_quality_check` |
| `evidence` | Decision Packet refs, option refs, evidence manifest refs, risk/waiver refs, residual-risk state refs when risk acceptance is involved, and user acceptance refs when user judgment is required. |
| `close_impact` | Missing required decision quality for blocking product judgment sets or keeps `decision_gate=required`, `pending`, or `blocked`. Keep `design_gate` impact only when the decision affects design quality. Unresolved user judgment, invalid deferral, or unaccepted residual risk blocks affected writes or close. Valid recorded acceptance may allow close with residual risk preserved in state refs. |

### Autonomy Boundary

| Field | Contract |
|---|---|
| `name` | `autonomy_boundary` |
| `applies_when` | Agent is shaping or executing work with ambiguous authority, user constraints, external side effects, irreversible edits, scope expansion, sensitive action, product judgment, public commitments, or known stop conditions. |
| `default_requirement` | Record what the agent may do without user input, what requires user judgment, and stop conditions. The canonical boundary is on the active Change Unit; Task or Shared Design may carry shaping/proposed boundary refs before a Change Unit exists. The boundary should let the agent proceed on low-risk implementation details while pausing on product direction, risk acceptance, public interface commitments, or policy waivers that require human judgment. Autonomy Boundary is not a scope grant and does not authorize paths, tools, commands, network, secrets, or sensitive categories outside the active Change Unit. |
| `allowed_waiver` | Allowed for narrow `direct` work where authority is obvious from the request and no stop condition can reasonably be triggered. Waiver must record why no autonomy boundary is needed. |
| `required_record` | Canonical Autonomy Boundary record on the active Change Unit; Task or Shared Design shaping/proposed boundary refs before a Change Unit exists; Decision Packet records for user-judgment items; and stop-condition refs when triggered. |
| `validator` | `autonomy_boundary_check` |
| `evidence` | User request refs, task constraints, policy refs, Decision Packet refs, stop-condition events, user response refs. |
| `close_impact` | At `prepare_write`, triggered stop conditions or boundary gaps block the write. Product-judgment gaps should request or reference a Decision Packet and affect `decision_gate`; design-quality gaps may affect `design_gate`. Scope, approval, and capability gaps remain visible as their own blockers. Unresolved stop conditions can block close until resolved, deferred, or accepted with recorded risk. |

### Domain Language

| Field | Contract |
|---|---|
| `name` | `domain_language` |
| `applies_when` | New product term appears, an existing term is used with a new meaning, code and product language diverge, multiple names refer to one concept, or reviewer/evaluator finds a term mismatch. |
| `default_requirement` | Record or update affected terms with meaning, code representation, "not this" boundary, related terms, source, and status. Implementation agents pull only task-relevant terms; reviewers/evaluators receive relevant terms and any active terminology uncertainty. |
| `allowed_waiver` | Allowed when the work has no domain term impact or the term is intentionally local/temporary. Waiver must record why no canonical term update is needed. |
| `required_record` | `domain_terms` records; `DOMAIN-LANGUAGE` is projection only. |
| `validator` | `domain_language_consistency` |
| `evidence` | Domain term refs, code refs, test naming refs, reconcile item refs for proposals. |
| `close_impact` | If required terms are missing or conflicting, mark `design_gate=partial` or `stale`; block close when the mismatch affects acceptance criteria, public behavior, or verification confidence. |

### Vertical Slice

| Field | Contract |
|---|---|
| `name` | `vertical_slice` |
| `applies_when` | Feature work, user-visible behavior, workflow change, integration behavior, or medium/large `work` task. |
| `default_requirement` | Prefer a thin end-to-end Change Unit that connects trigger/input, domain logic, persistence or state, API/caller boundary, observable output, test evidence, and optional Manual QA. |
| `allowed_waiver` | Horizontal/enabling Change Units are allowed when scaffold, test harness, deep module boundary, migration safety, or public interface decisions must come first. The Change Unit must record `horizontal_exception_reason`, link a Decision Packet when the exception is a design or architecture choice, and record a follow-up vertical Change Unit unless no meaningful end-to-end path exists yet; if not applicable, record why. |
| `required_record` | Change Unit fields: `slice_type`, end-to-end path, completion conditions, follow-up vertical Change Unit, and validator results. |
| `validator` | `vertical_slice_shape` |
| `evidence` | Change Unit record, run summary, evidence manifest, tests, Manual QA refs if user-visible. |
| `close_impact` | If vertical slice is required and neither satisfied nor waived, set `design_gate=partial` or `blocked`. A justified horizontal exception may allow close only when the follow-up risk is recorded. |

### Feedback Loop

| Field | Contract |
|---|---|
| `name` | `feedback_loop` |
| `applies_when` | Before implementation starts, before a behavior-affecting write, when TDD is waived, when Manual QA is expected, or when the agent needs a credible way to learn whether the change works. |
| `default_requirement` | Define the feedback loop before implementation: test, typecheck, lint, build, browser smoke, Manual QA, or an explicit alternate loop. The selected loop should be the smallest credible loop for the risk. TDD trace is one implementation of this policy, not the only implementation. |
| `allowed_waiver` | Allowed for docs-only edits, comments, formatting, or advisory work with no implementation or product behavior impact. Waiver must record why no executable, browser, Manual QA, or alternate loop is useful. |
| `required_record` | Task or Change Unit feedback-loop fields, selected-loop refs, validator results, `tdd_traces` when TDD is selected, Manual QA record when Manual QA is selected and performed, `qa_gate=pending` when required QA has no satisfying record yet, and evidence manifest refs when executed. |
| `validator` | `feedback_loop_check` |
| `evidence` | Planned loop refs, test/typecheck/lint/build/browser smoke logs, Manual QA refs, alternate-loop justification, TDD trace refs when used. |
| `close_impact` | Missing feedback loop definition keeps `design_gate=pending` or `partial`. Missing execution evidence can make evidence insufficient. Manual QA loop failures affect `qa_gate` through the Manual QA policy. |

### TDD Trace

| Field | Contract |
|---|---|
| `name` | `tdd_trace` |
| `applies_when` | Domain logic, service module, bug fix, parser/validator, state transition, deep module internals, or edge-case-heavy behavior. Recommended for API/caller boundaries and integration behavior. |
| `default_requirement` | Use TDD as the selected feedback loop when it is the best fit. Record red, green, and refactor evidence for at least one acceptance criterion or behavior slice. Link the trace to the evidence manifest. |
| `allowed_waiver` | Allowed for docs, typos, throwaway prototypes, exploratory UI prototypes, initial scaffolds, or when the user/operator records a non-TDD justification and alternate feedback loop. |
| `required_record` | `tdd_traces` records and `TDD-TRACE` projection when rendered. |
| `validator` | `tdd_trace_required` |
| `evidence` | Failing test log, passing test log, refactor check log, diff refs, non-TDD justification when waived. |
| `close_impact` | Missing required TDD trace makes `design_gate=partial` and may make evidence insufficient. A valid non-TDD justification may satisfy design policy but does not by itself prove behavior. |

### Deep Module / Interface

| Field | Contract |
|---|---|
| `name` | `deep_module_interface` |
| `applies_when` | Public interface changes, module boundary changes, schema/data model changes, auth/security boundaries, compatibility impact, deep module internals, or shallow-module risk. |
| `default_requirement` | Identify affected modules, current role, proposed public interface, internal complexity hidden behind the interface, callers impacted, compatibility impact, and test boundary. Prefer small simple public interfaces with enough internal capability behind them. Use Decision Packets for public interface, compatibility, or architecture choices. |
| `allowed_waiver` | Allowed for localized internal changes with no public boundary impact, no dependency direction change, and low compatibility risk. Must record why module/interface review is unnecessary. |
| `required_record` | `module_map_items`, `interface_contracts`, decision records, and optionally `MODULE-MAP` / `INTERFACE-CONTRACT` projections. |
| `validator` | `module_interface_review` |
| `evidence` | Module map refs, interface contract refs, caller impact list, boundary tests, design decisions, compatibility notes. |
| `close_impact` | Missing required review keeps `design_gate=pending` or `partial`; public interface or compatibility risk without review can block close or require user acceptance of residual risk. |

### Codebase Stewardship

| Field | Contract |
|---|---|
| `name` | `codebase_stewardship` |
| `applies_when` | Work touches durable code structure, domain concepts, module ownership, interface contracts, architecture direction, deep-module boundaries, testing strategy, or cross-cutting exceptions. |
| `default_requirement` | Group the stewardship view for the Change Unit: domain language, module map, interface contracts, TDD/feedback loops, architecture watchpoints, and deep-module boundaries. Stewardship review is not a general code review checklist. It prevents local task completion from hiding degradation in domain language, module boundary, interface contract, feedback loop, testability, maintainability, or future-change cost. Use owner records as source of truth, record only task-relevant refs, and create reconcile items for drift instead of duplicating schemas or DDL. |
| `allowed_waiver` | Allowed for isolated docs, comments, formatting, or leaf edits with no durable structure, domain, interface, or feedback-loop impact. Waiver must record why stewardship review is unnecessary. |
| `required_record` | Task or Change Unit stewardship refs, `domain_terms`, `module_map_items`, `interface_contracts`, feedback loop or `tdd_traces` refs, decision records, Task/Change Unit watchpoints, Journey Spine Entry refs, and reconcile items for drift. Dedicated architecture watchpoint refs may be used only if a later DDL batch defines them. |
| `validator` | `codebase_stewardship_check` |
| `evidence` | Domain language refs, module map refs, interface contract refs, feedback loop refs, TDD trace refs when used, Task/Change Unit watchpoints, Journey Spine Entry refs, deep-module notes, reconcile item refs, and dedicated architecture watchpoint refs only if later defined. |
| `close_impact` | Missing required stewardship review keeps `design_gate=pending`, `partial`, or `stale`; unresolved drift can block close when it affects public behavior, module boundaries, acceptance criteria, or verification confidence. |

### StewardshipImpactSummary Display Shape

`StewardshipImpactSummary` is a derived display/summary shape for the Design Stewardship Default and the `codebase_stewardship` policy contract. It is not a Kernel Authority Invariant. It is a derived display, not a canonical current record. It is derived from owner records, validator results, and refs; it does not create a new canonical source of truth.

Domain terms, module map items, interface contracts, feedback loop/TDD records, residual risk, and Decision Packets remain the owner records. The summary renders compact close-relevant status and refs back to those owners.

| Field | Values |
|---|---|
| `domain_language_impact` | `none` \| `updated` \| `conflict` \| `unresolved` |
| `module_boundary_impact` | `none` \| `local` \| `public_boundary` \| `unresolved` |
| `interface_contract_impact` | `none` \| `compatible` \| `breaking` \| `unresolved` |
| `feedback_loop_status` | `defined` \| `missing` \| `waived` |
| `future_change_risk` | `none` \| `visible` \| `accepted` \| `unresolved` |
| `close_impact` | `none` \| `blocks_close` \| `requires_decision` \| `residual_risk` |

### Manual QA

| Field | Contract |
|---|---|
| `name` | `manual_qa` |
| `applies_when` | UI change, UX flow change, copy/error message change, onboarding/checkout/auth/billing or other critical flow, accessibility impact, visual output, browser-only behavior, or any result that needs product taste judgment. |
| `default_requirement` | Record a Manual QA profile, setup, checklist, result, findings, evidence refs, performer, product taste judgment when relevant, and next action. Profiles include `ui_quality`, `workflow`, `copy`, `accessibility`, `browser_smoke`, and `performance_smoke`. |
| `allowed_waiver` | Allowed when the user/operator explicitly waives QA and records a waiver reason. QA waiver requires decision quality when accepting known product or user risk. Not appropriate when legal, safety, privacy, or high-impact user harm requires inspection. |
| `required_record` | `manual_qa_records`; `qa_gate` is the canonical aggregate gate. |
| `validator` | `manual_qa_required` |
| `evidence` | Manual QA record, screenshots, notes, browser logs, walkthrough refs, finding refs. |
| `close_impact` | If Manual QA is required, `qa_gate=pending` or `failed` blocks successful close. `qa_gate=waived` requires a waiver reason. QA failed should create rework, block close, or require an explicit follow-up path. |

### Context Hygiene

| Field | Contract |
|---|---|
| `name` | `context_hygiene` |
| `applies_when` | Work resumes after interruption, old PRDs/design docs/issues exist, code paths have moved, acceptance criteria changed, module/interface/domain records changed, or evaluator/reviewer needs a focused bundle. |
| `default_requirement` | Push current Task summary, Journey Card and relevant Journey Spine refs, latest run/eval/evidence refs, relevant policy refs, and current acceptance criteria. Pull stale PRDs, closed issues, old design docs, coding standards, and long logs only when needed as pull-only references. Mark stale docs and avoid treating chat as state. |
| `allowed_waiver` | Allowed for short advisor-only work where no product state, design state, or evidence state is being changed. |
| `required_record` | Task summary, projection freshness, reconcile items for drift, evidence manifest, and validator results. |
| `validator` | `context_hygiene_check` |
| `evidence` | Current projection refs, freshness state, stale refs, reconcile item refs, bundle contents for evaluator. |
| `close_impact` | Stale critical context may mark `design_gate=stale`, evidence stale, or projection stale. It can block write or close when the agent cannot safely determine scope, evidence, or current acceptance criteria. |

## Waiver Rules

Waivers must be explicit, scoped, and recorded. A waiver should include:

- policy name
- task and Change Unit
- reason
- accepted risk
- actor who waived
- expiry or follow-up when needed
- affected gate or close impact

Policy waivers can satisfy a design-quality requirement only where the policy contract allows it. They do not waive scope for product writes, sensitive-change approval, required evidence coverage, or required acceptance. Verification waivers are owned by the kernel close semantics and must not produce `assurance_level=detached_verified`.

Waivers that involve verification, QA, public API/interface commitment, scope expansion, architecture direction, or acceptance with known risk should also satisfy `decision_quality` and respect any active `autonomy_boundary`.

## Policy-To-Validator Mapping

| Policy | Validator | Primary gate or state impact |
|---|---|---|
| `shared_design` | `shared_design_alignment` | `design_gate` pending/partial/passed/waived |
| `decision_quality` | `decision_quality_check` | `decision_gate` required/pending/blocked/passed; `design_gate` where applicable |
| `autonomy_boundary` | `autonomy_boundary_check` | `prepare_write` blockers, `decision_gate`, `design_gate` |
| `domain_language` | `domain_language_consistency` | `design_gate` partial/stale/passed |
| `vertical_slice` | `vertical_slice_shape` | `design_gate` partial/blocked/passed |
| `feedback_loop` | `feedback_loop_check` | `design_gate` and evidence sufficiency |
| `tdd_trace` | `tdd_trace_required` | `design_gate` and evidence sufficiency |
| `deep_module_interface` | `module_interface_review` | `design_gate` partial/blocked/passed |
| `codebase_stewardship` | `codebase_stewardship_check` | `design_gate` pending/partial/stale/passed and close blockers |
| `manual_qa` | `manual_qa_required` | `qa_gate` pending/passed/failed/waived |
| `context_hygiene` | `context_hygiene_check` | projection freshness, reconcile, evidence/design stale |

The reference MVP may implement minimal validators first, but it should keep validator IDs stable so conformance fixtures can grow without changing policy names.
