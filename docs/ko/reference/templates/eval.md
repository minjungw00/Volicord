# EVAL Template

## 사용 시점

검증 결과와 독립성 맥락을 함께 읽기 쉽게 보여줘야 할 때 `EVAL`을 사용합니다.

이 문서는 template 참조 문서입니다. 재설계 문서가 승인되기 전에는 runtime/server 구현, 생성된 운영 파일, 실행 가능한 fixture 파일, runtime data를 만들라는 뜻이 아닙니다. 첫 구현/증명 대상은 계속 Kernel Smoke입니다. Agency-Hardened MVP와 post-MVP automation은 owner 문서가 승격하고 증명하기 전까지 범위 밖입니다.

## 기준 기록

- Eval 기록
- verification target
- verdict
- independence qualifier
- 자체 확인(self-check)과 detached verification 경계
- baseline relationship과 evaluator-bundle freshness
- performed check
- 검토한 근거(evidence)
- blocker
- artifact ref와 redaction state, input availability

## 렌더링 섹션

- Target
- Verdict
- Environment And Independence
- Checks And Validator Outcomes
- Evidence Reviewed
- Acceptance Criteria Review
- Design Quality Review
- Rationale
- Blockers Or Rework
- Redaction And Availability
- User Follow-Up

## 전체 템플릿

````md
---
doc_type: eval
eval_id: EVAL-0001
task_id: TASK-0001
change_unit_id: CU-01
verdict: passed
surface_id: reference
source_state_version: 45
updated_at: 2026-05-06T10:05:00+09:00
---

# EVAL-0001 Verification Result

> Projection 보기: `source_state_version`와 `updated_at` 기준으로 렌더링되며 Eval state와 검토한 ref를 표시합니다. Verdict, assurance, gate effect는 Eval과 Core gate record를 통해서만 바뀝니다.

## Target
- task_id:
- change_unit_id: CU-01 | null
- target_run_id:
- evaluator_run_id:

## Verdict
- verdict: passed | failed | blocked | inconclusive
- assurance impact:
- verification gate impact:
- detached candidate status:
- self-check vs detached boundary:
- Manual QA impact:
- acceptance impact:
- next action:

## Environment And Independence
- fresh run:
- evaluator surface:
- context independence: same_session | subagent_context | fresh_session | fresh_worktree | sandbox | manual_bundle
- same-session self-review guard:
- write capable:
- product file write allowed:
- baseline verified:
- bundle freshness:
- repo drift observed:
- source input: chat_history | task_summary | bundle | allowed_raw_artifacts | refs_with_redaction_notes
- source bundle:
- parent run:

## Checks And Validator Outcomes
### Core Checks And Preconditions
- [ ] changed_paths
- [ ] approval_scope
- [ ] same_session_verify_guard
- [ ] evidence_sufficiency
- [ ] bundle_integrity
- [ ] acceptance_review
- [ ] baseline_freshness
- [ ] public_interface_change_review
- [ ] lint
- [ ] test
- [ ] build

### ValidatorResult IDs
- [ ] vertical_slice_shape
- [ ] shared_design_alignment
- [ ] decision_quality_check
- [ ] autonomy_boundary_check
- [ ] feedback_loop_check
- [ ] tdd_trace_required
- [ ] domain_language_consistency
- [ ] module_interface_review
- [ ] codebase_stewardship_check
- [ ] residual_risk_visibility_check
- [ ] manual_qa_required
- [ ] surface_capability_check

## Evidence Reviewed
- task summary:
- Journey Spine:
- Decision Packets:
- Residual Risks:
- Autonomy Boundary:
- domain term refs:
- module map item refs:
- interface contract refs:
- run summary:
- feedback loop:
- TDD trace:
- Manual QA:
- evidence manifest:
- diff:
- bundle:
- logs:
- approvals:
- decisions:

## Redaction And Availability
| Artifact Ref | Redaction State | Verification Effect | Note |
|---|---|---|---|
| ART-EVAL-0001 | secret_omitted | 보이는 nonsecret fact 검토; 생략된 값은 증명 안 됨 | |
| ART-EVAL-0002 | blocked | 사용할 수 없는 입력; verdict가 원본 payload에 의존하면 안 됨 | |

## Acceptance Criteria Review
| AC ID | Statement | Evidence Reviewed | Result | Notes |
|---|---|---|---|---|

## Design Quality Review
- vertical slice:
- Decision Packets:
- Autonomy Boundary:
- Residual Risks:
- feedback loop:
- TDD trace:
- module/interface:
- architecture drift:
- domain language consistency:

## Rationale
-

## Blockers Or Rework
-

## User Follow-Up
- trade-off needing confirmation:
- remaining options:
- Manual QA need:
````

## 메모

Eval verdict만으로는 assurance를 높일 수 없습니다. `detached_verified`에는 valid independence, passed verification, current baseline and bundle inputs, same-session self-review violation 부재가 필요합니다.

Independence가 유효하지 않거나 같은 세션 자체 확인(self-check)에 그치는 review라면 그 경계를 명시하고 detached assurance는 그대로 둡니다. `subagent_context` review는 기본적으로 detached가 아닙니다. 기록된 context가 `fresh_session`, `fresh_worktree`, `sandbox`, `manual_bundle` 요구를 충족할 때만 detached candidate로 렌더링합니다.

Evaluator bundle, baseline, included artifacts, Evidence Manifest, approval/Decision Packet refs, close-relevant Residual Risk refs가 stale이면 stale input을 렌더링하고 replacement 또는 compatible re-verification이 기록될 때까지 assurance를 그대로 둡니다.

Eval projection은 생략되었거나 차단된 원본 bytes를 검토한 것처럼 암시하면 안 됩니다. `secret_omitted` evidence는 보이는 nonsecret claim만 뒷받침할 수 있습니다. Eval이 `blocked` payload에 의존한다면 replacement, waiver, Decision Packet outcome, 받아들인 위험, documented fallback이 verification 경로를 해소할 때까지 result는 `blocked` 또는 `inconclusive`로 남거나 `EVIDENCE_INSUFFICIENT`를 반환해야 합니다.
