# TASK Template

## 권한 규칙

- Projection은 Core가 소유한 상태 기록과 아티팩트 참조에서 파생됩니다.
- Projection은 Core 상태가 아닙니다.
- 사용자가 Projection을 편집해도 그 내용이 자동으로 받아들여진 상태가 되지는 않습니다.
- Chat과 Markdown은 Core 상태를 덮어쓸 수 없습니다.

## 사용 시점

전체 보고서가 명시적으로 유용한 later/profile 단계에서, 진행 중인 작업을 이어서 파악할 수 있는 continuity 또는 reference projection이 필요할 때 `TASK`를 사용합니다. 이 template은 Scope, User Judgments, Evidence, Close Readiness, 작업의 현재 위치, 사용자 판단 맥락, 막힘 소유자, Autonomy Boundary, Write Authority Summary, Implementation Micro-Plan, Review Stages, Stewardship Impact, 다음 근거, Residual Risk, Close Summary, 필요할 때의 kernel gate detail, active Change Unit, 대기 중인 judgment, 관련 보고서 ref, 읽기용 보기 최신성을 보여줄 수 있습니다.

경계: projection template일 뿐이며 runtime/server 구현이나 생성된 운영 산출물에 권한을 주지 않습니다. 공통 phase와 projection 규칙은 [템플릿 참조](README.md#사용-시점)를 따릅니다.

구현 계층: Future/diagnostic projections입니다. `TASK`는 v0.2 First User-Value Slice projection이 아닙니다. v0.2의 사용자 대상 status는 [Compact Status Card](compact-status-card.md)가 담당하고, 사용자 판단이 필요하면 Decision Packet prompt 또는 resource가 담당합니다. 전체 `TASK` body는 later profile polish입니다.

이 repository에 `TASK` template이 있다는 사실은 현재 단계에서 full `TASK` Markdown이 필요하다는 뜻이 아닙니다.

## 기준 기록

- `state.sqlite` Task와 task gate
- active Change Unit과 Change Unit dependency
- mode, lifecycle, next action, 가장 먼저 해소할 막힘, 가장 작은 해소 방법, guarantee level, 읽기용 보기 최신성(projection freshness)을 위한 현재 상태 표시 input
- 기존 owner 기록, gate, blocker, ref에서 파생되는 범위, 사용자 판단, 근거, 닫기 준비 상태 표시 그룹 input
- Write Authorization 기록과 Write Authority Summary 표시 input
- Decision Packet과 Residual Risk, 렌더링할 때의 schema가 소유하는 Decision Packet `judgment_category`
- 최신 Run, Evidence Manifest, Eval, 수동 QA 기록, 민감 동작 승인 기록
- Write Authorization, Decision Packet, Approval, Evidence Manifest, Eval, 수동 QA, 작업 수락 context, Residual Risk, Artifact refs, redaction state, projection freshness 권한 claim을 표시할 때 필요한 compact source refs
- 가장 먼저 해소할 막힘, 추가 막힘, 가장 작은 해소 방법 표시 summary
- changed scope, 민감 동작 승인, 근거, 검증, 수동 QA, 잔여 위험 표시, 잔여 위험 수용, 작업 수락, 면제 판단 상태, close reason을 포함하는 close summary 표시 input
- Journey Spine 기준 기록
- `domain_terms`, `module_map_items`, `interface_contracts`, `feedback_loops`
- TDD가 선택된 경우 `tdd_traces`
- design-quality validator 결과
- 예상되는 근거 필요 항목
- 기존 owner 기록과 ref에서 온 Review Stage 표시 input
- artifact ref 및 읽기용 보기 최신성(projection freshness)

`TASK`의 생성된 gate group summary, 사용자 판단 표시 text, close, waiver, review-stage, stewardship, projection-freshness 항목은 표시 binding입니다. 위에 나열한 owner record, gate, artifact, ref로 해소되어야 하며, 그런 source가 없으면 명시적인 absence/blocking 상태로 렌더링해야 합니다. Schema-owned `judgment_category`를 렌더링해도 기준 기록, gate, `ProjectionKind` value, 근거, 수동 QA, 검증, 작업 수락, 잔여 위험 수용, close, Write Authorization을 만들지 않습니다.

## 렌더링 섹션

Later profile이 full report를 켜면 `TASK`는 다음과 같은 section을 렌더링할 수 있습니다.

- Gate Group Summary
- Current Summary
- Where We Are
- User Judgment Context
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
- 수용 기준
- Active Change Unit
- Pending Decisions
- Evidence And Reports
- User Notes and Proposals

장기 `work` Task는 shared design, domain term ref, module/interface ref, Change Unit dependency, implementation detail, Journey Spine을 위한 expanded managed section을 표시할 수 있습니다.

## 전체 템플릿

이것은 future/profile report shape입니다. MVP compact card가 아니며 source of truth도 아닙니다.

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
## Gate Group Summary
- Scope:
  - what may change:
  - out of bounds:
  - write authority:
  - blocker / smallest unblocker:
  - source refs:
- 사용자 판단:
  - pending items (one line per decision; merge하지 않음):
  - direction judgments:
    - 제품/UX 판단:
    - 기술 구조 판단:
    - 보안/개인정보 판단:
    - 범위/자율성 판단:
  - permission:
    - 민감 동작 승인:
  - waivers:
    - QA 면제 판단:
    - 검증 면제 판단:
  - acceptance:
    - 작업 수락:
  - risk acceptance:
    - 잔여 위험 수용:
    - 수용하는 named risk:
  - decision / approval / waiver / acceptance / risk refs:
  - blocker / smallest unblocker:
  - what agent may continue:
- Evidence:
  - evidence status:
  - supporting refs:
  - missing or stale support:
  - artifact redaction or omission state:
  - 대체하지 않는 것: 검증, 수동 QA, 작업 수락, 잔여 위험 수용
  - next evidence action:
- Close Readiness:
  - verification:
  - 수동 QA:
  - 민감 동작 승인:
  - 작업 수락:
  - 잔여 위험 표시:
  - 잔여 위험 수용:
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
- 대기 중인 judgment:
- 대기 중인 judgment type:
- user is judging:
- risk:
- gate display groups: 범위=; 사용자 판단=; 근거=; 닫기 준비 상태=
- guarantee level:
- kernel gate detail: scope=; decision=; approval=; design=; evidence=; verification=; 수동 QA=; acceptance=
- active change unit:
- Write Authority Summary:
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

## User Judgment Context
- 대기 중인 judgment packet:
- 대기 중인 judgment items:
- judgment type:
- judgment title:
- judgment_category:
- display label:
- judgment_route:
- display_depth:
- why needed now:
- what user is judging:
- options:
- trade-offs:
- recommendation:
- uncertainty:
- deferral consequence:
- residual risk when relevant:
- 수용하는 named risk:
- what agent may decide without user:
- 이 decision이 확정하지 않는 것:
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
- 수동 QA:
- Acceptance Decision Packet:
- Acceptance context:
- Residual Risk:
- Artifact refs and redaction state:
- Projection freshness:

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
- note: managed display only; Role Lens/playbook 라벨은 gate, record, `ProjectionKind` value, 민감 동작 승인, 근거, 검증, 수동 QA, 작업 수락, 잔여 위험 수용, close, Write Authorization을 만들지 않는다. Same-session review는 분리 검증이 아니다. 발견 사항은 기존 owner record, ref, gate, blocker로 연결한다.

### Spec Compliance Review
- 수용 기준 coverage:
- Change Unit completion conditions:
- scope / Write Authority compatibility:
- Decision Packet compatibility:
- evidence coverage:
- 잔여 위험 표시:
- routed outcome (existing path/ref only):

### Code Quality / Stewardship Review
- domain language:
- module / interface boundary:
- vertical slice shape:
- feedback loop / TDD:
- codebase stewardship:
- context hygiene:
- 후속 위험:
- routed outcome (existing path/ref only):

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
- status value:
- named risk being accepted:
- 잔여 위험 수용 status:
- accepted residual-risk refs:
- 후속 작업 필요:
- 닫기 영향:

## Close Summary
- changed scope:
- evidence:
- verification:
- 수동 QA:
- 민감 동작 승인:
- 잔여 위험 표시:
- 잔여 위험 수용:
- 작업 수락:
- 작업 수락이 대체하지 않는 것:
- waiver status:
- authority source refs:
- display state label (plain text, schema value 아님):
- self-check refs:
- 분리 검증 Eval ref:
- 검증 면제 판단 ref:
- QA 면제 판단 ref:
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

## 수용 기준
- [ ] AC-01:
- [ ] AC-02:

## Active Change Unit
| ID | Purpose | Status | Slice Type | TDD | 수동 QA | Core Verification |
|---|---|---|---|---|---|---|
| CU-01 | | | vertical | trace 상태: required \| recorded \| waived \| not_required; RED/GREEN ref 표시 | pending | |

## Pending Decisions
| Type | Question | Route / refs | Status | Next action |
|---|---|---|---|---|
| 제품/UX 판단 \| 기술 구조 판단 \| 보안/개인정보 판단 \| 범위/자율성 판단 \| 민감 동작 승인 \| QA 면제 판단 \| 검증 면제 판단 \| 작업 수락 \| 잔여 위험 수용 | | | | |

## Evidence And Reports
- Evidence Manifest:
- Run Summary:
- Eval:
- Direct Result:
- TDD Trace:
- 수동 QA:
- Approval:
- Decision:
- Diff:
- Logs:
- Artifact refs with redaction state:
- Projection freshness:
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
- boundary: 기준 상태 아님, 범위 권한 아님, Approval 아님, Write Authorization 아님; active Change Unit이 범위의 기준 출처로 남음

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
- 후속 vertical CU:
- autonomy profile:
- agent may do:
  - implementation detail:
  - local refactor inside scope:
  - evidence collection:
- user judgment required:
  - 제품/UX 판단:
  - 기술 구조 판단:
  - 보안/개인정보 판단:
  - 범위/자율성 판단:
  - 민감 동작 승인:
  - QA 면제 판단:
  - 검증 면제 판단:
  - 작업 수락:
  - public interface 또는 호환성 약속:
  - 잔여 위험 수용:
- AFK stop conditions:
  - boundary exceeded:
  - evidence cannot be produced:
  - close-relevant risk discovered:
- end-to-end path:
  - trigger / input:
  - domain logic:
  - persistence:
  - API / 호출자 경계:
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
  - trace 상태: required | recorded | waived | not_required
  - 요구/출처:
  - RED target / plan:
  - RED evidence (actual):
  - green evidence:
  - non-TDD justification:
- 수동 QA:
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

`TASK`의 Stewardship Impact는 owner 기록, validator 결과, 참조에서 파생되는 `StewardshipImpactSummary` 표시입니다. Domain Language, Module Map, Interface Contract, Feedback Loop, TDD Trace, 잔여 위험(residual risk), Decision Packet owner 기록을 대체하지 않습니다.

`TASK`의 Implementation Micro-Plan은 현재 Task와 Change Unit 상태에서 생성되거나 그 상태와 정렬된 가벼운 실행 보조 정보입니다. [Document Projection Reference](../document-projection.md#projection-principles)의 projection/report 경계 안에 머물며, `prepare_write`나 owner state change를 대체하지 않습니다.

`TASK`의 Review Stages는 Role Lens, playbook, two-stage review guidance를 위한 관리되는 표시 섹션입니다. 정확한 권한 없음 규칙은 [Design Quality Policies](../design-quality-policies.md#two-stage-review-display)와 [Agent Integration](../agent-integration.md#role-lens-동작)이 담당합니다. 기준 기록, `ProjectionKind` value, 민감 동작 승인, 근거, 검증, 수동 QA, 작업 수락, 잔여 위험 수용, close, Write Authorization을 만들지 않으며, 발견 사항은 기존 owner path로 연결해야 합니다.

생성된 summary는 사용자가 읽기 쉬운 평범한 말을 먼저 쓰고, 정확한 Harness term은 유용한 label이나 ref로 붙입니다. Projection이 명령어처럼 보이거나 표시 문구만으로 상태가 만들어진 것처럼 암시하면 안 됩니다.

Gate Group Summary는 읽는 사람이 raw gate detail보다 실제 막힘 이야기를 먼저 보도록 첫 managed section으로 둡니다. 범위, 사용자 판단, 근거, 닫기 준비 상태는 기존 owner 기록, gate, blocker, ref에서 파생되는 표시 그룹입니다. 기준 field, 정확한 gate value의 alias, 새 gate, recompute input, close semantics, authority path가 아닙니다. 사용자 판단은 구조화되어 있으며 하나의 넓은 판단 또는 승인 bucket처럼 렌더링하면 안 됩니다. 정확한 gate 값과 recompute rule은 [커널 참조](../kernel.md#gates)가 담당하고, close 동작은 [`close_task`](../kernel.md#close_task)가 담당합니다.

`TASK`의 Decision Packet 표시는 기준 schema field와 렌더링 label을 분리해야 합니다. `judgment_category`는 사용자에게 보이는 판단 묶음입니다. `judgment_route`는 owner path와 recorded-answer route입니다. `display_depth`는 prompt depth입니다. Template은 `judgment_category`를 제품/UX 판단, 기술 구조 판단, 보안/개인정보 판단, QA/verification, 작업 수락, 잔여 위험, 범위/자율성 판단, 복합 같은 label로 렌더링해 사용자가 판단 영역을 빠르게 보게 할 수 있습니다. 동시에 route와 owner refs에서 파생되는 구체적인 판단 유형도 보여줘야 합니다. 제품/UX 판단, 기술 구조 판단, 보안/개인정보 판단, 범위/자율성 판단, 민감 동작 승인, QA/verification waiver, 작업 수락, 잔여 위험 수용을 구분합니다. Judgment가 여러 영역에 걸쳐 있으면 category를 배타적으로 다루지 말고 부차적인 고려사항을 장단점, 영향받는 gate, risk, evidence, follow-up에 렌더링해야 합니다. `judgment_category`는 enum 값으로 validate되지만 payload branch를 고르거나 gate를 다시 계산하지 않으며, gate, status, close aggregation rule, authority path, `judgment_route`의 대체물이 아닙니다. `display_depth` 또는 `judgment_category`에서 파생한 표시용 label은 validator input이 아니며 `judgment_route`, 민감 동작 승인, 작업 수락, QA, 잔여 위험 수용, close, Write Authorization의 owner contract를 흐리면 안 됩니다.

대기 중인 결정은 한 줄로 합치면 안 됩니다. 민감 동작 승인, 작업 수락, 잔여 위험 수용이 모두 대기 중이면 세 가지 label로 세 항목을 렌더링합니다. Approval card는 작업 수락처럼 보이면 안 되고, 잔여 위험 수용은 수용하는 위험을 이름 붙여야 합니다.

`TASK`의 authority claim은 source ref 또는 명시적 absence로 해소되어야 합니다. Write authority claim은 Write Authorization ref를, 민감 동작 permission은 Approval ref를, 근거 충분성은 Evidence Manifest ref를, 분리 검증은 Eval ref를, 수동 QA는 수동 QA record 또는 valid waiver ref를, 작업 수락은 Acceptance Decision Packet ref를, 잔여 위험 표시는 Residual Risk refs 또는 `ResidualRiskSummary.status=none`을, 잔여 위험 수용은 accepted Residual Risk refs를 가리켜야 합니다. Ref가 없으면 completed authority가 아니라 missing support로 렌더링해야 합니다.

Residual-risk display는 `status=none`과 `not_visible`을 구분해야 합니다. `status=none`은 requested action에 대해 알려진 close-relevant residual risk가 없다는 뜻입니다. `not_visible`은 알려진 close-relevant risk가 있지만 작업 수락 또는 close에 충분히 보이지 않았다는 뜻이므로, risk와 refs가 보일 때까지 blocker 또는 next action으로 남아야 합니다.

`TASK`의 close와 assurance 표시는 self-checked work, `detached_verified`, 검증 면제 판단, QA 면제 판단, 잔여 위험 수용 close를 눈에 보이게 분리해야 합니다. 잔여 위험 수용 close는 수락된 Residual Risk refs와 필요한 Decision Packet을 가리켜야 합니다. 검증 면제 판단은 `verification_gate=waived_by_user`와 필요한 경우 그 Decision Packet을, QA 면제 판단은 `qa_gate=waived`, 수동 QA record 또는 waiver reason, 필요한 경우 QA 면제 판단 Decision Packet을 가리켜야 합니다.

`TASK`의 waiver 표시는 요약일 뿐입니다. 닫기에 영향을 주는 QA 또는 검증 면제 판단은 waiver를 유효하게 만드는 기존 기록을 가리켜야 합니다. QA 면제 판단은 `manual_qa_records`/`qa_gate=waived`와 필요한 경우 QA 면제 판단 Decision Packet을, 검증 면제 판단은 `verification_gate=waived_by_user`와 필요한 경우 그 Decision Packet을 가리킵니다. Policy 또는 gate, Task와 Change Unit, 생략한 확인이나 대상, reason, actor, 필요할 때 expiry 또는 잔여 위험 후속 작업, 관련 refs, 닫기 영향, 그리고 필요할 때 잔여 위험 경로로 보여주거나 수용해야 하는 close-relevant 잔여 위험도 함께 보여줘야 합니다. QA 면제 판단은 수동 QA가 되지 않고, 검증 면제 판단은 분리 검증을 만들지 않습니다.

`TASK`의 Close Summary는 진행 중이거나 최근 닫힌 `work` Task를 위한 이어가기 표시 요약입니다. Gate 상태나 잔여 위험을 숨기면 안 됩니다. 닫기가 성공했거나, 막혔거나, 취소됐거나, 잔여 위험 수용으로 닫혔을 때 changed scope, 민감 동작 승인, 근거, 검증, 수동 QA, 잔여 위험 표시, 잔여 위험 수용, 작업 수락, 면제 판단 상태, close reason, 잔여 위험 후속 작업을 해당되는 만큼 보여주고 owner record로 돌아가는 ref를 포함해야 합니다. 민감 동작 승인, 작업 수락, 잔여 위험 수용은 반드시 별도 줄로 유지합니다. 민감 동작 승인은 이름 붙은 민감 동작에 대한 허가이고, 작업 수락은 사용자의 result judgment이며, 잔여 위험 수용은 수용한 위험을 이름 붙이고 accepted Residual Risk refs를 cite해야 합니다.

Close Summary는 민감 동작 승인, 근거, 검증, 수동 QA, 작업 수락, 잔여 위험 표시, 잔여 위험 수용을 하나의 "완료" flag로 합치면 안 됩니다. 테스트가 통과했지만 민감 동작 승인, 수동 QA, 작업 수락, 잔여 위험 수용이 pending이면 close display는 정확히 그 범주를 blocker로 보여줘야 합니다.

Direct 작업은 `DIRECT-RESULT`에서 가벼운 close impact summary를 보여주고, Journey Card close context는 간결한 status/resume 표시입니다. `TASK` Close Summary는 [projection/report 경계](../document-projection.md#projection-principles) 안의 이어가기 표시이며, close와 gate effect는 여전히 owner record에서 옵니다.

`TASK`, Journey, evidence, report section에 표시되는 artifact ref는 `redaction_state`를 보존해야 합니다. `secret_omitted` ref는 보이는 nonsecret 근거만 뒷받침할 수 있고, `blocked` ref는 원본 content가 아니라 committed metadata-only notice와 사용할 수 없는 입력을 보여줍니다.
