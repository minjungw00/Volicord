# EVAL 템플릿

## 사용 시점

검증 결과와 독립성 맥락을 함께 읽기 쉽게 보여줘야 할 때 `EVAL`을 사용합니다.

경계: 상태 보기 템플릿(projection template)일 뿐이며 하네스 서버/런타임 구현이나 생성된 운영 산출물에 권한을 주지 않습니다. 공통 단계와 상태 보기 규칙은 [템플릿 참조](README.md#사용-시점)를 따릅니다.

구현 계층: 향후/진단용 상태 보기(projection)입니다. Later verification 프로필의 상세 Evaluation 기록에 사용하며 간결한 검증 표시는 카드 형태를 사용할 수 있습니다.

## 기준 기록

- Eval(분리 검증 결과) 기록
- 검증 대상
- verdict
- 독립성 한정자
- 자체 확인(self-check)과 분리 검증 경계
- baseline 관계와 evaluator-bundle 최신성
- 수행한 확인
- 검토한 근거(evidence)
- 막힘
- 아티팩트 참조와 가림 상태, 입력 사용 가능성
- 표시되는 주장이 있을 때 관련 사용자 판단(User Judgment), 민감 동작 승인(Approval), 근거 목록(Evidence Manifest), 수동 QA, 작업 수락 맥락, 잔여 위험(Residual Risk), 아티팩트 참조, 가림 상태, 읽기용 보기 최신성(projection freshness)

## 렌더링 섹션

- 출처 참조
- 대상
- 판정
- 환경과 독립성
- 확인과 Validator 결과
- 검토한 근거
- 수용 기준 검토
- 설계 품질 검토
- 근거 설명
- 막힘 또는 재작업
- 가림과 사용 가능성
- 사용자 후속 조치

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

# EVAL-0001 검증 결과

> 상태 보기(Projection): `source_state_version`와 `updated_at` 기준으로 렌더링되며 Eval 상태와 검토한 ref를 표시합니다. Verdict, assurance, gate effect는 Eval과 Core gate record를 통해서만 바뀝니다.

## 출처 참조
- 근거 목록(Evidence Manifest):
- 사용자 판단:
- 민감 동작 승인(Approval):
- 수동 QA:
- 작업 수락 맥락:
- 잔여 위험(Residual Risk):
- 아티팩트 참조:
- 가림 상태:
- 보기 최신성:

## 대상
- task_id:
- change_unit_id: CU-01 | null
- target_run_id:
- evaluator_run_id:

## 판정
- verdict: passed | failed | blocked | inconclusive
- assurance 영향:
- verification gate 영향:
- 분리 검증 후보 상태:
- 자체 확인과 분리 검증 경계:
- 수동 QA 영향:
- 작업 수락 영향:
- 다음 행동:

## 환경과 독립성
- 새 실행:
- evaluator 접점:
- 맥락 독립성: same_session | subagent_context | fresh_session | fresh_worktree | sandbox | manual_bundle
- 같은 세션 자체 검토 guard:
- 쓰기 가능 여부:
- 제품 파일 쓰기 허용 여부:
- baseline 확인:
- bundle 최신성:
- repo drift 관찰:
- 출처 입력: chat_history | task_summary | bundle | allowed_raw_artifacts | refs_with_redaction_notes
- 출처 번들:
- 상위 run:

## 확인과 Validator 결과
### Core 확인과 전제 조건
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

## 검토한 근거
- Task 요약:
- Journey Spine:
- 사용자 판단:
- 잔여 위험(Residual Risk):
- 자율성 경계(Autonomy Boundary):
- domain term 참조:
- module map item 참조:
- interface contract 참조:
- 실행 요약:
- feedback loop:
- TDD trace:
- 수동 QA:
- 근거 목록(Evidence Manifest):
- diff:
- 번들:
- 로그:
- 아티팩트 참조만 포함하며 큰 근거 본문은 포함하지 않음:
- 민감 동작 승인 참조(approvals; later Approval 프로필이 활성화된 경우에만; 그 외에는 none):
- 판단 참조(decisions):

## 가림과 사용 가능성
| 아티팩트 참조 | 가림 상태 | 검증 영향 | 메모 |
|---|---|---|---|
| ART-EVAL-0001 | secret_omitted | 보이는 nonsecret fact 검토; 생략된 값은 증명 안 됨 | |
| ART-EVAL-0002 | blocked | 사용할 수 없는 입력; verdict가 원본 payload에 의존하면 안 됨 | |

## 수용 기준 검토
| AC ID | 진술 | 검토한 근거 | 결과 | 메모 |
|---|---|---|---|---|

## 설계 품질 검토
- vertical slice:
- 사용자 판단:
- 자율성 경계(Autonomy Boundary):
- Residual Risk:
- feedback loop:
- TDD trace:
- module/interface:
- 아키텍처 drift:
- 도메인 언어 일관성:

## 근거 설명
-

## 막힘 또는 재작업
-

## 사용자 후속 조치
- 확인이 필요한 trade-off:
- 남은 선택지:
- 수동 QA 필요성:
````

## 메모

Eval 판정(verdict)만으로는 assurance를 높일 수 없습니다. `detached_verified`에는 유효한 독립성, 통과한 검증, 최신 baseline과 bundle 입력, 같은 세션 자체 검토 위반 부재가 필요합니다.

독립성이 유효하지 않거나 같은 세션 자체 확인(self-check)에 그치는 review라면 그 경계를 명시하고 detached assurance는 그대로 둡니다. `subagent_context` review는 기본적으로 detached가 아닙니다. 기록된 context가 `fresh_session`, `fresh_worktree`, `sandbox`, `manual_bundle` 요구를 충족할 때만 detached candidate로 렌더링합니다.

Evaluator bundle, baseline, 포함된 artifacts, 근거 목록(Evidence Manifest), approval/user judgment refs, close-relevant Residual Risk refs가 오래되었으면 오래된 입력으로 렌더링하고 replacement 또는 compatible re-verification이 기록될 때까지 assurance를 그대로 둡니다.

Eval(분리 검증 결과) 상태 보기는 생략되었거나 차단된 원본 bytes를 검토한 것처럼 암시하면 안 됩니다. `secret_omitted` 근거는 보이는 nonsecret claim만 뒷받침할 수 있습니다. Eval이 `blocked` payload에 의존한다면 replacement, waiver, user judgment outcome, 받아들인 위험, documented fallback이 verification 경로를 해소할 때까지 result는 `blocked` 또는 `inconclusive`로 남거나 `EVIDENCE_INSUFFICIENT`를 반환해야 합니다.

Eval(분리 검증 결과) template은 검토한 근거 참조를 간결하게 유지해야 합니다. 큰 로그, bundle, screenshot, diff, trace는 가림 상태와 사용 가능성이 있는 ArtifactRef ref로 남깁니다. Eval 본문은 무엇을 검토했는지 기록할 뿐이며 원본 evidence payload를 붙여 넣지 않습니다.
