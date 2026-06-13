# API 값 집합

이 문서는 기준 범위의 지원되는 API 값 집합과 enum 형태 공개 값을 담당합니다. 예약된 값이나 지원 범위 밖 값을 이름 붙이는 것만으로 기준 범위가 넓어지지 않습니다.

## 담당하는 것 / 담당하지 않는 것

이 문서가 담당합니다.

- 지원되는 공개 메서드 이름 값
- API `response_kind`와 `effect_kind` 값
- 지원되는 `access_class` 값
- 공유 상태 참조에서 쓰는 기록/참조 판별 값
- 지원되는 생명주기, 닫기 상태, 출처 종류, 판단 종류, 표시 형식, 필요 판단 위치, 아티팩트 가림 처리, 아티팩트 가용성 표시, `ValidatorResult.status`, `ValidatorResult.severity`, 보장 표시 등 API 값 집합
- 지원되는 공개 `ValidatorResult.validator_id` 값의 경계
- 지원되는 스키마 해석에 영향을 주는 프로필 조건부 또는 예약 값 경계
- 렌더링된 라벨이 기준 스키마 값이 아니라는 규칙

이 문서는 담당하지 않습니다.

- 공개 `ErrorCode` 값과 우선순위: [API 오류 코드](error-codes.md), [API 오류 우선순위](error-precedence.md)
- 차단 사유 처리 경로: [API 차단 사유 처리 경로](blocker-routing.md)
- 이 값을 쓰는 필드 형태: [API 코어 스키마](schema-core.md), [API 상태 스키마](schema-state.md), [API 아티팩트 스키마](schema-artifacts.md), [API 판단 스키마](schema-judgment.md)
- 메서드 동작: [API 메서드](methods.md)와 메서드 담당 문서
- 보안 보장 의미: [보안](../security.md)
- 지원 범위 밖 기능 승격: [범위 참조](../scope.md)

## 경계

이 문서가 지원 값으로 둔 값만 지원되는 API 값입니다.

- 프로필 조건부 값은 사용하는 자리에서 프로필이나 역량 조건을 이름 붙여야 합니다.
- 지원 목록 밖의 값은 [범위 참조](../scope.md)와 영향받는 의미 담당 문서가 지원 동작을 정의하기 전까지 기준 범위 API 값이 아닙니다.
- 지원 목록 밖의 이름을 적는 것만으로 기준 범위가 넓어지지 않습니다.
- 화면에 보이는 라벨은 표시 텍스트일 뿐이며, 이 문서의 기준 값을 대신하지 않습니다.

<a id="method-name-values"></a>
## 메서드 이름 값

지원되는 공개 메서드 이름 집합은 아래와 같습니다.

```text
harness.intake
harness.update_scope
harness.status
harness.prepare_write
harness.stage_artifact
harness.record_run
harness.request_user_judgment
harness.record_user_judgment
harness.close_task
```

메서드 동작은 [API 메서드](methods.md)가 안내하는 메서드 담당 문서가 담당합니다. 메서드 이름은 `Task` 생명주기 값이 아닙니다.

<a id="response-and-effect-values"></a>
## 응답과 효과 값

`ToolResultBase.response_kind`는 아래 값을 사용합니다.

```text
result
rejected
dry_run
```

`ToolResultBase.effect_kind`는 아래 값을 사용합니다.

```text
read_only
core_committed
staging_created
no_effect
```

`response_kind`와 `effect_kind`는 분기 메타데이터 값입니다. 공통 분기 형태는 [API 코어 스키마](schema-core.md#common-response)가 담당하고, 메서드별 상태 효과는 메서드 담당 문서가 담당합니다. 거절 분기의 공개 오류 의미는 [API 오류 코드](error-codes.md)와 [API 오류 처리 경로](error-routing.md)가 담당합니다.

<a id="access-class-values"></a>
## 접근 등급 값

`VerifiedSurfaceContext.access_class`는 공개 API 요청 하나마다 요청 수준 값 하나만 사용합니다.

| 값 | 의미 담당 문서 |
|---|---|
| `read_status` | 읽기 전용 상태와 닫기 확인 읽기. |
| `core_mutation` | 별도 분류가 없는 Core 상태 변경. |
| `write_authorization` | `harness.prepare_write`. |
| `run_recording` | `harness.record_run`. |
| `artifact_registration` | `harness.stage_artifact`. |
| `artifact_read` | 아티팩트 담당 문서가 지원을 정의한 아티팩트 본문 읽기. |

접근 등급은 하네스 API 호환성 분류이지 OS 권한 분류가 아닙니다. 메서드별 접근 요구사항은 [API 메서드](methods.md)가 안내하는 메서드 담당 문서가 담당하고, 로컬 접점 확인 동작은 [에이전트 통합](../agent-integration.md)과 [보안](../security.md)이 담당합니다.

<a id="record-and-reference-values"></a>
## 기록과 참조 값

`StateRecordRef.record_kind`는 아래 값을 사용합니다.

```text
project_state
task
change_unit
write_authorization
user_judgment
run
evidence_summary
artifact
blocker
task_event
local_surface_registration
```

이 값들은 API 참조에서 지속 Core 기록이나 로컬 접점 기록의 종류를 식별합니다. 저장소 테이블 이름, DDL, 메서드별 담당 규칙을 대신하지 않습니다.

<a id="task-lifecycle-values"></a>
## `Task` 생명주기 값

`StateSummary.mode`와 지속 저장되는 확정 `Task.mode`는 아래 값을 사용합니다.

```text
advisor
direct
work
```

`harness.intake`의 `requested_mode`는 입력 전용으로 `auto`도 받습니다. `auto`는 지속 저장되거나 표시되는 `Task` 상태가 되기 전에 `advisor`, `direct`, `work` 중 하나로 확정되어야 합니다.

`Task.lifecycle_phase`는 아래 값을 사용합니다.

```text
shaping
ready
executing
waiting_user
blocked
completed
cancelled
superseded
```

`CloseTaskResult.close_state`는 아래 값을 사용합니다.

```text
ready
blocked
closed
cancelled
superseded
```

`StatusResult.close_state`는 현재 적용 닫기 상태가 없을 때 `none`도 허용합니다.

`Task.close_reason`은 아래 값을 사용합니다.

```text
none
completed_self_checked
completed_with_risk_accepted
cancelled
superseded
```

`Task.result`는 아래 값을 사용합니다.

```text
none
advice_only
completed
cancelled
superseded
```

Run 실패, 위반, 차단된 닫기, 증거 공백은 종료 `Task.result` 값이 아닙니다.

## 메서드 내부 값

`harness.intake`의 `resume_policy`는 아래 값을 사용합니다.

```text
resume_active
create_new
supersede_active
reject_if_active
```

`harness.close_task.intent`는 아래 값을 사용합니다.

```text
check
complete
cancel
supersede
```

`PrepareWriteResult.decision`은 아래 값을 사용합니다.

```text
allowed
blocked
approval_required
decision_required
```

`PrepareWriteResult.authorization_effect`는 아래 값을 사용합니다.

```text
none
would_create
created
returned
```

`RecordRunRequest.kind`와 `RunSummary.kind`는 아래 값을 사용합니다.

```text
shaping_update
implementation
direct
```

<a id="state-and-blocker-values"></a>
## 상태와 차단 사유 값

`CloseReadinessBlocker` 객체 형태는 [API 상태 스키마](schema-state.md#close-readiness-and-validation-shapes)가 담당합니다. 이 절은 지원되는 `CloseReadinessBlocker.category` 값과 인접 상태/차단 사유 값을 담당합니다.

`PlannedBlocker.source_kind`는 아래 값을 사용합니다.

```text
write_decision
close_readiness
```

`CloseReadinessBlocker.category`는 아래 값을 사용합니다.

```text
task
open_run
scope
user_judgment
sensitive_approval
write_compatibility
baseline
surface_capability
evidence
artifact_availability
final_acceptance
residual_risk_visibility
residual_risk_acceptance
recovery
```

`EvidenceSummary.status`는 아래 값을 사용합니다.

```text
unknown
insufficient
sufficient
blocked
```

`EvidenceCoverageItem.coverage_state`는 아래 값을 사용합니다.

```text
unsupported
partial
supported
not_applicable
stale
blocked
```

`ValidatorResult.status`는 아래 값을 사용합니다.

```text
passed
warning
failed
blocked
```

`ValidatorResult.severity`는 아래 값을 사용합니다.

```text
info
warning
error
blocking
```

이 기준 범위 값 집합 담당 문서는 지원되는 안정 `ValidatorResult.validator_id` 집합을 공개하지 않습니다. 영향받는 담당 문서가 정확한 안정 값을 이 문서에 공개하고 그 의미를 정의하기 전까지 `validator_id` 문자열은 보고용 라벨입니다.

`GuaranteeDisplay.level`은 기준 범위 지원 값으로 아래를 사용합니다.

```text
cooperative
detective
```

<a id="artifact-values"></a>
## 아티팩트 값

`ArtifactInput.source_kind`는 아래 값을 사용합니다.

```text
staged_artifact
existing_artifact
```

값 의미:
- `staged_artifact`는 아티팩트 담당 동작을 통해 호환되는 임시 스테이징 핸들을 선택합니다.
- `existing_artifact`는 새 바이트를 등록하지 않고 이미 지속되는 같은 프로젝트 아티팩트를 선택합니다.

선택된 출처 값은 어느 `ArtifactInput` 출처 필드가 적용되는지 정합니다. 정확한 형태 불변조건은 [API 아티팩트 스키마](schema-artifacts.md#artifactinput)가 담당합니다.

이 목록 밖의 값은 지원되는 출처 값이 아닙니다. 새 출처 동작에는 이 문서의 지원 값과 영향받는 의미 담당 문서가 모두 필요합니다.

`redaction_state`는 아래 값을 사용합니다.

```text
none
redacted
secret_omitted
blocked
```

아티팩트 가용성 표시 값은 아래를 사용합니다.

```text
available
unavailable
missing
integrity_failed
blocked
unusable
```

아티팩트 저장소 생명주기와 본문 읽기 자격은 [아티팩트 저장소](../storage-artifacts.md)가 담당합니다.

<a id="judgment-values"></a>
## 판단 값

`judgment_kind`는 아래 값을 사용합니다.

```text
product_decision
technical_decision
scope_decision
sensitive_approval
final_acceptance
residual_risk_acceptance
cancellation
```

`presentation`은 아래 값을 사용합니다.

```text
short
```

`required_for`는 아래 값을 사용합니다.

```text
next_action
write
run
close
acceptance
risk
```

`UserJudgment.status`는 아래 값을 사용합니다.

```text
pending
resolved
rejected
deferred
blocked
stale
superseded
incompatible
```

`UserJudgmentOption.option_id`의 범위는 그 판단 안으로 제한되며 전역 값 집합이 아닙니다. 화면에 보이는 선택지 라벨은 기준 값이 아니라 표시 텍스트일 뿐입니다.

## 오류 세부사항 보조 값

`ToolError.details.authorization_reason`과 `ToolError.details.artifact_input_error.reason` 보조 값은 [API 오류 세부사항](error-details.md#error-detail-helper-values)이 담당합니다. 이 값 집합 문서는 기계 판독용 오류 세부사항 의미를 정의하지 않습니다.

## 프로필 조건부 및 예약 값

예약된 값이나 프로필 조건부 값은 기준 범위의 기본 지원 값이 아닙니다. 이 문서는 지원되지 않는 값 이름을 지원되는 값 집합으로 공개하지 않습니다.

경계:
- 지원 목록 밖의 이름은 메모, 예시, 경로 문서, 렌더링된 라벨에 나온다는 이유만으로 기준 범위 동작이 되지 않습니다.
- 예약된 값이나 프로필 조건부 값의 동작을 지원된다고 설명하려면 [범위 참조](../scope.md) 경계와 영향받는 의미 담당 문서가 먼저 필요합니다.

## 관련 담당 문서

- [기준 범위](../scope.md): 값이 기준 범위에 속하는지 판단.
- [API 오류 코드](error-codes.md): 공개 오류 코드 의미.
- [API 오류 우선순위](error-precedence.md): 공개 오류 우선순위.
- [API 차단 사유 처리 경로](blocker-routing.md): 차단 사유 처리 경로.
- [API 오류 세부사항](error-details.md): 기계 판독용 오류 세부사항 보조 값.
- [API 코어 스키마](schema-core.md), [API 상태 스키마](schema-state.md), [API 아티팩트 스키마](schema-artifacts.md), [API 판단 스키마](schema-judgment.md): 이 값을 쓰는 필드.
- [API 메서드](methods.md)와 메서드 담당 문서: 이 값을 사용하는 메서드 동작.
- [범위 참조](../scope.md): 예약된 값과 프로필 조건부 값의 경계.
