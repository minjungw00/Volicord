# API Schema Core

## 이 문서로 할 수 있는 일

[MVP API](mvp-api.md)의 MVP-1 method를 뒷받침하는 shared API shape를 확인할 때 이 참조를 사용합니다. Request envelope, common response, read-only resource schema, shared ref, artifact input, user-judgment payload, next-action summary, 활성 MVP-1 value set을 다룹니다.

이 문서는 향후 하네스 서버 동작을 계획하고 검토하기 위한 참조입니다. 현재 문서 저장소에 MCP server가 구현되어 있다는 뜻이 아닙니다.

## 계약 위치 지도

| 필요한 것 | 섹션 |
|---|---|
| Active MVP-1 tools | [MVP API](mvp-api.md) |
| Error code, MVP-1 status/error condition, precedence, idempotency, stale-state behavior | [Errors](errors.md) |
| Later/profile-gated schemas and methods | [Schema Later](schema-later.md) |
| Core Model state semantics | [Core Model 참조](../core-model.md) |
| Storage and DDL | [Storage](../storage.md) |
| Compact view behavior와 template body | [Projection과 Template 참조](../projection-and-templates.md)와 [Template 참조](../templates/README.md) |

## Schema notation convention

이 API 문서의 YAML-like block은 예시라고 명시하지 않는 한 normative schema notation입니다.

- `field: Type`은 field가 required이고 non-null이어야 한다는 뜻입니다.
- `field: Type | null`은 field가 required이고 JSON `null`을 허용한다는 뜻입니다.
- Optional field는 prose나 profile-extension text에서 optional이라고 명시해야 합니다.
- `Type[]`은 item이 `Type`과 맞는 array입니다. `[]`는 present empty collection입니다.
- `one_of:`는 listed branch 중 정확히 하나만 present해야 한다는 뜻입니다.
- `a | b | c`는 section이 extensible이라고 명시하지 않는 한 closed enum입니다.
- 명시되지 않은 field는 explicit extension container 밖에서 reject됩니다.
- Later/profile-gated enum value와 branch는 MVP-1에서 valid하지 않으며, 아래 활성 schema block에 들어가지 않습니다. 해당 값은 [Schema Later](schema-later.md)에 정의합니다.

Storage validation은 별도 소유권 경계입니다. API payload와 API-shaped stored JSON은 먼저 이 API reference로 validate합니다. Storage-only JSON `TEXT`, DDL nullability, column default, storage hardening은 [Storage](../storage.md)이 담당합니다.

## Stage Profile Manifest

이 문서의 schema block은 활성 MVP-1 API shape를 정의합니다. 내부 엔지니어링 점검은 그중 더 좁은 subset을 사용할 수 있습니다. Later/profile value는 이 문서에 넓게 넣은 뒤 prose로 막지 않고 [Schema Later](schema-later.md)에 따로 둡니다.

| Stage/profile | Active API slice | 해당 slice에서 active가 아닌 것 |
|---|---|---|
| 내부 엔지니어링 점검 | Minimal status/blocker read, owner-valid setup path 하나, registered reference `capability_profile` 하나, active Task, active Change Unit/scope boundary, `harness.prepare_write`, compatible `harness.record_run` 하나, artifact/evidence ref 하나, structured status/blocker output, 좁은 close-blocker check. | Full natural-language intake, stored user judgment path, full Evidence Manifest, detached verification, Manual QA, final acceptance, residual-risk acceptance, rich projections, export/recover, broad connector APIs, hosted connector registry, cross-surface orchestration, broad operations. |
| MVP-1 사용자 작업 루프 | Active method set은 [MVP API](mvp-api.md#mvp-1-method-set)가 담당하며, 다음 안전한 행동 output은 `harness.status.next_actions`에 담깁니다. 같은 reference `capability_profile` 하나가 guarantee display와 capability blocker를 제어합니다. Method set은 정확히 `harness.status`, `harness.intake`, `harness.request_user_judgment`, `harness.record_user_judgment`, `harness.prepare_write`, `harness.record_run`, `harness.close_task`입니다. | 별도 `harness.next`, detached verification launch/Eval, full Manual QA matrix, committed Approval hardening, export/recover, advanced connector APIs, hosted connector registry, cross-surface orchestration, broad operations, detailed diagnostic projections. |
| 보증 프로필, 운영 프로필, later | Owner docs가 승격할 때 verification, Eval, Manual QA, waiver, full residual-risk acceptance, reconcile, validators, projection/report/export/recover, operations, advanced connectors. | 내부 엔지니어링 점검이나 minimum MVP-1 requirement가 아닙니다. |

## Read-only resources

MCP resource는 read-only view입니다. Task, user judgment, projection job, reconciliation, evidence, QA, final acceptance, residual-risk acceptance, Write Authorization, close state를 만들면 안 됩니다.

Read-only resource도 세 부분 맥락 모델을 따릅니다. `harness://status/card`는 사용자 상태 카드입니다. Current Core state와 ref에서 만든 짧은 읽기용 보기입니다. Agent 접점은 read-only resource를 사용해 다음 안전한 행동에 필요한 최소 state, ref, freshness, owner-section pointer를 담은 에이전트 맥락 패킷을 만들 수 있습니다. Core 상태가 로컬 권한 기록이며 유일한 운영 기준입니다. 오래된 card나 projection은 authority가 아니며, 렌더링된 template은 민감 동작 승인, 최종 수락, 잔여 위험 수락, 증거, 닫기 준비 상태를 만들 수 없습니다.

### 내부 엔지니어링 점검 resources

| Resource | Profile meaning |
|---|---|
| `harness://project/current` | Current registered project identity, reference `capability_profile` availability facts, local MCP availability facts. |
| `harness://task/active` | Task를 만들지 않고 active Task pointer 또는 explicit `none` / `unknown`을 반환합니다. |
| `harness://task/{task_id}` | 좁은 권한 루프를 위한 current Task state. |
| `harness://task/{task_id}/summary` | Optional compact Task status/blocker summary. |
| `harness://status/card` | Current Core state와 ref에서 파생한 optional compact current-position 사용자 상태 카드. |

### MVP-1 resources

| Resource | Profile meaning |
|---|---|
| `harness://task/{task_id}/user-judgments` | Active, resolved, deferred, blocked `user_judgment` summary. |
| `harness://task/{task_id}/judgment-context` | 사용자 판단에 필요한 minimum current context. |

MVP-1 evidence와 close-readiness path는 output이 current Core state와 refs에서 파생된다면 정확한 사용자용 작은 출력인 `status-card`, `judgment-request`, `run-evidence-summary`, `close-result` 또는 `harness.status`, `harness://task/{task_id}/summary`, `harness://status/card`로 표시할 수 있습니다. Agent 접점은 current Core state와 refs에서 별도의 에이전트용 `agent-context-packet`을 만들 수 있습니다. 정확한 compact view 동작과 template body는 [Projection과 Template 참조](../projection-and-templates.md)와 [Template 참조](../templates/README.md)에 남습니다.

### Later resources

Evidence-manifest read, report read, bundle read, design map, Journey view, broad projection resource 같은 assurance, operations, diagnostic resource는 later/profile-gated입니다. [Schema Later](schema-later.md#later-read-only-resources)를 봅니다.

## Tool envelope

모든 public tool request는 envelope를 가집니다. State-changing tool은 non-null `idempotency_key`와 `expected_state_version`을 요구합니다. Read-only tool은 tracing을 위해 같은 envelope를 받을 수 있으며 `expected_state_version`을 `null`로 둘 수 있습니다.

```yaml
ToolEnvelope:
  request_id: string
  idempotency_key: string | null
  expected_state_version: integer | null
  project_id: string
  task_id: string | null
  surface_id: string
  run_id: string | null
  actor_kind: user | lead_agent | evaluator | operator
  dry_run: boolean
```

Envelope field는 routing과 audit claim입니다. `surface_id`는 capability나 write authority를 부여하지 않습니다. Surface가 Core 밖에서 state를 바꾸도록 승인하지 않으며, user judgment, 민감 동작 승인, final acceptance, Manual QA, detached verification independence를 증명하지도 않습니다.

Primary Task가 필요한 request에서 Core는 tool-specific `task_id`, `ToolEnvelope.task_id`, active Task resolution 순서로 primary Task를 찾습니다. Task-scoped mutation은 `expected_state_version`을 `tasks.state_version`과 비교합니다. Resolved primary Task가 없는 project-scoped mutation은 `project_state.state_version`과 비교합니다.

## MCP boundary and caller trust

내부 엔지니어링 점검/default posture는 registered reference project surface 하나에 대한 local-only exposure입니다. Local-only는 예상 local user/profile의 local process, local socket, localhost-loopback connection을 뜻합니다. Unauthenticated shared endpoint, non-loopback bind, forwarded/tunneled endpoint, cloud/CI relay, cross-user socket/directory, remote caller는 registered connector profile이 stronger posture를 증명하기 전까지 제외합니다.

Public schema는 display-safe access material class, bind/reachability posture, freshness, profile refs, conformance/operator-check refs, safe handle/fingerprint를 담을 수 있습니다. 이 profile fact는 display, validation, diagnostic을 위한 것이며 hosted connector registry가 아닙니다. Raw token, secret, private configuration value, omitted secret value, blocked payload bytes를 담으면 안 됩니다.

Core에 닿지 못하면 authoritative Core response가 없습니다. `MCP_UNAVAILABLE` 또는 diagnostic `MCP_SERVER_UNAVAILABLE`을 보고합니다. Core나 operator가 reachable local caller/access path를 registered profile 밖으로 분류할 수 있으면 display-safe detail과 함께 `LOCAL_ACCESS_MISMATCH`를 사용합니다. Recognized profile이 required capability를 갖지 못하면 `CAPABILITY_INSUFFICIENT` 또는 equivalent structured blocked reason을 사용하고 guarantee display를 낮춥니다. Core unavailable, local access denied, unsupported surface, stale state에 대해 사용자에게 보이는 동작은 [Errors: MVP-1 guarantee와 상태/error taxonomy](errors.md#mvp-1-guarantee-and-status-taxonomy)가 담당합니다.

## Common response

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
  projection_jobs: ProjectionJobRef[]

ToolError:
  code: ErrorCode
  message: string
  retryable: boolean
  details: object
```

MVP-1 status/error condition이 적용될 때 `ToolError.message`는 [Errors](errors.md#mvp-1-guarantee-and-status-taxonomy)의 정직한 사용자 표시 문구 pattern을 따라야 합니다.

내부 엔지니어링 점검과 MVP-1에서 `projection_jobs`는 envelope compatibility를 위해 present하며 보통 `[]`입니다. 이 field가 `projection_jobs` storage table을 요구하지 않습니다. Durable projection job은 운영 프로필 또는 profile-promoted storage입니다.

`dry_run=true`는 validate하고 diagnostics 또는 transition plan을 반환하지만 current record 변경, event append, artifact 등록, consumable Write Authorization 생성, projection job enqueue, idempotency replay row create/update, `idempotency_key` 예약을 하지 않습니다.

State-changing operation에서 `ToolResponseBase.state_version`은 primary affected scope의 resulting version입니다. Task-scoped mutation 뒤에는 `tasks.state_version`이고, resolved primary Task가 없는 project-scoped mutation 뒤에는 `project_state.state_version`입니다. Read-only와 dry-run response는 primary read scope 또는 변경될 affected scope의 current version을 반환합니다.

## Shared schemas

```yaml
EventRef:
  event_id: string
  event_seq: integer
  event_type: string
  task_id: string | null
  state_version: integer

ProjectionJobRef:
  projection_job_id: string
  projection_kind: ProjectionKind
  target_ref: string
  projection_version: integer

StateSummary:
  mode: advisor | direct | work
  lifecycle_phase: intake | shaping | ready | executing | verifying | qa | waiting_user | blocked | completed | cancelled
  result: none | advice_only | passed | failed | cancelled
  close_reason: none | completed_verified | completed_self_checked | completed_with_risk_accepted | cancelled | superseded
  assurance_level: none | self_checked | detached_verified
  gates:
    scope_gate: not_required | required | pending | passed | failed | blocked
    decision_gate: not_required | required | pending | resolved | deferred | blocked
    approval_gate: not_required | required | pending | granted | denied | expired
    design_gate: not_required | required | pending | passed | partial | waived | stale | blocked
    evidence_gate: not_required | none | partial | sufficient | stale | blocked
    verification_gate: not_required | required | pending | passed | failed | waived_by_user | blocked
    qa_gate: not_required | required | pending | passed | failed | waived
    acceptance_gate: not_required | required | pending | accepted | rejected
```

`EventRef.state_version`은 해당 event 이후 affected-scope resulting version입니다. Event ordering key가 아니며 event ordering은 `event_seq`를 사용합니다.

`StateSummary.mode` values는 `advisor`, `direct`, `work`로 유지합니다. 사용자 접점은 이를 advice/read-only work, small direct work, tracked work로 표시할 수 있습니다. 그 label은 display text이지 enum value가 아닙니다.

### ProjectionKind support

`ProjectionKind`는 extensible이지만 profile-gated입니다.

| Support class | Values | Requirement |
|---|---|---|
| Core status output | none required | 내부 엔지니어링 점검은 persisted Markdown projection job 없이 status/blocker output을 노출할 수 있습니다. |
| MVP-1 작은 출력 | Persisted `ProjectionKind`는 필요하지 않습니다. 독자별 compact output name과 동작은 [Projection과 Template 참조](../projection-and-templates.md#mvp-1-보기-세트)와 [Template 참조](../templates/README.md#mvp-1-템플릿-세트)가 담당합니다. | 네 가지 사용자용 출력과 에이전트용 패킷 하나는 full template rendering 없이 MVP-1을 충족할 수 있습니다. `TASK`와 `DIRECT-RESULT`는 later/full-profile 또는 compatibility projection입니다. |
| Assurance reports | `APR`, `MANUAL-QA` | Matching approval, Manual QA, waiver, verification, assurance profile이 active일 때만 사용합니다. |
| Operations/export reports | `EXPORT` | Export, release-handoff, operations report profile이 active일 때만 사용합니다. |
| Future/diagnostic projections | `RUN-SUMMARY`, `EVIDENCE-MANIFEST`, `EVAL`, `TDD-TRACE`, `DOMAIN-LANGUAGE`, `MODULE-MAP`, `INTERFACE-CONTRACT`, `DEC`, `DESIGN`, `JOURNEY-CARD` | Owner-promoted later profile이 scope에 있을 때만 enable합니다. |

Projection support는 state, evidence, QA, verification, 민감 동작 승인, final acceptance, residual-risk acceptance, close readiness, close authority, Write Authorization을 만들지 않습니다.

## Sensitive Categories

Sensitive category는 approval-risk label이지 command language가 아닙니다.

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

하나의 intended write가 여러 category를 가질 수 있습니다. Category는 왜 민감 동작 승인이 필요한지 설명할 뿐이며 product, architecture, security, QA, verification, final acceptance, residual-risk acceptance, policy judgment를 해결하지 않습니다.

## ArtifactRef

Artifact ref는 artifact store에 등록된 durable evidence file을 가리킵니다. Artifact registration은 느슨한 파일 덤프가 아닙니다. Core는 `ArtifactRef`를 반환하기 전에 staging/capture source, stored-byte integrity, `redaction_state`, Task-scoped owner relation을 validate합니다.

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

Reference implementation에서 `uri`는 `harness-artifact://{project_id}/{artifact_id}`를 사용합니다. Local file path는 API payload의 absolute path를 신뢰하지 않고 storage를 통해 resolve합니다.

`ArtifactRef`는 contract level에서 artifact identity, owner scope, kind, URI, `sha256`, `size_bytes`, `content_type`, `redaction_state`, `produced_by`, relation owner, `retention_class`를 담습니다. Storage는 relation을 `artifact_links`로 저장할 수 있지만, API caller는 해당 artifact가 어떤 Core 소유 record를 뒷받침하는지 볼 수 있어야 합니다.

`redaction_state` meanings:

| State | Meaning |
|---|---|
| `none` | Stored bytes가 current policy에서 allowed evidence입니다. |
| `redacted` | Sensitive content가 storage 전에 제거되었습니다. |
| `secret_omitted` | Secret value 또는 PII가 의도적으로 omitted되거나 handle로 대체되었습니다. |
| `blocked` | Raw-payload storage가 blocked되었습니다. Metadata notice만 노출할 수 있습니다. |

`redacted`, `secret_omitted`, `blocked`에서 `sha256`와 `size_bytes`는 hidden original이 아니라 committed safe stored bytes를 설명합니다.

Raw secret, token, full sensitive log는 evidence artifact로 저장하면 안 됩니다. Secret 관련 evidence가 필요하면 registered ref는 redacted bytes, omission/blocked metadata notice, 또는 owner path가 허용한 다른 safe representation을 가리켜야 합니다.

## Stage-Specific Active Value Sets

이 table은 분리된 schema를 요약합니다. 왼쪽의 활성 MVP-1 value는 이미 이 문서의 normative schema block에 들어 있습니다. 더 넓은 enum을 prose로 거르는 방식이 아닙니다. Later/profile value는 [Schema Later](schema-later.md)가 소유하며 MVP-1 validator는 reject해야 합니다.

| Field | 활성 MVP-1 schema values | Schema Later가 소유하는 later/profile extension values |
|---|---|---|
| `ArtifactRef.kind`, `ArtifactInput.kind` | `diff`, `log`, `screenshot`, `checkpoint`, `other` | `bundle`, `manifest`, `qa_capture`, `export_component`, `design_probe`, `prototype`, `architecture_scan`, `decision_context` |
| `ArtifactRef.retention_class`, `ArtifactInput.retention_class` | `task`, `project`, `temporary` | `export` |

| Field | 활성 MVP-1 schema values | Schema Later가 소유하는 later/profile extension values |
|---|---|---|
| `ArtifactInput.relation.record_kind`, `ArtifactRef.relation_owner.record_kind` | `task`, `change_unit`, `run`, `user_judgment`, `evidence_summary`, `blocker` | `residual_risk`, `shared_design`, `evidence_manifest`, `eval`, `manual_qa_record`, `feedback_loop`, `tdd_trace`, `projection`, `journey_spine_entry` |
| `StateRecordRef.record_kind` | `task`, `change_unit`, `run`, `write_authorization`, `user_judgment`, `evidence_summary`, `blocker` | `approval`, `residual_risk`, `close_readiness`, `shared_design`, `domain_term`, `module_map_item`, `interface_contract`, `feedback_loop`, `evidence_manifest`, `eval`, `manual_qa_record`, `tdd_trace`, `change_unit_dependency`, `reconcile_item`, `projection` |
| `RecordRunRequest.kind`, `RecordRunPayload.kind` | `shaping_update`, `implementation`, `direct` | `verification_input` |

MVP-1 sensitive-action approval은 `record_kind=user_judgment`를 사용합니다. Committed `approval` ref는 Approval owner profile이 active일 때만 later-profile입니다.

내부 엔지니어링 점검은 이 활성 MVP-1 목록을 더 좁힐 수 있습니다. 예를 들어 stored judgment path가 active가 아니면 `user_judgment`를 생략할 수 있습니다. 그래도 활성 schema 밖의 값을 추가하지는 않습니다.

## ArtifactInput

```yaml
ArtifactInput:
  input_id: string
  source_kind: staged_file | capture_adapter | existing_artifact
  existing_artifact_ref: ArtifactRef | null
  staged: StagedArtifactSource | null
  capture: CaptureAdapterArtifactSource | null
  kind: diff | log | screenshot | checkpoint | other
  redaction_state: none | redacted | secret_omitted | blocked
  produced_by: lead_agent | evaluator | operator | harness
  retention_class: task | project | temporary
  relation:
    task_id: string
    run_id: string | null
    record_kind: task | change_unit | run | user_judgment | evidence_summary | blocker
    record_id_hint: string | null
  description: string | null

StagedArtifactSource:
  staged_uri: string
  display_name: string | null
  content_type: string
  expected_sha256: string | null
  expected_size_bytes: integer | null

CaptureAdapterArtifactSource:
  adapter_id: string
  capture_ref: string
  display_name: string | null
  content_type: string
  expected_sha256: string | null
  expected_size_bytes: integer | null
```

`source_kind=staged_file`은 `staged`를 요구하고 `existing_artifact_ref=null`, `capture=null`이어야 합니다. `source_kind=capture_adapter`는 `capture`를 요구하고 `staged=null`, `existing_artifact_ref=null`이어야 합니다. `source_kind=existing_artifact`는 이미 commit된 `ArtifactRef`를 요구하고 `staged=null`, `capture=null`이어야 합니다.

허용되는 artifact source는 Harness staging location, approved capture adapter output, 이미 commit된 artifact ref뿐입니다. `staged_uri`는 Harness staging locator이지 임의 파일을 읽을 승인이 아닙니다. `capture_ref`는 capture-adapter handle이지 caller가 넘긴 path가 아닙니다. Tool response는 committed `ArtifactRef` value를 반환하며, staged locator나 capture handle을 authority로 반환하지 않습니다.

Critical 또는 close-relevant evidence는 supporting Core state와 각 required `ArtifactRef`가 current owner relation, availability, `sha256`, `size_bytes`, `content_type`, `redaction_state`, `produced_by`, `retention_class` metadata를 가질 때만 sufficient로 취급할 수 있습니다. Artifact가 missing이거나 relation owner가 resolve되지 않았거나 integrity metadata가 없거나 `hash_mismatch` 같은 integrity failure가 있으면 affected evidence는 `stale` 또는 `blocked`가 됩니다. Required evidence가 affected이면 close는 계속 blocked입니다.

## StateRecordRef

```yaml
StateRecordRef:
  record_kind: task | change_unit | run | write_authorization | user_judgment | evidence_summary | blocker
  record_id: string
```

`record_kind=user_judgment`는 sensitive-action approval, final acceptance, residual-risk acceptance judgment를 포함한 사용자 소유 판단의 canonical MVP-1 ref kind입니다. MVP-1 evidence coverage와 blocker는 `record_kind=evidence_summary`, `record_kind=blocker`를 사용합니다. Durable 증거 바이트는 `ArtifactRef`를 사용합니다. Standalone accepted-risk ref kind는 없습니다.

`approval`, `residual_risk`, `close_readiness`, `shared_design`, `domain_term`, `module_map_item`, `interface_contract`, `feedback_loop`, `evidence_manifest`, `eval`, `manual_qa_record`, `tdd_trace`, `change_unit_dependency`, `reconcile_item`, `projection` 같은 later/profile-only ref kind는 [Schema Later](schema-later.md#later-profile-ref-and-artifact-values)에 정의합니다. 이 활성 schema는 해당 값을 accept하지 않습니다. `projection_path` 같은 projection-specific metadata도 later/profile material입니다.

## Evidence and pre-write scope schemas

```yaml
EvidenceRefs:
  state_refs: StateRecordRef[]
  artifact_refs: ArtifactRef[]

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

ApprovalScope:
  sensitive_categories: string[]
  allowed_paths: string[]
  allowed_tools: string[]
  allowed_commands: string[]
  allowed_command_classes: string[]
  allowed_network_targets: string[]
  secret_scope: string[]
  baseline_ref: string | null

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
  guarantee_level: cooperative | detective | preventive | isolated

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
  guarantee_display:
    level: cooperative | detective | preventive | isolated
    notes: string[]
  note: "Autonomy Boundary is judgment latitude, not a pre-write scope check."
```

`AuthorizedAttemptScope`는 authorized write attempt scope를 위한 활성 MVP의 유일한 shape입니다. Core는 `harness.prepare_write`의 proposed write와 resolved Core context에서 이 shape를 만듭니다. 여기에는 `task_id`, `change_unit_id`, `basis_state_version`, `surface_id`, related user judgment refs, 표시되는 guarantee level이 포함됩니다. `write_authorizations.attempt_scope_json`, `WriteAuthorizationSummary.attempt_scope`, `record_run` 비교는 모두 같은 shape를 사용합니다.

`AuthorizedAttemptScope.related_user_judgment_refs`는 minimum MVP-1에서 민감 동작 승인이 필요할 때 compatible resolved `judgment_kind=sensitive_approval` user judgment를 포함합니다. Committed Approval ref는 later Approval owner profile이 active일 때만 나타납니다.

`WriteAuthorizationSummary`, `WriteAuthoritySummary`, `AuthorizedAttemptScope`는 API/internal 이름입니다. MVP-1 사용자 표시에서는 먼저 쓰기 전 범위 확인이라고 설명해야 합니다. `intended_paths`, `intended_tools`, `decision=allowed`, `status=active`, `surface_id`, `guarantee_display` 같은 field는 협력형 기록/확인에 대한 하네스 호환성과 display context만 뜻합니다. OS 권한, sandboxing, 변조 방지 enforcement, 사전 차단, 권한 격리, surface가 부여한 write authority를 뜻하지 않습니다. `allowed`는 `PrepareWriteResponse.decision`에 속합니다. `blocked`에는 authorization row나 lifecycle value가 없습니다.

`EvidenceSummary`는 활성 MVP-1의 compact evidence contract입니다. `status`는 정확히 `not_required`, `none`, `partial`, `sufficient`, `stale`, `blocked`를 사용합니다. Item coverage는 정확히 `supported`, `unsupported`, `partial`, `not_applicable`, `stale`, `blocked`를 사용합니다. 이 값은 status와 close check에 쓰는 Core 소유 상태입니다. Full Evidence Manifest, detached verification result, Manual QA record, 최종 수락, 잔여 위험 수락, projection이 아닙니다.

## UserJudgment

MVP-1 judgment model은 작지만 명시적입니다. 사용자는 초점이 분명한 질문 하나를 보고, API payload는 compact `judgment_kind`와 `presentation`을 가집니다.

```yaml
UserJudgment:
  user_judgment_id: string
  task_id: string
  change_unit_id: string | null
  status: proposed | pending_user | resolved | deferred | rejected | blocked | superseded
  judgment_kind: product_decision | technical_decision | scope_decision | sensitive_approval | qa_waiver | verification_risk_acceptance | final_acceptance | residual_risk_acceptance | cancellation
  presentation: short | full
  context:
    why_now: string
    source_refs: StateRecordRef[]
    evidence_refs: EvidenceRefs
  state_summary_at_request: StateSummary
  question: string
  what_user_is_judging: string
  why_agent_cannot_decide: string
  no_decision_consequence: string
  what_agent_may_decide_without_user: string[]
  affected_scope: UserJudgmentScope
  affected_gates: UserJudgmentGateRef[]
  affected_acceptance_criteria: UserJudgmentCriterionRef[]
  judgment_payload: UserJudgmentPayload
  resolution: UserJudgmentResolution | null
  expires_at: string | null
  created_at: string
  updated_at: string
  resolved_at: string | null

UserJudgmentScope:
  task_ref: StateRecordRef
  change_unit_ref: StateRecordRef | null
  affected_object_refs: StateRecordRef[]
  write_refs: StateRecordRef[]
  close_refs: StateRecordRef[]
  scope_refs: StateRecordRef[]
  product_areas: string[]
  files_or_paths: string[]
  acceptance_criteria_refs: StateRecordRef[]
  note: string | null

UserJudgmentGateRef:
  gate: scope_gate | decision_gate | approval_gate | design_gate | evidence_gate | verification_gate | qa_gate | acceptance_gate
  blocked_action: string | null

UserJudgmentCriterionRef:
  criteria_id: string
  statement: string

UserJudgmentResolution:
  selected_option_id: string | null
  judgment: RecordUserJudgmentPayload | null
  note: string | null
```

`presentation=short`는 작은 one-screen prompt의 default입니다. `presentation=full`은 complex 또는 high-risk judgment를 위한 full-format Decision Packet-style presentation입니다. Presentation은 렌더링 context 양을 바꿀 뿐 authority를 바꾸지 않습니다.

`judgment_kind`가 판단 유형을 뜻하는 기준 field입니다. 제품 판단, 기술 판단, 범위 판단, 민감 동작 승인, QA 면제 판단, 검증 위험 수락, 최종 수락, 잔여 위험 수락, 취소 판단 같은 사용자 표시 라벨은 `judgment_kind`와 locale에서 렌더러가 파생합니다. Active MVP-1 request, record, validator, storage, state-compatibility, gate logic은 `display_label`을 권한 있는 입력으로 받거나 비교하면 안 됩니다.

Legacy field와 method는 canonical name으로 매핑됩니다.

| Legacy | Canonical |
|---|---|
| `harness.request_user_decision` / `harness.record_user_decision` | `harness.request_user_judgment` / `harness.record_user_judgment` |
| `judgment_type` | `judgment_kind` |
| `judgment_domain` | `judgment_kind` plus locale-derived rendered label |
| `decision_kind` | `judgment_kind` plus route-specific validation |
| `decision_profile` | `presentation` |
| `product_choice` / `technical_choice` / `sensitive_action_approval` / `work_acceptance` | `product_decision` / `technical_decision` / `sensitive_approval` / `final_acceptance` |

### UserJudgment payload

```yaml
JudgmentOption:
  option_id: string
  label: string
  details: JudgmentOptionDetails | null

JudgmentOptionDetails:
  benefits: string[]
  costs: string[]
  risks: string[]
  reversibility: reversible | partially_reversible | irreversible | unknown
  confidence: low | medium | high
  suitable_when: string[]
  evidence_refs: EvidenceRefs

JudgmentRecommendation:
  option_id: string | null
  reason: string
  uncertainty: string | null
  when_to_revisit: string | null

JudgmentUserContext:
  minimum_context: string[]
  optional_pull_refs: StateRecordRef[]

UserJudgmentPayload:
  options: JudgmentOption[]
  recommendation: JudgmentRecommendation | null
  rationale: string
  uncertainty: string | null
  deferral_consequence: string | null
  user_context: JudgmentUserContext | null
  approval_scope: ApprovalScope | null
  covers: string[]
  does_not_cover: string[]
  acceptance: AcceptanceJudgment | null
  qa_waiver: QAWaiverJudgment | null
  verification_risk_acceptance: VerificationRiskAcceptanceJudgment | null
  residual_risk_acceptance: ResidualRiskAcceptanceJudgment | null
  cancellation: CancellationJudgment | null
  separate_judgments_required: string[]

AcceptanceJudgment:
  result_ref: StateRecordRef | null
  result_summary: string
  evidence_status_refs: StateRecordRef[]
  verification_status_refs: StateRecordRef[]
  qa_status_refs: StateRecordRef[]
  residual_risk_visibility: ResidualRiskSummary
  does_not_replace: string[]

ResidualRiskAcceptanceJudgment:
  risk_refs: StateRecordRef[]
  accepted_scope: string[]
  acceptance_consequence: string
  follow_up_required: boolean
  follow_up: string | null
  evidence_refs: EvidenceRefs

QAWaiverJudgment:
  qa_requirement_ref: StateRecordRef | null
  waiver_allowed_by_ref: StateRecordRef | null
  skipped_qa: string
  risk_summary: string
  does_not_create_evidence: boolean

VerificationRiskAcceptanceJudgment:
  verification_requirement_ref: StateRecordRef | null
  missing_or_waived_verification: string
  risk_refs: StateRecordRef[]
  acceptance_consequence: string
  does_not_create_detached_verification: boolean

CancellationJudgment:
  cancellation_scope: string
  close_effect: string
  follow_up: string | null
```

`judgment_kind=sensitive_approval`에서는 `approval_scope`가 required입니다. `judgment_kind=qa_waiver`에서는 `qa_waiver`가 required이고 policy가 waiver를 허용해야 합니다. `judgment_kind=verification_risk_acceptance`에서는 `verification_risk_acceptance`가 required이며 `assurance_level=detached_verified`를 만들면 안 됩니다. `judgment_kind=final_acceptance`에서는 `acceptance`가 required입니다. `judgment_kind=residual_risk_acceptance`에서는 `residual_risk_acceptance`가 required입니다. `judgment_kind=cancellation`에서는 `cancellation`이 required입니다. Later reconcile branch는 [Schema Later](schema-later.md#later-user-judgment-branches)에 있습니다.

<a id="userjudgmentcandidate"></a>

### UserJudgmentCandidate

`UserJudgmentCandidate`는 진행, 쓰기 호환성, close를 계속하기 전에 사용자 소유 판단이 필요할 때 읽기 또는 검증 경로가 반환하는 상태 변경 없는 후보입니다. Committed `user_judgment` record가 아니며 `StateRecordRef`도 없습니다. 이것만으로 `decision_gate`나 `approval_gate`가 충족되지 않고, 민감 동작 승인, 최종 수락, 잔여 위험 수락, evidence, Write Authorization, projection, close state를 만들지 않습니다.

```yaml
UserJudgmentCandidate:
  candidate_id: string
  task_id: string
  change_unit_id: string | null
  judgment_kind: product_decision | technical_decision | scope_decision | sensitive_approval | qa_waiver | verification_risk_acceptance | final_acceptance | residual_risk_acceptance | cancellation
  presentation: short | full
  context:
    why_now: string
    source_refs: StateRecordRef[]
    evidence_refs: EvidenceRefs
  state_summary_at_request: StateSummary | null
  question: string
  what_user_is_judging: string
  why_agent_cannot_decide: string
  no_decision_consequence: string
  what_agent_may_decide_without_user: string[]
  affected_scope: UserJudgmentScope
  affected_gates: UserJudgmentGateRef[]
  affected_acceptance_criteria: UserJudgmentCriterionRef[]
  judgment_payload: UserJudgmentPayload
  expires_at: string | null
```

이 후보의 body는 이후 `harness.request_user_judgment`를 호출할 때 쓰는 schema-normalized 초안입니다. 호출자가 새 `ToolEnvelope`를 제공하고 Core가 현재 상태를 다시 검증한 뒤에만 기록될 수 있습니다. `judgment_kind=sensitive_approval`이면 candidate는 `judgment_payload.approval_scope`를 사용합니다. 활성 MVP-1에는 `ApprovalRequestCandidate` field나 committed Approval request lifecycle이 없습니다.

<a id="acceptedriskinput"></a>

### AcceptedRiskInput

`AcceptedRiskInput`은 `harness.record_user_judgment`에서 `judgment_kind=residual_risk_acceptance`일 때만 받습니다. 대기 중인 `UserJudgment` context에서 이미 보였던 이름 있는 close-relevant risk를 사용자가 명시적으로 수락했음을 기록합니다. Minimum MVP-1에서는 standalone accepted-risk record kind가 아니며 rich Residual Risk lifecycle metadata를 만들지 않습니다.

```yaml
AcceptedRiskInput:
  visible_risk_ref: StateRecordRef
  risk_summary: string
  accepted_scope: string[]
  acceptance_consequence: string
  evidence_refs: EvidenceRefs
  follow_up_required: boolean
  follow_up: string | null
```

활성 MVP-1에서 `visible_risk_ref.record_kind`는 `blocker`여야 합니다. 이 blocker는 close와 관련되어야 하고, pending residual-risk acceptance `UserJudgment`에서 보였어야 하며, 같은 Task 범위여야 합니다. 다른 모든 `judgment_kind`에서 `accepted_risks`는 `[]`여야 합니다. Rich residual-risk owner ref, lifecycle status, review metadata, accepted-risk metadata는 [Schema Later](schema-later.md#later-user-judgment-branches)의 later/profile material입니다.

<a id="record-run-payloads"></a>

## Record-run payloads

아래 schema는 `harness.record_run`의 활성 payload branch입니다. Top-level `RecordRunRequest.kind`, `RecordRunPayload.kind`, non-null payload branch는 서로 일대일로 맞아야 합니다. `shaping_update`, `implementation`, `direct` 중 정확히 하나만 non-null이고, 나머지 branch field는 `null`이어야 합니다. 활성 MVP-1에서는 다른 branch kind가 valid하지 않습니다.

```yaml
RecordRunPayload:
  kind: shaping_update | implementation | direct
  shaping_update: ShapingUpdatePayload | null
  implementation: ImplementationPayload | null
  direct: DirectPayload | null

ShapingUpdatePayload:
  shaping_kind: requirements | scope | acceptance_criteria | constraint | judgment_routing
  task_update: TaskShapingUpdate | null
  change_unit_update: ChangeUnitShapingUpdate | null
  user_judgment_candidates: UserJudgmentCandidate[]
  discovered_facts: string[]
  assumptions: string[]
  open_questions: string[]
  source_refs: StateRecordRef[]
  evidence_refs: EvidenceRefs

TaskShapingUpdate:
  title: string | null
  goal: string | null
  mode: advisor | direct | work | null
  acceptance_criteria: string[]
  constraints:
    allowed_paths: string[]
    non_goals: string[]
    sensitive_categories: string[]

ChangeUnitShapingUpdate:
  change_unit_id: string | null
  operation: propose | activate | update | supersede
  scope_summary: string
  allowed_paths: string[]
  denied_paths: string[]
  non_goals: string[]
  success_criteria: string[]
  sensitive_categories: string[]
  baseline_ref: string | null
  autonomy_boundary: AutonomyBoundaryUpdate | null

AutonomyBoundaryUpdate:
  autonomy_profile: human_in_loop | afk_eligible | evaluator_only | read_only_advisor | null
  what_agent_may_do: string[]
  what_agent_may_decide_without_user: string[]
  what_requires_user_judgment: string[]
  stop_conditions: string[]

ImplementationPayload:
  outcome: completed | partial | blocked | failed
  product_write: boolean
  observed_changes: ObservedChanges
  command_results: CommandResult[]
  tool_invocations: ToolInvocationSummary[]
  network_accesses: NetworkAccessObservation[]
  secret_accesses: SecretAccessObservation[]
  evidence_updates: EvidenceUpdates
  implementation_notes: string[]
  follow_up_needed: string[]

DirectPayload:
  result_kind: answer | product_write | no_change | blocked
  product_write: boolean
  direct_summary: string
  observed_changes: ObservedChanges
  command_results: CommandResult[]
  tool_invocations: ToolInvocationSummary[]
  network_accesses: NetworkAccessObservation[]
  secret_accesses: SecretAccessObservation[]
  evidence_updates: EvidenceUpdates
  user_visible_result: string
  follow_up_needed: string[]

ObservedChanges:
  changed_paths: ChangedPath[]
  diff_artifact_input_ids: string[]
  no_product_changes: boolean

ChangedPath:
  path: string
  change_kind: added | modified | deleted | moved | copied | permission_changed | unknown
  product_file: boolean
  within_change_unit: boolean
  before_sha256: string | null
  after_sha256: string | null

CommandResult:
  command: string
  command_class: string
  exit_code: integer | null
  status: succeeded | failed | blocked | skipped | unknown
  writes_product_files: boolean
  started_at: string | null
  completed_at: string | null
  artifact_input_ids: string[]
  summary: string | null

ToolInvocationSummary:
  tool_name: string
  purpose: string
  status: succeeded | failed | blocked | skipped | unknown
  artifact_input_ids: string[]
  summary: string | null

NetworkAccessObservation:
  target: string
  direction: read | write
  observed: boolean
  note: string | null

SecretAccessObservation:
  secret_handle: string
  access_kind: read | write
  observed: boolean
  approved_by_ref: StateRecordRef | null
  note: string | null

EvidenceUpdates:
  coverage_updates: EvidenceCoverageUpdate[]
  gap_blocker_refs: StateRecordRef[]
  summary: string

EvidenceCoverageUpdate:
  claim_or_criterion: string
  coverage_state: supported | unsupported | partial | not_applicable | stale | blocked
  supporting_state_refs: StateRecordRef[]
  supporting_artifact_input_ids: string[]
  note: string | null
```

`ShapingUpdatePayload`는 Discovery와 요구사항 구체화 중 활성 Task, Change Unit, User Judgment 경계에 지속되는 업데이트를 위한 활성 MVP payload입니다. Shared Design, Feedback Loop, TDD Trace, Evidence Manifest, Projection, Approval, Residual Risk 또는 다른 later/profile record를 만들지 않습니다. `task_update`, `change_unit_update`, `user_judgment_candidates` 중 적어도 하나는 비어 있지 않은 업데이트를 담아야 합니다.

`ImplementationPayload`는 Task 또는 Change Unit에 대한 구현 작업을 기록합니다. `product_write=true`이면 `RecordRunRequest.write_authorization_id`가 compatible active Write Authorization을 가리켜야 하고, `observed_changes.changed_paths`는 관찰된 제품 파일 변경을 설명해야 합니다. `observed_changes`, `command_results`, `tool_invocations`, `network_accesses`, `secret_accesses`, `artifact_inputs`, `evidence_updates`를 포함한 request body는 request 검증과 canonical idempotency hash에 들어갑니다. Storage는 이 payload를 Run row의 observed payload JSON field, linked artifact, evidence-summary update에 매핑합니다.

`DirectPayload`는 작은 direct result를 기록합니다. 제품 파일 변경이 없는 직접 답변이나 결과도 포함합니다. `result_kind=product_write`이거나 `product_write=true`이면 `ImplementationPayload`와 같은 Write Authorization, observed-change, artifact, evidence validation rule을 따릅니다. `product_write=false`이면 `write_authorization_id`는 `null`이어야 하고 `observed_changes.no_product_changes`는 `true`여야 합니다.

Product-write Run에서 Core는 observed payload를 stored `AuthorizedAttemptScope`와 비교합니다. 관련 관찰이 unsupported이거나 absent이면 surface가 commands, command classes, network access, secret access, changed paths를 verified로 표시하면 안 됩니다. 결과는 API와 error owner에 따라 narrowed, blocked, 또는 `CAPABILITY_INSUFFICIENT` / insufficient surface capability로 표시해야 합니다.

## NextActionSummary

```yaml
NextActionSummary:
  action_kind: ask_user | prepare_write | implement | request_acceptance | close_task | idle
  summary: string
  required_tool: string | null
  related_refs: StateRecordRef[]
  blocker_code: ErrorCode | null
```

MVP-1은 별도 `harness.next` method가 아니라 `harness.status.next_actions`를 사용합니다. 위 schema block이 활성 MVP-1 enum 전체입니다. `launch_verify`, `record_eval`, `record_manual_qa`, `reconcile` 같은 later/profile action kind는 [Schema Later](schema-later.md#later-next-action-values)에 정의하며 MVP-1 validator는 reject해야 합니다.

## Current-position display schemas

```yaml
AutonomyBoundarySummary:
  change_unit_id: string | null
  status: absent | proposed | active | exceeded | stale
  autonomy_profile: human_in_loop | afk_eligible | evaluator_only | read_only_advisor | null
  what_agent_may_do: string[]
  what_agent_may_decide_without_user: string[]
  what_requires_user_judgment: string[]
  stop_conditions: string[]
  triggered_stop_conditions: string[]
  related_user_judgment_refs: StateRecordRef[]

ResidualRiskSummary:
  status: none | visible | not_visible | accepted | blocked
  close_relevant_count: integer
  visible_refs: StateRecordRef[]
  not_visible_refs: StateRecordRef[]
  unaccepted_refs: StateRecordRef[]
  accepted_refs: StateRecordRef[]
  summary: string

AcceptanceVisibilityContext:
  residual_risk_summary: ResidualRiskSummary | null
  unaccepted_close_relevant_risk_refs: StateRecordRef[]
  evidence_summary: EvidenceSummary | null
  evidence_refs: StateRecordRef[]
  verification_status: not_required | required | pending | passed | failed | waived_by_user | blocked
  qa_status: not_required | required | pending | passed | failed | waived
  acceptance_status: not_required | required | pending | accepted | rejected
  what_acceptance_does_not_replace: string[]
```

`ResidualRiskSummary.status=none`은 현재 Task/requested action에서 Core가 known close-relevant residual risk를 모른다는 뜻입니다. Known close-relevant risk가 있지만 충분한 context로 보이지 않은 `not_visible`과 다릅니다.

MVP-1에서 residual-risk summary ref는 보통 `blocker`와 `user_judgment` record를 가리킵니다. Rich `residual_risk` record는 later/profile-promoted storage입니다.

Autonomy Boundary summary는 judgment latitude를 설명합니다. 쓰기 전 범위 확인 호환성이 아닙니다. Write Authorization record를 만들거나 path, tool, command, network target, secret access, sensitive category를 compatible하게 만들거나 active scope와 required 민감 동작 승인을 넓히지 않습니다.

## ValidatorResult

`ValidatorResult`는 common response가 validator result를 가질 수 있기 때문에 여기에 둡니다. 활성 MVP-1은 이 schema를 좁게 유지합니다. 활성 `validator_kind`는 `capability`뿐이며, 활성 stable validator ID는 `surface_capability_check`입니다. 이 finding은 blocked reason, fallback behavior, guarantee display에 영향을 주지만 그 자체로 Core write authority를 만들지는 않습니다. 더 넓은 validator kind와 ID는 later/profile material입니다.

```yaml
ValidatorResult:
  validator_id: string
  validator_kind: capability
  status: passed | warning | failed | blocked | skipped
  guarantee_level: cooperative | detective | preventive | isolated
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

활성 MVP-1에서 `validator_id`는 `surface_capability_check`입니다. 추가 validator kind와 stable ID는 [Schema Later](schema-later.md#validatorresult-stable-ids)에 있습니다.
