# MANUAL-QA 템플릿

## 사용 시점

수동 QA가 `required`, `performed`, `waived`, `pending` 상태이거나 `qa_gate`에 반영되어 있고 해당 기록을 읽기 쉬운 상태 보기로 볼 때 `MANUAL-QA`를 사용합니다.

경계: 상태 보기 템플릿(projection template)일 뿐이며 하네스 서버/런타임 구현이나 생성된 운영 산출물에 권한을 주지 않습니다. 공통 단계와 상태 보기 규칙은 [템플릿 참조](README.md#사용-시점)를 따릅니다.

구현 계층: 보증 프로필 보고서입니다. 수동 QA 기록 또는 활성 QA 프로필이 있을 때만 렌더링하며 활성 MVP-1 작은 출력 세트의 일부가 아닙니다.

## 기준 기록

- `manual_qa_records`
- Task와 작업 조각(Change Unit) 참조
- `qa_gate`
- 수동 QA 프로필, 준비 사항, 확인 목록, 결과, 면제, 발견 사항
- 사람 검사자 또는 역할과 확인한 품질이나 워크플로
- 스크린샷, 브라우저 로그, `qa_capture`, 비디오, 워크플로 기록, 수동 제공 메모 아티팩트 참조와 `redaction_state`
- QA 면제 또는 실패와 관련된 면제 사유, 필요한 경우 QA 면제 사용자 판단 참조, 잔여 위험(Residual Risk) 참조
- 표시되는 주장이 있을 때 근거 목록(Evidence Manifest), Eval(분리 검증 결과), 최종 수락 맥락, 민감 동작 승인(Approval), 아티팩트 참조, `redaction_state`, 읽기용 보기 최신성(projection freshness)
- `manual_qa` 관련 design-quality 검증기 결과
- 읽기용 보기 최신성(projection freshness) 입력

## 렌더링 섹션

- 식별 정보
- 권한과 닫기 참조
- 준비 사항
- 확인 목록
- 결과
- 면제와 위험
- 발견 사항
- 근거 참조
- 가림과 사용 가능성

## 전체 템플릿

````md
---
doc_type: manual_qa
manual_qa_record_id: null
task_id: TASK-0001
change_unit_id: CU-01
qa_gate: pending
result: null
source_state_version: 45
updated_at: 2026-05-06T10:05:00+09:00
---

# 수동 QA

> 상태 보기(Projection): `source_state_version`와 `updated_at` 기준으로 렌더링되며 수동 QA 기록과 `qa_gate`를 표시합니다. QA 결과와 QA 면제는 `manual_qa_records`와 `qa_gate`에 기록됩니다. 제품/사용자 위험이 있는 QA 면제는 연결된 QA 면제 사용자 판단을 사용하고, 잔여 위험 수락은 잔여 위험(Residual Risk) 참조에 기록됩니다. 브라우저 QA 아티팩트는 뒷받침 참조일 뿐이며 사람이 하는 수동 QA 판단, 최종 수락, 분리 검증을 대체하지 않습니다.

## 식별 정보
- manual_qa_record_id: QA-0001 | null
- task_id:
- change_unit_id: CU-01 | null
- qa_profile: ui_quality | workflow | copy | accessibility | browser_smoke | performance_smoke | other
- required: yes | no
- 수행한 사람:

## 권한과 닫기 참조
- 수동 QA 기록:
- QA 면제 사용자 판단:
- 근거 목록(Evidence Manifest):
- Eval(분리 검증 결과):
- 민감 동작 승인(Approval):
- 최종 수락 맥락:
- 잔여 위험(Residual Risk):
- 아티팩트 참조:
- `redaction_state`:
- 보기 최신성:

## 준비 사항
- 빌드/실행 명령:
- 테스트 계정/데이터:
- 경로 또는 화면:
- 브라우저 캡처 지원: supported | unsupported | not applicable

## 확인 목록
- [ ] 주요 워크플로가 동작함
- [ ] 오류가 이해 가능함
- [ ] 시각적 레이아웃이 수용 가능함
- [ ] 접근성 스모크 확인
- [ ] 명백한 회귀 없음

## 결과
- 기록 결과: passed | failed | waived | 기록이 없으면 null
- qa_gate: not_required | required | pending | passed | failed | waived
- qa_gate 메모: 기준 닫기 관련 관문이며, 이 상태 보기는 표시 전용
- QA 면제 표시: `qa_gate=waived`와 수동 QA 기록 또는 면제 사유, 필요한 경우 QA 면제 사용자 판단
- 자동 확인 상태: {뒷받침 참조만; 수동 QA 결과 아님}
- 검증 상태: {별도 Eval(분리 검증 결과)/관문 상태; 이 템플릿이 만들지 않음}
- 최종 수락 상태: {별도 사용자 판단; 이 템플릿이 만들지 않음}
- 사람의 확인 요약:
- 요약:
- 면제 사유:

## 면제와 위험
- 면제 기록:
- QA 면제 사용자 판단:
- 생략한 확인 또는 대상:
- 면제 전에 표시된 위험:
- 수락하는 위험:
- 후속 작업:
- 잔여 위험(Residual Risk) 참조:
- 수락한 잔여 위험(Residual Risk) 요약:
- 닫기 영향:

## 발견 사항
| 심각도 | 발견 사항 | 제안 조치 | 후속 CU |
|---|---|---|---|
| minor | | | |

## 근거 참조
- 스크린샷:
- qa_capture:
- 브라우저 로그:
- 비디오:
- 메모:
- 수동 제공 아티팩트:
- 지원되지 않는 접점의 대체 메모:

## 가림과 사용 가능성
| 아티팩트 참조 | `redaction_state` | QA 영향 | 메모 |
|---|---|---|---|
| ART-QA-0001 | secret_omitted | 관찰 가능한 발견 사항만 지원 | |
| ART-QA-0002 | blocked | 캡처 사용 불가; 대체되거나 유효하게 면제되기 전까지 QA 경로는 미해결이며 `qa_gate`는 상황에 따라 `pending`/`failed` 또는 `waived` | |
````

## 메모

이 템플릿은 렌더링 결과일 뿐 기준 상태가 아닙니다. `qa_gate`가 기준 닫기 관련 관문이며, 이 상태 보기는 그 값을 표시만 합니다.

수동 QA 표시는 `passed` 수동 QA 기록, `failed` 수동 QA 기록, `pending required` QA, QA 면제를 눈에 띄게 구분해야 합니다. `qa_gate=waived`는 필요한 경우 참조와 수락한 위험/후속 작업을 동반하는 면제 표시입니다. 통과한 수동 QA 결과, 최종 수락, 분리 검증이 아닙니다.

수동 QA는 자동 검증이 아닙니다. 테스트 결과, 브라우저 스모크, 스크린샷, 브라우저 QA 아티팩트는 사람의 확인 맥락을 뒷받침할 수 있지만, 수동 QA 담당 경로가 결과 또는 유효한 면제를 기록하지 않았다면 이 템플릿은 이를 수동 QA 통과처럼 렌더링하면 안 됩니다.

수동 QA 상태 보기는 안전한 생략 메모, 핸들(handle), 차단된 아티팩트 알림(blocked artifact notice)을 보여줄 수 있지만 생략된 비밀 정보/PII 값이나 차단된 캡처 페이로드(payload)를 포함하면 안 됩니다. `secret_omitted` 아티팩트는 보이는 워크플로, UI, 문구(copy), 접근성, 스모크 테스트 관찰을 뒷받침할 수 있습니다. `blocked` 캡처는 대체 근거, 면제, 사용자 판단 결과, 수락한 위험, 문서화된 대체 경로(fallback)가 QA 경로를 해소하지 않는 한 사용할 수 없는 QA 입력입니다.

스크린샷, 브라우저 로그, 비디오, `qa_capture` 출력, 워크플로 기록, 메모는 QA 근거 참조입니다. 브라우저 QA 캡처(Browser QA Capture)는 owner 문서가 명시적으로 승격하고 증명하기 전까지 로드맵 후보입니다. 수동 QA 결과는 기록된 사람의 확인 또는 유효한 면제이지, 이런 캡처가 존재한다는 사실만으로 만들어지지 않습니다. 브라우저 QA 아티팩트는 별도의 Eval(분리 검증 결과) 경로가 검증 독립성을 충족하지 않는 한 최종 수락 또는 분리 검증도 기록하지 않습니다. 어떤 접점이 브라우저 캡처를 지원하지 않으면 사람이 작성한 수동 QA 메모와 수동 제공 아티팩트를 대체 경로(fallback)로 사용합니다.
