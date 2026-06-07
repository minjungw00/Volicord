# API Schema Core

## 이 문서로 할 수 있는 일

현재 MVP에서 쓰는 활성 공용 API 형태를 확인할 때 이 참조를 사용합니다. `ToolEnvelope`, 공통 응답, `ArtifactRef`, `StateRecordRef`, `UserJudgment`, Write Authorization 요약, 증거 요약, 실행 요약, 닫기 차단 사유, 다음 행동 요약, 현재 MVP 값 집합, profile-gated 표시 값 이름, later 후보 값 이름의 경계를 다룹니다.

이 문서는 향후 하네스 서버 동작을 계획하고 검토하기 위한 참조입니다. 현재 문서 저장소에 MCP server가 구현되어 있다는 뜻이 아닙니다. 향후 schema 후보는 [Later 후보 색인](../../later/index.md#later-schema-candidates)에 남습니다.

## 계약 위치 지도

| 필요한 것 | 담당 문서 |
|---|---|
| 활성 method와 method 요청/응답 소유권 | [MVP API](mvp-api.md) |
| 공개 오류, 우선순위, idempotency, 차단 응답, stale-state 동작 | [API Errors](errors.md) |
| Core 상태 의미와 lifecycle 의미 | [Core Model 참조](../core-model.md) |
| 저장소 테이블, JSON `TEXT`, enum hardening, artifact persistence | [Storage](../storage.md) |
| 보안 보장 의미 | [보안 참조](../security.md) |
| 향후 API/schema 후보 | [Later 후보 색인](../../later/index.md#later-schema-candidates) |

## 스키마 표기 규칙

이 문서의 YAML 형식 표기는 예시라고 표시하지 않는 한 규범 스키마 표기입니다.

- `field: Type`은 필드가 필수이고 non-null이라는 뜻입니다.
- `field: Type | null`은 필드가 필수이고 JSON `null`을 허용한다는 뜻입니다.
- `Type[]`은 필드가 존재하고 배열을 담는다는 뜻입니다. 빈 배열은 `[]`로 씁니다.
- `a | b | c`는 닫힌 활성 enum입니다.
- 명시되지 않은 필드는 명시적인 extension container 밖에서 거부됩니다.

Storage validation은 별도 담당 문서 경계입니다. API payload와 API-shaped stored JSON은 먼저 이 API 참조로 검증합니다. DDL, storage-only JSON, default, lock, migration은 [Storage](../storage.md)가 담당합니다.

<a id="tool-envelope"></a>

## Tool Envelope

모든 public tool 요청은 `ToolEnvelope`를 가집니다. 상태를 바꾸는 tool은 non-null `idempotency_key`와 current `expected_state_version`을 요구합니다. `harness.status`는 read-only이며 `expected_state_version`을 `null`로 둘 수 있습니다.

```yaml
ToolEnvelope:
  request_id: string
  idempotency_key: string | null
  expected_state_version: integer | null
  project_id: string
  task_id: string | null
  surface_id: string
  actor_kind: user | lead_agent | evaluator | operator
  dry_run: boolean
```

Envelope 필드는 call의 경로를 정하고 감사 추적에 쓰입니다. `surface_id`는 capability, write authority, local access, user judgment, 민감 동작 승인, final acceptance, residual-risk acceptance, close를 부여하지 않습니다.

<a id="common-response"></a>

## 공통 응답

```yaml
ToolResponseBase:
  request_id: string
  idempotency_key: string | null
  project_id: string
  task_id: string | null
  state_version: integer
  dry_run: boolean
  errors: ToolError[]
  validator_results: ValidatorResult[]
  events: EventRef[]

ToolError:
  code: ErrorCode
  message: string
  retryable: boolean
  details: object

EventRef:
  event_id: string
  event_seq: integer
  event_type: string
  task_id: string | null
  state_version: integer
```

`ToolResponseBase.state_version`은 committed mutation에서는 영향을 받은 범위의 resulting version이고, read-only와 dry-run 응답에서는 current readable version 또는 would-be affected version입니다. `dry_run=true`는 current record, event, artifact, evidence summary, Write Authorization, close state, idempotency replay row를 만들지 않습니다.

## StateSummary

```yaml
StateSummary:
  mode: advisor | direct | work
  lifecycle_phase: intake | shaping | ready | executing | waiting_user | blocked | completed | cancelled
  result: none | advice_only | passed | failed | cancelled
  close_reason: none | completed_self_checked | completed_with_risk_accepted | cancelled | superseded
  assurance_level: none | self_checked
  gates:
    scope_gate: not_required | required | pending | passed | failed | blocked
    decision_gate: not_required | required | pending | resolved | deferred | blocked
    approval_gate: not_required | required | pending | granted | denied | expired
    design_gate: not_required | required | pending | passed | partial | waived | stale | blocked
    evidence_gate: not_required | none | partial | sufficient | stale | blocked
    acceptance_gate: not_required | required | pending | accepted | rejected

GuaranteeDisplay:
  level: cooperative | detective
  notes: string[]
```

화면에 표시되는 라벨은 canonical schema value가 아닙니다. `GuaranteeDisplay.level`은 문서화된 접점 역량과 증명 수준을 보여 주는 표시 주장입니다. 권한이나 상태 권한을 부여하지 않습니다. 현재 MVP의 기본 guarantee-display 값은 `cooperative`와 `detective`입니다. `preventive`와 `isolated`는 profile-gated 표시 값이며 현재 MVP의 기본 보장이 아닙니다.

<a id="staterecordref"></a>

## StateRecordRef

```yaml
StateRecordRef:
  record_kind: project | task | change_unit | run | write_authorization | user_judgment | evidence_summary | blocker
  record_id: string
```

Durable evidence byte는 `StateRecordRef`가 아니라 `ArtifactRef`를 사용합니다. 민감 동작 승인, final acceptance, residual-risk acceptance, QA waiver, verification-risk acceptance, cancellation은 matching `UserJudgment.judgment_kind`를 가진 `record_kind=user_judgment`로 표현합니다.

<a id="artifactref"></a>

## ArtifactRef

`ArtifactRef`는 Harness storage에 등록된 durable evidence file을 가리킵니다. 호출자가 임의로 준 path가 아닙니다.

```yaml
ArtifactRef:
  artifact_id: string
  kind: diff | log | screenshot | checkpoint | other
  uri: string
  sha256: string
  size_bytes: integer
  content_type: string
  redaction_state: none | redacted | secret_omitted | blocked
  task_id: string
  run_id: string | null
  relation_owner: ArtifactRelationOwner
  created_at: string
  produced_by: lead_agent | evaluator | operator | harness
  retention_class: task | project | temporary

ArtifactRelationOwner:
  task_id: string
  run_id: string | null
  record_kind: task | change_unit | run | user_judgment | evidence_summary | blocker
  record_id: string
  relation: string
```

`uri`는 Harness storage를 통해 해소되며 보통 `harness-artifact://{project_id}/{artifact_id}`입니다. Raw secret, token, full sensitive log를 evidence로 저장하면 안 됩니다. Content가 redacted, omitted, blocked이면 `sha256`와 `size_bytes`는 hidden original이 아니라 committed safe bytes를 설명합니다.

<a id="artifactinput"></a>

## ArtifactInput

`ArtifactInput`은 `harness.record_run`에서 staging, capture-adapter, existing-artifact handle로만 받습니다. 임의 파일 읽기 권한을 부여하지 않습니다.

```yaml
ArtifactInput:
  artifact_input_id: string
  source_kind: staged_file | capture_adapter | existing_artifact
  relation: string
  staged_uri: string | null
  capture_ref: string | null
  existing_artifact_ref: ArtifactRef | null
  display_name: string | null
  content_type: string
  expected_sha256: string | null
  expected_size_bytes: integer | null
```

`source_kind`에 맞는 source field 하나만 있어야 합니다. 잘못된 source shape, 호출자가 임의로 준 path, raw secret, token, full sensitive log는 mutation 전에 거부됩니다.

<a id="evidence-and-pre-write-scope-schemas"></a>

## Evidence와 쓰기 전 범위 schema

```yaml
EvidenceCoverageItem:
  claim_or_criterion: string
  coverage_state: supported | unsupported | partial | not_applicable | stale | blocked
  supporting_state_refs: StateRecordRef[]
  supporting_artifact_refs: ArtifactRef[]
  gap_blocker_refs: StateRecordRef[]
  note: string | null

EvidenceSummary:
  evidence_summary_ref: StateRecordRef | null
  task_id: string
  change_unit_id: string | null
  status: not_required | none | partial | sufficient | stale | blocked
  coverage_items: EvidenceCoverageItem[]
  supporting_run_refs: StateRecordRef[]
  supporting_artifact_refs: ArtifactRef[]
  gap_blocker_refs: StateRecordRef[]
  summary: string
  updated_at: string

AuthorizedAttemptScope:
  task_id: string
  change_unit_id: string
  basis_state_version: integer
  surface_id: string
  intended_operation: string
  intended_paths: string[]
  intended_tools: string[]
  intended_commands:
    - command: string
      command_class: string
      writes_product_files: boolean
  product_file_write_intended: boolean
  intended_network:
    - target: string
      direction: read | write
  intended_secret_scope:
    - secret_handle: string
      access_kind: read | write
  sensitive_categories: string[]
  baseline_ref: string | null
  related_user_judgment_refs: StateRecordRef[]
  guarantee_level: cooperative | detective

WriteAuthorizationSummary:
  write_authorization_id: string
  attempt_scope: AuthorizedAttemptScope
  status: active | consumed | expired | stale | revoked
  consumed_by_run_id: string | null
  created_at: string
  consumed_at: string | null

WriteAuthoritySummary:
  active_change_unit_ref: StateRecordRef | null
  write_authorization_ref: StateRecordRef | null
  active_authorized_attempt_scope: AuthorizedAttemptScope | null
  approval_status: not_required | required | pending | granted | denied | expired | unknown
  guarantee_display: GuaranteeDisplay
```

`EvidenceSummary`는 활성 compact evidence record입니다. 상세 증거 보고서, 별도 보증 결과, 최종 수락, 잔여 위험 수락, 렌더링된 보기가 아닙니다.

`AuthorizedAttemptScope`는 `write_authorizations.attempt_scope_json`에 저장되고 나중에 `harness.record_run`에서 비교하는 exact scope입니다. `WriteAuthorizationSummary.status`는 durable authorization lifecycle입니다. `blocked`는 Write Authorization status가 아닙니다. 차단된 쓰기는 소비 가능한 authorization 없이 blocker를 반환합니다.

<a id="record-run-payloads"></a>

## Record-Run Payloads

```yaml
ObservedChanges:
  product_write: boolean
  changed_paths: string[]
  no_product_changes: boolean
  summary: string

RunSummary:
  run_ref: StateRecordRef
  kind: shaping_update | implementation | direct
  status: completed | interrupted | blocked | violation
  product_write: boolean
  write_authorization_ref: StateRecordRef | null
  evidence_summary_ref: StateRecordRef | null
  artifact_refs: ArtifactRef[]
  summary: string
  started_at: string | null
  completed_at: string
```

`status=completed`만 정상 담당 ref를 통해 evidence를 뒷받침할 수 있습니다. `interrupted`, `blocked`, `violation`은 audit/recovery fact이며 evidence, final acceptance, residual-risk acceptance, close를 스스로 충족하지 않습니다.

<a id="userjudgment"></a>

## UserJudgment

```yaml
UserJudgment:
  user_judgment_id: string
  task_id: string
  change_unit_id: string | null
  status: proposed | pending_user | resolved | deferred | rejected | blocked | superseded
  judgment_kind: product_decision | technical_decision | scope_decision | sensitive_approval | qa_waiver | verification_risk_acceptance | final_acceptance | residual_risk_acceptance | cancellation
  presentation: short
  question: string
  options: UserJudgmentOption[]
  context: UserJudgmentContext
  affected_refs: StateRecordRef[]
  required_for: next_action | write | run | close | acceptance | risk
  resolution: UserJudgmentResolution | null
  expires_at: string | null
  created_at: string
  updated_at: string
  resolved_at: string | null

UserJudgmentOption:
  option_id: string
  label: string
  meaning: approve | reject | defer | choose | cancel
  consequence: string

UserJudgmentContext:
  why_now: string
  source_refs: StateRecordRef[]
  evidence_summary_ref: StateRecordRef | null
  what_user_is_judging: string
  why_agent_cannot_decide: string
  no_decision_consequence: string

UserJudgmentResolution:
  selected_option_id: string
  answer: RecordUserJudgmentPayload
  note: string | null
```

`judgment_kind`는 canonical decision-type field입니다. 렌더링된 라벨과 지역화된 라벨은 schema value가 아닙니다. `presentation=short`가 활성 MVP presentation입니다. Expanded presentation body는 활성 API schema가 아닙니다.

<a id="userjudgmentcandidate"></a>

## UserJudgmentCandidate

```yaml
UserJudgmentCandidate:
  judgment_kind: product_decision | technical_decision | scope_decision | sensitive_approval | qa_waiver | verification_risk_acceptance | final_acceptance | residual_risk_acceptance | cancellation
  presentation: short
  question: string
  options: UserJudgmentOption[]
  context: UserJudgmentContext
  affected_refs: StateRecordRef[]
  required_for: next_action | write | run | close | acceptance | risk
```

Candidate는 committed `user_judgment` row가 아닙니다. `StateRecordRef`가 없고 gate를 충족하지 않으며 민감 동작 승인, final acceptance, residual-risk acceptance, evidence, Write Authorization, close state를 만들지 않습니다.

```yaml
RecordUserJudgmentPayload:
  selected_option_id: string
  approval_scope: AuthorizedAttemptScope | null
  accepted_result_refs: StateRecordRef[]
  cancellation_reason: string | null
  note: string | null
```

`judgment_kind=sensitive_approval`에서는 `approval_scope`가 pending judgment와 맞아야 합니다. `final_acceptance`에서는 `accepted_result_refs`가 visible basis를 이름 붙입니다. `cancellation`에서는 `cancellation_reason`이 필요합니다.

<a id="acceptedriskinput"></a>

## AcceptedRiskInput

```yaml
AcceptedRiskInput:
  visible_risk_ref: StateRecordRef
  accepted: boolean
  user_note: string | null
```

`AcceptedRiskInput`은 `judgment_kind=residual_risk_acceptance`에서만 valid합니다. `visible_risk_ref`는 같은 Task의 visible close-relevant `blocker`를 가리켜야 합니다. Standalone residual-risk record를 만들지 않습니다.

<a id="current-position-display-schemas"></a>

## Current-Position Display Schemas

```yaml
CloseBlocker:
  category: task | open_run | scope | user_judgment | sensitive_approval | design_policy | write_compatibility | baseline | surface_capability | evidence | artifact_availability | final_acceptance | residual_risk_visibility | residual_risk_acceptance | cancellation | supersession | recovery
  code: ErrorCode
  message: string
  related_refs: StateRecordRef[]
  required_judgment_kind: product_decision | technical_decision | scope_decision | sensitive_approval | qa_waiver | verification_risk_acceptance | final_acceptance | residual_risk_acceptance | cancellation | null
  next_action: string

NextActionSummary:
  action_kind: ask_user | prepare_write | implement | request_acceptance | close_task | idle
  summary: string
  required_tool: harness.intake | harness.status | harness.prepare_write | harness.record_run | harness.request_user_judgment | harness.record_user_judgment | harness.close_task | null
  related_refs: StateRecordRef[]
  blocker_code: ErrorCode | null
```

`CloseBlocker`는 structured blocker result입니다. Prose-only status text, report, rendered view는 blocker result가 아닙니다.

<a id="nextactionsummary"></a>

## NextActionSummary

`NextActionSummary`는 [Current-position display schemas](#current-position-display-schemas)에 정의되어 있습니다. 활성 `action_kind` 값은 정확히 다음과 같습니다.

```text
ask_user | prepare_write | implement | request_acceptance | close_task | idle
```

<a id="validatorresult"></a>

## ValidatorResult

```yaml
ValidatorResult:
  validator_id: surface_capability_check
  validator_kind: capability
  status: passed | warning | failed | blocked | skipped
  guarantee_level: cooperative | detective
  checked_at: string
  target:
    task_id: string | null
    change_unit_id: string | null
    run_id: string | null
    artifact_id: string | null
  summary: string
  findings:
    - code: string
      severity: info | warning | error | blocker
      message: string
      path: string | null
      artifact_ref: ArtifactRef | null
  blocked_reasons: string[]
  suggested_next_action: string | null
```

활성 stable validator ID는 `surface_capability_check`입니다. Validator output은 blocker, 대체 동작, guarantee display에 영향을 줄 수 있습니다. Write Authorization, user judgment, evidence, final acceptance, residual-risk acceptance, close를 만들지 않습니다.

<a id="sensitive-categories"></a>

## Sensitive Categories

Sensitive category는 왜 민감 동작 승인이 필요할 수 있는지 설명합니다. 제품 판단, 기술 판단, 범위 판단, QA, verification, acceptance, residual-risk, policy question을 결정하지 않습니다.

```text
auth_change
permission_model_change
schema_change
dependency_change
public_api_change
destructive_write
network_write
external_service_write
secret_access
production_config_change
ci_cd_change
infra_or_deployment_change
privacy_or_pii_change
data_export
telemetry_or_logging_change
license_or_compliance_change
billing_or_cost_change
model_or_prompt_policy_change
policy_override
```

<a id="current-mvp-value-sets"></a>

## 현재 MVP 값 집합

아래 값은 승격된 profile 없이 사용할 수 있는 현재 MVP 값 집합입니다. 여기에 없는 값은 현재 MVP의 기본 활성 값이 아닙니다. 화면에 표시되는 라벨은 canonical schema value가 아닙니다.

| 필드 | 현재 MVP 값 |
|---|---|
| 활성 메서드 집합 | `harness.intake`, `harness.status`, `harness.prepare_write`, `harness.record_run`, `harness.request_user_judgment`, `harness.record_user_judgment`, `harness.close_task` |
| `ToolEnvelope.actor_kind` | `user`, `lead_agent`, `evaluator`, `operator` |
| `StateSummary.mode` | `advisor`, `direct`, `work` |
| `StateSummary.lifecycle_phase` | `intake`, `shaping`, `ready`, `executing`, `waiting_user`, `blocked`, `completed`, `cancelled` |
| `UserJudgment.status` | `proposed`, `pending_user`, `resolved`, `deferred`, `rejected`, `blocked`, `superseded` |
| `UserJudgment.judgment_kind` | `product_decision`, `technical_decision`, `scope_decision`, `sensitive_approval`, `qa_waiver`, `verification_risk_acceptance`, `final_acceptance`, `residual_risk_acceptance`, `cancellation` |
| `WriteAuthorizationSummary.status` | `active`, `consumed`, `expired`, `stale`, `revoked` |
| `RunSummary.kind` | `shaping_update`, `implementation`, `direct` |
| `RunSummary.status` | `completed`, `interrupted`, `blocked`, `violation` |
| `EvidenceSummary.status` | `not_required`, `none`, `partial`, `sufficient`, `stale`, `blocked` |
| `EvidenceCoverageItem.coverage_state` | `supported`, `unsupported`, `partial`, `not_applicable`, `stale`, `blocked` |
| `ArtifactRef.redaction_state` | `none`, `redacted`, `secret_omitted`, `blocked` |
| `CloseBlocker.category` | `task`, `open_run`, `scope`, `user_judgment`, `sensitive_approval`, `design_policy`, `write_compatibility`, `baseline`, `surface_capability`, `evidence`, `artifact_availability`, `final_acceptance`, `residual_risk_visibility`, `residual_risk_acceptance`, `cancellation`, `supersession`, `recovery` |
| `NextActionSummary.action_kind` | `ask_user`, `prepare_write`, `implement`, `request_acceptance`, `close_task`, `idle` |
| `GuaranteeDisplay.level` | `cooperative`, `detective` |
| `AuthorizedAttemptScope.guarantee_level` | `cooperative`, `detective` |
| `ValidatorResult.guarantee_level` | `cooperative`, `detective` |

`GuaranteeDisplay.level`에서 `cooperative`는 현재 MVP의 기본값입니다. `detective`도 현재 MVP 값이지만, 활성 접점이 관련 사실을 정직하게 관찰할 수 있는 곳에서만 사용할 수 있습니다. 두 값 모두 OS 권한, 임의 도구 샌드박스, 변조 방지 저장소, 도구 실행 전 차단, 격리를 뜻하지 않습니다.

<a id="profile-gated-value-names"></a>

## Profile-Gated 값 이름

아래 이름은 승격된 profile이 해당 보장을 명시적으로 지원할 때만 나타날 수 있습니다. 현재 MVP의 기본 보장이 아닙니다. `preventive`와 `isolated`는 profile-gated 표시 값이며 현재 MVP의 기본 보장이 아닙니다.

| 필드 | profile-gated 값 이름 | 요구사항 |
|---|---|---|
| `GuaranteeDisplay.level` | `preventive` | 대상 동작에 대한 명시적 도구 실행 전 차단 지원이 필요합니다. 또한 담당 문서가 동작, 대체 동작, 증명 경로를 정의해야 합니다. |
| `GuaranteeDisplay.level` | `isolated` | 대상 경계에 대한 명시적 격리 지원이 필요합니다. 또한 이름 붙은 경계, 담당 문서가 정의한 동작, 대체 동작, 증명 경로가 있어야 합니다. |

Profile-gated 표시 값 이름은 그 자체로 Write Authorization, 검증기, 저장소, 오류 동작을 넓히지 않습니다. 지원되지 않는 값의 사용 또는 표시 요청은 역량 부족 또는 검증 실패로 남습니다. 더 강한 보장이 존재한다는 증거가 아닙니다.

<a id="later-candidate-value-names"></a>

## Later 후보 값 이름

Later 후보 값 이름은 승격된 담당 문서가 정확한 활성 필드, 값 집합, validator, 대체 동작, 증명 기대치를 이 문서나 다른 활성 담당 문서에 추가하기 전까지 [Later 후보 색인](../../later/index.md#later-schema-candidates)에만 남는 catalog-only 이름입니다.

이 활성 API 참조는 later schema body를 일부러 정의하지 않습니다. Later 후보 이름은 catalog에 있다는 이유만으로 활성 enum member, schema field, storage value, public method, validator requirement, close blocker, guarantee value가 되지 않습니다.

| 출처 | 활성 API 경계 |
|---|---|
| Later schema extensions | 후보 이름일 뿐이며 활성 요청, 응답, 공용 schema, enum member가 아닙니다. |
| Later ref and artifact values | 후보 이름일 뿐이며 활성 `ArtifactRef`, `StateRecordRef`, storage, evidence, QA, export, projection 값이 아닙니다. |
| Later template, fixture, conformance, operation, export, diagnostic names | 후보 이름일 뿐이며 활성 API payload, runtime operation, error family, conformance-runner behavior, close effect가 아닙니다. |
