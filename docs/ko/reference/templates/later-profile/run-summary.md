# RUN-SUMMARY 템플릿

## 사용 시점

`record_run`으로 실행(Run)이 기록된 뒤, 무엇을 실행했고 무엇이 바뀌었는지, 확인 또는 검증기가 무엇을 보고했는지, 원본 근거가 어떤 아티팩트에 남았는지 요약해야 할 때 `RUN-SUMMARY`를 사용합니다.

경계: 상태 보기 템플릿(projection template)일 뿐이며 하네스 서버/런타임 구현이나 생성된 운영 산출물에 권한을 주지 않습니다. 공통 단계와 상태 보기 규칙은 [템플릿 참조](README.md#사용-시점)를 따릅니다.

구현 계층: 향후/진단용 상태 보기(projection)입니다. 나중 프로필을 위한 상세 실행(Run) 보기로 유지하며 초기 필수 범위가 아닙니다.

## 기준 기록

- 실행(Run) 기록
- 행위자/접점 식별 정보
- 기준선
- 작업 조각(Change Unit)
- compatible product-write Run이 소비한 경우의 쓰기 허가 기록(Write Authorization) 참조. Attempted invalid authorization ref는 violation/audit context에만 둡니다.
- 변경된 경로
- 명령 결과
- 검증기 결과
- 기록된 경우 기존 owner 참조로 연결된 검토 단계 표시 발견 사항
- 아티팩트 참조
- 근거 업데이트와 후속 작업

## 렌더링 섹션

- 실행(Run) 식별 정보
- 범위
- 변경된 파일
- 명령과 확인
- 확인과 검증기 결과
- 검토 단계
- TDD 트레이스 요약
- 주요 변경
- 이슈와 후속 조치
- 이어가기 축(Journey Spine) 업데이트
- 근거 참조

## 전체 템플릿

````md
---
doc_type: run_summary
run_id: RUN-20260506-093015-LEAD-01
task_id: TASK-0001
change_unit_id: CU-01
profile: lead
kind: implementation
surface_id: reference
source_state_version: 43
updated_at: 2026-05-06T09:45:10+09:00
---

# RUN-SUMMARY

> 상태 보기(Projection): `source_state_version`와 `updated_at` 기준으로 렌더링되며 커밋된 실행(Run)과 아티팩트 참조를 표시합니다. 이 문서를 편집해도 실행(Run), 근거, 관문, `state.sqlite.task_events`는 바뀌지 않습니다.

## 실행(Run) 식별 정보
- run_id:
- 행위자 유형:
- 접점:
- baseline_ref:
- state_version:
- status:

## 범위
- task_id:
- change_unit_id:
- 조각 유형:
- 쓰기 허가 기록:
- 허용 경로:
- 허용 도구:
- 허용 명령:
- 허용 네트워크 대상:
- 비밀 정보 범위:
- 민감 범주:
- 민감 동작 승인 참조(나중의 민감 동작 승인(Approval) 프로필이 활성화된 경우에만; 그 외에는 none):

## 변경된 파일
- `path/to/file`

## 명령과 확인
```bash
npm test -- --runInBand
```

## 확인과 검증기 결과
### Core 확인과 명령 확인
- changed_paths:
- approval_scope:
- lint:
- test:
- build:
- evidence_sufficiency:

### ValidatorResult IDs
- vertical_slice_shape:
- shared_design_alignment:
- decision_quality_check:
- autonomy_boundary_check:
- feedback_loop_check:
- tdd_trace_required:
- domain_language_consistency:
- module_interface_review:
- codebase_stewardship_check:
- residual_risk_visibility_check:
- manual_qa_required:

## 검토 단계
- 메모: 실행 로컬(run-local) 검토 표시 전용입니다. 기록(Record), `ProjectionKind` value, 민감 동작 승인(Approval), 근거, 검증, QA, 작업 수락, 잔여 위험 수용, 닫기, 쓰기 허가 기록(Write Authorization)을 만들지 않습니다. 검토 단계(review-stage) 경계는 [설계 품질 정책(Design Quality Policies)](../../design-quality-policies.md#two-stage-review-display)이 담당합니다. 발견 사항은 기존 참조, 관문, 막힘으로 연결합니다.
- 쓰기 권한 메모: attempted invalid authorization ref는 validator finding, violation payload, event payload에만 나타날 수 있습니다. Consumed Write Authorization이 아니며 completion evidence로 쓰면 안 됩니다.

### 명세 준수 검토
- 수용 기준 뒷받침 범위:
- 작업 조각(Change Unit) 완료 조건:
- 범위 / 쓰기 권한 호환성:
- 사용자 판단 호환성:
- 근거 뒷받침 범위:
- 잔여 위험 표시:
- 결과 참조(기존 경로/참조만):

### 코드 품질 / 스튜어드십 검토
- 도메인 언어:
- 모듈 / 인터페이스 경계:
- 수직 조각(vertical slice) 형태:
- 피드백 루프 / TDD:
- 코드베이스 스튜어드십:
- 맥락 정돈:
- 후속 위험:
- 결과 참조(기존 경로/참조만):

## TDD 트레이스 요약
- 필수 여부:
- 피드백 루프 참조:
- RED 대상 / 계획:
- RED 근거(실제):
- GREEN 근거:
- 리팩터링(refactor) 메모:
- 면제 / 대체 루프(loop):
- trace 참조:

## 주요 변경
-

## 이슈와 후속 조치
-

## 이어가기 축(Journey Spine) 업데이트
- 새 사실:
- 거절된 선택지:
- 도메인 언어 업데이트:
- 모듈/인터페이스 업데이트:
- 주의 지점 변경:
- 다음 실행(Run)이 알아야 할 것:

## 근거 참조
- 근거 목록(Evidence Manifest):
- TDD 트레이스:
- 수동 QA:
- 변경 차이:
- 로그:
- 번들:
- 체크포인트:
- 생략되거나 차단된 아티팩트 영향:
````

## 메모

원본 로그와 변경 차이는 아티팩트로 남기고, 보고서에는 링크만 둡니다. `RUN-SUMMARY`에 담긴 같은 세션 검토(review) 내용은 자체 확인(self-check) 또는 스튜어드십 신호로만 취급하며 [review-stage 경계](../../design-quality-policies.md#two-stage-review-display)를 따릅니다. 발견 사항은 기존 관문, 사용자 판단, 근거, Eval(분리 검증 결과), 수동 QA, 잔여 위험(Residual Risk), 민감 동작 승인(Approval), 작업 조각(Change Unit) 업데이트, 닫기 막힘(close-blocker) 참조로 연결하며, 보고서 자체가 그 기록이나 권한을 만들지는 않습니다.

이 보고서의 근거 참조는 `redaction_state`를 보존해야 합니다. `secret_omitted` 참조는 보이는 비밀 정보가 아닌 근거만 뒷받침할 수 있고, `blocked` 참조는 원본 로그, 변경 차이, 스크린샷, 번들(bundle)이 아니라 사용할 수 없는 입력을 표시하는 커밋된 메타데이터 전용 알림(metadata-only notice)입니다.
