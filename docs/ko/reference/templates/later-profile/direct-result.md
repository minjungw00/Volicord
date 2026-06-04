# DIRECT-RESULT 템플릿

## 사용 시점

작은 직접 작업이 닫혔거나 `work`로 전환된 뒤 결과를 간결하고 부담 없이 보여줘야 할 때 `DIRECT-RESULT`를 사용합니다. 전체 Task gate 보고서가 아니라 직접 결과처럼 읽혀야 합니다.

경계: 상태 보기 템플릿(projection template)일 뿐이며 하네스 서버/런타임 구현이나 생성된 운영 산출물에 권한을 주지 않습니다. 공통 단계와 상태 보기 규칙은 [템플릿 참조](README.md#사용-시점)를 따릅니다.

구현 계층: 향후/진단용 상태 보기(projection)입니다. 해당 프로필이 활성화된 경우 선택적 간결 직접 작업 결과 표시로 사용하며 MVP-1 사용자 작업 루프 상태 보기나 첫 커널 증명에는 필요하지 않습니다.

## 기준 기록

- 직접 실행 기록
- 직접 작업에 제품 파일 쓰기가 있었다면 소비된 쓰기 허가 기록(Write Authorization) 참조
- 변경 경로
- 범위 밖 또는 유지된 범위 요약
- 실행한 확인
- 표시되는 주장이 있을 때 사용자 판단(User Judgment) 참조, 민감 동작 승인 사용자 판단 참조, later 민감 동작 승인(Approval) 참조, `evidence_ref` 참조와 파생 근거 요약, 전체 근거 프로필이 활성화된 경우의 근거 목록(Evidence Manifest), Eval(분리 검증 결과), 수동 QA, 작업 수락 사용자 판단 참조, Residual Risk, 아티팩트 참조
- 가림 상태와 사용 가능성을 포함한 artifact 참조
- 읽기용 보기 최신성(projection freshness) 입력
- escalation flag
- 닫기 보장 수준
- 해당되는 경우 근거, 검증, 수동 QA, 작업 수락, 잔여 위험 표시, 잔여 위험 수용 관련 닫기 영향 요약

닫기 요약 줄은 기존 gate와 owner-record ref에서 파생한 표시 전용 요약입니다. 직접 작업은 자신이 요약하는 기록 밖에 별도의 close field를 만들지 않습니다.

## 렌더링 섹션

- 요청
- 범위
- 결과
- 변경된 범위
- 확인
- 보장 수준(Assurance)
- 권한 참조
- 닫기 영향 요약
- 전환
- 근거 참조
- 보기 최신성

## 전체 템플릿

````md
---
doc_type: direct_result
task_id: TASK-0001
run_id: RUN-20260506-093015-LEAD-01
result: passed
assurance_level: self_checked
surface_id: reference
source_state_version: 41
updated_at: 2026-05-06T09:40:00+09:00
---

# DIRECT-RESULT

> 상태 보기(Projection): `source_state_version`와 `updated_at` 기준으로 렌더링되며 직접 Run 결과를 표시합니다. 이 문서를 편집해도 result, assurance, escalation, close state는 바뀌지 않습니다.

## 요청
- 사용자 요청:

## 범위
- 직접 실행 범위:
- 제한:
- 쓰기 허가 기록 참조:
- 허용 path:
- 민감 동작 승인 사용자 판단 참조(minimum MVP-1, 해당되는 경우):
- 민감 동작 승인 참조(later Approval 프로필이 활성화된 경우에만; 그 외에는 none):

## 결과
- 결과 요약:
- 닫기 이유:

## 변경된 범위
- 변경된 파일: `path/to/file`
- 파일 변경 없는 결과:
- 범위 밖 유지:

## 확인
- 자체 확인:
- tests/build:
- validator 결과:
- 아티팩트 참조와 가림 상태:
- 아티팩트 사용 가능성:

## 보장 수준(Assurance)
- assurance_level:
- 의미:
- 분리 검증 필요:
- 자체 확인 참조:
- 분리 검증 Eval ref:
- 검증 면제 ref:
- QA waiver 참조:
- 위험 수용 닫기 참조:

## 권한 참조
- 쓰기 허가 기록:
- 사용자 판단:
- 민감 동작 승인 사용자 판단 / Approval 참조:
- 근거 참조 / 파생 요약:
- 근거 목록(전체 근거 프로필이 활성화된 경우에만):
- Eval(분리 검증 결과):
- 수동 QA:
- 작업 수락 사용자 판단:
- 잔여 위험(Residual Risk):
- 아티팩트 참조:
- 가림 상태:
- 보기 최신성:

## 닫기 영향 요약
- 표시 상태 라벨(일반 문구, schema value 아님):
- 근거:
- 검증:
- 수동 QA:
- 작업 수락:
- 잔여 위험 표시:
- 잔여 위험 수용:
- 검증 면제 ref:
- QA waiver 참조:
- 후속 작업:

## 전환
- escalated_to_work: yes | no
- 이유:

## 근거 참조
- 로그:
- diff:
- 후속 보고서:
- 생략/차단 artifact 영향:

## 보기 최신성
- 최신성:
- source_state_version:
- stale 또는 reconcile 영향:
````

## 메모

정책 또는 사용자가 분리 검증 또는 다른 gate를 요구하지 않으면 직접 작업은 기본적으로 자체 확인(self-checked) 상태로 닫힐 수 있습니다. 소비된 쓰기 허가 기록(Write Authorization) 참조를 표시할 수 있지만, 상태 보기가 기준 허가 기록이 되는 것은 아닙니다.

직접 작업 결과(Direct Result)는 self-checked, `detached_verified`, verification-waived, QA-waived, risk-accepted-close 상태를 별도 줄로 표시해야 합니다. Waiver 줄은 waiver 참조를 가리키거나 아직 기록되지 않았다고 말하며, verification 또는 QA가 되지 않습니다. Risk-accepted close는 detached verified처럼 보이지 않게, MVP-1에서는 residual-risk acceptance user judgment와 관련 blocker/evidence ref를 가리키고, accepted Residual Risk ref는 해당 later 프로필이 활성화된 경우에만 가리킵니다.

직접 작업 결과(Direct Result)의 checks와 tests는 근거 또는 자체 확인(self-check) 맥락입니다. 조건을 충족하는 Eval(분리 검증 결과) 없이는 분리 검증이 되지 않고, 수동 QA 결과 또는 유효한 waiver 없이는 수동 QA가 되지 않으며, 작업 수락을 암시하지도 않습니다. 직접 작업이 잔여 위험 수용으로 닫힌다면 닫기 영향 요약은 결과를 detached verified처럼 보여주는 대신 residual-risk acceptance user judgment, 관련 blocker/evidence ref, 프로필이 활성화된 경우의 later accepted Residual Risk ref, 후속 작업을 가리켜야 합니다. 알려진 닫기 관련 위험이 없다면 gate 목록을 덧붙이기보다 그 사실을 직접 말합니다.

직접 작업 결과(Direct Result)의 권한 주장은 출처 참조 또는 명시적인 부재를 인용해야 합니다. 쓰기 허가는 쓰기 허가 기록(Write Authorization)을 사용합니다. Minimum MVP-1 민감 동작 허가에는 `judgment_type=sensitive_action_approval`인 해소된 `user_judgment`를 사용하고, later Approval 프로필이 활성화된 경우에만 Approval ref를 사용합니다. MVP-1 근거 표시는 있을 때 `evidence_ref`, Run 참조, ArtifactRef 참조, 보이는 공백 요약을 사용합니다. Result가 전체 기준-근거 충분성을 주장하고 전체 근거 프로필이 활성화된 경우에만 근거 목록(Evidence Manifest)을 사용합니다. 분리 검증은 해당 프로필이 활성화된 경우 Eval(분리 검증 결과)을, QA는 해당 프로필이 활성화된 경우 수동 QA record 또는 waiver path를, 작업 수락은 작업 수락 user judgment path를 사용합니다. MVP-1 잔여 위험 표시는 blocker/user-judgment ref 또는 `ResidualRiskSummary.status=none`을 사용하고, MVP-1 잔여 위험 수용은 residual-risk acceptance user judgment와 관련 blocker/evidence ref를 사용합니다. Rich Residual Risk ref는 해당 later 프로필이 활성화된 경우에만 사용합니다. `not_visible` 잔여 위험을 "none"처럼 렌더링하면 안 됩니다.

`DIRECT-RESULT`는 직접 작업을 위한 가벼운 닫기 영향 표시입니다. `TASK`는 진행 중이거나 최근 닫힌 `work` Task의 이어가기용 닫기 요약 표시를 담당하고, Journey Card 닫기 맥락은 간결한 상태/이어가기 표시입니다. 이 표시들은 [projection/report 경계](../../projection-and-templates.md#projection-principles)를 따르며, 닫기와 gate effect는 여전히 owner record에서 옵니다.

직접 작업 결과(Direct Result)의 ArtifactRef는 `redaction_state`를 보이게 유지해야 합니다. `secret_omitted`는 보이는 nonsecret 근거만 뒷받침하고, `blocked`는 replacement, waiver, user judgment outcome, 받아들인 위험, documented fallback으로 해소될 때까지 원본 입력을 사용할 수 없다는 뜻입니다.
