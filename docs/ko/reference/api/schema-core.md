# API Schema Core

## 이 문서로 할 수 있는 일

[MVP API](mvp-api.md)의 MVP-1 method를 뒷받침하는 shared API shape를 확인할 때 이 참조를 사용합니다. Request envelope, common response, read-only resource schema, shared ref, artifact input, user-judgment payload, next-action summary, API staged value set을 다룹니다.

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
- Later/profile-gated enum value와 branch는 owning profile이 active가 아니면 MVP-1에서 valid하지 않습니다.

Storage validation은 별도 소유권 경계입니다. API payload와 API-shaped stored JSON은 먼저 이 API reference로 validate합니다. Storage-only JSON `TEXT`, DDL nullability, column default, storage hardening은 [Storage](../storage.md)이 담당합니다.

## Stage Profile Manifest

이 manifest는 API schema를 stage/profile별로 걸러 줍니다. Field나 enum이 이 참조에 있다는 사실만으로 더 이른 stage에서 active가 되지 않습니다.

| Stage/profile | Active API slice | 해당 slice에서 active가 아닌 것 |
|---|---|---|
| 내부 엔지니어링 점검 | Minimal status/blocker read, owner-valid setup path 하나, active Task, active Change Unit/scope boundary, `harness.prepare_write`, compatible `harness.record_run` 하나, artifact/evidence ref 하나, structured status/blocker output, 좁은 close-blocker check. | Full natural-language intake, stored user judgment path, full Evidence Manifest, detached verification, Manual QA, work acceptance, residual-risk acceptance, rich projections, export/recover, broad operations. |
| MVP-1 사용자 작업 루프 | Active method set은 [MVP API](mvp-api.md#mvp-1-method-set)가 담당하며, 다음 안전한 행동 output은 `harness.status.next_actions`에 담깁니다. Method set은 정확히 `harness.status`, `harness.intake`, `harness.request_user_judgment`, `harness.record_user_judgment`, `harness.prepare_write`, `harness.record_run`, `harness.close_task`입니다. | 별도 `harness.next`, detached verification launch/Eval, full Manual QA matrix, committed Approval hardening, export/recover, advanced connector APIs, broad operations, detailed diagnostic projections. |
| 보증 프로필, 운영 프로필, later | Owner docs가 승격할 때 verification, Eval, Manual QA, waiver, full residual-risk acceptance, reconcile, validators, projection/report/export/recover, operations, advanced connectors. | 내부 엔지니어링 점검이나 minimum MVP-1 requirement가 아닙니다. |

## Read-only resources

MCP resource는 read-only view입니다. Task, user judgment, projection job, reconciliation, evidence, QA, work acceptance, residual-risk acceptance, Write Authorization, close state를 만들면 안 됩니다.

Read-only resource도 세 부분 맥락 모델을 따릅니다. `harness://status/card`는 사용자 상태 카드입니다. Current Core state와 ref에서 만든 짧은 읽기용 보기입니다. Agent 접점은 read-only resource를 사용해 다음 안전한 행동에 필요한 최소 state, ref, freshness, owner-section pointer를 담은 에이전트 맥락 패킷을 만들 수 있습니다. Core 상태가 로컬 권한 기록이며 유일한 운영 기준입니다. 오래된 card나 projection은 authority가 아니며, 렌더링된 template은 민감 동작 승인, 작업 수락, 잔여 위험 수용, 근거, 닫기 준비 상태를 만들 수 없습니다.

### 내부 엔지니어링 점검 resources

| Resource | Profile meaning |
|---|---|
| `harness://project/current` | Current registered project identity와 local MCP availability facts. |
| `harness://task/active` | Task를 만들지 않고 active Task pointer 또는 explicit `none` / `unknown`을 반환합니다. |
| `harness://task/{task_id}` | 좁은 권한 루프를 위한 current Task state. |
| `harness://task/{task_id}/summary` | Optional compact Task status/blocker summary. |
| `harness://status/card` | Current Core state와 ref에서 파생한 optional compact current-position 사용자 상태 카드. |

### MVP-1 resources

| Resource | Profile meaning |
|---|---|
| `harness://task/{task_id}/user-judgments` | Active, resolved, deferred, blocked `user_judgment` summary. |
| `harness://task/{task_id}/judgment-context` | 사용자 판단에 필요한 minimum current context. |

MVP-1 evidence와 close-readiness path는 output이 current Core state와 refs에서 파생된다면 정확한 compact view set인 `status-card`, `agent-context-packet`, `judgment-request`, `run-evidence-summary`, `close-result` 또는 `harness.status`, `harness://task/{task_id}/summary`, `harness://status/card`로 표시할 수 있습니다. 정확한 compact view 동작과 template body는 [Projection과 Template 참조](../projection-and-templates.md)와 [Template 참조](../templates/README.md)에 남습니다.

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

Envelope field는 routing과 audit claim입니다. Surface가 Core 밖에서 state를 바꾸도록 허가하지 않으며, user judgment, sensitive-action permission, work acceptance, Manual QA, detached verification independence를 증명하지도 않습니다.

## MCP boundary and caller trust

내부 엔지니어링 점검/default posture는 registered project surface에 대한 local-only exposure입니다. Local-only는 예상 local user/profile의 local process, local socket, localhost-loopback connection을 뜻합니다. Unauthenticated shared endpoint, non-loopback bind, forwarded/tunneled endpoint, cloud/CI relay, cross-user socket/directory, remote caller는 registered connector profile이 stronger posture를 증명하기 전까지 제외합니다.

Public schema는 display-safe access material class, bind/reachability posture, freshness, profile refs, conformance/operator-check refs, safe handle/fingerprint를 담을 수 있습니다. Raw token, secret, private configuration value, omitted secret value, blocked payload bytes를 담으면 안 됩니다.

Core에 닿지 못하면 authoritative Core response가 없습니다. `MCP_UNAVAILABLE` 또는 diagnostic `MCP_SERVER_UNAVAILABLE`을 보고합니다. Core나 operator가 reachable local caller/access path를 registered profile 밖으로 분류할 수 있으면 display-safe detail과 함께 `LOCAL_ACCESS_MISMATCH`를 사용합니다. Core unavailable, local access denied, unsupported surface, stale state에 대해 사용자에게 보이는 동작은 [Errors: MVP-1 guarantee와 상태/error taxonomy](errors.md#mvp-1-guarantee-and-status-taxonomy)가 담당합니다.

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

`dry_run=true`는 validate하고 diagnostics 또는 transition plan을 반환하지만 current record 변경, event append, artifact 등록, consumable Write Authorization 생성, projection job enqueue, idempotency replay row create/update를 하지 않습니다.

State-changing operation에서 `state_version`은 Core가 primary Task를 resolve하면 resulting Task State Version이고, 그렇지 않으면 Project State Version입니다. Read-only와 dry-run response는 primary read/affected scope의 current version을 반환합니다.

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

`StateSummary.mode` values는 `advisor`, `direct`, `work`로 유지합니다. 사용자 접점은 이를 advice/read-only work, small direct work, tracked work로 표시할 수 있습니다. 그 label은 display text이지 enum value가 아닙니다.

### ProjectionKind support

`ProjectionKind`는 extensible이지만 profile-gated입니다.

| Support class | Values | Requirement |
|---|---|---|
| Core status output | none required | 내부 엔지니어링 점검은 persisted Markdown projection job 없이 status/blocker output을 노출할 수 있습니다. |
| MVP-1의 작은 보기 | Persisted `ProjectionKind`는 필요하지 않습니다. 정확한 compact view name과 동작은 [Projection과 Template 참조](../projection-and-templates.md#mvp-1-보기-세트)와 [Template 참조](../templates/README.md#mvp-1-템플릿-세트)가 담당합니다. | 이 보기들은 full template rendering 없이 MVP-1을 충족할 수 있습니다. `TASK`와 `DIRECT-RESULT`는 later/full-profile 또는 compatibility projection입니다. |
| Assurance reports | `APR`, `MANUAL-QA` | Matching approval, Manual QA, waiver, verification, assurance profile이 active일 때만 사용합니다. |
| Operations/export reports | `EXPORT` | Export, release-handoff, operations report profile이 active일 때만 사용합니다. |
| Future/diagnostic projections | `RUN-SUMMARY`, `EVIDENCE-MANIFEST`, `EVAL`, `TDD-TRACE`, `DOMAIN-LANGUAGE`, `MODULE-MAP`, `INTERFACE-CONTRACT`, `DEC`, `DESIGN`, `JOURNEY-CARD` | Owner-promoted later profile이 scope에 있을 때만 enable합니다. |

Projection support는 state, evidence, QA, verification, 민감 동작 승인, work acceptance, residual-risk acceptance, close readiness, close authority, Write Authorization을 만들지 않습니다.

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

하나의 intended write가 여러 category를 가질 수 있습니다. Category는 왜 sensitive-action permission이 필요한지 설명할 뿐이며 product, architecture, security, QA, verification, work acceptance, residual-risk acceptance, policy judgment를 해결하지 않습니다.

## ArtifactRef

Artifact ref는 artifact store에 등록된 durable evidence file을 가리킵니다. Artifact registration은 느슨한 파일 덤프가 아닙니다. Core는 `ArtifactRef`를 반환하기 전에 staging/capture source, stored-byte integrity, `redaction_state`, Task-scoped owner relation을 validate합니다.

```yaml
ArtifactRef:
  artifact_id: string
  kind: diff | log | screenshot | checkpoint | bundle | manifest | qa_capture | export_component | design_probe | prototype | architecture_scan | decision_context | other
  uri: string
  sha256: string
  size_bytes: integer
  content_type: string
  redaction_state: none | redacted | secret_omitted | blocked
  task_id: string
  run_id: string | null
  created_at: string
  produced_by: lead_agent | evaluator | operator | harness
  retention_class: task | project | export | temporary
```

Reference implementation에서 `uri`는 `harness-artifact://{project_id}/{artifact_id}`를 사용합니다. Local file path는 API payload의 absolute path를 신뢰하지 않고 storage를 통해 resolve합니다.

`redaction_state` meanings:

| State | Meaning |
|---|---|
| `none` | Stored bytes가 current policy에서 allowed evidence입니다. |
| `redacted` | Sensitive content가 storage 전에 제거되었습니다. |
| `secret_omitted` | Secret value 또는 PII가 의도적으로 omitted되거나 handle로 대체되었습니다. |
| `blocked` | Raw-payload storage가 blocked되었습니다. Metadata notice만 노출할 수 있습니다. |

`redacted`, `secret_omitted`, `blocked`에서 hash와 size는 hidden original이 아니라 committed safe stored bytes를 설명합니다.

## Stage-Specific Active Value Sets

이 table은 staged implementation의 active validator set입니다. Full later value는 [Schema Later](schema-later.md)에 exact하게 남지만, caller와 validator는 active stage/profile이 enable한 value만 accept합니다.

| Field | 내부 엔지니어링 점검 / MVP-1 active values | Later-profile values | Future candidates |
|---|---|---|---|
| `ArtifactRef.kind`, `ArtifactInput.kind` | `diff`, `log`, `screenshot`, `checkpoint`, `other` | `bundle`, `manifest`, `qa_capture`, `export_component` | `design_probe`, `prototype`, `architecture_scan`, `decision_context` |

| Field | 내부 엔지니어링 점검 active owner kinds | MVP-1 active owner kinds | Later-profile owner kinds | Future candidates |
|---|---|---|---|---|
| `ArtifactInput.relation.record_kind` | `task`, `change_unit`, `run`, `evidence_ref`, `blocker` | `task`, `change_unit`, `run`, `user_judgment`, `evidence_ref`, `blocker` | `residual_risk`, `shared_design`, `evidence_manifest`, `eval`, `manual_qa_record`, `feedback_loop`, `tdd_trace`, `projection` | `journey_spine_entry` |
| `StateRecordRef.record_kind` | `task`, `change_unit`, `run`, `write_authorization`, `evidence_ref`, `blocker` | `task`, `change_unit`, `run`, `write_authorization`, `user_judgment`, `evidence_ref`, `blocker` | `approval`, `residual_risk`, `evidence_summary`, `close_readiness`, `shared_design`, `feedback_loop`, `evidence_manifest`, `eval`, `manual_qa_record`, `tdd_trace`, `reconcile_item`, `projection` | `change_unit_dependency`, `journey_spine_entry`, `domain_term`, `module_map_item`, `interface_contract` |

MVP-1 sensitive-action approval은 `record_kind=user_judgment`를 사용합니다. Committed `approval` ref는 Approval owner profile이 active일 때만 later-profile입니다.

## ArtifactInput

```yaml
ArtifactInput:
  input_id: string
  source_kind: staged_file | existing_artifact
  existing_artifact_ref: ArtifactRef | null
  staged: StagedArtifactSource | null
  kind: diff | log | screenshot | checkpoint | bundle | manifest | qa_capture | export_component | design_probe | prototype | architecture_scan | decision_context | other
  redaction_state: none | redacted | secret_omitted | blocked
  produced_by: lead_agent | evaluator | operator | harness
  retention_class: task | project | export | temporary
  relation:
    task_id: string
    run_id: string | null
    record_kind: task | change_unit | run | user_judgment | evidence_ref | blocker | residual_risk | shared_design | evidence_manifest | eval | manual_qa_record | feedback_loop | tdd_trace | projection | journey_spine_entry
    record_id_hint: string | null
  description: string | null

StagedArtifactSource:
  staged_uri: string
  display_name: string | null
  content_type: string
  expected_sha256: string | null
  expected_size_bytes: integer | null
```

`source_kind=existing_artifact`는 `existing_artifact_ref`를 요구하고 `staged=null`이어야 합니다. `source_kind=staged_file`은 `staged`를 요구하고 `existing_artifact_ref=null`이어야 합니다.

`staged_uri`는 Harness staging locator 또는 registered capture-adapter output입니다. 임의 파일을 읽을 permission이 아닙니다. Tool response는 committed `ArtifactRef` value를 반환하며, staged locator를 authority로 반환하지 않습니다.

## StateRecordRef

```yaml
StateRecordRef:
  record_kind: task | change_unit | run | approval | write_authorization | user_judgment | evidence_ref | blocker | residual_risk | evidence_summary | close_readiness | shared_design | domain_term | module_map_item | interface_contract | feedback_loop | evidence_manifest | eval | manual_qa_record | tdd_trace | change_unit_dependency | reconcile_item | projection
  record_id: string
  projection_path: string | null
```

`record_kind=user_judgment`는 sensitive-action approval, work acceptance, residual-risk acceptance judgment를 포함한 사용자 소유 판단의 canonical MVP-1 ref kind입니다. MVP-1 evidence와 blocker는 `record_kind=evidence_ref`, `record_kind=blocker`를 사용합니다. `record_kind=approval`, `record_kind=residual_risk`, `record_kind=evidence_summary`, `record_kind=close_readiness`, `record_kind=projection`은 owner profile이 active가 아닌 한 later/profile-promoted 또는 derived-view ref입니다. Standalone accepted-risk ref kind는 없습니다.

`record_kind=projection`에서 `record_id`는 운영/projection profile이 active일 때 projection job identity입니다. `projection_path`는 optional display/recovery metadata이지 alternate key가 아닙니다.

`projection` 또는 `close_readiness` 같은 derived-view ref는 읽기용 보기 또는 later/profile-promoted display record를 가리킵니다. 그 보기 뒤의 owner record를 대체하지 않습니다. 오래된 derived-view ref는 state-dependent action에 사용하기 전에 refresh 또는 reconcile해야 합니다.

## Evidence and pre-write scope schemas

```yaml
EvidenceRefs:
  state_refs: StateRecordRef[]
  artifact_refs: ArtifactRef[]

ApprovalScope:
  sensitive_categories: string[]
  allowed_paths: string[]
  allowed_tools: string[]
  allowed_commands: string[]
  allowed_network_targets: string[]
  secret_scope: string[]
  baseline_ref: string | null

WriteAuthorizationSummary:
  write_authorization_id: string
  task_id: string
  change_unit_id: string
  basis_state_version: integer
  intended_operation: string
  intended_paths: string[]
  intended_tools: string[]
  sensitive_categories: string[]
  baseline_ref: string | null
  approval_refs: StateRecordRef[]
  user_judgment_refs: StateRecordRef[]
  guarantee_level: cooperative | detective | preventive | isolated
  status: active | consumed | expired | stale | revoked
  consumed_by_run_id: string | null
  created_at: string
  consumed_at: string | null

WriteAuthoritySummary:
  active_change_unit_ref: StateRecordRef | null
  write_authorization_ref: StateRecordRef | null
  allowed_paths: string[]
  allowed_tools: string[]
  allowed_commands: string[]
  allowed_command_classes: string[]
  allowed_network_targets: string[]
  secret_scope: string[]
  sensitive_categories: string[]
  approval_status: not_required | required | pending | granted | denied | expired | unknown
  baseline_ref: string | null
  guarantee_display:
    level: cooperative | detective | preventive | isolated
    notes: string[]
  note: "Autonomy Boundary is judgment latitude, not a pre-write scope check."
```

Minimum MVP-1에서 `WriteAuthorizationSummary.approval_refs`는 empty입니다. Resolved sensitive-action approval user judgment는 `user_judgment_refs`에 나타납니다. Committed Approval ref는 Approval owner profile이 active일 때만 나타납니다.

`WriteAuthorizationSummary`와 `WriteAuthoritySummary`는 API/internal 이름입니다. MVP-1 사용자 표시에서는 먼저 쓰기 전 범위 확인이라고 설명해야 합니다. `allowed_paths`, `allowed_tools`, `decision=allowed`, `status=active` 같은 field는 협력형 기록/확인에 대한 하네스 호환성만 뜻합니다. OS 권한, sandboxing, 변조 방지 enforcement, 사전 차단, 권한 격리를 뜻하지 않습니다. `allowed`는 `PrepareWriteResponse.decision`에 속합니다. `blocked`에는 authorization row나 lifecycle value가 없습니다.

## UserJudgment

MVP-1 judgment model은 작게 유지합니다. 사용자는 다섯 display label 중 하나를 보고, API payload는 compact `judgment_type`과 `presentation`을 가집니다.

```yaml
UserJudgment:
  user_judgment_id: string
  task_id: string
  change_unit_id: string | null
  status: proposed | pending_user | resolved | deferred | rejected | blocked | superseded
  judgment_type: product_choice | technical_choice | sensitive_action_approval | work_acceptance | residual_risk_acceptance
  presentation: short | full
  display_label: Product/UX judgment | Technical judgment | Sensitive action approval | Work acceptance | Residual risk acceptance
  context:
    why_now: string
    source_refs: StateRecordRef[]
    evidence_refs: EvidenceRefs
  state_summary_at_request: StateSummary
  what_user_is_judging: string
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

Legacy field와 method는 canonical name으로 매핑됩니다.

| Legacy | Canonical |
|---|---|
| `harness.request_user_decision` / `harness.record_user_decision` | `harness.request_user_judgment` / `harness.record_user_judgment` |
| `judgment_domain` | `judgment_type` plus display label |
| `decision_kind` | `judgment_type` plus route-specific validation |
| `decision_profile` | `presentation` |

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
  uncertainty: string | null
  deferral_consequence: string | null
  user_context: JudgmentUserContext | null
  approval_scope: ApprovalScope | null
  covers: string[]
  does_not_cover: string[]
  acceptance: AcceptanceJudgment | null
  residual_risk_acceptance: ResidualRiskAcceptanceJudgment | null
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
```

`judgment_type=sensitive_action_approval`에서는 `approval_scope`가 required입니다. `judgment_type=work_acceptance`에서는 `acceptance`가 required입니다. `judgment_type=residual_risk_acceptance`에서는 `residual_risk_acceptance`가 required입니다. Later waiver와 reconcile branch는 [Schema Later](schema-later.md#later-user-judgment-branches)에 있습니다.

## NextActionSummary

```yaml
NextActionSummary:
  action_kind: ask_user | prepare_write | implement | launch_verify | record_eval | record_manual_qa | request_acceptance | close_task | reconcile | idle
  summary: string
  required_tool: string | null
  related_refs: StateRecordRef[]
  blocker_code: ErrorCode | null
```

MVP-1은 별도 `harness.next` method가 아니라 `harness.status.next_actions`를 사용합니다. Active MVP-1 values는 다음과 같습니다.

```text
ask_user | prepare_write | implement | request_acceptance | close_task | idle
```

Later values `launch_verify`, `record_eval`, `record_manual_qa`, `reconcile`은 owner profile이 active일 때만 valid합니다.

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
  evidence_refs: StateRecordRef[]
  verification_status: not_required | required | pending | passed | failed | waived_by_user | blocked
  qa_status: not_required | required | pending | passed | failed | waived
  acceptance_status: not_required | required | pending | accepted | rejected
  what_acceptance_does_not_replace: string[]
```

`ResidualRiskSummary.status=none`은 현재 Task/requested action에서 Core가 known close-relevant residual risk를 모른다는 뜻입니다. Known close-relevant risk가 있지만 충분한 context로 보이지 않은 `not_visible`과 다릅니다.

MVP-1에서 residual-risk summary ref는 보통 `blocker`와 `user_judgment` record를 가리킵니다. Rich `residual_risk` record는 later/profile-promoted storage입니다.

Autonomy Boundary summary는 judgment latitude를 설명합니다. 쓰기 전 범위 확인 호환성이 아닙니다. Write Authorization record를 만들거나 path, tool, command, network target, secret access, sensitive category를 compatible하게 만들거나 active scope와 required sensitive-action permission을 넓히지 않습니다.

## ValidatorResult

`ValidatorResult`는 profile-gated입니다. Common response가 validator result를 가질 수 있기 때문에 여기에 둡니다. 하지만 owner profile이 특정 check를 승격하지 않는 한 MVP-1은 broad validator emission을 요구하지 않습니다.

```yaml
ValidatorResult:
  validator_id: string
  validator_kind: state | scope | decision | approval | evidence | verification | qa | acceptance | design | autonomy_boundary | residual_risk | artifact | projection | connector | capability
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

Stable later-profile validator ID는 [Schema Later](schema-later.md#validatorresult-stable-ids)에 있습니다.
