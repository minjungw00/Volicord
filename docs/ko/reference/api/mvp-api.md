# 현재 MVP API

## 담당하는 것

이 문서는 현재 MVP API 메서드 묶음의 안정적인 경로 문서입니다. 담당하는 것은 아래와 같습니다.

- 활성 공개 API 메서드 목록
- 메서드 담당 문서 경로
- 공통 요청 래퍼와 응답 분기를 읽는 규칙
- 스키마 담당 문서 링크
- 저장 효과 담당 문서 링크
- 메서드 담당 문서가 사용하는 안정적인 API 예시 시나리오 요약

## 담당하지 않는 것

이 문서는 아래 항목을 담당하지 않습니다.

- 메서드별 필수 입력, 접근 요구사항, 결과 필드, `dry_run` 동작, 대표 요청과 응답 본문
- 공통 API 요청 래퍼와 응답 스키마 본문
- 상태, 아티팩트, 사용자 판단, 값 집합, 오류 스키마 정의
- 저장 DDL, 저장 기록 레이아웃, 아티팩트 생명주기, 상태 버전 저장 규칙, 보안 보장
- 향후 또는 이후 후보 API 메서드

## API 메서드 묶음 경계

현재 MVP API는 사용자 작업 루프 하나를 위한 작은 로컬 MCP 접점입니다. 이 접점은 작업 접수, 상태 보기, 활성 범위 갱신, 제안된 제품 쓰기 확인, 아티팩트 스테이징, 실행과 증거 참조 기록, 사용자 소유 판단 요청과 기록, 활성 차단 사유가 허용하는 닫기를 다룹니다.

API는 협력형 하네스 기록과 점검 동작만 반환합니다. 보안 비주장과 보장 표현은 [보안](../security.md)이 담당합니다. 향후 API나 스키마 후보는 이 활성 참조가 아니라 [이후 후보 색인](../../later/index.md)에 둡니다.

<a id="active-mvp-method-behavior"></a>

## 현재 MVP API 메서드 목록

정확한 활성 메서드 이름 값 집합은 [API 값 집합](schema-value-sets.md)이 담당합니다. 활성 메서드는 아래 담당 문서로 이동합니다.

| 메서드 | 활성 역할 | 담당 문서 |
|---|---|---|
| <a id="harnessintake"></a>`harness.intake` | 일반 사용자 작업을 시작, 재개, 분류합니다. | [접수 메서드](method-intake.md) |
| <a id="harnessstatus"></a>`harness.status` | 현재 상태 요약, 차단 사유, 대기 중인 판단, 증거 요약, 닫기 상태, 다음 안전한 행동을 반환합니다. | [상태 메서드](method-status.md) |
| <a id="harnessupdate_scope"></a>`harness.update_scope` | `harness.intake` 이후 활성 Task 범위와 활성 Change Unit을 갱신합니다. | [범위 갱신 메서드](method-update-scope.md) |
| <a id="harnessprepare_write"></a>`harness.prepare_write` | 제안된 제품 파일 쓰기를 현재 범위, 상태, 필요한 별도 민감 동작 승인, 기준선, 접점 역량과 비교합니다. | [쓰기 준비 메서드](method-prepare-write.md) |
| <a id="harnessstage_artifact"></a>`harness.stage_artifact` | 호출자가 제공한 안전한 아티팩트 바이트 또는 안전한 알림을 나중에 `record_run`이 승격할 수 있는 임시 핸들로 스테이징합니다. | [아티팩트 스테이징 메서드](method-stage-artifact.md) |
| <a id="harnessrecord_run"></a>`harness.record_run` | 구체화, 직접 응답, 구현 작업과 간결한 증거 및 아티팩트 참조를 기록합니다. | [실행 기록 메서드](method-record-run.md) |
| <a id="harnessrequest_user_judgment"></a>`harness.request_user_judgment` | 대기 중인 사용자 소유 판단 요청 하나를 만듭니다. | [사용자 판단 메서드](method-user-judgment.md#harnessrequest_user_judgment) |
| <a id="harnessrecord_user_judgment"></a>`harness.record_user_judgment` | 기존 대기 중인 `UserJudgment`에 대한 사용자의 답을 기록합니다. | [사용자 판단 메서드](method-user-judgment.md#harnessrecord_user_judgment) |
| <a id="harnessclose_task"></a>`harness.close_task` | 닫기 준비 상태를 확인하고 차단 사유가 허용할 때만 닫기, 취소, 대체를 수행합니다. | [Task 닫기 메서드](method-close-task.md) |

## 메서드 담당 문서 경로

| 질문 | 담당 문서 |
|---|---|
| 접수 동작, 모드 확정, 초기 범위 후보, `IntakeResult` 예시 | [접수 메서드](method-intake.md) |
| 활성 범위 변경, Change Unit 변경, 오래된 Write Authorization 결과, `UpdateScopeResult` 예시 | [범위 갱신 메서드](method-update-scope.md) |
| 읽기 전용 상태 동작, 포함할 요약, `StatusResult` 예시 | [상태 메서드](method-status.md) |
| 쓰기 호환성 확인, Write Authorization 생성 또는 비허용 판단, `PrepareWriteResult` 예시 | [쓰기 준비 메서드](method-prepare-write.md) |
| 임시 아티팩트 스테이징 동작과 `StageArtifactResult` 예시 | [아티팩트 스테이징 메서드](method-stage-artifact.md) |
| 실행 기록, 증거 갱신, 아티팩트 승격, `RecordRunResult` 예시 | [실행 기록 메서드](method-record-run.md) |
| 사용자 소유 판단 요청 생성 또는 해결과 관련 예시 | [사용자 판단 메서드](method-user-judgment.md) |
| 닫기 준비 상태 점검, 닫기 차단 사유, 종료 닫기 변경, `CloseTaskResult` 예시 | [Task 닫기 메서드](method-close-task.md) |

<a id="shared-request-rules"></a>
<a id="공통-요청-규칙"></a>

## 공통 요청 래퍼와 응답 분기 읽기 규칙

모든 공개 메서드는 [`ToolEnvelope`](schema-core.md#tool-envelope)를 사용합니다. 각 공개 메서드 응답에는 정확히 하나의 분기만 있습니다.

- 구체적인 메서드별 `MethodResult`
- `ToolRejectedResponse`
- `ToolDryRunResponse`

메서드 결과는 [공통 응답 분기](schema-core.md#common-response)의 `ToolResultBase`를 사용하고, `response_kind=result`를 설정하며, 메서드 담당 문서가 허용하는 읽기, 스테이징, Core 커밋, 커밋된 차단 결과의 구체 결과를 이름 붙입니다.

`ToolRejectedResponse`와 `ToolDryRunResponse`는 [공통 응답 분기](schema-core.md#common-response)의 공유 응답 스키마를 사용합니다. 이 분기들은 메서드별 결과 전용 필드를 상속하지 않습니다.

커밋되는 `dry_run=false` 상태 변경 호출에는 null이 아닌 `idempotency_key`와 현재 프로젝트 전체 `expected_state_version`이 필요합니다. 읽기 전용 호출, 유효한 `dry_run` 미리보기, 스테이징 유틸리티 호출은 각 메서드 담당 문서의 예외 규칙을 따릅니다.

메서드별 `task_id`가 있으면 Core는 주 Task를 아래 순서로 해석합니다.

1. 메서드 필드.
2. `ToolEnvelope.task_id`.
3. 활성 Task.

비주장: Task 해석은 담당 기록을 선택할 뿐 별도 상태 시계를 만들지 않습니다.

## 스키마 담당 문서 링크

| 스키마 영역 | 담당 문서 |
|---|---|
| 공통 요청 래퍼, 공통 응답 분기, `ToolResultBase`, `ToolRejectedResponse`, `ToolDryRunResponse`, `ToolError`, `EventRef` | [API 코어 스키마](schema-core.md) |
| 상태 요약, 참조, 닫기 준비 상태 형태, 증거 요약, 쓰기 권한 요약 | [API 상태 스키마](schema-state.md) |
| 아티팩트 입력, 스테이징된 아티팩트 핸들, 아티팩트 참조 | [API 아티팩트 스키마](schema-artifacts.md) |
| 사용자 판단, 판단 옵션, 판단 답변, 민감 동작 범위, 수락한 위험 입력 | [API 판단 스키마](schema-judgment.md) |
| 활성 메서드 이름, 메서드 로컬 값, 응답/효과 종류, 접근 등급, 생명주기 값 | [API 값 집합](schema-value-sets.md) |
| 공개 오류 코드, 오래된 상태 우선순위, 닫기 차단 사유 경로 | [API 오류](errors.md) |

## 저장 효과 담당 문서 링크

| 저장 영역 | 담당 문서 |
|---|---|
| 메서드별 저장 효과 의미, 효과 없음 경계, `dry_run` 저장 효과, 커밋된 차단 결과의 저장 결과, 읽기 전용 저장 경계 | [저장 효과](../storage-effects.md) |
| 지속 기록 레이아웃, DDL 담당, 기록 열 의미, 저장소 소유 JSON 배치 | [저장소 기록](../storage-records.md) |
| 상태 시계, idempotency 재실행 동작, 버전 충돌 저장 규칙 | [저장소 버전 관리](../storage-versioning.md) |
| 아티팩트 스테이징, 검증, 승격, 연결, 본문 읽기 생명주기 | [아티팩트 저장소](../storage-artifacts.md) |

## 안정적인 API 예시 시나리오 요약

메서드 담당 문서의 예시는 오래 유지되는 계정 데이터 내보내기 확인 시나리오를 사용합니다. `Task` 요약은 계정 데이터 내보내기 확인이고, 범위는 계정 데이터 내보내기 확인 UI와 계정 내보내기 테스트를 포함하며, 계정 삭제 동작과 청구 내보내기 동작은 제외합니다. 수락 기준은 다운로드 전에 명시적 확인 단계가 필요하다는 것입니다. 다른 메서드 예시는 같은 시나리오에 계정 내보내기 테스트와 대표 실행 및 증거 데이터를 더할 수 있습니다.

예시는 전체 스키마 정의가 아니라 간결한 분기 예시입니다. 중첩 형태의 전체 정의는 스키마 담당 문서를 따르며, 공유 시나리오 참조는 `state_version`, 아티팩트 참조, 실행 참조, 판단 참조, 닫기 준비 상태 증거, 민감 동작 승인 사유, 만료 timestamp 사이에서 내부 정합성을 유지합니다. API 예시를 교체하거나 검토하는 유지보수 규칙은 [작성 가이드](../../maintain/authoring-guide.md)와 [점검](../../maintain/checks.md)에 있습니다.

## 메서드 담당 문서

- [접수 메서드](method-intake.md)
- [범위 갱신 메서드](method-update-scope.md)
- [상태 메서드](method-status.md)
- [쓰기 준비 메서드](method-prepare-write.md)
- [아티팩트 스테이징 메서드](method-stage-artifact.md)
- [실행 기록 메서드](method-record-run.md)
- [사용자 판단 메서드](method-user-judgment.md)
- [Task 닫기 메서드](method-close-task.md)
