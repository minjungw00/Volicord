# 현재 MVP API

## 담당하는 것

이 문서는 현재 MVP API 메서드 묶음의 안정적인 경로 문서입니다. 담당하는 것은 아래와 같습니다.

- 활성 공개 API 메서드 목록
- 메서드 담당 문서 경로
- 공통 요청 래퍼와 응답 분기 읽기 규칙
- 스키마 담당 문서 링크
- 저장 효과 담당 문서 링크
- 메서드 담당 문서가 사용하는 안정적인 API 예시 시나리오 요약

## 담당하지 않는 것

이 문서는 아래 항목을 담당하지 않습니다.

- 각 메서드 동작의 전체 세부사항. 여기에는 메서드별 필수 입력, 접근 요구사항, 결과 필드, `dry_run` 동작, 대표 요청과 응답 본문이 포함됩니다.
- 공통 API 요청 래퍼 본문, 응답 분기 스키마 본문, 스키마 필드 정의
- 상태, 아티팩트, 사용자 판단, 값 집합, 오류 스키마 정의
- 저장 효과 세부사항, 저장 DDL, 저장 기록 레이아웃, 아티팩트 생명주기, 상태 버전 저장 규칙, 보안 보장
- 공개 오류 코드 의미
- 향후 또는 이후 후보 API 메서드

## API 메서드 묶음 경계

현재 MVP API는 사용자 작업 루프 하나를 위한 작은 로컬 MCP 접점입니다. 이 접점은 작업 접수, 상태 보기, 활성 범위 갱신, 제안된 제품 쓰기 확인, 아티팩트 스테이징, 실행과 증거 참조 기록, 사용자 소유 판단 요청과 기록, 활성 차단 사유가 허용하는 닫기를 다룹니다.

API는 협력형 하네스 기록과 점검 동작만 반환합니다. 보안 비주장과 보장 표현은 [보안](../security.md)이 담당합니다. 향후 API나 스키마 후보는 이 활성 참조가 아니라 [이후 후보 색인](../../later/index.md)에 둡니다.

<a id="active-mvp-method-behavior"></a>

## 현재 MVP API 메서드 목록

이 문서는 활성 공개 API 메서드 목록과 담당 문서 경로를 맡습니다. 정확한 활성 API 메서드 이름 값 집합은 [API 값 집합](schema-value-sets.md)이 담당합니다. 활성 메서드는 아래 담당 문서로 이동합니다.

| 메서드 | 활성 역할 | 담당 문서 |
|---|---|---|
| <a id="harnessintake"></a>`harness.intake` | 일반 사용자 작업을 시작, 재개, 분류합니다. | [접수 메서드](method-intake.md) |
| <a id="harnessupdate_scope"></a>`harness.update_scope` | `harness.intake` 이후 활성 Task 범위와 활성 Change Unit을 갱신합니다. | [범위 갱신 메서드](method-update-scope.md) |
| <a id="harnessstatus"></a>`harness.status` | 현재 상태 요약, 차단 사유, 대기 중인 판단, 증거 요약, 닫기 상태, 다음 안전한 행동을 반환합니다. | [상태 메서드](method-status.md) |
| <a id="harnessprepare_write"></a>`harness.prepare_write` | 제품 파일 쓰기 호환성을 Write Authorization 전에 확인합니다. | [쓰기 준비 메서드](method-prepare-write.md) |
| <a id="harnessstage_artifact"></a>`harness.stage_artifact` | 안전한 바이트나 안전한 알림을 나중에 `record_run`이 승격할 수 있게 스테이징합니다. | [아티팩트 스테이징 메서드](method-stage-artifact.md) |
| <a id="harnessrecord_run"></a>`harness.record_run` | 구체화, 직접 응답, 구현 작업과 간결한 증거 및 아티팩트 참조를 기록합니다. | [실행 기록 메서드](method-record-run.md) |
| <a id="harnessrequest_user_judgment"></a>`harness.request_user_judgment` | 대기 중인 사용자 소유 판단 요청 하나를 만듭니다. | [사용자 판단 메서드](method-user-judgment.md#harnessrequest_user_judgment) |
| <a id="harnessrecord_user_judgment"></a>`harness.record_user_judgment` | 기존 대기 중인 `UserJudgment`에 대한 사용자의 답을 기록합니다. | [사용자 판단 메서드](method-user-judgment.md#harnessrecord_user_judgment) |
| <a id="harnessclose_task"></a>`harness.close_task` | 닫기 준비 상태를 확인하고 차단 사유가 허용할 때만 닫기, 취소, 대체를 수행합니다. | [Task 닫기 메서드](method-close-task.md) |

## 메서드 담당 문서 경로

메서드 동작을 확인할 때는 아래 담당 문서를 먼저 엽니다. 공통 응답 분기 스키마, 중첩 스키마 필드, 저장 효과, 공개 오류 코드는 아래 담당 문서 링크를 따릅니다.

| 질문 | 담당 문서 |
|---|---|
| `harness.intake` 메서드 동작 | [접수 메서드](method-intake.md) |
| `harness.update_scope` 메서드 동작 | [범위 갱신 메서드](method-update-scope.md) |
| `harness.status` 메서드 동작 | [상태 메서드](method-status.md) |
| `harness.prepare_write` 메서드 동작 | [쓰기 준비 메서드](method-prepare-write.md) |
| `harness.stage_artifact` 메서드 동작 | [아티팩트 스테이징 메서드](method-stage-artifact.md) |
| `harness.record_run` 메서드 동작 | [실행 기록 메서드](method-record-run.md) |
| 사용자 소유 판단 메서드 동작 | [사용자 판단 메서드](method-user-judgment.md) |
| `harness.close_task` 메서드 동작 | [Task 닫기 메서드](method-close-task.md) |

<a id="shared-request-rules"></a>
<a id="공통-요청-규칙"></a>

## 공통 요청 래퍼와 응답 분기 읽기 규칙

모든 공개 메서드는 [`ToolEnvelope`](schema-core.md#tool-envelope)를 사용합니다. 각 공개 메서드 응답에는 아래 분기 중 정확히 하나만 있습니다.

- 구체적인 메서드별 `MethodResult`
- `ToolRejectedResponse`
- `ToolDryRunResponse`

메서드 결과는 [공통 응답 분기](schema-core.md#common-response)의 `ToolResultBase`를 사용하고, `response_kind=result`를 설정하며, 메서드 담당 문서가 허용하는 읽기, 스테이징, Core 커밋, 커밋된 차단 결과의 구체 결과를 이름 붙입니다.

`ToolRejectedResponse`와 `ToolDryRunResponse`는 [공통 응답 분기](schema-core.md#common-response)의 공유 응답 스키마를 사용합니다. 두 공유 분기는 메서드별 결과 전용 필드를 상속하지 않습니다.

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
| 메서드별 저장 효과와 효과 없음 경계 | [저장 효과](../storage-effects.md) |
| 지속 기록 레이아웃, DDL 담당, 기록 열 의미, 저장소 소유 JSON 배치 | [저장소 기록](../storage-records.md) |
| 상태 시계, idempotency 재실행 동작, 버전 충돌 저장 규칙 | [저장소 버전 관리](../storage-versioning.md) |
| 아티팩트 스테이징, 검증, 승격, 연결, 본문 읽기 생명주기 | [아티팩트 저장소](../storage-artifacts.md) |

## 안정적인 API 예시 시나리오 요약

메서드 담당 문서의 예시는 계정 데이터 내보내기 전에 명시적 확인 단계를 추가하는 오래 유지되는 시나리오를 사용합니다.

- `Task` 요약: 계정 데이터 내보내기 전에 명시적 확인 단계를 추가한다.
- 범위: 계정 내보내기 확인 UI와 테스트.
- 범위 밖: 계정 삭제 동작과 청구 내보내기 동작.
- 수락 기준: 다운로드 전에 명시적 확인 단계가 필요합니다.
- 확장: 메서드 예시는 계정 내보내기 확인 테스트, 대표 실행, 증거 데이터를 더할 수 있습니다.

예시는 전체 스키마 정의가 아니라 간결한 분기 예시입니다.

정합성 요구사항:
- 중첩 형태는 스키마 담당 문서를 따릅니다.
- 공유 시나리오 참조는 `state_version`, 아티팩트 참조, 실행 참조, 판단 참조, 닫기 준비 상태 증거, 민감 동작 승인 사유, 만료 타임스탬프 사이에서 맞아야 합니다.

API 예시를 교체하거나 검토하는 유지보수 규칙은 [작성 가이드](../../maintain/authoring-guide.md)와 [점검](../../maintain/checks.md)에 있습니다.

## 메서드 담당 문서

- [접수 메서드](method-intake.md)
- [범위 갱신 메서드](method-update-scope.md)
- [상태 메서드](method-status.md)
- [쓰기 준비 메서드](method-prepare-write.md)
- [아티팩트 스테이징 메서드](method-stage-artifact.md)
- [실행 기록 메서드](method-record-run.md)
- [사용자 판단 메서드](method-user-judgment.md)
- [Task 닫기 메서드](method-close-task.md)
