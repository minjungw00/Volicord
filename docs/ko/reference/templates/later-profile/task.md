# TASK 템플릿

## 권한 규칙

- Projection은 Core가 소유한 상태 기록과 아티팩트 참조에서 파생됩니다.
- Projection은 Core 상태가 아닙니다.
- 사용자가 Projection을 편집해도 그 내용이 자동으로 받아들여진 상태가 되지는 않습니다.
- Chat과 Markdown은 Core 상태를 덮어쓸 수 없습니다.

## 사용 시점

전체 보고서가 명시적으로 유용한 later/profile 단계에서, 진행 중인 작업을 이어서 파악할 수 있는 continuity 또는 reference projection이 필요할 때 `TASK`를 사용합니다. 이 template은 범위, 사용자 판단, 근거, 닫기 준비 상태, 작업의 현재 위치, 사용자 판단 맥락, 막힘 소유자, Autonomy Boundary(자율성 경계), Write Authority Summary(쓰기 권한 요약), 구현 마이크로 계획, 검토 단계, Stewardship 영향, 다음 근거, 잔여 위험, 닫기 요약, 필요할 때의 kernel gate detail, active Change Unit, 대기 중인 judgment, 관련 보고서 ref, 읽기용 보기 최신성을 보여줄 수 있습니다.

경계: projection template일 뿐이며 runtime/server 구현이나 생성된 운영 산출물에 권한을 주지 않습니다. 공통 phase와 projection 규칙은 [템플릿 참조](README.md#사용-시점)를 따릅니다.

구현 계층: 향후/진단용 projections입니다. `TASK`는 MVP-1 사용자 작업 루프 projection이 아닙니다. MVP-1의 사용자 대상 status는 [상태 카드](../status-card.md)가 담당하고, 사용자 판단이 필요하면 [판단 요청](../judgment-request.md) 또는 user-judgment resource가 담당합니다. Standalone Decision Packet은 복잡한 판단을 위한 선택적 full-format presentation입니다. 전체 `TASK` body는 later profile polish입니다.

이 repository에 `TASK` template이 있다는 사실은 현재 단계에서 full `TASK` Markdown이 필요하다는 뜻이 아닙니다.

## 기준 기록

- `state.sqlite` Task와 task gate
- active Change Unit과 Change Unit dependency
- mode, lifecycle, next action, 가장 먼저 해소할 막힘, 가장 작은 해소 방법, 보장 수준, 읽기용 보기 최신성(projection freshness)을 위한 현재 상태 표시 input
- 기존 owner 기록, gate, blocker, ref에서 파생되는 범위, 사용자 판단, 근거, 닫기 준비 상태 표시 그룹 input
- Write Authorization 기록과 Write Authority Summary 표시 input
- User Judgment record와 Residual Risk, 해당 profile이 켜졌을 때 full-format Decision Packet presentation field
- 최신 Run, evidence summary, ArtifactRefs, 그리고 matching profile이 active일 때 Evidence Manifest, Eval, 수동 QA 기록, 민감 동작 승인 기록
- Write Authorization, User Judgment, 민감 동작 승인 user judgment refs, later Approval refs, `evidence_ref` ref와 파생 evidence summary, active일 때 Evidence Manifest, Eval, 수동 QA, 작업 수락 context, Residual Risk, Artifact refs, redaction state, projection freshness 권한 claim을 표시할 때 필요한 compact source refs
- 가장 먼저 해소할 막힘, 추가 막힘, 가장 작은 해소 방법 표시 summary
- changed scope, 민감 동작 승인, 근거, 검증, 수동 QA, 잔여 위험 표시, 잔여 위험 수용, 작업 수락, 면제 판단 상태, close reason을 포함하는 close summary 표시 input
- Journey Spine 기준 기록
- `domain_terms`, `module_map_items`, `interface_contracts`, `feedback_loops`
- TDD가 선택된 경우 `tdd_traces`
- design-quality validator 결과
- 예상되는 근거 필요 항목
- 기존 owner 기록과 ref에서 온 Review Stage 표시 input
- artifact ref 및 읽기용 보기 최신성(projection freshness)

`TASK`의 생성된 gate group summary, 사용자 판단 표시 text, close, waiver, review-stage, stewardship, projection-freshness 항목은 표시 binding입니다. 위에 나열한 owner record, gate, artifact, ref로 해소되어야 하며, 그런 source가 없으면 명시적인 absence/blocking 상태로 렌더링해야 합니다. 제품/UX 판단 또는 작업 수락 같은 label을 렌더링해도 기준 기록, gate, `ProjectionKind` value, 근거, 수동 QA, 검증, 작업 수락, 잔여 위험 수용, close, Write Authorization을 만들지 않습니다.

## 렌더링 섹션

Later profile이 full report를 켜면 `TASK`는 다음과 같은 section을 렌더링할 수 있습니다.

- Gate 그룹 요약
- 현재 요약
- 현재 위치
- 사용자 판단 맥락
- 권한 출처 참조
- Autonomy Boundary(자율성 경계)
- Write Authority Summary(쓰기 권한 요약)
- 구현 마이크로 계획
- 검토 단계
- 다음 근거
- 잔여 위험
- 닫기 요약
- Stewardship 영향
- 목표
- 범위
- 수용 기준
- Active Change Unit(활성 Change Unit)
- 대기 중인 판단
- 근거와 보고서
- 사용자 메모와 제안

장기 `work` Task는 shared design, domain term ref, module/interface ref, Change Unit dependency, implementation detail, Journey Spine을 위한 expanded managed section을 표시할 수 있습니다.

## 전체 템플릿

이것은 future/profile report shape입니다. MVP 상태 카드가 아니며 source of truth도 아닙니다.

````md
---
doc_type: task
task_id: TASK-0001
display_state: executing
projection_version: 7
source_state_version: 42
updated_at: 2026-05-06T09:30:15+09:00
---

# TASK-0001 Task 제목

> Projection 보기: `source_state_version`와 `updated_at` 기준으로 렌더링된 보기입니다. Managed section은 생성된 표시 영역이며, 그 안의 edit는 상태 변경이 아니라 drift 또는 reconcile candidate입니다.

<!-- HARNESS:BEGIN managed -->
## Gate 그룹 요약
- 범위:
  - 바뀔 수 있는 것:
  - 범위 밖:
  - 쓰기 전 범위 확인 / Write Authorization:
  - 막힘 / 가장 작은 해소 방법:
  - source refs:
- 사용자 판단:
  - 대기 중인 항목(판단마다 한 줄, merge하지 않음):
  - 판단 요청:
    - 제품/UX 판단:
    - 기술 판단:
  - permission:
    - 민감 동작 승인:
  - waivers:
    - 관련 user judgment refs:
  - acceptance:
    - 작업 수락:
    - 잔여 위험 수용:
  - 잔여 위험 수용:
    - 잔여 위험 수용:
    - 수용하는 named risk:
  - decision / approval / waiver / acceptance / risk refs:
  - 막힘 / 가장 작은 해소 방법:
  - 에이전트가 계속할 수 있는 것:
- 근거:
  - evidence status:
  - supporting refs:
  - missing or stale support:
  - artifact redaction or omission state:
  - 대체하지 않는 것: 검증, 수동 QA, 작업 수락, 잔여 위험 수용
  - 다음 evidence action:
- 닫기 준비 상태:
  - verification:
  - 수동 QA:
  - 민감 동작 승인:
  - 작업 수락:
  - 잔여 위험 표시:
  - 잔여 위험 수용:
  - waiver status:
  - 닫기 막힘 / close reason:
  - 가장 작은 해소 방법:
- note: 이 항목들은 표시 그룹일 뿐입니다. 정확한 gate 값, recompute rule, close semantics는 Core Model Reference가 담당합니다.

## 현재 요약
- mode:
- lifecycle phase:
- result:
- close reason:
- assurance:
- 범위 요약:
- 범위 밖:
- 다음 action:
- 확인한 것:
- 남은 것:
- 가장 먼저 해소할 막힘:
- 막힘 소유자:
- 가장 작은 해소 방법:
- 추가 막힘:
- 대기 중인 judgment:
- 대기 중인 judgment type:
- 사용자가 판단하는 것:
- risk:
- gate display groups: 범위=; 사용자 판단=; 근거=; 닫기 준비 상태=
- 보장 수준:
- kernel gate detail: scope=; decision=; approval=; design=; evidence=; verification=; 수동 QA=; acceptance=
- active change unit:
- Write Authority Summary:
- authority source refs: write=; decision=; sensitive_action_permission=; evidence_summary=; evidence_manifest_when_active=; eval=; manual_qa=; work_acceptance=; residual_risk=; artifacts=
- redaction state:
- latest report:
- projection freshness:

## 현재 위치
- 현재 위치:
- active path:
- 확인한 것:
- 남은 것:
- 가장 먼저 해소할 막힘:
- 막힘 소유자:
- 가장 작은 해소 방법:
- 추가 막힘:
- 최신 meaningful evidence:
- 다음 state transition:

## 사용자 판단 맥락
- 대기 중인 user judgment:
- 대기 중인 judgment items:
- user_judgment_ref:
- judgment type:
- judgment title:
- judgment_type:
- presentation:
- display label:
- 지금 필요한 이유:
- 사용자가 판단하는 것:
- options:
- trade-offs:
- recommendation:
- uncertainty:
- 미룰 때의 영향:
- 해당되는 경우 residual risk:
- 수용하는 named risk:
- 에이전트가 사용자 없이 결정해도 되는 것:
- 이 판단이 확정하지 않는 것:
- generic consent handling:
- reversibility:
- 영향받는 범위:
- 판단에 필요한 최소 맥락:
- 영향받는 표시 group:
- 영향받는 gate refs:

## 권한 출처 참조
- Write Authorization:
- User Judgment:
- 민감 동작 승인 user judgment / Approval:
- Evidence summary / Evidence Manifest when active:
- Eval:
- 수동 QA:
- 작업 수락 user judgment:
- Acceptance context:
- Residual Risk:
- Artifact refs and redaction state:
- Projection freshness:

## Autonomy Boundary(자율성 경계)
- profile:
- agent may do:
- user judgment required:
- AFK stop conditions:
- boundary status:

## Write Authority Summary(쓰기 권한 요약)
- active Change Unit:
- write authorization:
- 허용 path:
- 허용 tool:
- 허용 command:
- 허용 network target:
- secret scope:
- 민감 category:
- approval status:
- baseline:
- guarantee:
- note: Autonomy Boundary는 판단 재량이지 쓰기 전 범위 확인이나 쓰기 허가 기록이 아니다.

## 구현 마이크로 계획
- note: 실행 보조 정보일 뿐입니다. active Change Unit 범위가 write를 제한하고 `prepare_write`가 Write Authorization을 만듭니다.
- TDD note: required이면 selected feedback loop, RED target, GREEN target, non-test implementation이 actual RED evidence 또는 waiver를 기다리는지 표시한다.

| Step / Slice | Purpose | Active Change Unit Scope / Likely Paths | Feedback Loop / TDD | Expected Evidence | Stop / Ask User When |
|---|---|---|---|---|---|
| 1 | | | | | |

## 검토 단계
- note: managed display only; Role Lens/playbook 라벨은 gate, record, `ProjectionKind` value, 민감 동작 승인, 근거, 검증, 수동 QA, 작업 수락, 잔여 위험 수용, close, Write Authorization을 만들지 않는다. Same-session review는 분리 검증이 아니다. 발견 사항은 기존 owner record, ref, gate, blocker로 연결한다.

### 명세 준수 검토
- 수용 기준 coverage:
- Change Unit completion conditions:
- scope / Write Authority compatibility:
- User judgment compatibility:
- evidence coverage:
- 잔여 위험 표시:
- routed outcome(existing path/ref only):

### 코드 품질 / Stewardship 검토
- domain language:
- module / interface boundary:
- vertical slice shape:
- feedback loop / TDD:
- codebase stewardship:
- context hygiene:
- 후속 위험:
- routed outcome(existing path/ref only):

## 다음 근거
- 다음 evidence action:
- evidence가 필요한 이유:
- TDD RED target / plan:
- TDD RED evidence:
- TDD GREEN evidence:
- TDD refactor/check evidence:
- 예상 artifact refs:
- 생략/차단 artifact 영향:
- stale 또는 missing evidence:

## 잔여 위험
- close-relevant risk:
- visibility status:
- status value:
- 수용하는 named risk:
- 잔여 위험 수용 status:
- accepted residual-risk refs:
- 후속 작업 필요:
- 닫기 영향:

## 닫기 요약
- 변경된 범위:
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
- 표시 상태 라벨(plain text, schema value 아님):
- self-check refs:
- 분리 검증 Eval ref:
- 검증 면제 판단 ref:
- QA 면제 판단 ref:
- accepted residual-risk refs:
- close reason:
- 남은 follow-up:

## Stewardship 영향
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
  - User Judgments:

## 목표
-

## 범위
### 포함
-

### 제외
-

## 수용 기준
- [ ] AC-01:
- [ ] AC-02:

## Active Change Unit(활성 Change Unit)
| ID | Purpose | Status | Slice Type | TDD | 수동 QA | Core Verification |
|---|---|---|---|---|---|---|
| CU-01 | | | vertical | trace 상태: required \| recorded \| waived \| not_required; RED/GREEN ref 표시 | pending | |

## 대기 중인 사용자 판단
| Display label | Question | `judgment_type` / refs | Status | Next action |
|---|---|---|---|---|
| 제품/UX 판단 \| 기술 판단 \| 민감 동작 승인 \| 작업 수락 \| 잔여 위험 수용 | | | | |

## 근거와 보고서
- Evidence summary / Evidence Manifest when active:
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

## 사용자 메모와 제안
<!-- Human-editable: 여기의 note와 proposal은 reconcile input이며, Core를 통해 accepted되기 전에는 상태를 바꾸지 않습니다. -->
-
````

장기 `work` task를 위한 expanded TASK section:

````md
<!-- HARNESS:BEGIN managed -->
## Shared Design 개념
### 해소된 질문
| ID | Question | User Answer | Decision / Assumption |
|---|---|---|---|

### 남은 모호함
- item / owner / stop condition:

## Domain Term 참조
- 적용 중인 Terms:
  - Term:

## Module과 Interface 참조
- module map item refs:
- interface contract refs:
- rendered projection refs, if shown: MODULE-MAP, INTERFACE-CONTRACT
- DESIGN:

## Change Unit 의존성
| ID | blocked_by | unblocks | parallelizable_with | merge risk |
|---|---|---|---|---|

## 구현 마이크로 계획 상세
- source alignment: current Task, active Change Unit, gates, related refs
- boundary: 기준 상태 아님, 범위 권한 아님, Approval 아님, Write Authorization 아님; active Change Unit이 범위의 기준 출처로 남음

### Step Queue(단계 대기열)
| Step | State Alignment | Scope Alignment / Likely Paths | Feedback Loop / TDD Status | Evidence Target | Stop Condition |
|---|---|---|---|---|---|

## Journey Spine(이어가기 spine)
### 적용 중인 사실
- fact / evidence ref:

### 적용 중인 assumptions
- assumption / expiry condition:

### 적용 중인 decisions
- DEC-0001:

### 적용 중인 domain terms
- term / meaning / code representation:

### Module / Interface 영향
- module / impact / interface / test boundary:

### 거절한 options
- option / reason / DEC:

### Watchpoints(주의 지점)
- regression:
- security/performance/operations:
- architecture drift:

### 이어가기 메모
- 다음 session이 알아야 할 것:
- 가장 먼저 해소할 막힘:
- 가장 작은 해소 방법:
<!-- HARNESS:END managed -->
````

Change Unit block 하위 템플릿:

````md
### CU-01 제목
- 목적:
- 목표가 아닌 것:
- slice type: vertical | enabling | cleanup | horizontal-exception
- horizontal exception 이유:
- 후속 vertical CU:
- autonomy profile:
- agent may do:
  - implementation detail:
  - local refactor inside scope:
  - evidence collection:
- user judgment required:
  - 제품/UX 판단:
  - 기술 판단:
  - 민감 동작 승인:
  - 작업 수락:
  - 잔여 위험 수용:
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
- 허용 path:
  - `src/...`
  - `tests/...`
- 허용 tool:
  - read
  - edit
  - shell: `npm test -- ...`
- 확인 profile:
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
- 민감 category:
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
- 의존성:
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

`TASK`의 Stewardship Impact는 owner 기록, validator 결과, 참조에서 파생되는 `StewardshipImpactSummary` 표시입니다. Domain Language, Module Map, Interface Contract, Feedback Loop, TDD Trace, 잔여 위험(residual risk), User Judgment owner 기록을 대체하지 않습니다.

`TASK`의 Implementation Micro-Plan은 현재 Task와 Change Unit 상태에서 생성되거나 그 상태와 정렬된 가벼운 실행 보조 정보입니다. [Projection And Templates Reference](../../projection-and-templates.md#projection-principles)의 projection/report 경계 안에 머물며, `prepare_write`나 owner state change를 대체하지 않습니다.

`TASK`의 검토 단계(Review Stages)는 Role Lens, playbook, two-stage review guidance를 위한 관리되는 표시 섹션입니다. 정확한 권한 없음 규칙은 [Design Quality Policies](../../design-quality-policies.md#two-stage-review-display)와 [Agent Integration](../../agent-integration.md#role-lens-동작)이 담당합니다. 기준 기록, `ProjectionKind` value, 민감 동작 승인, 근거, 검증, 수동 QA, 작업 수락, 잔여 위험 수용, close, Write Authorization을 만들지 않으며, 발견 사항은 기존 owner path로 연결해야 합니다.

생성된 summary는 사용자가 읽기 쉬운 평범한 말을 먼저 쓰고, 정확한 Harness term은 유용한 label이나 ref로 붙입니다. Projection이 명령어처럼 보이거나 표시 문구만으로 상태가 만들어진 것처럼 암시하면 안 됩니다.

Gate Group Summary는 읽는 사람이 raw gate detail보다 실제 막힘 이야기를 먼저 보도록 첫 managed section으로 둡니다. 범위, 사용자 판단, 근거, 닫기 준비 상태는 기존 owner 기록, gate, blocker, ref에서 파생되는 표시 그룹입니다. 기준 field, 정확한 gate value의 alias, 새 gate, recompute input, close semantics, authority path가 아닙니다. 사용자 판단은 구조화되어 있으며 하나의 넓은 판단 또는 승인 bucket처럼 렌더링하면 안 됩니다. 정확한 gate 값과 recompute rule은 [Core Model 참조](../../core-model.md#gates)가 담당하고, close 동작은 [`close_task`](../../core-model.md#close_task)가 담당합니다.

`TASK`의 User Judgment 표시는 기준 schema field와 렌더링 label을 분리해야 합니다. `judgment_type`은 내부 판단 유형이고, `presentation`은 compact 또는 full 표시 깊이를 제어하며, `display_label`은 제품/UX 판단, 기술 판단, 민감 동작 승인, 작업 수락, 잔여 위험 수용 중 하나입니다. `judgment_type` 예시는 `product_choice`, `technical_choice`, `sensitive_action_approval`, `work_acceptance`, `residual_risk_acceptance`입니다. Judgment가 여러 영역에 걸쳐 있으면 label을 배타적으로 다루지 말고 부차적인 고려사항을 장단점, 영향받는 gate, risk, evidence, follow-up에 렌더링해야 합니다. `judgment_category`, `judgment_route`, `display_depth` 같은 legacy field는 migration note나 compatibility drill-down에서만 나타날 수 있습니다. 이런 field는 새 payload branch selector, gate, status value, gate recompute input, close aggregation rule, authority path, `judgment_type`의 대체물이 아닙니다. `presentation` 또는 `display_label`에서 파생한 표시용 label은 validator input이 아니며 민감 동작 승인, 작업 수락, QA, 잔여 위험 수용, close, Write Authorization의 owner contract를 흐리면 안 됩니다.

대기 중인 사용자 판단은 한 줄로 합치면 안 됩니다. 민감 동작 승인, 작업 수락, 잔여 위험 수용이 모두 대기 중이면 세 가지 label로 세 항목을 렌더링합니다. Approval card는 작업 수락처럼 보이면 안 되고, 잔여 위험 수용은 수용하는 위험을 이름 붙여야 합니다.

`TASK`의 authority claim은 source ref 또는 명시적 absence로 해소되어야 합니다. Write authority claim은 Write Authorization ref를 가리킵니다. 민감 동작 permission은 minimum MVP-1에서는 `judgment_type=sensitive_action_approval`인 resolved `user_judgment`를 가리키고, later Approval profile이 active일 때만 Approval ref를 가리킵니다. Minimum MVP-1 근거 표시는 있을 때 `evidence_ref`, Run refs, ArtifactRefs, 보이는 gap summary를 가리킵니다. Active owner path가 full evidence sufficiency를 세울 수 없으면 full sufficiency를 주장하지 않아야 합니다. Full criteria-to-evidence sufficiency는 Evidence Manifest profile이 active일 때만 Evidence Manifest refs를 가리킵니다. 분리 검증은 해당 profile이 active일 때만 Eval refs를 가리킵니다. 수동 QA는 해당 profile이 active일 때 수동 QA records 또는 valid waiver refs를 가리킵니다. 작업 수락은 작업 수락 user judgment path를 가리킵니다. MVP-1에서 잔여 위험 표시는 blocker/user-judgment ref 또는 `ResidualRiskSummary.status=none`을 가리키고, rich Residual Risk ref는 해당 profile이 active일 때만 가리킵니다. MVP-1에서 잔여 위험 수용은 residual-risk acceptance user judgment와 관련 blocker/evidence ref를 가리키고, accepted Residual Risk ref는 해당 later profile이 active일 때만 가리킵니다. Ref가 없으면 completed authority가 아니라 missing support로 렌더링해야 합니다.

Residual-risk display는 `status=none`과 `not_visible`을 구분해야 합니다. `status=none`은 requested action에 대해 알려진 close-relevant residual risk가 없다는 뜻입니다. `not_visible`은 알려진 close-relevant risk가 있지만 작업 수락 또는 close에 충분히 보이지 않았다는 뜻이므로, risk와 refs가 보일 때까지 blocker 또는 next action으로 남아야 합니다.

`TASK`의 close와 assurance 표시는 self-checked work, `detached_verified`, 검증 면제 판단, QA 면제 판단, 잔여 위험 수용 close를 눈에 보이게 분리해야 합니다. 잔여 위험 수용 close는 MVP-1에서는 residual-risk acceptance user judgment와 관련 blocker/evidence ref를 가리키고, accepted Residual Risk ref는 해당 later profile이 active일 때만 가리켜야 합니다. 검증 면제 판단은 `verification_gate=waived_by_user`와 필요한 경우 그 user judgment를, QA 면제 판단은 `qa_gate=waived`, 수동 QA record 또는 waiver reason, 필요한 경우 QA 면제 판단 user judgment를 가리켜야 합니다.

`TASK`의 waiver 표시는 요약일 뿐입니다. 닫기에 영향을 주는 QA 또는 검증 면제 판단은 waiver를 유효하게 만드는 기존 기록을 가리켜야 합니다. QA 면제 판단은 `manual_qa_records`/`qa_gate=waived`와 필요한 경우 QA 면제 판단 user judgment를, 검증 면제 판단은 `verification_gate=waived_by_user`와 필요한 경우 그 user judgment를 가리킵니다. Policy 또는 gate, Task와 Change Unit, 생략한 확인이나 대상, reason, actor, 필요할 때 expiry 또는 잔여 위험 후속 작업, 관련 refs, 닫기 영향, 그리고 필요할 때 잔여 위험 경로로 보여주거나 수용해야 하는 close-relevant 잔여 위험도 함께 보여줘야 합니다. QA 면제 판단은 수동 QA가 되지 않고, 검증 면제 판단은 분리 검증을 만들지 않습니다.

`TASK`의 Close Summary는 진행 중이거나 최근 닫힌 `work` Task를 위한 이어가기 표시 요약입니다. Gate 상태나 잔여 위험을 숨기면 안 됩니다. 닫기가 성공했거나, 막혔거나, 취소됐거나, 잔여 위험 수용으로 닫혔을 때 changed scope, 민감 동작 승인, 근거, 검증, 수동 QA, 잔여 위험 표시, 잔여 위험 수용, 작업 수락, 면제 판단 상태, close reason, 잔여 위험 후속 작업을 해당되는 만큼 보여주고 owner record로 돌아가는 ref를 포함해야 합니다. 민감 동작 승인, 작업 수락, 잔여 위험 수용은 반드시 별도 줄로 유지합니다. 민감 동작 승인은 이름 붙은 민감 동작에 대한 허가이고, 작업 수락은 사용자의 result judgment이며, 잔여 위험 수용은 수용한 위험을 이름 붙이고 MVP-1에서는 residual-risk acceptance user judgment와 관련 blocker/evidence ref를 cite해야 합니다. Accepted Residual Risk ref는 해당 later profile이 active일 때만 cite합니다.

Close Summary는 민감 동작 승인, 근거, 검증, 수동 QA, 작업 수락, 잔여 위험 표시, 잔여 위험 수용을 하나의 "완료" flag로 합치면 안 됩니다. 테스트가 통과했지만 민감 동작 승인, 수동 QA, 작업 수락, 잔여 위험 수용이 pending이면 close display는 정확히 그 범주를 blocker로 보여줘야 합니다.

Direct 작업은 `DIRECT-RESULT`에서 가벼운 close impact summary를 보여주고, Journey Card close context는 간결한 status/resume 표시입니다. `TASK` Close Summary는 [projection/report 경계](../../projection-and-templates.md#projection-principles) 안의 이어가기 표시이며, close와 gate effect는 여전히 owner record에서 옵니다.

`TASK`, Journey, evidence, report section에 표시되는 artifact ref는 `redaction_state`를 보존해야 합니다. `secret_omitted` ref는 보이는 nonsecret 근거만 뒷받침할 수 있고, `blocked` ref는 원본 content가 아니라 committed metadata-only notice와 사용할 수 없는 입력을 보여줍니다.
