# 현재 MVP API

## 담당하는 것

이 문서는 현재 MVP API method 묶음의 안정적인 경로 문서입니다. 담당하는 것은 아래와 같습니다.

- 활성 공개 API method 목록
- method 담당 문서 경로
- 공통 요청 래퍼와 응답 분기 담당 문서 링크
- 스키마 담당 문서 링크
- 저장 효과 담당 문서 링크
- method 담당 문서가 사용하는 안정적인 API 예시 시나리오 요약

## 담당하지 않는 것

이 문서는 아래 항목을 담당하지 않습니다.

- 각 method 동작의 전체 세부사항. 여기에는 method별 필수 입력, 접근 요구사항, 결과 필드, `dry_run` 동작, 대표 요청과 응답 본문이 포함됩니다.
- 공통 API 요청 래퍼 본문, 응답 분기 스키마 본문, 스키마 필드 정의
- 상태, 아티팩트, 사용자 판단, 값 집합, 오류 스키마 정의
- API 예시 정합성 규칙 또는 필드 이름 일관성 규칙
- 저장 효과 세부사항, 저장 DDL, 저장 기록 레이아웃, 아티팩트 생명주기, 상태 버전 저장 규칙, 보안 보장
- 공개 오류 코드 의미
- 향후 또는 이후 후보 API method

## API method 묶음 경계

현재 MVP API는 사용자 작업 루프 하나를 위한 작은 로컬 MCP 접점입니다. 이 접점은 작업 접수, 상태 보기, 활성 범위 갱신, 제안된 제품 쓰기 확인, 아티팩트 스테이징, 실행과 증거 참조 기록, 사용자 판단 method를 통한 요청과 기록, 활성 차단 사유가 허용하는 닫기를 다룹니다.

API는 협력형 하네스 기록과 점검 동작만 반환합니다. 보안 비주장과 보장 표현은 [보안](../security.md)이 담당합니다. 향후 API나 스키마 후보는 이 활성 참조가 아니라 [이후 후보 색인](../../later/index.md)에 둡니다.

<a id="active-mvp-method-behavior"></a>

## 현재 MVP API method 목록

이 문서는 활성 공개 API method 목록과 method 담당 문서 경로를 맡습니다. 정확한 활성 API method 이름 값 집합은 [API 값 집합](schema-value-sets.md)이 담당합니다. 활성 method는 아래 담당 문서로 이동합니다.

<a id="harnessintake"></a>
<a id="harnessupdate_scope"></a>
<a id="harnessstatus"></a>
<a id="harnessprepare_write"></a>
<a id="harnessstage_artifact"></a>
<a id="harnessrecord_run"></a>
<a id="harnessrequest_user_judgment"></a>
<a id="harnessrecord_user_judgment"></a>
<a id="harnessclose_task"></a>

| method | 활성 역할 | 담당 문서 |
|---|---|---|
| `harness.intake` | 일반 사용자 작업을 시작, 재개, 분류합니다. | [접수 method 담당 문서](method-intake.md) |
| `harness.update_scope` | `harness.intake` 이후 활성 Task 범위와 활성 Change Unit을 갱신합니다. | [범위 갱신 method 담당 문서](method-update-scope.md) |
| `harness.status` | 현재 상태와 다음 안전한 행동을 반환합니다. | [상태 method 담당 문서](method-status.md) |
| `harness.prepare_write` | 제품 파일 쓰기 호환성을 Write Authorization 전에 확인합니다. | [쓰기 준비 method 담당 문서](method-prepare-write.md) |
| `harness.stage_artifact` | 안전한 바이트나 안전한 알림을 스테이징합니다. | [아티팩트 스테이징 method 담당 문서](method-stage-artifact.md) |
| `harness.record_run` | 작업, 증거, 아티팩트 참조를 기록합니다. | [실행 기록 method 담당 문서](method-record-run.md) |
| `harness.request_user_judgment` | 대기 중인 사용자 소유 판단 요청 하나를 만듭니다. | [사용자 판단 method 담당 문서](method-user-judgment.md#harnessrequest_user_judgment) |
| `harness.record_user_judgment` | 대기 중인 판단에 대한 사용자의 답을 기록합니다. | [사용자 판단 method 담당 문서](method-user-judgment.md#harnessrecord_user_judgment) |
| `harness.close_task` | 닫기 준비 상태를 확인하거나 허용될 때 닫습니다. | [Task 닫기 method 담당 문서](method-close-task.md) |

<a id="메서드-담당-문서-경로"></a>

## method 담당 문서 경로

아래 표는 method 동작 질문을 담당 문서로 보냅니다.

| 질문 | 담당 문서 |
|---|---|
| `harness.intake` method 동작 | [접수 method 담당 문서](method-intake.md) |
| `harness.update_scope` method 동작 | [범위 갱신 method 담당 문서](method-update-scope.md) |
| `harness.status` method 동작 | [상태 method 담당 문서](method-status.md) |
| `harness.prepare_write` method 동작 | [쓰기 준비 method 담당 문서](method-prepare-write.md) |
| `harness.stage_artifact` method 동작 | [아티팩트 스테이징 method 담당 문서](method-stage-artifact.md) |
| `harness.record_run` method 동작 | [실행 기록 method 담당 문서](method-record-run.md) |
| 사용자 판단 method(`harness.request_user_judgment`, `harness.record_user_judgment`) 동작 | [사용자 판단 method 담당 문서](method-user-judgment.md) |
| `harness.close_task` method 동작 | [Task 닫기 method 담당 문서](method-close-task.md) |

method별 질문은 다음 담당 문서로 보냅니다.

- method 동작: 위 표의 해당 method 담당 문서
- method별 payload 필드: 위 표의 해당 method 담당 문서
- 요청과 응답 분기 형태: [`schema-core.md`](schema-core.md)
- 중첩 상태 필드: [`schema-state.md`](schema-state.md)
- 아티팩트 필드: [`schema-artifacts.md`](schema-artifacts.md)
- 사용자 판단 필드: [`schema-judgment.md`](schema-judgment.md)
- 값 집합: [`schema-value-sets.md`](schema-value-sets.md)
- 저장 효과: [`../storage-effects.md`](../storage-effects.md)
- 공개 오류: [`errors.md`](errors.md)
- API 예시 정합성: [작성 가이드](../../maintain/authoring-guide.md), [점검](../../maintain/checks.md)

<a id="shared-request-rules"></a>
<a id="공통-요청-규칙"></a>

## 공통 요청 래퍼와 응답 분기 경로

공통 API 형태는 [API 코어 스키마](schema-core.md)가 담당합니다.

- 요청 래퍼: [`ToolEnvelope`](schema-core.md#tool-envelope)
- 응답 분기: [공통 응답 분기](schema-core.md#common-response)
- 공통 결과 기반: [`ToolResultBase`](schema-core.md#common-response)
- 거부와 `dry_run` 분기: [공통 응답 분기](schema-core.md#common-response)의 `ToolRejectedResponse`와 `ToolDryRunResponse`
- method 결과 분기 허용 여부: 해당 method 담당 문서
- `idempotency_key`, `expected_state_version`, `dry_run` 예외: 해당 method 담당 문서
- Task 선택 우선순위: method별 `task_id`, `ToolEnvelope.task_id`, 활성 Task 순서
- method별 `task_id` 필드: 해당 method 담당 문서

## 스키마 담당 문서 링크

- 요청과 응답 분기 형태: [API 코어 스키마](schema-core.md)
- 중첩 상태 필드: [API 상태 스키마](schema-state.md)
- 아티팩트 필드: [API 아티팩트 스키마](schema-artifacts.md)
- 사용자 판단 필드: [API 판단 스키마](schema-judgment.md)
- 값 집합: [API 값 집합](schema-value-sets.md)
- 공개 오류: [API 오류](errors.md)

## 저장 효과 담당 문서 링크

- method별 저장 효과와 효과 없음 경계: [저장 효과](../storage-effects.md)
- 지속 기록 레이아웃과 DDL 담당: [저장소 기록](../storage-records.md)
- 상태 시계와 버전 충돌 저장 규칙: [저장소 버전 관리](../storage-versioning.md)
- 아티팩트 스테이징, 승격, 생명주기: [아티팩트 저장소](../storage-artifacts.md)

## 안정적인 API 예시 시나리오 요약

method 담당 문서의 예시는 계정 데이터 내보내기 전에 명시적 확인 단계를 추가하는 오래 유지되는 시나리오를 사용합니다.

- `Task` 요약: 계정 데이터 내보내기 전에 명시적 확인 단계를 추가한다.
- 범위: 계정 데이터 내보내기 흐름과 계정 데이터 내보내기 확인 테스트.
- 범위 밖: 계정 삭제 동작.
- 수락 기준: 계정 데이터 내보내기 파일을 다운로드하기 전에 명시적 확인 단계가 필요하다.
- 확장: method 예시는 계정 데이터 내보내기 확인 테스트의 대표 실행과 증거 데이터를 더할 수 있습니다.

예시는 전체 스키마 정의가 아니라 간결한 분기 예시입니다.

API 예시 질문은 다음 담당 문서로 보냅니다.

- 정합성 규칙과 교체 기준: [작성 가이드](../../maintain/authoring-guide.md), [점검](../../maintain/checks.md)
- 중첩 형태: 위의 스키마 담당 문서 링크
- method payload 필드: 해당 method 담당 문서
- 스키마 필드: 해당 스키마 담당 문서
- 저장소 담당 예시 필드: 해당 저장소 담당 문서
- 공유 시나리오 참조 정합성: [작성 가이드](../../maintain/authoring-guide.md), [점검](../../maintain/checks.md)

## method 담당 문서

- [접수 method 담당 문서](method-intake.md)
- [범위 갱신 method 담당 문서](method-update-scope.md)
- [상태 method 담당 문서](method-status.md)
- [쓰기 준비 method 담당 문서](method-prepare-write.md)
- [아티팩트 스테이징 method 담당 문서](method-stage-artifact.md)
- [실행 기록 method 담당 문서](method-record-run.md)
- [사용자 판단 method 담당 문서](method-user-judgment.md): `harness.request_user_judgment`, `harness.record_user_judgment`
- [Task 닫기 method 담당 문서](method-close-task.md)
