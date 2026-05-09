# Kernel Spec

## Document Role

This document owns the operational kernel specification for the harness. It defines the entity model, lifecycle model, gates, state compatibility rules, transition table, close semantics, waiver semantics, `prepare_write` state logic, `close_task` state logic, and invariant enforcement mapping.

This document does not define MCP wire schemas, SQLite DDL, projection template text, design-quality playbook procedures, connector capability schemas, or capability as a first-class kernel gate.

## Kernel Scope

The kernel is the canonical state machine for local AI-assisted product work. It decides:

- which Task is active
- which Change Unit scopes product writes
- whether a write may proceed
- which decisions, approvals, evidence, verification, QA, and acceptance gates apply
- whether a task may close
- which state events are appended
- which projections need refresh

Operational state is canonical in `state.sqlite` current records plus `state.sqlite.task_events`.
Raw evidence is canonical in the artifact store. Markdown reports are projections generated from state records and artifact refs. Human-editable sections are input surfaces.

The kernel records references to raw evidence and projections, but neither chat text nor generated Markdown replaces canonical state.

## Work Modes

`advisor` is for read-only explanation, comparison, review, and decision support. It does not authorize product writes. Advisor tasks usually close with `result=advice_only`; evidence, verification, QA, and acceptance gates are normally not required unless policy or the user explicitly requires them.

`direct` is for small, low-risk product changes whose scope and result are obvious. It is write-capable, so product writes still require an active scoped Change Unit. Direct work may close as `self_checked` by default. If optional detached verification is performed and passes with a valid independence qualifier, direct work may be marked `detached_verified`.

`work` is for structured implementation, non-local change, riskier change, or work that needs independent verification. It is write-capable, requires an active scoped Change Unit before product writes, and cannot be marked `detached_verified` by same-session self-review.

### Direct Fast Path

Direct still needs an active scoped Change Unit before product writes. For small obvious requests, the Change Unit may be minimal and derived from the user's request, as long as it records the intended operation and scoped write surface clearly enough for `prepare_write` and `record_run` compatibility checks.

No Decision Packet is created unless blocking product judgment is detected. Evidence can be lightweight according to the applicable evidence profile, such as a changed path list, patch summary or diff artifact, command result when relevant, and self-check summary.

Manual QA, detached verification, and residual-risk acceptance are not required for direct work unless policy, changed surface, user request, or detected risk requires them. If scope, risk, affected interface, or evidence expectations grow beyond the direct assumptions, the same Task escalates to `work` rather than continuing as direct.

## Entity Model

### Task

A Task is the user value unit. It carries the current mode, lifecycle phase, result, close reason, assurance level, gate states, current summary, acceptance criteria, Decision Packet references, Residual Risk references, active Change Unit, active Run, latest record references, optional Journey Spine Entry references, and projection freshness. A Task is the primary state record used by status, resume, and close decisions.

### Change Unit

A Change Unit is the scoped implementation unit for product writes. It records purpose, non-goals, slice type, intended end-to-end path, autonomy boundary, allowed paths, allowed tools, validator profile, sensitive categories, approval needs, evidence expectations, QA expectations, dependencies, merge risk, completion conditions, and evaluator focus.

Every product write requires an active Change Unit whose scope covers the intended write. A Task may have one or many Change Units, but only the active Change Unit scopes the current write. Core allows a specific write attempt through `prepare_write`, which creates a Write Authorization when the gates pass or returns the already committed response for idempotent replay of the same request.

### Autonomy Boundary

An Autonomy Boundary is part of Change Unit semantics. It records the product-judgment boundary inside which the agent may proceed without asking the user for another decision. It includes allowed latitude for goals, scope, design direction, trade-offs, codebase stewardship, residual risk, and implementation choices.

The Autonomy Boundary is not a scope grant. It does not authorize paths, tools, commands, network targets, secret access, or sensitive categories outside the active Change Unit. A Decision Packet may authorize updating the Autonomy Boundary or proposing a Change Unit update, but the resulting write still requires compatible Change Unit scope and granted approval when sensitive categories apply.

The Autonomy Boundary does not replace Change Unit scope, sensitive approval, policy checks, evidence, verification, QA, acceptance, or `prepare_write`. If an intended operation exceeds the active Autonomy Boundary, the kernel blocks the operation and requests a user decision through a Decision Packet when product judgment can resolve it.

### Decision Packet

A Decision Packet is the canonical state entity for blocking product judgment. It records the decision needed, options, recommendation when available, trade-offs, affected scope, supporting evidence, residual risk, owner, status, and next action.

Decision Packets feed `decision_gate`. A blocking product judgment cannot be satisfied by chat text, broad approval, or projection prose alone. The recorded Decision Packet and its resolution, deferral, or blocked status are the kernel authority path for that judgment.

Minimal MVP implementations may omit `decision_requests`. If an implementation keeps them, they are routing, interaction, replay, or legacy handoff metadata only. They are not authority for product judgment, and a `decision_request` row alone never satisfies `decision_gate`, approval, acceptance, waiver, residual-risk acceptance, or close.

Decision Packet status is record-level:

```text
proposed | pending_user | resolved | deferred | rejected | blocked | superseded
```

- `proposed` means the packet has been drafted or detected but is not yet the active user request.
- `pending_user` means the packet is waiting for the user's product judgment.
- `resolved` means the user decision or accepted state decision is recorded and compatible with its affected scope.
- `deferred` means the user intentionally deferred the decision and the packet records close impact, residual risk, and follow-up visibility where relevant.
- `rejected` means the packet or proposed decision path was rejected.
- `blocked` means the packet cannot currently be resolved or deferred under the present state.
- `superseded` means another Decision Packet, Change Unit, or Task state replaces it.

### Journey Spine

A Journey Spine is the state-derived continuity model for the ordered work journey of a Task. It is reconstructed from Task, Change Unit, Run, Decision Packet, Approval, Evidence Manifest, Eval, Manual QA, Residual Risk, `task_gates.acceptance_gate`, acceptance Decision Packet user-decision state, close events, artifact references, and `state.sqlite.task_events`.

Journey Spine is not a separate source of truth. Journey Card and Journey Spine Markdown views are projections. They help humans resume and inspect work, but they do not override Task state, gate fields, Decision Packets, Evidence Manifests, Residual Risk records, artifact records, or `state.sqlite.task_events`.

### Journey Spine Entry

A Journey Spine Entry is the canonical support record for durable continuity annotations that cannot be fully reconstructed from existing state events or source records. It may record an annotation kind, ordering relationship, source refs, affected scope, summary, actor, time, and artifact refs.

Journey Spine Entry records supplement reconstruction; they do not replace the owner records for Task state, Change Units, Runs, Decision Packets, Residual Risk, evidence, verification, QA, acceptance gate/decision state, close state/events, or artifacts.

### Run

A Run is an execution attempt by a lead agent, evaluator, operator, or other actor. It records actor identity, surface identity, mode, Change Unit, baseline, intended operation, observed changes, command results, artifact references, and summary. A lead Run may shape or implement. An evaluator Run verifies from a separate verification boundary and is not allowed to become detached verification unless its independence qualifier is valid.

Implementation and direct Runs must consume a compatible, unexpired, unconsumed Write Authorization unless the Run is read-only or shaping-only. The consumed authorization links the Run back to the `prepare_write` decision that allowed the write attempt.

### Approval

An Approval is a scope-bound prior decision for sensitive change. It records what was approved: paths, tools, commands or command classes, network targets, secret scope, baseline, sensitive categories, expiry conditions, and user decision. Approval authorizes sensitive categories inside defined scope. It does not prove correctness, replace evidence, satisfy QA, imply acceptance, or provide the authority path for product judgment.

If a sensitive action also includes a product trade-off, architecture choice, QA waiver, verification risk, acceptance, residual-risk acceptance, or public interface commitment, the Approval record may authorize the sensitive category only. The product judgment still requires a compatible Decision Packet.

### Write Authorization

A Write Authorization is the durable state record created when `prepare_write` allows a product write.

It records the Task, active Change Unit, `basis_state_version`, intended operation, intended paths, intended tools, intended commands, intended network targets, intended secret access, sensitive categories, baseline, approval refs, relevant Decision Packet refs, guarantee level, status, created time, and consumption by a Run.

`basis_state_version` is the affected-scope state version Core used as the compatibility basis for the allowed write attempt after stale-state checks and before creating the authorization. For MVP Write Authorizations this is the Task State Version for the authorization's Task. It supports idempotent replay audit, stale detection, and explaining why older unconsumed authorizations became stale, expired, or revoked.

A Write Authorization is not scope by itself. It is evidence that Core allowed a specific write attempt under the active scope and gates.

A Write Authorization does not replace approval, evidence, verification, QA, acceptance, or residual-risk visibility.

`authorization_effect=returned` is reserved for idempotent replay of the same committed `prepare_write` request with the same idempotency key, request hash, and `basis_state_version`, or for returning the already committed response. A distinct compatible `prepare_write` request creates a distinct Write Authorization; compatibility does not make authorizations reusable. Core may stale, expire, or revoke older unconsumed authorizations if their compatibility basis changes.

Write Authorization status is record-level:

```text
allowed | consumed | expired | stale | revoked
```

- `allowed` means `prepare_write` allowed the write attempt and the authorization is unconsumed, unexpired, and not stale or revoked.
- `consumed` means one committed implementation or direct `record_run` has used the authorization. A Write Authorization is single-use except for idempotent replay of the same committed `record_run` request.
- `expired` means the authorization's time, baseline, state-version, or other expiry condition passed before consumption.
- `stale` means later state changed the compatibility basis, such as active Change Unit scope, baseline, approval, relevant Decision Packet, sensitive category, or guarantee level.
- `revoked` means Core, policy, or an explicit user decision withdrew the authorization before consumption.

### Evidence Manifest

An Evidence Manifest maps acceptance criteria or completion conditions to evidence references. It records whether each criterion is supported, unsupported, or not applicable, and it references durable artifacts, run summaries, Eval records, TDD traces, Manual QA records, or other recorded evidence. Evidence sufficiency is judged from this manifest and related records.

### Eval

An Eval is a verification result record. It records the verification target, verdict, checks performed, evidence reviewed, independence qualifier, baseline relationship, blockers, and artifact references. An Eval verdict alone does not upgrade assurance. `assurance_level=detached_verified` requires a passed verification result, a valid independence qualifier, and no same-session self-review violation.

### Manual QA

Manual QA is a human inspection record for UX, workflow, copy, accessibility, visual output, product taste, or any other result that needs human judgment. `manual_qa_record.result` is the record-level result of an actual Manual QA record and is limited to `passed`, `failed`, or `waived`. Pending required QA is not represented as a Manual QA record result; it is represented by the aggregate `qa_gate=pending`.

### Residual Risk

Residual Risk is a canonical close-relevant support record for known remaining uncertainty, trade-off, limitation, or unchecked condition. It records source refs, affected scope, related Decision Packet when applicable, visibility status, accepted risk when applicable, follow-up requirement, and close impact.

Residual Risk records make remaining risk visible before acceptance or risk-accepted close. They do not create detached verification, replace evidence, waive QA, grant sensitive approval, or imply final acceptance.

Accepted risk is not a separate canonical state record in MVP. Risk acceptance updates accepted-risk metadata/status on the relevant Residual Risk record and may append residual-risk acceptance events. Any public accepted-risk ref field that remains in an API or projection must point to a `StateRecordRef` with `record_kind=residual_risk`, not to an `accepted_risk` or `ARISK-*` record.

### Artifact

An Artifact is a durable evidence file in the artifact store, such as a diff, log, bundle, manifest, screenshot, checkpoint, or exported bundle component. Artifact records identify and verify these files by reference and integrity metadata. Raw artifacts are distinct from Markdown reports and state records.

### Reconcile Item

A Reconcile Item is the canonical candidate record created when human-editable content or generated projection drift may need to affect state. Reconcile decisions may merge, reject, convert to note, create a decision, or defer the item. Human-editable text is input; accepted state changes occur only through reconcile action and state events.

### Design Support Records

The kernel also owns the entity meaning for design support records:

- Shared Design records capture goals, scope, assumptions, rejected options, acceptance criteria, and decisions.
- Domain Term records are the canonical source for Domain Language.
- Module Map Item records are the canonical source for Module Map.
- Interface Contract records are the canonical source for Interface Contract.
- TDD Trace records capture red, green, refactor evidence or a recorded non-TDD justification.

Their policy requirements are owned by the design-quality policy pack. Their storage DDL is owned by the reference MVP document.

## Authority Rules

User Notes authority is:

```text
human-editable input -> reconcile_items -> accepted state event/record
```

Domain Language canonical source is `domain_terms`.

Module Map canonical source is `module_map_items`.

Interface Contract canonical source is `interface_contracts`.

The `DOMAIN-LANGUAGE`, `MODULE-MAP`, and `INTERFACE-CONTRACT` Markdown documents are projections and proposal surfaces. They do not override their canonical records.

Decision Packet and Residual Risk canonical source is kernel state. Decision Packet and residual-risk Markdown views are projections or proposal surfaces.

Journey Spine is derived from kernel state, registered artifact references, and `state.sqlite.task_events`. Journey Spine Entry canonical source is kernel state when durable continuity annotations are needed. Journey Cards and Journey Spine Markdown views are projections and cannot repair, close, or mutate state by themselves.

Approval and Decision Packet authority are separate. Approval authorizes sensitive categories inside defined scope; it is not the authority path for product judgment. If a sensitive action also includes a product trade-off, architecture choice, QA waiver, verification risk, acceptance, residual-risk acceptance, or public interface commitment, the Approval may authorize only the sensitive category. The product judgment still requires a compatible Decision Packet.

## Lifecycle Model

The kernel uses lifecycle fields plus gates. Compact display states are derived from these canonical fields.

### Mode

```text
advisor | direct | work
```

### Lifecycle Phase

```text
intake | shaping | ready | executing | verifying | qa |
waiting_user | blocked | completed | cancelled
```

### Result

```text
none | advice_only | passed | failed | cancelled
```

### Close Reason

```text
none | completed_verified | completed_self_checked |
completed_with_risk_accepted | cancelled | superseded
```

### Assurance Level

```text
none | self_checked | detached_verified
```

Assurance is not approval, QA, or acceptance. It summarizes the technical checking level supported by runs, evidence, Eval records, and verification independence.

## Gate Model

Gates are canonical kernel fields used by `prepare_write`, `close_task`, status display, and conformance fixtures.

### Scope Gate

```text
not_required | required | pending | passed | failed | blocked
```

`scope_gate` applies to all write-capable product work. Advisor-only tasks normally use `not_required`. Direct and work product writes require a scoped Change Unit and a passed scope gate before writing.

### Decision Gate

```text
not_required | required | pending | resolved | deferred | blocked
```

`decision_gate` records whether product judgment blocks progress, write, or close. It is the aggregate Task gate fed by blocking Decision Packets.

- `not_required` means no blocking product judgment currently applies.
- `required` means a blocking product judgment has been detected and a Decision Packet must be recorded or associated.
- `pending` means a blocking Decision Packet exists and is awaiting the user's decision.
- `resolved` means all blocking Decision Packets relevant to the current operation have recorded compatible decisions.
- `deferred` means the user recorded a deferral. It is compatible only when the affected operation can proceed without resolving that judgment now, or when residual risk and follow-up visibility are recorded.
- `blocked` means product judgment remains blocking and cannot proceed under the current state.

`decision_gate` does not replace scope confirmation, sensitive approval, design policy, evidence, verification, Manual QA, acceptance, or residual-risk acceptance.

#### Decision Gate Aggregate Recompute

`decision_gate` is recomputed from relevant blocking Decision Packets plus currently detected blocking product-judgment needs. Relevant means the packet or detected blocker applies to the active Task, active Change Unit, requested operation, close intent, baseline, or affected scope. The recompute path reads `decision_packets` and detected blockers; it must not read `decision_requests` except through a linked compatible `decision_packet_id`.

Recompute precedence is:

1. `blocked` when any relevant blocking Decision Packet is `blocked`, is `rejected` without a compatible replacement, or is incompatible with the active Change Unit, Autonomy Boundary, baseline, intended operation, or close intent.
2. `pending` when any relevant blocking Decision Packet is `pending_user` and no higher-precedence blocked condition exists.
3. `required` when blocking product judgment is detected but no relevant Decision Packet exists, or only `proposed` packet drafts exist.
4. `deferred` when all relevant blocking Decision Packets are `deferred`, the deferral explicitly covers the current operation or close intent, and residual risk or follow-up visibility is recorded where relevant.
5. `resolved` when all relevant blocking Decision Packets are `resolved`, or `superseded` by compatible replacement state, and no unresolved detected blocker remains.
6. `not_required` when no blocking product judgment applies to the current operation or close intent.

A stored `decision_gate` value that disagrees with recomputation is stale state and must be repaired before write or close decisions rely on it.

### Approval Gate

```text
not_required | required | pending | granted | denied | expired
```

`approval_gate` is required only when sensitive categories are present. A display layer may show `passed` as an alias for `granted` when no approval drift exists, but the canonical value remains `granted`.

- `approval_gate=not_required` means no sensitive category currently requires approval.
- `approval_gate=required` means sensitive approval is needed, but no committed approval-shaped Decision Packet and linked pending Approval record exists yet. This is the state `prepare_write` reaches when it detects missing sensitive approval.
- `approval_gate=pending` means `harness.request_user_decision(decision_kind=approval)` has created the committed approval-shaped Decision Packet and linked pending Approval record, and the system is awaiting a user/operator decision.
- `approval_gate=granted` means a compatible Approval record covers the sensitive scope. It is not a Write Authorization and does not authorize product judgment; the write path must still pass a fresh compatible `prepare_write` decision before `record_run` can consume an authorization.
- `approval_gate=denied` means the linked Approval record was denied and the sensitive write remains blocked.
- `approval_gate=expired` means the linked Approval record expired, drifted, or no longer covers the current baseline or intended sensitive scope.

### Design Gate

```text
not_required | required | pending | passed | partial | waived | stale | blocked
```

`design_gate` reflects required design-quality preconditions. Policy determines when it applies and when a waiver is allowed.

### Evidence Gate

```text
not_required | none | partial | sufficient | stale | blocked
```

`evidence_gate=not_required` means evidence gate does not apply.

`evidence_gate=none` means evidence is required but no evidence has been recorded.

Where evidence is required, a successful completion requires `evidence_gate=sufficient`.

### Evidence Sufficiency Profiles

Evidence sufficiency is judged from the Evidence Manifest plus related state records and artifact refs. It must not be judged from chat text or report prose alone. A status card or Markdown report may summarize why evidence is missing, but the close decision uses the manifest, Task, gates, Change Units, Runs, approvals, Evals, Manual QA records, baseline relation, and registered artifacts.

| Evidence Profile | Minimum sufficiency guidance |
|---|---|
| `advisor` | `evidence_gate` is usually `not_required` unless the user or policy asks for a recorded decision, review bundle, or exportable artifact. |
| `direct docs-only` | Sufficient evidence may be changed path list, diff artifact or recorded patch summary, and self-check summary. |
| `direct code` | Sufficient evidence may be changed path list, diff artifact, relevant command/test/log artifact or explicit reason no automated check applies, and self-check summary. |
| `work feature` | Sufficient evidence requires acceptance-criteria-to-evidence mapping, changed file coverage, run summary, diff/log/test/build artifacts as applicable, and `evidence_manifest.status=sufficient`. |
| `UI/UX/copy work` | Requires `work feature` evidence plus Manual QA record or valid QA waiver when QA is required. |
| `sensitive work` | Requires normal task evidence plus approval ref, approval scope compatibility, baseline relation, and no approval drift. |
| `verification-required work` | Requires Evidence Manifest plus Eval record with reviewed evidence and valid independence if the task is to close as `completed_verified`. |

Close impact:

- Required evidence absent means `evidence_gate=none`.
- Required evidence incomplete means `evidence_gate=partial`.
- Evidence invalidated by baseline, changed files, approval drift, missing artifact, or relevant design record change means `evidence_gate=stale` or `blocked`.
- Successful close where evidence is required needs `evidence_gate=sufficient`.
- `evidence_gate=not_required` must not be used when evidence is required but missing.

Examples:

- Direct typo fix: changed path `docs/help.md`, diff artifact or patch summary, and self-check summary can support `direct docs-only` evidence.
- Work feature: AC-01 maps to passing test log and changed path coverage; AC-02 maps to build log plus run summary; the Evidence Manifest records both as supported.
- UI copy change: changed copy path, diff artifact, self-check, and required Manual QA record support close; until Manual QA is recorded or validly waived, close remains blocked.

### Verification Gate

```text
not_required | required | pending | passed | failed | waived_by_user | blocked
```

`verification_gate=waived_by_user` records that the user accepted remaining verification risk. It must not become `assurance_level=detached_verified`.

### Verification Independence Profiles

Verification independence profiles describe the minimum qualification needed before an Eval can support detached assurance.

| Profile | Minimum qualification |
|---|---|
| `same_session` | Not detached. May record self-check or review notes. Must not produce `detached_verified`. |
| `subagent_context` | Not detached by default. May qualify only if the implementation context, Write Authorization context, and reviewed bundle satisfy a stricter profile; otherwise treat as not detached. |
| `fresh_session` | Detached candidate if the evaluator receives a task/evidence bundle rather than continuing lead chat context, reviews the Evidence Manifest and changed files, and records an Eval. |
| `fresh_worktree` | Detached candidate if the evaluator checks baseline, changed paths, artifacts, and Evidence Manifest in a separate worktree or equivalent isolated repository state. |
| `sandbox` | Detached or isolated candidate if execution and verification happen across a meaningful process/filesystem boundary and artifacts are captured. |
| `manual_bundle` | Detached candidate if the evaluator receives task summary, acceptance criteria, Change Unit scope, approval scope, diff/log/test artifacts, Evidence Manifest, known risks, and records a verdict. |

Rules:

- Eval verdict alone does not upgrade assurance.
- Valid independence plus passed verification plus absence of a same-session self-review violation is required for `assurance_level=detached_verified`.
- User verification waiver must close as `completed_with_risk_accepted`, not `completed_verified`.
- A verifier that can write product files must disclose that in Eval independence context; write capability may reduce confidence and may require an additional guard or bundle review.

### QA Gate

```text
not_required | required | pending | passed | failed | waived
```

`qa_gate` is the canonical kernel gate for required human QA. Individual Manual QA records have record-level results; the gate is the aggregate close-relevant state. `qa_gate=pending` means required QA has not yet produced a satisfying Manual QA record, or the latest relevant Manual QA record does not satisfy policy. It does not mean `manual_qa_record.result=pending`.

### Acceptance Gate

```text
not_required | required | pending | accepted | rejected
```

`acceptance_gate` records the user's final acceptance judgment where acceptance is required. It does not replace QA or verification.

MVP final acceptance is stored through the canonical Decision Packet user-decision path, the Task's `acceptance_gate`, and `state.sqlite.task_events`. The kernel does not define a separate Acceptance state record for MVP.

Residual-risk visibility is satisfied in either of two ways. If no known close-relevant Residual Risk exists, the current judgment context reports `ResidualRiskSummary.status=none`. If known close-relevant Residual Risk exists, that risk must be visible in the current judgment context before any successful close. Acceptance, when required, can be recorded only after close-relevant residual risk is visible or confirmed as `ResidualRiskSummary.status=none`. A risk-accepted close additionally requires visible and accepted Residual Risk refs, and residual-risk acceptance never upgrades assurance to `detached_verified`. `ResidualRiskSummary.status=none` must not hide or replace known close-relevant risk.

### Capability Boundary

Capability is deliberately excluded from the kernel gate enum.

Surface capability belongs to:

- the `surface_capability_check` validator
- `prepare_write` blocked reasons
- guarantee level display

Capability can affect whether the kernel allows a write, how strongly it can enforce the rule, and what warning is shown, but it is not a first-class lifecycle gate.

## Compatibility Matrix

### Mode Compatibility

| Mode | Product write eligible | Change Unit required for write | Default close assurance | Detached verification |
|---|---:|---:|---|---|
| `advisor` | no | no | `none` | not required |
| `direct` | yes | yes | `self_checked` | optional |
| `work` | yes | yes | `none` until checked | required unless user accepts verification risk |

### Decision Gate Compatibility

| `decision_gate` | Proceed/write compatibility | Close compatibility |
|---|---|---|
| `not_required` | compatible when no product judgment blocker applies | compatible when no blocking Decision Packet applies |
| `required` | blocks until a Decision Packet is recorded or associated | blocks completion |
| `pending` | blocks affected operations until the user decision is recorded | blocks completion when the pending Decision Packet is blocking |
| `resolved` | compatible when the recorded decision covers the active Change Unit, Autonomy Boundary, baseline, and intended operation | compatible when all blocking Decision Packets relevant to close are resolved |
| `deferred` | compatible only if the deferral explicitly permits the intended operation now; otherwise blocks | compatible only when deferral is non-blocking for close and residual risk or follow-up visibility is recorded |
| `blocked` | blocks until state, scope, policy, or user decision changes | blocks completion |

### Completion Compatibility

All successful close paths require residual-risk visibility before close. If no known close-relevant Residual Risk exists, `ResidualRiskSummary.status=none` satisfies this check. If known close-relevant Residual Risk exists, it must be visible in the current judgment context. Risk-accepted close additionally requires accepted Residual Risk refs.

| Close path | Required compatible state |
|---|---|
| Advisor completed | no active Run; no product write pending; no blocking unresolved Decision Packet; `result=advice_only`; `close_reason=completed_self_checked` |
| Direct self-checked | no active Run; active Change Unit completed or not needed for non-write direct; scope passed for writes; blocking Decision Packets resolved or validly deferred for close; required approval granted; required evidence sufficient; `assurance_level=self_checked`; `close_reason=completed_self_checked` |
| Direct verified | direct self-checked requirements plus valid passed detached verification; `assurance_level=detached_verified`; `close_reason=completed_verified` |
| Work verified | no active Run; Change Unit complete or explicitly deferred; scope passed; blocking Decision Packets resolved; approval not required or granted; design passed or waived; evidence sufficient; verification passed with valid independence; QA passed or waived if required; residual risk visible or confirmed as `ResidualRiskSummary.status=none` before acceptance; acceptance accepted if required; `close_reason=completed_verified` |
| Work risk accepted | work close requirements for scope, approval, design, evidence, QA, and acceptance are satisfied; verification may be `waived_by_user`; blocking decisions are resolved or validly deferred with residual risk visibility; visible and accepted Residual Risk refs are recorded; assurance must be `none` or `self_checked`; `close_reason=completed_with_risk_accepted` |
| Cancelled | no active write in progress; `result=cancelled`; `close_reason=cancelled` or `superseded` |

### Invalid State Combinations

The following combinations are invalid and must be rejected or repaired by the kernel:

| Invalid combination | Required handling |
|---|---|
| `lifecycle_phase=completed` with `active_run_id` present | block close until the Run is recorded, interrupted, or cancelled |
| `lifecycle_phase=completed` with `result=none` | reject state transition |
| `lifecycle_phase=completed` with `close_reason=none` | reject state transition |
| `lifecycle_phase=cancelled` with `result` other than `cancelled` | reject state transition |
| Product write attempted with no active Task | block `prepare_write` |
| Product write attempted in `advisor` mode | block `prepare_write` |
| Product write attempted with no active Change Unit | block `prepare_write` |
| Product write attempted when `scope_gate` is not `passed` | block or request scope confirmation |
| Intended operation exceeds the active Change Unit Autonomy Boundary | block `prepare_write`; request user decision when product judgment can resolve it |
| Implementation or direct Run recorded with no compatible unexpired, unconsumed Write Authorization | reject `record_run` or record a violation/audit Run without populating `runs.write_authorization_id`; evidence, verification, QA, acceptance, and close cannot rely on it |
| Run attempted with an invalid, stale, missing, consumed, or scope-exceeded Write Authorization | do not record it as consumed; record the attempted authorization ref in validator findings, run violation payload, or `task_events.payload_json` when useful; mark scope, evidence, approval, verification, and projections stale or blocked as appropriate; the authorization remains unconsumed and may be stale, revoked, or expired |
| Blocking product judgment detected with `decision_gate=not_required` | repair to `required` and request a Decision Packet |
| `decision_gate=pending`, `resolved`, `deferred`, or product-judgment `blocked` without a linked Decision Packet | reject or repair by associating the canonical Decision Packet |
| Product write attempted with required blocking Decision Packet absent or unresolved | block `prepare_write`; return a Decision Packet request or candidate rather than broad approval |
| Approval used as the authority path for product judgment, whether or not the sensitive scope matches | reject or repair by requiring a compatible Decision Packet |
| `decision_gate=deferred` used for an operation not covered by the deferral | block `prepare_write` or close |
| `decision_gate=resolved` where the recorded decision no longer matches the active Change Unit, Autonomy Boundary, baseline, or intended operation | repair to `required`, `pending`, or `blocked` |
| Stored `decision_gate` differs from aggregate recomputation | recompute and repair before write or close |
| Sensitive change with `approval_gate=not_required` | repair to `approval_gate=required` and block `prepare_write`; do not create Approval, Decision Packet, Write Authorization, or `APR` until `request_user_decision(decision_kind=approval)` commits the approval request |
| Sensitive change with approval denied, expired, or outside approved scope | block `prepare_write` |
| Required evidence with `evidence_gate=not_required` | repair to `none`, `partial`, `sufficient`, `stale`, or `blocked` |
| `evidence_gate=none` while evidence records support required criteria | recompute evidence gate |
| Completed passed result where required evidence is `none`, `partial`, `stale`, or `blocked` | block close |
| `verification_gate=waived_by_user` with `assurance_level=detached_verified` | reject state transition |
| Same-session review producing `assurance_level=detached_verified` | reject assurance upgrade |
| Eval verdict passed without valid independence producing `detached_verified` | reject assurance upgrade |
| `qa_gate=waived` without waiver reason | reject waiver |
| Completed passed result with required `qa_gate=pending` or `failed` | block close |
| Completed passed result with required `acceptance_gate=pending` or `rejected` | block close |
| Completed passed result with blocking Decision Packet unresolved or incompatible with close intent | block close |
| Acceptance or risk-accepted close recorded while known close-relevant residual risk is hidden, unrecorded, or absent from the current judgment context | reject close until residual risk is visible, or until no known close-relevant risk is confirmed with `ResidualRiskSummary.status=none`; risk-accepted close still requires accepted Residual Risk refs |
| Projection stale or failed recorded as state failure by itself | repair display/projection status; do not change result solely for projection freshness |
| A Markdown projection used as canonical state | create reconcile item or reject as state mutation |
| A capability field introduced as a canonical lifecycle gate | reject schema/state mutation |

### Close Eligibility

`close_ready` is not a `lifecycle_phase`. It is a derived condition meaning that the Task has no open Run and all close-relevant required gates are compatible with the requested close intent. Only `close_task` moves a Task to `lifecycle_phase=completed`.

## Transition Table

State transitions append an event to `state.sqlite.task_events` in the same transaction as current state changes.

Each transition increments the correct affected-scope clock. Task-scoped transitions increment the Task State Version; project-level transitions with no Task increment the Project State Version. The appended event carries the resulting version for that affected scope.

Event ordering is the deterministic append order recorded by storage. Timestamps are audit metadata only and must not define the order used for Journey reconstruction, API event lists, or conformance `expected_events`.

Write Authorization lifecycle events use this kernel event vocabulary:

```text
write_authorization_created
write_authorization_returned
write_authorization_consumed
write_authorization_expired
write_authorization_staled
write_authorization_revoked
write_authorization_violation_detected
```

`scope_violation_detected` is a general observed scope event, not a Write Authorization lifecycle event.

### Stable Event Catalog

Stable event names are the `event_type` values that MVP conformance fixtures may require in `expected_events`. Events remain rows in `state.sqlite.task_events`; this catalog does not introduce a separate event store, stream, or payload schema. A name outside this catalog may appear in prose, tool descriptions, fixture seed shorthand, validator/check names, or future extensions, but it is not a stable MVP conformance assertion unless this catalog promotes it. Fixtures should assert validator outcomes under `expected_state.validators` and projection freshness under `expected_projection` or `expected_state.checks`, not by inventing event names.

| Area | Stable event names |
|---|---|
| Write Authorization lifecycle | `write_authorization_created`, `write_authorization_returned`, `write_authorization_consumed`, `write_authorization_expired`, `write_authorization_staled`, `write_authorization_revoked`, `write_authorization_violation_detected` |
| `prepare_write` and write gates | `prepare_write_allowed`, `prepare_write_blocked`, `scope_required`, `decision_required`, `autonomy_boundary_exceeded`, `approval_required`, `baseline_stale_detected`, `capability_insufficient_detected` |
| Run, evidence, and scope observation | `run_recorded`, `evidence_manifest_updated`, `scope_violation_detected` |
| Verification | `eval_recorded`, `verification_passed`, `verify_not_detached_detected` |
| Close and risk-accepted close | `close_requested`, `close_blocked`, `risk_accepted_close_recorded`, `task_closed`, `task_cancelled`, `task_superseded` |
| Projection, connector, and reconcile operations | `projection_refresh_failed`, `generated_file_drift_detected`, `reconcile_item_created` |

The catalog is deliberately compact. Optional detail events, implementation-local audit events, and future extension events may still be recorded in `task_events`, but MVP fixture authors must not require them in `expected_events` until they are added here.

| Trigger | From | To | Gate or record effect |
|---|---|---|---|
| User request is accepted | no active Task | `lifecycle_phase=intake`, `result=none` | create Task |
| Request classified as advisor | `intake` | `mode=advisor`, `lifecycle_phase=executing` | product write disabled |
| Request classified as direct | `intake` | `mode=direct`, `lifecycle_phase=ready` | create or select scoped Change Unit if write is expected |
| Request classified as work | `intake` | `mode=work`, `lifecycle_phase=shaping` | design and scope shaping begins |
| Blocking product judgment detected | any non-terminal phase | `waiting_user` or `blocked` | `decision_gate=required`; Decision Packet must be recorded or associated |
| Decision Packet requested | any non-terminal phase | `waiting_user` | create or update Decision Packet; `decision_gate=pending` |
| User decision resolved | `waiting_user` | previous runnable phase, `shaping`, or `ready` | Decision Packet resolution recorded; `decision_gate=resolved`; affected Change Unit, Autonomy Boundary, gates, or residual-risk records updated |
| Decision deferred | `waiting_user` | previous runnable phase, `waiting_user`, or `blocked` | Decision Packet deferral recorded; `decision_gate=deferred`; residual risk or follow-up visibility recorded when relevant |
| Change Unit scope is confirmed | `shaping` or `waiting_user` | `ready` | `scope_gate=passed` |
| Scope is missing for intended write | any non-terminal phase | `waiting_user` or `blocked` | `scope_gate=pending` or `blocked` |
| Sensitive approval need detected by `prepare_write` | any non-terminal phase | `waiting_user` or `blocked` | `approval_gate=required`; approval-required blocker may be recorded; no Approval, Decision Packet, Write Authorization, or `APR` is created for the candidate |
| Approval request committed by `request_user_decision(decision_kind=approval)` | any non-terminal phase | `waiting_user` | create approval-shaped Decision Packet and linked pending Approval record; `approval_gate=pending` |
| Sensitive approval granted by `record_user_decision` | `waiting_user` | previous runnable phase | linked Approval record updated; `approval_gate=granted` |
| Sensitive approval denied by `record_user_decision` | `waiting_user` | `blocked` | linked Approval record updated; `approval_gate=denied` |
| Approval scope drifts or expires | any non-terminal phase | `waiting_user` or `blocked` | `approval_gate=expired` |
| Autonomy boundary violation | any non-terminal phase | `waiting_user` or `blocked` | violation recorded; Decision Packet requested when product judgment can resolve it; otherwise scope or policy blocker recorded |
| `prepare_write` allows write | `ready` or `executing` | `executing` | create Write Authorization, or return the already committed response for idempotent replay; active Run may proceed |
| `prepare_write` blocks write | any non-terminal phase | `waiting_user` or `blocked` | blocked reason recorded; `decision_gate`, `scope_gate`, or `approval_gate` updated according to blocker type |
| Direct implementation and self-check recorded | `executing` | same phase with close eligibility or `waiting_user` | Run consumes compatible Write Authorization; artifacts and evidence recorded |
| Work implementation recorded | `executing` | `verifying` | Run consumes compatible Write Authorization; evidence manifest updated |
| Evidence required but absent | `executing` or `verifying` | `blocked` | `evidence_gate=none` or `partial` |
| Evidence becomes stale | any non-terminal phase | `blocked` or current phase with stale gate | `evidence_gate=stale` |
| Verification launched | `verifying` | `verifying` | evaluator Run or bundle recorded |
| Eval passed with valid independence | `verifying` | `qa`, `waiting_user`, or same phase with close eligibility | `verification_gate=passed`; assurance may become `detached_verified` |
| Eval passed without valid independence | `verifying` | `verifying` or `blocked` | no detached assurance upgrade |
| Eval failed | `verifying` | `executing`, `shaping`, or `blocked` | `verification_gate=failed` |
| User accepts verification risk | `waiting_user` or `verifying` | same phase with close eligibility | `verification_gate=waived_by_user`; no detached assurance |
| Residual risk accepted | `waiting_user`, `verifying`, or `qa` | same phase with close eligibility or `waiting_user` | residual-risk acceptance recorded; related Decision Packet may resolve or defer; no detached assurance upgrade |
| Manual QA requested | any non-terminal phase | `qa` or `waiting_user` | `qa_gate=pending` |
| Manual QA passed | `qa` or `waiting_user` | same phase with close eligibility or `waiting_user` | `qa_gate=passed` |
| Manual QA failed | `qa` or `waiting_user` | `executing`, `shaping`, or `blocked` | `qa_gate=failed` |
| QA waiver accepted | `waiting_user` | same phase with close eligibility | `qa_gate=waived`; waiver reason required |
| Acceptance requested | any non-terminal phase with close eligibility | `waiting_user` | `acceptance_gate=pending` |
| Acceptance accepted | `waiting_user` | same phase with close eligibility | `acceptance_gate=accepted` |
| Acceptance rejected | `waiting_user` | `shaping`, `executing`, or `cancelled` | `acceptance_gate=rejected` |
| `close_task` succeeds | any non-terminal phase with close eligibility | `completed` | result and close reason assigned |
| User cancels Task | any non-terminal phase | `cancelled` | `result=cancelled`; `close_reason=cancelled` |
| Task is superseded | any non-terminal phase | `cancelled` | `result=cancelled`; `close_reason=superseded` |
| Projection refresh fails | any phase | same lifecycle phase | projection status marked stale or failed; state result unchanged |

## Waiver Semantics

Waivers are explicit user or policy decisions that must be recorded with reason, actor, time, and affected gate.

Allowed waivers:

- `design_gate=waived` when policy allows design-quality waiver.
- `verification_gate=waived_by_user` when the user accepts remaining verification risk.
- `qa_gate=waived` when required QA is waived with reason.

Not allowed:

- Scope waiver for product writes.
- Approval waiver for sensitive changes.
- Evidence waiver where evidence is required for completion.
- Acceptance waiver where acceptance is required.

Verification waiver is not detached verification. A task closed through verification waiver uses `close_reason=completed_with_risk_accepted` and `assurance_level=none` or `self_checked`.

Decision deferral is not a waiver. A deferred Decision Packet must record the affected operation, why the Task can proceed without the decision now, and any residual risk or follow-up needed before close.

## `prepare_write` State Logic

`prepare_write` is the product-write decision point. It returns one of these state-level decisions:

```text
allowed | blocked | approval_required | decision_required | state_conflict
```

These state-level decisions do not define public `ErrorCode` selection. Public tool responses derived from this logic select the primary `ToolError.code` using API-owned [Primary Error Code Precedence](05-mcp-api-and-schemas.md#primary-error-code-precedence).

The decision algorithm is:

1. Check state version expectations. If the caller is acting on stale state, return `state_conflict`.
2. Resolve the active Task. If none exists, return `blocked`.
3. Confirm the Task mode is write-eligible. `advisor` mode blocks product writes.
4. Resolve the active Change Unit. If no active Change Unit scopes the intended write, return `blocked`.
5. Check the intended operation against the active Change Unit Autonomy Boundary. If the operation exceeds the recorded latitude, block the write. When product judgment can resolve the gap, set `decision_gate=required` or create/request a Decision Packet and return `decision_required`. A resolved Decision Packet may update the Autonomy Boundary or propose Change Unit scope changes, but the write remains blocked until the active Change Unit scope and any sensitive approval are compatible.
6. Check intended paths, tools, commands, network targets, and secret access against the Change Unit. Scope gaps return `blocked` or require scope confirmation.
7. Check baseline freshness. If the baseline is stale, return `blocked` and mark dependent approvals, Decision Packets, or evidence stale or incompatible where applicable.
8. Determine sensitive categories. If sensitive categories exist and no matching Approval is granted, set or keep `approval_gate=required`, record approval-required blocker state for a committed non-dry-run decision when applicable, return `approval_required`, and optionally return an `approval_request_candidate` for display or a later `request_user_decision(decision_kind=approval)` call. This path must not create an Approval record, Decision Packet, Write Authorization, or `APR`.
9. Validate approval scope. Denied, expired, drifted, or insufficient approval returns `blocked` or `approval_required` depending on whether a new approval can resolve it. If a new approval can resolve it, the gate returns to `approval_gate=required` until `request_user_decision(decision_kind=approval)` commits the approval request.
10. Run design-policy precondition checks that apply before writing. Required unmet design preconditions return `blocked` or `decision_required` according to policy.
11. Evaluate Decision Packet requirements for the intended operation. A required blocking Decision Packet that is absent, pending, blocked, or deferred without coverage for the intended operation blocks the write and returns `decision_required` when user judgment can resolve it. A resolved Decision Packet must match the active Change Unit, Autonomy Boundary, baseline, and intended operation.
12. Run surface capability checks. Capability failures are recorded as validator results, blocked reasons, and guarantee display changes; they do not create capability as a first-class kernel gate.
13. If all required checks pass, create a compatible unexpired Write Authorization for the intended operation, or return the already committed response for idempotent replay of the same request, record the decision, and return `allowed`.

Required checks include active Task, active Change Unit, mode write eligibility, Autonomy Boundary compatibility, baseline freshness, intended paths, intended tools, intended commands, network targets, secret access, sensitive categories, approval scope, Decision Packet state, surface capability profile, and design policy preconditions.

An `allowed` decision must create or reference a Write Authorization with `status=allowed` and a recorded `basis_state_version` for the affected scope used by the allow decision. `authorization_effect=returned` is reserved for idempotent replay of the same committed `prepare_write` request with the same idempotency key, request hash, and `basis_state_version`, or for returning the already committed response. A distinct compatible request creates a distinct Write Authorization; compatibility does not make authorizations reusable. Blocked, approval-required, decision-required, or state-conflict results must not create a consumable Write Authorization for the attempted write. Core may stale, expire, or revoke older unconsumed authorizations if their compatibility basis changes.

When product judgment is needed, `prepare_write` requests a user decision through a Decision Packet. It must not convert product judgment into broad approval. `approval_required` is reserved for sensitive-change approval.

When `approval_required` is returned, no consumable Write Authorization exists and no Approval record, Decision Packet, or `APR` projection is created for the candidate. `request_user_decision(decision_kind=approval)` creates the approval-shaped Decision Packet and linked pending Approval record, which moves `approval_gate` from `required` to `pending`. `record_user_decision` updates that linked Approval record and moves `approval_gate` to `granted`, `denied`, or `expired`. Core creates a Write Authorization only if a subsequent compatible `prepare_write` retry returns `allowed`.

If MCP is unavailable on a cooperative-only surface, product writes must be held by instruction. If a stronger guard or isolation layer exists, the same decision may be enforced preventively or by isolation.

## `record_run` State Logic

`record_run` is the Run, artifact, and evidence recording point for shaping updates, implementation, direct work, and verification input. It does not retroactively authorize product writes.

Implementation and direct `record_run` calls that report product writes must consume a compatible, unexpired, unconsumed Write Authorization. The consumed authorization must match the active Task, active Change Unit, baseline, intended operation, sensitive categories, approval refs, relevant Decision Packet refs, and guarantee level required by the write.

`runs.write_authorization_id` is populated only when a Run successfully consumes a compatible Write Authorization. A violation or audit Run that attempted to use an invalid, stale, missing, consumed, or scope-exceeded authorization must not populate `runs.write_authorization_id` as a consumed authorization. The attempted authorization ref, when useful for audit, should be recorded in validator findings, run violation payload, or `task_events.payload_json`.

Core must verify observed changed paths against both the consumed Write Authorization and the active Change Unit. It also verifies recorded tools, commands, network targets, and secret access against the authorization when those observations are available from command results, artifacts, surface telemetry, or declared run data.

If no product writes are reported and Write Authorization is still required by the Run kind, active Change Unit, or intended operation, Core rejects `record_run` when authorization is missing. If observed product writes already occurred but authorization is missing or exceeded, Core may record a blocked or violation Run for recovery and audit. That Run must not satisfy evidence sufficiency, detached verification, QA, acceptance, or close readiness, and Core marks affected scope, evidence, approval, verification, and projection state stale or blocked. The corresponding Write Authorization, if any, remains unconsumed and may be marked stale, revoked, or expired according to the violation and compatibility basis. When the observed behavior asserts a general scope violation, Core may also append `scope_violation_detected`.

The Task cannot rely on a blocked or violation Run for close until the state is repaired through compatible scope, approval, Decision Packet resolution, evidence update, verification, or a new write authorization and Run.

MVP `shaping_update` is not a product-write recording path. Shaping-only Runs may be recorded without consuming Write Authorization, but they must not include product file changes. If a `shaping_update` also reports observed product writes, Core rejects it and requires `kind=implementation` or `kind=direct` with a compatible Write Authorization.

Read-only Runs may be recorded without consuming Write Authorization, but they must not include product file changes. If such a Run observes product changes, Core treats it as an implementation/direct compatibility failure.

## `close_task` State Logic

`close_task` is the single completion decision point. Agent reports, Eval reports, QA notes, and acceptance messages may provide inputs, but they do not close the Task by themselves.

When multiple close blockers exist, public responses select the primary `ToolError.code` using API-owned [Primary Error Code Precedence](05-mcp-api-and-schemas.md#primary-error-code-precedence); this section owns the kernel checks and state transitions.

The decision algorithm is:

1. Resolve the active Task and requested close intent.
2. If the intent is cancellation or supersession, ensure no write is in an unsafe in-progress state, then set `lifecycle_phase=cancelled`, `result=cancelled`, and the matching close reason.
3. Reject completion if an active Run is still open.
4. Check the active Change Unit. Write-capable Tasks need the active Change Unit completed, explicitly deferred, or superseded according to policy.
5. Check `scope_gate`. Product writes require passed scope.
6. Check `decision_gate` and blocking Decision Packets. Required, pending, blocked, absent, or incompatible blocking decisions block close. Deferred decisions are compatible only when close impact, residual risk, and follow-up visibility are recorded.
7. Check `approval_gate`. Sensitive changes require granted approval with no drift or expiry.
8. Check `design_gate`. Required design gates must be passed or validly waived; stale, blocked, pending, or partial required design gates block close unless policy converts them to a recorded waiver.
9. Check `evidence_gate`. Where evidence is required, only `sufficient` can close successfully.
10. Check `verification_gate`. Work requires passed detached verification or explicit user verification waiver. Direct work defaults to not required, but optional passed detached verification may upgrade assurance. Same-session review cannot produce detached assurance. Verification waiver is separate from detached verification and cannot contribute to `assurance_level=detached_verified`.
11. Check `qa_gate`. Required QA must be passed or validly waived. A Manual QA record result alone does not close the gate unless the kernel aggregates it into `qa_gate`.
12. Check close-relevant residual risk. If no known close-relevant Residual Risk exists, `ResidualRiskSummary.status=none` satisfies residual-risk visibility. If known close-relevant Residual Risk exists, it must be visible in the current judgment context before any successful close. Risk-accepted close additionally requires visible and accepted Residual Risk refs. Verification risk acceptance additionally sets `verification_gate=waived_by_user`.
13. Check `acceptance_gate`. Required acceptance can be recorded only after close-relevant residual risk is visible in the current judgment context or confirmed as `ResidualRiskSummary.status=none`. Rejection routes the Task back to shaping, execution, or cancellation.
14. Assign `assurance_level`, `result`, and `close_reason`:
    - advisor completion: `result=advice_only`, `assurance_level=none`, `close_reason=completed_self_checked`
    - direct self-check: `result=passed`, `assurance_level=self_checked`, `close_reason=completed_self_checked`
    - detached verified completion: `result=passed`, `assurance_level=detached_verified`, `close_reason=completed_verified`
    - risk accepted close: `result=passed`, `assurance_level=none` or `self_checked`, `close_reason=completed_with_risk_accepted`, with accepted Residual Risk refs
15. Report projection freshness. Projection stale or failed status is shown to the user and export, but it does not by itself make the Task failed.
16. Update current records, append a close event, and enqueue projection refresh.

## Close Semantics

`completed_verified` means detached verification actually passed and the independence qualifier is valid.

`completed_self_checked` means the result was checked by the implementing path or did not require detached verification.

`completed_with_risk_accepted` means the user accepted close-relevant residual risk, including verification risk when verification was waived. This is a successful close with explicit risk, not detached verification.

Residual-risk acceptance means known remaining risk was made visible and accepted for the requested close. It never upgrades assurance to `detached_verified`, and it does not imply detached verification, Manual QA, sensitive approval, or final acceptance unless those separate gates are also satisfied.

`ResidualRiskSummary.status=none` means there is no known close-relevant residual risk to accept. It satisfies visibility for ordinary close and acceptance, but it is not accepted risk and cannot support `completed_with_risk_accepted`.

`cancelled` means the Task stopped without a passed result.

`superseded` means another Task or Change Unit replaces this one. Supersession does not imply success.

## Invariant Enforcement Mapping

| Kernel Authority Invariant | Kernel enforcement points |
|---|---|
| Chat is not state. | State-changing actions create state records and `task_events`; projections and chat text cannot mutate state without MCP action or reconcile. |
| Product write requires an active scoped Change Unit. | `prepare_write` blocks write-capable actions without active Task, active Change Unit, and passed scope gate; allowed writes create Write Authorization or return the committed idempotent replay response, and implementation/direct Runs must consume a compatible authorization. |
| Sensitive change requires explicit approval. | `prepare_write` detects sensitive categories, checks approval gate and approval scope, and blocks denied, expired, missing, or drifted approval; approval cannot satisfy product judgment outside its sensitive scope. |
| Blocking product judgment requires a recorded Decision Packet. | `decision_gate`, `prepare_write`, `record_run`, and `close_task` require a canonical Decision Packet for blocking product judgment; unresolved or incompatible blocking packets prevent affected writes and close. |
| Completion requires evidence coverage where evidence is required. | `close_task` requires `evidence_gate=sufficient` when evidence applies; required evidence cannot be waived for passed completion. |
| Work cannot self-certify detached verification. | Eval plus valid independence is required for `detached_verified`; same-session review and verification waiver cannot upgrade assurance. |
| Required QA and acceptance are separate gates. | `qa_gate` and `acceptance_gate` are checked independently; Manual QA records do not imply acceptance, and acceptance does not imply QA. |
| Projection cannot override canonical state. | Projection edits create reconcile items; projection freshness affects display and delivery, not canonical result by itself. |
