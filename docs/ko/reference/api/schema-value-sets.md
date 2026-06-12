# API 값 집합

이 문서는 현재 MVP의 활성 API 값 집합과 enum 형태 공개 값을 담당합니다. 참조 문서일 뿐이며 이후 후보 이름을 적는 것만으로 활성 범위를 넓히지 않습니다.

## 담당하는 것 / 담당하지 않는 것

이 문서가 담당합니다.

- 활성 공개 메서드 이름 값
- API `response_kind`와 `effect_kind` 값
- 활성 `access_class` 값
- 공유 상태 참조에서 쓰는 기록/참조 판별 값
- 활성 생명주기, 닫기 상태, 출처 종류, 판단 종류, 표시 형식, 필요 판단 위치, 선택지 표시, 아티팩트, 가림 처리, validator, 보장 표시 등 API 값 집합
- 활성 스키마 해석에 영향을 주는 프로필 조건부 또는 예약 값 경계
- 렌더링된 라벨이 기준 스키마 값이 아니라는 규칙

이 문서는 담당하지 않습니다.

- 공개 `ErrorCode` 값과 우선순위: [API 오류](errors.md)
- 이 값을 쓰는 필드 형태: [API 코어 스키마](schema-core.md), [API 상태 스키마](schema-state.md), [API 아티팩트 스키마](schema-artifacts.md), [API 판단 스키마](schema-judgment.md)
- 메서드 동작: [API 메서드](methods.md)와 메서드 담당 문서
- 보안 보장 의미: [보안](../security.md)
- 이후 후보 승격: [범위 참조](../scope.md)

## 경계

이 문서가 활성 값으로 둔 값만 활성 API 값입니다.

조건:

- 프로필 조건부 값은 사용하는 자리에서 프로필이나 역량 조건을 이름 붙여야 합니다.
- 이후 이름은 승격된 담당 문서가 정확한 활성 필드, 대체 동작, 증명 기대를 추가하기 전까지 목록 전용입니다.

비주장:

- 이후 이름을 적는 것만으로 활성 범위가 넓어지지 않습니다.
- 화면에 보이는 라벨은 표시 텍스트일 뿐이며, 이 문서의 기준 값을 대신하지 않습니다.

<a id="method-name-values"></a>
## 메서드 이름 값

활성 공개 메서드 이름 집합은 아래와 같습니다.

```text
harness.intake
harness.status
harness.update_scope
harness.prepare_write
harness.stage_artifact
harness.record_run
harness.request_user_judgment
harness.record_user_judgment
harness.close_task
```

메서드 동작은 [API 메서드](methods.md)가 안내하는 메서드 담당 문서가 담당합니다. 메서드 이름은 Task 생명주기 값이 아닙니다.

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

`response_kind`와 `effect_kind`는 분기 메타데이터 값입니다. 공통 분기 읽기 규칙은 [공통 요청 래퍼와 응답 분기 경로](methods.md#공통-요청-규칙)가 담당하고, 메서드별 상태 효과는 메서드 담당 문서가 담당합니다. 거절 분기의 공개 오류 의미는 [API 오류](errors.md)가 담당합니다.

<a id="access-class-values"></a>
## 접근 등급 값

`VerifiedSurfaceContext.access_class`는 공개 API 요청 하나마다 요청 수준 값 하나만 사용합니다.

| 값 | 활성 담당 경로 |
|---|---|
| `read_status` | 읽기 전용 상태와 닫기 확인 읽기. |
| `core_mutation` | 별도 분류가 없는 Core 상태 변경. |
| `write_authorization` | `harness.prepare_write`. |
| `run_recording` | `harness.record_run`. |
| `artifact_registration` | `harness.stage_artifact`. |
| `artifact_read` | 담당 경로가 노출하는 아티팩트 본문 읽기. |

접근 등급의 의미는 아래 경계를 따릅니다.

- 결과: 접근 등급은 하네스 API 호환성 분류입니다.
- 비주장: 접근 등급은 OS 권한 분류가 아닙니다.
- 담당 문서: 로컬 접점 확인 동작은 [공통 요청 래퍼와 응답 분기 경로](methods.md#공통-요청-규칙), [에이전트 통합](../agent-integration.md), [보안](../security.md)에 남습니다.

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
## Task 생명주기 값

`StateSummary.mode`와 지속 저장되는 확정 `Task.mode`는 아래 값을 사용합니다.

```text
advisor
direct
work
```

`harness.intake`의 `requested_mode`는 입력 전용으로 `auto`도 받습니다.

- 조건: 입력이 `requested_mode=auto`입니다.
- 결과: 지속 저장되거나 표시되는 Task 상태가 되기 전에 `advisor`, `direct`, `work` 중 하나로 확정되어야 합니다.
- 비주장: `auto`는 지속 저장되는 Task 상태 값이 아닙니다.

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

`StatusResult.close_state`는 활성 닫기 상태가 없을 때 `none`도 허용합니다.

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

아래 항목은 종료 `Task.result` 값이 아닙니다.

- Run 실패.
- 위반.
- 차단된 닫기.
- 증거 공백.

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

`GuaranteeDisplay.level`은 현재 MVP 활성 값으로 아래를 사용합니다.

```text
cooperative
detective
```

`changed_path_detection_verification`은 아래 값을 사용합니다.

```text
passed
failed
stale
not_run
```

예전 `planned_not_run`은 활성 값이 아닙니다.

- 비주장: `planned_not_run`은 `detective`의 근거가 될 수 없습니다.
- 담당 문서: 보장 수준 의미는 [보안](../security.md)이 담당합니다.

<a id="artifact-values"></a>
## 아티팩트 값

`ArtifactInput.source_kind`는 아래 값을 사용합니다.

```text
staged_artifact
existing_artifact
```

비활성 또는 거절되는 출처 이름은 아래와 같습니다.

```text
captured_artifact
native_capture
raw_path
raw_log
capture_adapter_output
```

비활성 이름의 경계는 아래와 같습니다.

- 비주장: 아티팩트 캡처를 승인하지 않습니다.
- 비주장: 로컬 파일 읽기를 승인하지 않습니다.

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

`UserJudgmentOption.option_id`의 범위는 해당 판단 안으로 제한됩니다.

- 비주장: 전역 값 집합이 아닙니다.
- 비주장: 화면에 보이는 선택지 라벨은 기준 값이 아니라 표시 텍스트일 뿐입니다.

## 오류 세부사항 보조 값

`ToolError.details.authorization_reason`은 아래 값을 사용합니다.

```text
missing
expired
stale
revoked
consumed
incompatible
```

`ToolError.details.artifact_input_error.reason`은 [공개 `ErrorCode` 표](errors.md#error-taxonomy)에 있는 스테이징된 핸들 사유 값을 사용합니다. 각 공개 오류 코드와 세부 사유의 의미는 [API 오류](errors.md)가 담당합니다.

## 프로필 조건부 및 예약 값

아래 이름들은 현재 MVP의 기본 활성 값이 아닙니다.

- 조건: 승격된 담당 문서가 필드, 대체 동작, 증명 기대를 정의하기 전입니다.
- 결과: 표에 이름이 있다는 사실은 값 집합 경계만 나타냅니다.
- 비주장: 이 표는 동작이나 보장을 활성화하지 않습니다.

| 이름 | 경계 |
|---|---|
| `preventive` | 프로필 조건부 `GuaranteeDisplay.level`입니다. 승격된 예방 메커니즘과 증명 경로가 필요합니다. |
| `isolated` | 이후 후보 또는 프로필 조건부 값으로 예약된 `GuaranteeDisplay.level` 라벨입니다. |
| `captured_artifact`와 접점 자체 캡처 이름 | 활성 `ArtifactInput.source_kind`로는 예약 또는 거절됩니다. |
| 접점 간 스테이징된 아티팩트 인계 | 활성 기능이 아닙니다. 스테이징된 아티팩트 승격에는 기록된 접점 출처가 일치해야 합니다. |
| QA 면제와 검증 위험 판단 종류 | 이후 후보입니다. 활성 `judgment_kind` 값이 아닙니다. |
| `design_policy` 차단 사유 범주 | 스키마와 닫기 준비 상태 담당 문서가 승격하기 전까지 이후 또는 비활성 값입니다. |

`isolated` 세부사항:
- 담당 경계: 이 문서는 값 집합 항목만 담당합니다.
- 비주장: 이 항목은 현재 격리 보장을 부여하거나 정의하지 않습니다.
- 담당 문서 링크: 보장 의미는 [보안](../security.md)이 담당합니다. 현재 MVP에서의 사용 가능 여부는 [현재 MVP 범위](../scope.md)가 담당합니다.

활성 아티팩트 입력은 `staged_artifact` 또는 `existing_artifact`를 사용합니다. 아티팩트 출처 의미는 [API 아티팩트 스키마](schema-artifacts.md)와 [아티팩트 저장소](../storage-artifacts.md)가 담당합니다.

## 관련 담당 문서

- [현재 MVP 범위](../scope.md): 값이 현재 MVP에 속하는지 판단.
- [API 오류](errors.md): 공개 오류 코드와 우선순위.
- [API 코어 스키마](schema-core.md), [API 상태 스키마](schema-state.md), [API 아티팩트 스키마](schema-artifacts.md), [API 판단 스키마](schema-judgment.md): 이 값을 쓰는 필드.
- [API 메서드](methods.md)와 메서드 담당 문서: 이 값을 사용하는 메서드 동작.
- [범위 참조](../scope.md): 비활성 값 이름.
