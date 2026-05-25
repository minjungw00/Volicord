# TASK Template

## 사용 시점

진행 중인 작업을 이어서 파악할 수 있는 projection이 필요할 때 `TASK`를 사용합니다. 이 template은 작업의 현재 위치와 판단 맥락을 보여줍니다. 또한 Autonomy Boundary, Write Authority Summary, Implementation Micro-Plan, Review Stages, Stewardship Impact, Residual Risk, gate, active Change Unit, 대기 중인 decision을 요약합니다. 다음 evidence, 관련 보고서 참조, projection 최신성도 함께 보여줍니다.

## 기준 기록

- `state.sqlite` Task와 task gate
- active Change Unit과 Change Unit dependency
- Write Authorization 기록과 Write Authority Summary 표시 input
- Decision Packet과 Residual Risk
- 최신 Run, Evidence Manifest, Eval, Manual QA 기록, approval 기록
- Journey Spine 기준 기록
- `domain_terms`, `module_map_items`, `interface_contracts`, `feedback_loops`
- TDD가 선택된 경우 `tdd_traces`
- design-quality validator 결과
- 예상되는 evidence 필요 항목
- Review Stage 표시 input
- artifact ref 및 projection 최신성

## 렌더링 섹션

- Current Summary
- Where We Are
- Judgment Context
- Autonomy Boundary
- Write Authority Summary
- Implementation Micro-Plan
- Review Stages
- Next Evidence
- Residual Risk
- Stewardship Impact
- Goal
- Scope
- Acceptance Criteria
- Active Change Unit
- Pending Decisions
- Evidence And Reports
- User Notes and Proposals

장기 `work` Task는 shared design, domain term ref, module/interface ref, Change Unit dependency, implementation detail, Journey Spine을 위한 expanded managed section을 표시할 수 있습니다.

## 전체 템플릿

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

> Projection 보기: `source_state_version`와 `updated_at` 기준으로 렌더링된 보기입니다. Managed section은 생성된 표시 영역이며, 그 안의 edit는 상태 변경이 아니라 drift 또는 reconcile candidate입니다.

<!-- HARNESS:BEGIN managed -->
## Current Summary
- mode:
- lifecycle phase:
- result:
- close reason:
- assurance:
- next action:
- pending decision:
- risk:
- scope gate:
- decision gate:
- approval gate:
- design gate:
- evidence gate:
- verification gate:
- Manual QA:
- acceptance gate:
- active change unit:
- Write Authority Summary:
- latest report:
- projection freshness:

## Where We Are
- current position:
- active path:
- current blocker:
- latest meaningful evidence:
- next state transition:

## Judgment Context
- pending decision packets:
- what user is deciding:
- what agent may decide without user:
- recommendation:
- main trade-off:
- reversibility:
- uncertainty:
- minimum context to judge:
- affected gates:

## Autonomy Boundary
- profile:
- agent may do:
- user judgment required:
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
- note: Autonomy Boundary는 판단 재량이지 쓰기 권한이 아니다.

## Implementation Micro-Plan
- note: execution aid only; active Change Unit scope bounds writes and `prepare_write` creates Write Authorization.
- TDD note: required이면 selected feedback loop, RED target, GREEN target, non-test implementation이 actual RED evidence 또는 waiver를 기다리는지 표시한다.

| Step / Slice | Purpose | Active Change Unit Scope / Likely Paths | Feedback Loop / TDD | Expected Evidence | Stop / Ask User When |
|---|---|---|---|---|---|
| 1 | | | | | |

## Review Stages
- note: managed display only; same-session review는 detached verification이 아니다.

### Spec Compliance Review
- acceptance criteria coverage:
- Change Unit completion conditions:
- scope / Write Authority compatibility:
- Decision Packet compatibility:
- evidence coverage:
- residual-risk visibility:
- routed outcome:

### Code Quality / Stewardship Review
- domain language:
- module / interface boundary:
- vertical slice shape:
- feedback loop / TDD:
- codebase stewardship:
- context hygiene:
- 후속 위험:
- routed outcome:

## Next Evidence
- next evidence action:
- evidence needed for:
- TDD RED target / plan:
- TDD RED evidence:
- TDD GREEN evidence:
- TDD refactor/check evidence:
- expected artifact refs:
- 생략/차단 artifact 영향:
- stale or missing evidence:

## Residual Risk
- close-relevant risk:
- visibility status:
- accepted residual-risk refs:
- 후속 작업 필요:
- close 영향:

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
| CU-01 | | | vertical | required: red_pending \| red_recorded \| green_recorded \| waived | pending | |

## Pending Decisions
-

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
<!-- HARNESS:END managed -->

## User Notes and Proposals
<!-- Human-editable: 여기의 note와 proposal은 reconcile input이며, Core를 통해 accepted되기 전에는 상태를 바꾸지 않습니다. -->
-
````

Long-running `work` task를 위한 expanded TASK section:

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
- boundary: 기준 상태 아님, 범위 권한 아님, Approval 아님, Write Authorization 아님; active Change Unit이 scope 기준 source로 남음

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
- current blocker:
<!-- HARNESS:END managed -->
````

Change Unit block sub-template:

````md
### CU-01 Title
- purpose:
- non-goals:
- slice type: vertical | enabling | cleanup | horizontal-exception
- horizontal exception reason:
- 후속 vertical CU:
- autonomy profile:
- agent may do:
  - implementation detail:
  - local refactor inside scope:
  - evidence collection:
- user judgment required:
  - 제품 방향:
  - 중요한 기술 방향:
  - public interface 또는 호환성 약속:
  - 남은 위험 수용:
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
  - required: yes | no | recommended
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

## 메모

`TASK`의 Stewardship Impact는 owner 기록, validator 결과, 참조에서 파생되는 `StewardshipImpactSummary` 표시입니다. Domain Language, Module Map, Interface Contract, Feedback Loop, TDD Trace, residual risk, Decision Packet owner 기록을 대체하지 않습니다.

`TASK`의 Implementation Micro-Plan은 현재 Task와 Change Unit 상태에서 생성되거나 그 상태와 정렬된 가벼운 실행 보조 정보입니다. Product write를 허가하거나, scope를 넓히거나, Approval을 충족하거나, 근거를 만들거나, edit만으로 상태를 변경하거나, `prepare_write`를 대체하지 않습니다.

`TASK`의 Review Stages는 관리되는 표시 섹션입니다. Gates를 충족하거나, write를 허가하거나, 위험을 수용하거나, Task를 닫거나, `detached_verified` assurance를 만들 수 없습니다.

`TASK`의 waiver 표시는 요약일 뿐입니다. close에 영향을 주는 QA 또는 verification waiver는 waiver를 유효하게 만드는 기존 기록을 가리켜야 합니다. QA waiver는 `manual_qa_records`/`qa_gate=waived`와 필요한 경우 QA waiver Decision Packet을, verification waiver는 `verification_gate=waived_by_user`와 필요한 경우 그 Decision Packet을 가리킵니다. 생략한 확인이나 대상, 수용하는 위험, 후속 작업, 관련 refs, 닫기 영향도 함께 보여줘야 합니다. QA waiver는 Manual QA가 되지 않고, verification waiver는 detached verification을 만들지 않습니다.

`TASK`, Journey, evidence, report section에 표시되는 artifact ref는 `redaction_state`를 보존해야 합니다. `secret_omitted` ref는 보이는 nonsecret evidence만 뒷받침할 수 있고, `blocked` ref는 원본 content가 아니라 committed metadata-only notice와 unavailable input을 보여줍니다.
