# MCP API와 스키마

## 이 문서가 도와주는 일

이 참조 문서는 Harness의 public MCP resource와 tool 계약을 구현·테스트·검토할 때 사용합니다. 이 문서는 읽기 전용 resource와 public tool, 공통 envelope, request/response schema를 다룹니다. 또한 shared ref, error taxonomy, idempotency, state conflict 동작, `ValidatorResult`, `ArtifactRef`의 public API shape를 정리합니다.

SQLite DDL과 storage layout, 전체 kernel transition table, projection template text, CLI command semantics, connector cookbook detail은 이 문서의 담당 범위가 아닙니다. Storage-owned JSON과 DDL 규칙은 [Storage와 DDL](storage-and-ddl.md)이 담당합니다.

## 이런 때 읽기

- MCP client 또는 server 접점을 Harness Core에 연결할 때.
- Harness tool의 정확한 public request 또는 response shape가 필요할 때.
- API response에 어떤 error, validator result, artifact 참조, projection 참조가 나타날 수 있는지 확인할 때.
- Public API behavior를 검증하는 conformance fixture를 작성할 때.

## API를 쉬운 말로

MCP resource는 읽기 전용 보기로 동작합니다. 현재 상태, projection 최신성, 사용자에게 보이는 요약을 보고할 수 있지만, 상태를 만들거나 복구하면 안 됩니다.

모든 상태 변경은 public tool과 Core를 거칩니다. Tool response에는 projection path와 artifact 참조가 포함될 수 있습니다. 하지만 이 값들은 기준 상태 기록이나 durable evidence file을 가리키는 참조일 뿐, 기준 상태를 대체하지 않습니다.

이 문서의 public request와 response schema는 API payload의 검증 기준입니다. 여기에는 Core가 나중에 저장하는 API-shaped payload도 포함됩니다. Core는 commit 전에 모든 storage JSON 값을 이 문서의 API-owned shape 또는 [Storage와 DDL](storage-and-ddl.md)의 storage-owned shape에 맞게 검증해야 합니다. 잘못된 JSON이나 schema와 맞지 않는 JSON은 유효하지 않은 상태입니다.

## 담당하는 참조 범위

이 문서는 다음 항목을 담당합니다.

- read-only MCP resources
- public MCP tools
- common tool envelope
- public request/response schemas
- `StateRecordRef`, `ArtifactRef`, projection refs를 포함한 shared refs
- public error taxonomy와 primary error precedence
- idempotency behavior
- API에 드러나는 state conflict 동작
- `ValidatorResult`
- public API shape로서의 artifact input과 artifact ref schema

## 여기서 다루지 않는 것

이 문서는 다음 항목을 담당하지 않습니다.

- SQLite DDL 또는 storage layout. [Storage와 DDL](storage-and-ddl.md)을 봅니다.
- storage-only JSON `TEXT` validation. [Storage와 DDL](storage-and-ddl.md)을 봅니다.
- lock policy. [Storage와 DDL](storage-and-ddl.md)을 봅니다.
- migrations. [Storage와 DDL](storage-and-ddl.md)을 봅니다.
- artifact directory layout. [Storage와 DDL](storage-and-ddl.md)을 봅니다.
- baseline capture storage format. [Storage와 DDL](storage-and-ddl.md)을 봅니다.
- projection job table. [Storage와 DDL](storage-and-ddl.md)을 봅니다.
- 전체 kernel transition table. [커널 참조](kernel.md)를 봅니다.
- projection template 본문. [Template 참조](templates/README.md)를 봅니다. Projection rule은 [문서 Projection 참조](document-projection.md)가 담당합니다.
- operator command syntax. [운영과 Conformance](operations-and-conformance.md)가 담당합니다.
- connector capability profile. [Agent 통합 참조](agent-integration.md)를 봅니다.
- connector cookbook recipe. [Surface Cookbook](surface-cookbook.md)을 봅니다.

## 최소 호출 흐름

1. `harness.status`, `harness.next`, 또는 read-only resource로 status를 읽습니다.
2. Task를 추적해야 하면 `harness.intake`로 intake 또는 resume합니다.
3. Blocked 상태이면 `harness.request_user_decision`으로 decision을 요청합니다.
4. Product write 전에는 `harness.prepare_write`를 호출합니다.
5. Run/change 후에는 `harness.record_run`을 호출합니다.
6. 적용되는 경우 맞는 public tool 또는 Decision Packet path로 evidence/eval/QA/acceptance를 기록합니다.
7. Blocker가 사라지면 `harness.close_task`로 close합니다.

Capability는 first-class kernel gate가 아닙니다. Surface capability는 다음 경로로 나타납니다.

- `surface_capability_check` validator
- `harness.prepare_write.response.blocked_reasons`
- status와 write decisions의 guarantee display

Core precondition과 mechanical check는 validator보다 앞서 또는 validator와 함께 실행될 수 있습니다. `ValidatorResult`로 남아 `validator_runs`에 저장되는 stable ID만 validator ID입니다. `scope_coverage`, `changed_paths`, `changed_paths_intent`, `approval_scope`, `baseline_freshness`, `qa_waiver_reason`, `projection_freshness` 같은 check는 담당 문서 section이 명시적으로 승격하지 않는 한 Core check로 남습니다.

## Read-only resources

Resource 조회는 상태를 변경하지 않고 현재 상태와 projection 중심 요약을 보여줍니다.

```text
harness://project/current
harness://project/surfaces
harness://task/active
harness://task/{task_id}
harness://task/{task_id}/summary
harness://task/{task_id}/spine
harness://task/{task_id}/journey
harness://task/{task_id}/decision-packets
harness://task/{task_id}/change-unit-dag
harness://task/{task_id}/judgment-context
harness://task/{task_id}/reports/latest
harness://task/{task_id}/evidence-manifest
harness://task/{task_id}/bundle/current
harness://design/domain-language
harness://design/module-map
harness://design/interface-contracts
harness://policy/sensitive-categories
harness://status/card
```

이 목록은 read-only resource 접점을 묶어 보여줍니다. 조회는 현재 상태와 projection 최신성을 보고할 수 있지만 상태를 변경하지 않습니다.

Resource 조회는 Task record, decision, projection job, reconcile item을 만들면 안 됩니다. Resource 조회 중 최신이 아닌 projection을 감지하면 freshness만 보고하고 복구하지 않습니다.

Journey resource는 기준 상태를 바탕으로 한 projection 중심 조회입니다.

- `harness://task/{task_id}/journey`는 현재 Journey Card와 Journey Spine-oriented refs를 반환합니다.
- `harness://task/{task_id}/decision-packets`는 해당 Task의 active/resolved/deferred/blocked Decision Packet summary를 반환합니다.
- `harness://task/{task_id}/change-unit-dag`는 Change Unit dependency refs와 ordering summaries를 반환합니다.
- `harness://task/{task_id}/judgment-context`는 사용자 판단에 필요한 최소 current context를 반환하며, optional pull refs를 required context와 분리합니다.

## Tool envelope

모든 public tool request는 envelope를 가집니다. State-changing tool에는 non-null `idempotency_key`와 `expected_state_version`이 필요합니다. Read-only tool도 tracing을 위해 같은 envelope를 받을 수 있으며, `expected_state_version`을 `null`로 둘 수 있습니다.

Core는 operation이 가리키는 primary Task를 기준으로 State version scope를 결정합니다. Resolved primary Task는 `ToolEnvelope.task_id`, tool-specific `task_id`, 또는 active Task resolution으로 정해질 수 있습니다. Task 범위의 상태 변경은 `expected_state_version`을 해당 Task의 `tasks.state_version`과 비교합니다. Core가 primary Task를 찾지 못하고 operation이 project-scoped이면 `expected_state_version`을 `project_state.state_version`과 비교합니다.

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

### MCP boundary와 caller trust

Public MCP contract는 registered project surface를 위한 local process 또는 localhost connection을 전제로 합니다. MCP server를 이 local boundary 밖에 노출하면 threat model이 달라지므로, documented connector capability profile, access-control contract, guarantee display가 필요합니다. 그런 stronger profile이 없다면 MCP endpoint에 닿을 수 있는 caller도 Core가 검증해야 하는 claim의 출처일 뿐, 자동으로 신뢰되는 권한이 아닙니다.

Access-control contract는 localhost-only binding, local file permission으로 제한된 Unix-domain socket, per-project token, process-scoped configuration material, 또는 equivalent local control 같은 여러 방식으로 구현될 수 있습니다. 이 예시는 schema enum이 아니며 하나의 CLI syntax를 요구하지 않습니다. Public API contract에서 중요한 점은 caller의 access mode가 registered surface profile과 맞아야 하고, mutation 전에 Core가 모든 envelope claim을 계속 검증한다는 것입니다.

Unauthorized 또는 off-profile caller는 endpoint에 닿을 수 있다는 이유만으로 권한으로 승격되면 안 됩니다. API는 local-access profile mismatch를 위해 MVP `UNAUTHORIZED` error code를 추가하지 않습니다. Call이 Core에 닿을 수 없으면 authoritative Core response는 존재하지 않습니다. Core 또는 operator가 문제를 classify할 수 있으면 response는 existing `MCP_UNAVAILABLE` 또는 `CAPABILITY_INSUFFICIENT` path를 사용하며, access problem을 더 구체적으로 classify할 수 없을 때는 `details.mcp_unavailable_kind=unknown`을 사용합니다. Project, Task, surface, Run, actor claim mismatch는 addressed tool에 대한 normal record-compatibility, state-conflict, scope, capability, validator checks로 해석합니다.

Envelope field는 routing과 audit claim입니다.

- `project_id`, `task_id`, `surface_id`, `run_id`는 addressed operation과 호환되는 record로 해석되어야 합니다. Caller가 다른 project, Task, surface, Run을 이름으로 지정한다고 권한이 생기지 않습니다.
- `actor_kind`는 routing과 policy check를 위한 claimed actor role입니다. 그 자체만으로 approval, user acceptance, Decision Packet resolution, Manual QA judgment, detached verification independence를 충족하면 안 됩니다.
- `idempotency_key`는 committed mutation의 중복을 막습니다. Authorization token이 아니며, 같은 `(project_id, tool_name, idempotency_key)` scope에서 같은 canonical request payload일 때만 replay가 valid합니다.
- `expected_state_version`은 caller의 concurrency claim입니다. 최신이 아니거나 잘못된 version은 `STATE_CONFLICT`를 반환합니다. 이를 이용해 오래된 Task 또는 project view가 이기게 만들면 안 됩니다.
- `dry_run=true`는 diagnostic만 반환합니다. Idempotency key를 예약하거나, Write Authorization을 만들거나, artifact를 attach하거나, later write가 안전하다는 proof를 만들지 않습니다.

## Common response

Common response fields:

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
```

`dry_run=true`는 검증과 transition plan 반환까지만 수행합니다. 현재 기록 갱신, `state.sqlite.task_events` 추가, artifact 등록, consumable Write Authorization record 생성, projection job 대기열 추가, `tool_invocations` idempotency replay용 row 생성 또는 update는 하지 않습니다. `dry_run` output은 권한을 만들지 않는 진단 정보이며 그 `idempotency_key`는 replay를 위해 소비되지 않습니다.

`ToolResponseBase.state_version`은 primary affected scope의 resulting version을 반환합니다. State-changing operation에서 Core가 primary Task를 찾으면 Task State Version이고, 그렇지 않으면 Project State Version입니다. Read-only response는 primary read scope의 현재 `state_version`을 반환하며 증가시키지 않습니다. `dry_run=true`가 상태 변경 없이 검증하거나 계획할 때 `state_version`은 현재 primary affected 또는 read scope version을 보고합니다. Virtual resulting version, idempotency-key consumption, replay row, 추가된 event, would-be clock increment를 뜻하지 않습니다.

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
```

`EventRef.state_version`은 event의 affected scope에 대한 resulting version입니다. Task events는 `tasks.state_version`을 사용하고, `task_id=null`인 project-level events는 `project_state.state_version`을 사용합니다.

`EventRef.event_seq`는 `task_events.event_seq`를 mirror합니다. Responses는 events를 ascending `event_seq`로 나열합니다. Timestamps와 `event_id` lexical order는 deterministic event ordering에 사용하지 않습니다.

Fixture assertions를 위한 event stability는 [Kernel Stable Event Catalog](kernel.md#stable-event-catalog)가 담당합니다. 아래 tool sections는 response가 반환하거나 implementation이 저장할 수 있는 `EventRef.event_type` 값을 설명하지만, 두 번째 event taxonomy를 정의하지 않습니다. Stable로 label된 names는 catalog names입니다. Stable catalog에 없는 이름은 implementation-local detail 또는 audit events로 나타날 수 있지만 fixture-stable이 아니며 MVP `expected_events` fixtures가 요구하면 안 됩니다. ValidatorResult IDs, Core check names, projection status shorthands, fixture seed shorthand는 kernel catalog가 명시적으로 나열하지 않는 한 event names가 아닙니다.

`ProjectionKind`는 API가 MVP tier를 담당하는 extensible enum입니다.

| Tier | Values | Requirement |
|---|---|---|
| MVP-required | `TASK`, `APR`, `RUN-SUMMARY`, `EVIDENCE-MANIFEST`, `EVAL`, `DIRECT-RESULT` | Reference implementation은 이 kind들을 지원하고 source 기록이 변경될 때 대기열에 넣고 렌더링해야 합니다. |
| MVP-optional | `MANUAL-QA`, `TDD-TRACE`, `DOMAIN-LANGUAGE`, `MODULE-MAP`, `INTERFACE-CONTRACT` | Policy가 적용되거나, source 기록이 있거나, user/operator가 projection을 켤 때 지원하거나 대기열에 넣습니다. |
| Extension / optional | `DEC`, `DESIGN`, `EXPORT`, `JOURNEY-CARD` | 대응하는 선택 projection이 켜진 경우에만 지원할 수 있습니다. |

ProjectionKind extensibility가 projection을 기준 상태로 만들지는 않습니다. 모든 projection job은 여전히 owner 기록 및 artifact 참조에서 파생된 보기를 렌더링합니다. `DEC`는 해당 기능이 켜졌을 때 standalone Decision Packet Markdown에만 유효하며, MVP-required projection job이 아닙니다. Standalone `DEC` job이 없어도 MVP Decision Packet visibility가 줄어들면 안 되며, 이 visibility는 `TASK` projections, status/next responses, judgment-context resources, decision-packet resources를 통해 제공되어야 합니다. Persisted `JOURNEY-CARD` Markdown은 선택 사항입니다. `harness.status`, `harness.next`, significant resume flow의 현재 위치 Journey Card output은 agency conformance에 계속 필요합니다.

`EXPORT`는 export 기능이 켜졌을 때 Release Handoff 같은 보고서 profile을 포함할 수 있습니다. 이런 profile은 projection/보고서 접점일 뿐입니다. Deployment 권한, merge 권한, production-monitoring 권한, final acceptance, Residual Risk 수용, assurance 향상, Task close 권한을 만들지 않습니다.

```yaml
ToolError:
  code: ErrorCode
  message: string
  retryable: boolean
  details: object

ToolErrorMcpUnavailableDetails:
  mcp_unavailable_kind: server_unavailable | surface_mcp_unavailable | stale_connection | unknown

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

Sensitive categories:

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

## ArtifactRef

Artifact ref는 artifact store에 등록된 durable evidence file을 가리킵니다. Report projection과 record projection은 evidence-file reference가 필요할 때 artifact ref를 사용합니다. Projection 자체는 evidence file이 아닙니다.

Artifact 등록은 임의 파일을 쌓아 두는 느슨한 파일 덤프가 아닙니다. Staged file은 Core가 staging 또는 capture source, stored-byte integrity, `redaction_state`, Task-scoped owner relation을 검증한 뒤에만 public `ArtifactRef`가 됩니다.

Reference implementation에서 artifact 등록은 Task-scoped입니다. `ArtifactRef.task_id`와 `ArtifactInput.relation.task_id`는 required이며 `artifacts.task_id`와 `artifact_links.task_id`에 대응합니다. `retention_class=project`는 retention policy에 영향을 줄 뿐 artifact ownership scope를 바꾸지 않습니다.

Later Browser QA Capture는 새 MVP schema가 아니라 이 artifact 경계를 사용합니다. Screen capture는 보통 `screenshot`을 사용하고, grouped QA outputs는 `qa_capture`를 사용할 수 있습니다. Console log와 network trace는 `log` 또는 `qa_capture`를 사용할 수 있고, accessibility snapshot과 workflow recording은 clear description과 함께 `qa_capture` 또는 `other`를 사용할 수 있습니다. 이러한 artifacts는 모두 redaction, secret/PII handling, Task-scoped ownership, Manual QA record 또는 Feedback Loop attachment rules를 따라야 합니다.

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

Reference implementation에서 `uri`는 `harness-artifact://{project_id}/{artifact_id}`를 사용합니다. Local file path는 API payload의 absolute path를 신뢰하지 않고 `state.sqlite` 안의 per-project `artifacts` registry row를 통해 찾습니다.

`redaction_state`는 public artifact contract의 일부입니다.

| State | User/operator meaning |
|---|---|
| `none` | 현재 policy에서 등록된 bytes를 evidence로 사용할 수 있어 redaction, omission, blocking을 적용하지 않았다는 뜻입니다. |
| `redacted` | 저장 전에 민감한 내용이 제거되었습니다. Harness를 통해 unredacted original에 접근할 수 없습니다. |
| `secret_omitted` | Secret value 또는 PII를 의도적으로 생략하거나 handle로 대체했습니다. Secret이 아닌 evidence가 남아 있는 주장에는 도움이 될 수 있지만, 생략된 값 자체를 증명하는 evidence는 아닙니다. |
| `blocked` | 금지된 내용 때문에 capture 또는 원본 payload 저장이 차단되었습니다. Core가 blocked artifact ref를 기록했다면 metadata notice만 노출될 수 있으며 evidence, QA, verification, projection, export display는 원본 artifact를 사용할 수 있는 것처럼 보이지 않게 차단 상태를 표시해야 합니다. |

`redacted`, `secret_omitted`, `blocked`에서 `sha256`, `size_bytes`, `content_type`은 숨겨진 원본이 아니라 커밋된 안전 저장 bytes를 설명합니다. `blocked`의 경우 이 bytes는 Core가 audit과 이후 표시를 위해 commit한 metadata-only notice이며, 금지된 원본 payload가 아닙니다. 이 notice artifact 자체는 차단된 capture의 사용 가능한 원본 근거가 아닙니다.

Evidence를 만들거나 연결하는 request는 `ArtifactInput`을 사용합니다. Request는 기존 committed artifact를 reference하거나, Core가 검증하고 등록한 뒤 `ArtifactRef`로 반환할 staged file을 제공할 수 있습니다.

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
    record_kind: task | change_unit | run | decision_packet | shared_design | residual_risk | evidence_manifest | eval | manual_qa_record | feedback_loop | tdd_trace | journey_spine_entry | projection
    record_id_hint: string | null
  description: string | null

StagedArtifactSource:
    staged_uri: string
    display_name: string | null
    content_type: string
    expected_sha256: string | null
    expected_size_bytes: integer | null
```

Rules:

- `source_kind=existing_artifact` requires `existing_artifact_ref` and must set `staged` to `null`.
- `source_kind=staged_file` requires `staged` and must set `existing_artifact_ref` to `null`.
- Existing artifact를 새 record에 attach할 때 Core는 artifact의 task relation을 verify하고 incompatible reuse를 reject합니다.
- `staged_uri`는 Harness staging location 또는 approved capture adapter output을 가리키는 locator이지, 임의 파일을 읽어도 된다는 권한이 아닙니다. Absolute path, parent traversal, symlink escape, repo-local path, caller-supplied URI는 staging 또는 capture adapter가 approved source로 canonicalize하기 전까지 신뢰하지 않습니다.
- `staged_uri`, `display_name`, supplied `content_type`은 Core가 staging 또는 capture source, stored bytes, redaction state, owner relation을 검증하기 전까지 trusted input이 아닙니다.
- `expected_sha256` 또는 `expected_size_bytes`가 있으면 Core는 commit 전에 stored bytes를 확인합니다. 이 field가 제공되었는지와 무관하게 Core는 redaction, omission, blocking이 적용된 뒤의 safe stored bytes에서 committed `sha256`, `size_bytes`, `content_type`을 기록합니다.
- Core는 final storage 전에 redaction, omission, blocking policy를 적용하고 committed artifact를 `ArtifactRef`로 기록합니다.
- Secret 또는 PII를 포함할 수 있는 log, screenshot, network trace, export snapshot, 기타 captured evidence는 policy가 요구할 때 registration 전에 redacted, omitted, 또는 blocked 상태가 되어야 합니다.
- Policy가 omission 또는 blocking을 요구하면 committed ref는 `redaction_state=secret_omitted` 또는 `redaction_state=blocked`를 기록합니다. Caller는 생략되었거나 차단된 bytes를 available evidence, QA material, verification input, projection body text, export payload로 취급하면 안 됩니다.
- Core가 기록한 `blocked` metadata-only notice는 여전히 committed registered artifact record입니다. Artifact ref, hash, size, content type, owner relation, retention class는 metadata-only notice bytes에 적용되며, 금지된 원본 bytes를 사용할 수 있게 만들지 않고 audit/display continuity를 보존합니다.
- Tool response는 기록된 `ArtifactRef` 값을 `registered_artifacts`, `bundle_ref`, 기타 response field로 반환합니다. Response는 `staged_uri`를 권한이나 durable evidence URI처럼 다시 노출하면 안 됩니다.
- `relation.record_kind`는 Core가 검증할 수 있는 기존 기준 owner 기록 또는 렌더링된 projection 참조를 이름으로 지정해야 합니다. MVP의 non-projection owner에서는 concrete owner row가 `relation.task_id`와 같은 Task scope여야 합니다. 같은 owner kind의 project-scoped row는 future extension이 project-scoped artifact storage/API를 추가하기 전까지 artifact-link target이 아닙니다. Verification bundle은 `ArtifactRef.kind=bundle` 또는 `manifest`를 사용합니다. Export output은 `ArtifactRef.kind=export_component` 또는 `retention_class=export`를 사용합니다. `verification_bundle`과 `export`는 MVP artifact relation record kind가 아닙니다.
- `relation.record_kind=projection`은 Core가 `projection_jobs`를 통해 찾을 수 있는, 이미 렌더링되었거나 기록된 Task-scoped projection output에만 valid합니다. MVP에서 `record_id_hint`는 `projection_jobs.projection_job_id`를 이름으로 지정하고, job의 `task_id`는 `relation.task_id`와 일치해야 합니다. Core는 hint를 검증할 때 `target_ref`와 `output_path`를 사용할 수 있지만, 이 값들이 identity에서 job id를 대체하지 않습니다. Project-level projection job은 owner docs가 허용하는 곳에서 존재할 수 있지만, 현재 MVP artifact API는 이를 위한 project-scoped artifact link를 등록하지 않습니다.

이후 consumer도 같은 의미를 유지해야 합니다. Evidence Manifest, Manual QA, Eval, projection, export, Release Handoff, doctor, artifact integrity display는 ref, hash, 안전한 omission note, handle, blocked notice를 보여줄 수 있지만 생략되었거나 차단된 원본 value를 inline, reconstruct, summarize, export하면 안 됩니다. `secret_omitted`는 secret이 아닌 evidence가 보이는 주장만 충족할 수 있으며, 생략된 값이 필요한 주장은 unsupported 또는 insufficient로 남겨야 합니다. `blocked`는 replacement artifact, compatible waiver, Decision Packet outcome, accepted risk, 또는 다른 documented resolution이 그 경로를 해소하기 전까지 evidence, QA, verification, projection, export, Release Handoff에서 시도된 input을 unavailable로 취급한다는 뜻입니다.

Record 또는 projection references는 `ArtifactRef`가 아니라 `StateRecordRef`를 사용합니다.

```yaml
StateRecordRef:
  record_kind: task | change_unit | change_unit_dependency | run | approval | write_authorization | decision_packet | journey_spine_entry | shared_design | domain_term | module_map_item | interface_contract | feedback_loop | residual_risk | evidence_manifest | eval | manual_qa_record | tdd_trace | reconcile_item | projection
  record_id: string
  projection_path: string | null
```

`record_kind=projection`에서 `record_id`는 MVP projection identity인 `projection_jobs.projection_job_id`입니다. `projection_path`는 선택적 표시 및 복구 metadata입니다. 값이 있으면 job의 `output_path`를 mirror하거나 좁혀야 하며 같은 job 아래에서 찾을 수 있어야 합니다. Alternate key가 아니며 별도의 `projections` table을 뜻하지 않습니다.

MVP에는 `accepted_risk` `StateRecordRef.record_kind`가 없습니다. `accepted_risk_refs`, `accepted_refs`, 또는 accepted-risk equivalent로 이름 붙은 public fields는 `record_kind=residual_risk`인 `StateRecordRef` entries를 사용해야 합니다. Accepted risk는 그 Residual Risk records의 metadata/state입니다.

기준 design-support records에 대한 public refs는 해당 storage record id와 함께 `record_kind=domain_term`, `record_kind=module_map_item`, 또는 `record_kind=interface_contract`를 사용합니다. `DOMAIN-LANGUAGE`, `MODULE-MAP`, `INTERFACE-CONTRACT` 같은 렌더링된 Markdown projection 자체를 가리키고 `record_id=projection_jobs.projection_job_id`를 사용할 때만 `record_kind=projection`을 사용합니다.

기준 feedback-loop records에 대한 public refs는 `feedback_loops.feedback_loop_id`와 함께 `record_kind=feedback_loop`를 사용합니다. Red/green/refactor TDD evidence row에는 `record_kind=tdd_trace`만 사용합니다. Feedback Loop는 execution evidence로 TDD Trace를 cite할 수 있지만, TDD Trace가 selected-loop definition을 대체하지는 않습니다.

Evidence 참조, Approval 범위, Write Authorization, Write Authority Summary 표시, end-to-end path는 다음 shared shape를 사용합니다.

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
  intended_commands:
    - command: string
      command_class: string
      writes_product_files: boolean
  intended_network:
    - target: string
      direction: read | write
  intended_secrets:
    - secret_handle: string
      access_kind: read | write
  sensitive_categories: string[]
  baseline_ref: string | null
  approval_refs: StateRecordRef[]
  decision_packet_refs: StateRecordRef[]
  guarantee_level: cooperative | detective | preventive | isolated
  status: allowed | consumed | expired | stale | revoked
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
  note: "Autonomy Boundary는 판단 재량이지 쓰기 권한이 아니다."

EndToEndPath:
  trigger_or_input: string | null
  domain_logic: string | null
  persistence_or_state: string | null
  api_or_caller_boundary: string | null
  ui_or_observable_output: string | null
```

`WriteAuthorizationSummary`와 `WriteAuthoritySummary`는 API payload shape일 뿐입니다. 이 문서는 Write Authorization 기록에 대한 SQLite DDL을 정의하지 않습니다. `WriteAuthoritySummary`는 client가 Write Authority Summary를 Autonomy Boundary 판단 재량 옆에 표시하기 위해 사용하는 display/read shape입니다.

Client가 guard, freeze, careful-mode control을 렌더링할 때는 권한 field를 추가하지 않고 이 기존 display shape를 사용합니다. `guarantee_display.level`과 `guarantee_display.notes`는 실제 연결된 capability와 현재 적용 경로를 설명해야 합니다. `blocked_reasons[].message`는 scope, MCP availability, Approval, baseline, capability 같은 구체적인 보류 또는 차단 조건을 이름 붙여야 하며, "guard"나 "freeze" 같은 command label만으로 더 강한 guarantee를 암시하면 안 됩니다.

`DEC`, `DESIGN`, `EXPORT`, `JOURNEY-CARD` 같은 Extension / optional tier의 `ProjectionKind` 값은 해당 projection 기능이 켜졌을 때만 projection job kind로 유효합니다. MVP-required Decision Packet visibility는 `TASK` projections, status/next responses, judgment-context resources, decision-packet resources를 통해 제공됩니다. Persisted `JOURNEY-CARD` Markdown은 선택 사항으로 남지만 현재 위치 Journey Card output은 status, next, significant resume flows에서 필요합니다. 전체 projection template text는 [Template 참조](templates/README.md)에 있으며, 이 API schema file이 담당하지 않습니다.

Decision Packet, Write Authorization, Write Authority Summary, Journey Card, Judgment Context, Autonomy Boundary, Recommended Playbook, acceptance visibility, residual-risk summaries는 public MCP schemas입니다. 이 schemas는 API payload만 설명합니다. 기준 kernel records는 owner docs가 정의합니다. 이 목록에서 `RecommendedPlaybook`은 표시 전용 예외입니다. 자체 기준 kernel record, DDL table, task event, projection job이 없습니다.

Role Lens behavior는 이 기존 표시 및 routing schema를 사용합니다. Role lens는 `RecommendedPlaybook`으로 나타날 수 있고, existing Decision Packet으로 route할 수 있으며, `DecisionPacketCandidate`를 propose할 수 있습니다. 별도의 public payload schema, 권한 기록, 상태 전이를 도입하지 않습니다.

```yaml
DecisionPacket:
  decision_packet_id: string
  task_id: string
  change_unit_id: string | null
  status: proposed | pending_user | resolved | deferred | rejected | blocked | superseded
  decision_kind: approval | scope_confirmation | design_choice | architecture_choice | product_tradeoff | autonomy_boundary | verification_waiver | qa_waiver | acceptance | residual_risk_acceptance | reconcile
  context:
    why_now: string
    source_refs: StateRecordRef[]
    evidence_refs: EvidenceRefs
  state_summary_at_request: StateSummary
  what_user_is_deciding: string
  what_agent_may_decide_without_user: string[]
  affected_gates:
    - scope_gate | decision_gate | approval_gate | design_gate | evidence_gate | verification_gate | qa_gate | acceptance_gate
  affected_acceptance_criteria:
    - criteria_id: string
      statement: string
  options: DecisionPacketOption[]
  recommendation: DecisionPacketRecommendation | null
  deferral_consequence: string
  user_context: DecisionPacketUserContext
  approval_scope: ApprovalScope | null
  reconcile_item_id: string | null
  created_at: string
  resolved_at: string | null

DecisionPacketOption:
  option_id: string
  label: string
  benefits: string[]
  costs: string[]
  risks: string[]
  reversibility: reversible | partially_reversible | irreversible | unknown
  confidence: low | medium | high
  suitable_when: string[]
  evidence_refs: EvidenceRefs

DecisionPacketRecommendation:
  option_id: string | null
  reason: string
  uncertainty: string | null
  when_to_revisit: string | null

DecisionPacketUserContext:
  minimum_context: string[]
  optional_pull_refs: StateRecordRef[]

DecisionPacketCandidate:
  task_id: string
  change_unit_id: string | null
  decision_kind: approval | scope_confirmation | design_choice | architecture_choice | product_tradeoff | autonomy_boundary | verification_waiver | qa_waiver | acceptance | residual_risk_acceptance | reconcile
  context:
    why_now: string
    source_refs: StateRecordRef[]
    evidence_refs: EvidenceRefs
  state_summary_at_request: StateSummary
  what_user_is_deciding: string
  what_agent_may_decide_without_user: string[]
  affected_gates:
    - scope_gate | decision_gate | approval_gate | design_gate | evidence_gate | verification_gate | qa_gate | acceptance_gate
  affected_acceptance_criteria:
    - criteria_id: string
      statement: string
  options: DecisionPacketOption[]
  recommendation: DecisionPacketRecommendation | null
  deferral_consequence: string
  user_context: DecisionPacketUserContext
  expires_at: string | null
  approval_scope: ApprovalScope | null
  reconcile_item_id: string | null

RecommendedPlaybook:
  playbook_id: string
  label: string
  reason: string
  applies_to:
    focus: status | shaping | decision | implementation | verification | qa | acceptance | reconcile
    state_refs: StateRecordRef[]
  route:
    display_route: continue_guidance | show_existing_decision_packet | propose_decision_packet_request | write_readiness_guidance | evidence_guidance | verification_guidance | manual_qa_guidance | close_readiness_guidance | reconcile_guidance
    decision_packet_ref: StateRecordRef | null
    decision_packet_route: none | existing_decision_packet | decision_packet_candidate_or_request_path
  guidance_refs: StateRecordRef[]
  authority_note: string

JourneyCardSummary:
  task_id: string
  state: StateSummary
  current_position: string
  next_action: string
  recommended_playbooks: RecommendedPlaybook[]
  active_change_unit_ref: StateRecordRef | null
  write_authority_summary: WriteAuthoritySummary | null
  active_decision_packet_refs: StateRecordRef[]
  blocker_refs: StateRecordRef[]
  residual_risk_summary: ResidualRiskSummary | null
  projection_freshness:
    status: current | stale | failed | unknown
    stale_refs: StateRecordRef[]

JudgmentContext:
  task_ref: StateRecordRef
  journey_card: JourneyCardSummary | null
  current_state_summary: StateSummary
  minimum_context: string[]
  relevant_refs: StateRecordRef[]
  evidence_refs: EvidenceRefs
  active_decision_packet_refs: StateRecordRef[]
  optional_pull_refs: StateRecordRef[]
  stale_or_missing_refs: StateRecordRef[]
  acceptance_visibility: AcceptanceVisibilityContext | null

AutonomyBoundarySummary:
  change_unit_id: string | null
  status: absent | proposed | active | exceeded | stale
  autonomy_profile: human_in_loop | afk_eligible | evaluator_only | read_only_advisor | null
  what_agent_may_do: string[]
  what_agent_may_decide_without_user: string[]
  what_requires_user_judgment: string[]
  stop_conditions: string[]
  triggered_stop_conditions: string[]
  related_decision_packet_refs: StateRecordRef[]

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
  evidence_summary_refs: StateRecordRef[]
  verification_status: not_required | required | pending | passed | failed | waived_by_user | blocked
  qa_status: not_required | required | pending | passed | failed | waived
  acceptance_status: not_required | required | pending | accepted | rejected
  what_acceptance_does_not_replace: string[]
```

`ResidualRiskSummary.status=none`은 현재 Task와 requested action에 대해 Core가 알고 있는 close-relevant Residual Risk가 없다는 뜻입니다. 이는 acceptance와 ordinary successful close에서 residual-risk visibility를 충족하며, 이때 `close_relevant_count=0`이고 risk-ref array는 비어 있습니다. Core가 hidden, blocked, 또는 표시되지 않은 close-relevant risk를 알고 있다면 이 status를 반환하면 안 되며, 그런 경우 `not_visible` 또는 `blocked`를 사용합니다.

`ResidualRiskSummary.visible_refs`, `not_visible_refs`, `unaccepted_refs`, `accepted_refs`, related acceptance visibility risk-ref array는 `record_kind=residual_risk`인 `StateRecordRef` entry를 포함합니다. `visible_refs`는 현재 judgment context에서 visible한 close-relevant Residual Risk record를 나열하며, risk acceptance가 아직 필요하면 `unaccepted_refs`가 visible risk와 overlap될 수 있습니다. Accepted risk는 Residual Risk record의 metadata/state로 남습니다.

Autonomy Boundary summary는 범위 권한이 아니라 판단 재량을 설명합니다. Active Change Unit scope와 required Approval 밖의 paths, tools, commands, network targets, secret access, sensitive categories를 허가하지 않습니다.

`decision_kind=approval`은 stable public enum value로 유지됩니다. `DecisionPacket`과 `DecisionPacketCandidate` 모두에서 이 값은 sensitive-change Approval만을 위한 Approval 형태의 judgment context를 뜻합니다. 제품 장단점, 설계 방향, 아키텍처 판단이나 중요한 기술 판단, 해결되지 않은 security 또는 product-security 판단, QA 면제, verification risk, final acceptance, Residual Risk 수용 같은 사용자 소유 판단은 별도의 compatible Decision Packets와 gate updates로 표현되지 않는 한 이 값으로 해소할 수 없습니다.

## ValidatorResult

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

`surface_capability_check` validator는 이 schema를 `validator_kind=capability`로 사용합니다.

`ValidatorResult`를 통해 내보내는 Stable MVP validator IDs는 다음과 같습니다.

- `decision_gate_check`
- `decision_quality_check`
- `autonomy_boundary_check`
- `feedback_loop_check`
- `tdd_trace_required`
- `codebase_stewardship_check`
- `residual_risk_visibility_check`
- `shared_design_alignment`
- `vertical_slice_shape`
- `domain_language_consistency`
- `module_interface_review`
- `manual_qa_required`
- `context_hygiene_check`
- `surface_capability_check`

Status, next, write, close flow에서 자주 드러나는 agency-critical subset은 다음과 같습니다.

- `decision_quality_check`
- `autonomy_boundary_check`
- `feedback_loop_check`
- `tdd_trace_required`
- `codebase_stewardship_check`
- `residual_risk_visibility_check`
- `context_hygiene_check`

이 smaller subset에서 빠진 design-quality validator, 즉 `shared_design_alignment`, `vertical_slice_shape`, `domain_language_consistency`, `module_interface_review`는 위 full stable MVP ValidatorResult-emitting set에 계속 포함됩니다.

아래 tool description은 `ValidatorResults emitted`와 Core check/precondition을 구분합니다. Core check는 transition을 막거나, gate를 갱신하거나, blocked reason을 채우거나, fixture assertion에 나타날 수 있지만 위에 나열되지 않는 한 validator ID가 아닙니다.

## Error taxonomy

| Code | Meaning |
|---|---|
| `STATE_CONFLICT` | `expected_state_version`이 관련 state version scope에서 최신이 아니거나, lock ownership이 바뀌었거나, 같은 idempotency key가 다른 payload로 reused됨 |
| `NO_ACTIVE_TASK` | a Task is required but none is active or addressed |
| `NO_ACTIVE_CHANGE_UNIT` | a write-capable operation has no active scoped Change Unit |
| `SCOPE_REQUIRED` | scope confirmation is required before the requested write can proceed |
| `SCOPE_VIOLATION` | intended paths, tools, commands, network, secrets, or categories exceed scope |
| `WRITE_AUTHORIZATION_REQUIRED` | write-capable run에 `prepare_write`가 반환한 required Write Authorization이 없습니다 |
| `WRITE_AUTHORIZATION_INVALID` | supplied Write Authorization이 absent, expired, `stale`, revoked, idempotent replay 밖에서 already consumed, 또는 Task, Change Unit, baseline, intended operation, approval refs, Decision Packet refs와 incompatible합니다 |
| `DECISION_REQUIRED` | 사용자 소유 판단이 requested action 진행을 막고 있어 Decision Packet이 필요함 |
| `DECISION_UNRESOLVED` | relevant Decision Packet이 pending, deferred without coverage, rejected, blocked, `stale` 또는 requested action과 incompatible함 |
| `AUTONOMY_BOUNDARY_EXCEEDED` | the intended operation exceeds the active Change Unit Autonomy Boundary |
| `APPROVAL_REQUIRED` | sensitive change requires approval before proceeding |
| `APPROVAL_DENIED` | the relevant approval was denied |
| `APPROVAL_EXPIRED` | approval expired or drifted from baseline/scope |
| `CAPABILITY_INSUFFICIENT` | 연결된 접점이 required validator 또는 enforcement condition을 충족할 수 없음 |
| `MCP_UNAVAILABLE` | required MCP access가 unavailable, `stale`, 또는 unreachable임 |
| `EVIDENCE_INSUFFICIENT` | required evidence coverage가 absent, partial, `stale`, 또는 blocked임 |
| `VERIFY_NOT_DETACHED` | verification cannot count as detached verification |
| `QA_REQUIRED` | required Manual QA is pending, failed, or missing |
| `ACCEPTANCE_REQUIRED` | required user acceptance is pending or rejected |
| `PROJECTION_STALE` | requested action에 필요한 projection 최신성이 `stale` 또는 `failed`임 |
| `RECONCILE_REQUIRED` | human-editable or managed-block drift requires reconcile |
| `RESIDUAL_RISK_NOT_VISIBLE` | known close-relevant residual risk has not been made visible before acceptance or successful close |
| `ARTIFACT_MISSING` | a referenced artifact file is missing or integrity check failed |
| `BASELINE_STALE` | baseline no longer matches the repository state required by the operation |
| `VALIDATOR_FAILED` | 하나 이상의 required validators가 failed이고 더 specific한 typed `ErrorCode`가 적용되지 않을 때 사용하는 generic fallback |

`WRITE_AUTHORIZATION_REQUIRED`와 `WRITE_AUTHORIZATION_INVALID`는 missing 또는 invalid Write Authorization에만 사용합니다. Observed paths, tools, commands, network targets, secrets, sensitive categories가 authorized 또는 active scope를 넘는 경우 scope violations는 계속 `SCOPE_VIOLATION`을 사용합니다.

`MCP_UNAVAILABLE`은 stable public `ErrorCode`로 유지합니다. Diagnostic detail은 public error code를 추가하지 않고 `MCP_SERVER_UNAVAILABLE`과 `SURFACE_MCP_UNAVAILABLE`을 구분합니다.

- `MCP_SERVER_UNAVAILABLE`: tool call이 Core에 닿을 수 없어 authoritative Core response가 불가능합니다. Caller는 상태 변경을 주장하기 전에 진단하거나 reconnect해야 합니다.
- `SURFACE_MCP_UNAVAILABLE`: Core 또는 operator는 연결된 접점에서 사용할 수 있는 MCP가 없거나, MCP configuration이 오래되었거나, required MCP tool을 호출할 수 없는 상태를 관찰할 수 있습니다. 제품 파일 쓰기는 cooperative 접점에서는 지시로 보류되고, 사용할 수 있는 더 강한 guard가 해당 operation을 포괄할 때만 실행 전에 차단됩니다. Core response는 상황에 따라 `details.mcp_unavailable_kind`와 함께 `MCP_UNAVAILABLE` 또는 `CAPABILITY_INSUFFICIENT`를 사용할 수 있습니다.

MCP availability problem에 대해 `ToolError` object를 사용할 수 있는 경우 `details.mcp_unavailable_kind`는 `server_unavailable`, `surface_mcp_unavailable`, `stale_connection`, `unknown` 중 하나일 수 있습니다.

`DECISION_REQUIRED`, `DECISION_UNRESOLVED`, `WRITE_AUTHORIZATION_REQUIRED`, `WRITE_AUTHORIZATION_INVALID`, `AUTONOMY_BOUNDARY_EXCEEDED`, `RESIDUAL_RISK_NOT_VISIBLE`, `MCP_UNAVAILABLE`은 stable public `ErrorCode` values입니다. Validator-specific detail은 여전히 `ValidatorResult.findings`에 속합니다.

### Primary Error Code Precedence

Public tool response는 Core가 여러 blocker를 동시에 관찰해도 하나의 primary `ToolError.code`만 가집니다. `ToolResponseBase.errors`가 비어 있지 않으면 `errors[0]`가 이 precedence table로 선택된 primary `ToolError`이고, 나머지 entry는 secondary blocker를 나타낼 수 있습니다. Tool subsection이 더 좁은 order를 정의하지 않는 한 primary code는 아래 precedence list에서 처음 적용되는 code입니다. Secondary blocker는 `blocked_reasons`, `CloseTaskResponse.blockers`, validator result, `ToolError.details`, state summary 같은 tool-specific field에 유지합니다.

`Possible errors` list는 tool에서 사용할 수 있는 code를 나열합니다. 이는 per-tool precedence table이 아닙니다.

MCP server나 caller가 Core에 전혀 닿을 수 없으면 접점 또는 operator는 `MCP_UNAVAILABLE`을 보고할 수 있습니다. 하지만 권한 있는 Core response나 상태 변경을 주장할 수는 없습니다. Core가 request를 평가할 수 있으면 다음 order를 적용합니다.

| Precedence | Primary `ErrorCode` | Selection note |
|---:|---|---|
| 1 | `STATE_CONFLICT` | 최신이 아닌 `expected_state_version`, state lock conflict, 또는 같은 idempotency key가 다른 payload로 reused됨 |
| 2 | `MCP_UNAVAILABLE` | Core 또는 operator가 availability problem을 classify한 뒤 required MCP access가 unavailable, `stale`, unreachable임 |
| 3 | `NO_ACTIVE_TASK` | operation에 Task가 필요하지만 active 또는 addressed Task가 없음 |
| 4 | `NO_ACTIVE_CHANGE_UNIT` | operation이 write-capable 또는 close-relevant인데 active scoped Change Unit이 적용되지 않음 |
| 5 | `BASELINE_STALE` | requested operation이 최신이 아닌 baseline에 의존함 |
| 6 | `SCOPE_REQUIRED` | requested operation이 proceed하기 전에 scope confirmation이 필요함 |
| 7 | `SCOPE_VIOLATION` | intended 또는 observed paths, tools, commands, network, secrets, categories가 active 또는 authorized scope를 초과함 |
| 8 | `WRITE_AUTHORIZATION_REQUIRED` | write-capable Run에 required Write Authorization이 없음 |
| 9 | `WRITE_AUTHORIZATION_INVALID` | supplied Write Authorization이 `stale`, expired, revoked, replay 밖에서 consumed, 또는 incompatible함 |
| 10 | `APPROVAL_DENIED` | relevant sensitive-change Approval이 denied됨 |
| 11 | `APPROVAL_EXPIRED` | relevant sensitive-change Approval이 expired되었거나 scope 또는 baseline에서 drift됨 |
| 12 | `APPROVAL_REQUIRED` | sensitive change에 Approval이 필요하지만 compatible granted Approval이 없음 |
| 13 | `DECISION_UNRESOLVED` | existing relevant Decision Packet이 pending, deferred without coverage, rejected, blocked, `stale`, 또는 incompatible함 |
| 14 | `AUTONOMY_BOUNDARY_EXCEEDED` | intended operation이 active Change Unit Autonomy Boundary를 초과하며, next step이 Decision Packet이어도 이 code를 사용함 |
| 15 | `DECISION_REQUIRED` | 사용자 소유 판단이 action 진행을 막고 있어 Decision Packet이 필요함 |
| 16 | `CAPABILITY_INSUFFICIENT` | 연결된 접점이 required capability 또는 enforcement condition을 충족할 수 없음 |
| 17 | `EVIDENCE_INSUFFICIENT` | required evidence coverage가 absent, partial, `stale`, 또는 blocked임 |
| 18 | `VERIFY_NOT_DETACHED` | verification이 detached verification으로 count될 수 없음 |
| 19 | `QA_REQUIRED` | required Manual QA가 pending, failed, missing, 또는 validly waived되지 않음 |
| 20 | `RESIDUAL_RISK_NOT_VISIBLE` | known close-relevant residual risk가 acceptance 또는 close 전에 visible하지 않음. `ResidualRiskSummary.status=none`이 no known close-relevant risk를 confirm한 경우에는 선택하지 않음 |
| 21 | `ACCEPTANCE_REQUIRED` | residual-risk visibility가 satisfied된 뒤에도 required user acceptance가 pending 또는 rejected임 |
| 22 | `PROJECTION_STALE` | requested action에 필요한 projection freshness가 `stale` 또는 `failed`임 |
| 23 | `RECONCILE_REQUIRED` | human-editable 또는 managed-block drift에 reconcile이 필요함 |
| 24 | `ARTIFACT_MISSING` | referenced artifact file이 missing이거나 integrity check에 failed함 |
| 25 | `VALIDATOR_FAILED` | 위의 더 specific한 typed blocker가 적용되지 않을 때만 선택되는 generic validator fallback |

#### `harness.close_task` Close Blockers

`harness.close_task`는 여러 close blocker를 반환할 수 있습니다. `CloseTaskResponse.base.errors`의 primary `ToolError`는 위 precedence를 사용합니다. Present하면 `CloseTaskResponse.base.errors[0].code`가 primary close error code입니다. `CloseTaskResponse.blockers`는 관찰된 close blocker를 같은 relative order로 포함해야 합니다. Required acceptance는 close-relevant residual risk가 visible한 뒤에만 record하거나 rely할 수 있으므로 close 및 acceptance flow에서 residual-risk visibility는 `ACCEPTANCE_REQUIRED`보다 앞에 둡니다.

## Idempotency

Idempotency keys는 `(project_id, tool_name, idempotency_key)`에 scoped됩니다. 같은 key로 같은 payload를 반복하면 original committed response를 반환합니다. 같은 key를 다른 payload로 reuse하면 `STATE_CONFLICT`를 반환합니다.

`request_hash`는 UTF-8로 encode한 정규화된 JSON에서 계산합니다. 정규화된 input은 `tool_name`, schema-normalized request body, 그리고 `request_id`와 `idempotency_key`를 제외한 모든 `ToolEnvelope` field를 포함합니다. 포함되는 envelope fields는 `expected_state_version`, `project_id`, `task_id`, `surface_id`, `run_id`, `actor_kind`, `dry_run`입니다. Hashing 전에 optional fields는 request schema의 default 및 null/empty-field rule에 따라 normalize하고, object keys는 sort하며, arrays는 schema가 order-insignificant라고 명시한 경우가 아니면 schema-defined order를 유지하고, Unicode strings는 NFC를 사용해 일관되게 normalize합니다.

## State Conflict 동작

State-changing tool에서 Core는 `expected_state_version`을 현재 project/Task state와 비교합니다. 일치하지 않으면 `STATE_CONFLICT`를 반환하고 `details`에 현재 state version과 status summary를 포함합니다. Caller는 상태를 새로 읽은 뒤 새 idempotency key로 retry하거나 exact previous request를 replay해야 합니다.

State conflict 비교는 scope-specific입니다. Core는 먼저 `ToolEnvelope.task_id`, tool-specific `task_id`, 또는 active Task resolution에서 operation이 가리키는 primary Task를 찾습니다. Task-scoped tool은 해당 Task의 `tasks.state_version`과 비교하고, 찾은 primary Task가 없는 project-scoped tool은 `project_state.state_version`과 비교합니다. `STATE_CONFLICT.details`에는 `scope`(`task` 또는 `project`), `current_state_version`, `expected_state_version`, relevant `project_id`, 그리고 `scope=task`일 때 `task_id`를 포함해야 합니다. Refresh guidance를 위한 compact status summary도 포함할 수 있습니다.

## Public tools

### `harness.status`

Purpose: project, surface, active Task, Journey Card, gate, guarantee, projection, active Decision Packet, Autonomy Boundary, Write Authority Summary, residual-risk, pending-decision status를 반환합니다.

Allowed actor: `user`, `lead_agent`, `evaluator`, `operator`.

Request schema:

```yaml
StatusRequest:
  envelope: ToolEnvelope
  include:
    task: boolean
    gates: boolean
    projections: boolean
    pending_decisions: boolean
    guarantees: boolean
    journey_card: boolean
    decision_packets: boolean
    autonomy_boundary: boolean
    write_authority: boolean
    residual_risk: boolean
    recommended_playbooks: boolean
```

Response schema:

```yaml
StatusResponse:
  base: ToolResponseBase
  active_task: StateSummary | null
  status_card: string
  journey_card: JourneyCardSummary | null
  pending_decisions: StateRecordRef[]
  active_decision_packet_refs: StateRecordRef[]
  recommended_playbooks: RecommendedPlaybook[]
  autonomy_boundary_summary: AutonomyBoundarySummary | null
  write_authority_summary: WriteAuthoritySummary | null
  residual_risk_summary: ResidualRiskSummary | null
  projection_freshness:
    status: current | stale | failed | unknown
    stale_refs: StateRecordRef[]
  guarantee_display:
    level: cooperative | detective | preventive | isolated
    notes: string[]
```

State transition summary: state transition 없음.

반환될 수 있는 EventRef values: 없음.

Projection job 대기열 추가: 없음.

`pending_decisions`는 해소되지 않은 user-action Decision Packets를 포함합니다. `active_decision_packet_refs`는 pending, deferred, blocked, recently resolved packet을 포함해 현재 phase 또는 requested action과 relevant한 모든 Decision Packet을 포함합니다. 두 field는 모두 `record_kind=decision_packet`인 `StateRecordRef` entry를 사용합니다.

`recommended_playbooks`는 접점 또는 agent stage router를 위한 non-authoritative display guidance이며, status/next display를 위해 현재 상태와 policy/playbook context에서 계산됩니다. Shared design, review, TDD, QA, guard check, release handoff, browser-QA candidacy 같은 절차를 제안할 수 있습니다. `RecommendedPlaybook.playbook_id`는 stable display/routing string identifier이지 Core-owned closed enum이나 DDL-backed value set이 아닙니다. Known initial ID에는 `shared-design`, `product-review`, `eng-review`, `design-review`, `security-review`, `tdd-loop`, `spec-review`, `code-quality-review`, `qa-review`, `guard-check`, `release-handoff`, `browser-qa-candidate`가 포함되며, 이 목록은 future display/playbook documentation 전체를 포괄하지 않습니다. Recommended Playbook은 자체 기준 kernel 기록, DDL table, `task_events` entry, projection job이 없습니다. 상태를 변경하거나, write를 허가하거나, gate를 충족하거나, 근거를 만들거나, verification을 수행하거나, QA를 기록하거나, risk를 수용하거나, result를 수용하거나, Task를 닫지 않습니다. Recommendation을 따를 때 사용자 소유 판단이 필요하면 route는 affected write 또는 close가 진행되기 전에 existing Decision Packet 또는 normal Decision Packet candidate/request path를 가리켜야 합니다. `route.display_route` 값은 display route이지 public tool name이 아니며 상태 변경 tool call 지시도 아닙니다.

`StatusResponse.recommended_playbooks`와 `StatusResponse.journey_card.recommended_playbooks`가 둘 다 present이면, 둘은 같은 computed guidance를 다른 display level에 렌더링한 것입니다. Top-level field는 full Journey Card를 렌더링하지 않는 status 접점용이고, Journey Card field는 같은 guidance를 현재 위치 summary와 함께 유지합니다.

`write_authority_summary`는 `include.write_authority=true`일 때 반환됩니다. `include.journey_card=true`이면 같은 current Write Authority Summary display가 `journey_card.write_authority_summary`에도 나타날 수 있습니다.

ValidatorResults emitted: optional `surface_capability_check`, optional `decision_gate_check`, optional `autonomy_boundary_check`.

Core checks/preconditions: optional residual-risk visibility read, optional projection freshness read.

Possible errors: `MCP_UNAVAILABLE`, `PROJECTION_STALE`.

Idempotency behavior: read-only입니다. Repeated request는 상태를 변경하지 않습니다.

### `harness.intake`

Purpose: user intent에서 Task를 create 또는 resume하고 advisor, direct, work로 분류합니다.

Allowed actor: `user`, `lead_agent`, `operator`.

Request schema:

```yaml
IntakeRequest:
  envelope: ToolEnvelope
  user_request: string
  requested_mode: advisor | direct | work | auto
  resume_policy: resume_active | create_new | supersede_active | reject_if_active
  acceptance_criteria: string[]
  constraints:
    allowed_paths: string[]
    non_goals: string[]
    sensitive_categories: string[]
  initial_context_refs: StateRecordRef[]
```

Response schema:

```yaml
IntakeResponse:
  base: ToolResponseBase
  task_id: string
  created: boolean
  resumed: boolean
  state: StateSummary
  next_action: string
  change_unit_id: string | null
```

State transition summary: Task를 create 또는 resume합니다. `mode`와 initial `lifecycle_phase`를 set하고, write-capable direct/work에는 initial Change Unit을 만들 수 있습니다.

반환될 수 있는 stable EventRef values: 기존 Task가 superseded될 때 `task_superseded`.

implementation-local detail/audit를 위해 반환될 수 있는 non-stable EventRef values: `task_intake_recorded`, `task_created`, `task_resumed`, `change_unit_created`.

Projection job 대기열 추가: `TASK`; intake가 design support record를 accepted했다면 optional `DOMAIN-LANGUAGE`, `MODULE-MAP`, `INTERFACE-CONTRACT`.

ValidatorResults emitted: `surface_capability_check`.

Core checks/preconditions: `state_envelope`, `active_task_policy`.

Possible errors: `STATE_CONFLICT`, `MCP_UNAVAILABLE`, `VALIDATOR_FAILED`, `CAPABILITY_INSUFFICIENT`.

Idempotency behavior: 같은 key는 같은 Task/resume decision을 반환합니다. 같은 key에 다른 payload를 사용하면 `STATE_CONFLICT`입니다.

### `harness.next`

Purpose: 현재 Task의 next safe action, instruction bundle, pending decisions를 반환합니다.

Allowed actor: `user`, `lead_agent`, `evaluator`, `operator`.

Request schema:

```yaml
NextRequest:
  envelope: ToolEnvelope
  task_id: string | null
  focus: status | shaping | decision | implementation | verification | qa | acceptance | reconcile
  include_instruction_bundle: boolean
```

Response schema:

```yaml
NextResponse:
  base: ToolResponseBase
  state: StateSummary | null
  next_action:
    action_kind: ask_user | prepare_write | implement | launch_verify | record_eval | record_manual_qa | request_acceptance | close_task | reconcile | idle
    summary: string
    required_tool: string | null
  recommended_playbooks: RecommendedPlaybook[]
  instruction_bundle:
    summary: string
    constraints: string[]
    relevant_refs: StateRecordRef[]
    artifact_refs: ArtifactRef[]
  pending_decisions: StateRecordRef[]
  judgment_context: JudgmentContext | null
  autonomy_boundary: AutonomyBoundarySummary | null
```

State transition summary: state transition 없음.

반환될 수 있는 EventRef values: 없음.

Projection job 대기열 추가: 없음.

`pending_decisions`는 해소되지 않은 user-action Decision Packets를 포함합니다. 현재 phase 또는 requested action에 아직 영향을 주는 deferred, blocked, recently resolved packet은 `judgment_context.active_decision_packet_refs`를 통해 나타납니다.

`recommended_playbooks`는 반환된 next safe action에 맞는 절차를 caller가 선택하도록 돕습니다. 이는 현재 상태와 policy/playbook context에서 계산되는 API/display guidance일 뿐입니다. `playbook_id`는 display/routing string identifier로 남으며 기준 kernel enum이 아닙니다. 기준 상태를 갱신하거나, `task_events` entry를 추가하거나, projection job을 대기열에 넣거나, `decision_gate`, `approval_gate`, `evidence_gate`, `verification_gate`, `qa_gate`, `acceptance_gate`를 충족하거나, `prepare_write`, evidence, verification, QA, risk acceptance, 결과 수용, close를 대체하면 안 됩니다. 사용자 소유 판단을 새로 요구하는 playbook recommendation은 affected write 또는 close가 진행되기 전에 Decision Packet candidate/request path 또는 existing Decision Packet으로 라우팅해야 합니다. `route.display_route` 값은 display route이지 public tool name이 아니며 상태 변경 tool call 지시도 아닙니다.

`focus=acceptance`일 때 `judgment_context.acceptance_visibility`는 non-null이어야 합니다. 이 context는 residual-risk summary, unaccepted close-relevant risk refs, evidence summary refs, verification status, QA status, acceptance status, what acceptance does not replace를 포함해야 합니다. 이 context는 known close-relevant risk가 없다는 뜻의 `ResidualRiskSummary.status=none`과, known close-relevant risk가 아직 hidden이라는 뜻의 `not_visible`을 구분해야 합니다. Acceptance request 전에 acceptance가 evidence sufficiency, verification, Manual QA, approval, scope, residual-risk visibility를 대체하지 않는다는 점을 명확히 보여줘야 합니다.

ValidatorResults emitted: optional `surface_capability_check`, optional `decision_gate_check`, optional `autonomy_boundary_check`, optional `context_hygiene_check`.

Possible errors: `DECISION_REQUIRED`, `DECISION_UNRESOLVED`, `NO_ACTIVE_TASK`, `MCP_UNAVAILABLE`, `PROJECTION_STALE`, `AUTONOMY_BOUNDARY_EXCEEDED`, `RECONCILE_REQUIRED`.

Idempotency behavior: read-only입니다. Repeated request는 상태를 변경하지 않습니다.

### `harness.prepare_write`

Purpose: agent가 write하기 전에 intended product write가 allowed인지 결정합니다.

Allowed actor: `lead_agent`, `operator`.

Request schema:

```yaml
PrepareWriteRequest:
  envelope: ToolEnvelope
  task_id: string
  change_unit_id: string | null
  intended_operation: string
  intended_paths: string[]
  intended_tools: string[]
  intended_commands:
    - command: string
      command_class: string
      writes_product_files: boolean
  intended_network:
    - target: string
      direction: read | write
  intended_secrets:
    - secret_handle: string
      access_kind: read | write
  sensitive_categories: string[]
  baseline_ref: string | null
```

Response schema:

```yaml
PrepareWriteResponse:
  base: ToolResponseBase
  decision: allowed | blocked | approval_required | decision_required | state_conflict
  state: StateSummary | null
  change_unit_id: string | null
  baseline_ref: string | null
  write_authorization_ref: StateRecordRef | null
  write_authorization: WriteAuthorizationSummary | null
  authorization_effect: none | would_create | created | returned
  active_decision_packet_refs: StateRecordRef[]
  blocked_reasons:
    - code: string
      message: string
      related_error: ErrorCode
  approval_request_candidate: ApprovalRequestCandidate | null
  decision_packet_candidate: DecisionPacketCandidate | null
  guarantee_display:
    level: cooperative | detective | preventive | isolated
    notes: string[]

ApprovalRequestCandidate:
  sensitive_categories: string[]
  allowed_paths: string[]
  allowed_tools: string[]
  allowed_commands: string[]
  allowed_network_targets: string[]
  secret_scope: string[]
  baseline_ref: string | null
```

`approval_request_candidate`는 `decision=approval_required`이거나 Core가 새 Approval 요청을 제안할 수 있을 때만 포함합니다. 그 외에는 `null`입니다. 이는 이후 `harness.request_user_decision(decision_kind=approval)` 호출의 `approval_scope`에 사용할, 상태를 변경하지 않는 candidate입니다. `prepare_write`가 이를 반환해도 Approval 기록, Decision Packet, Write Authorization, `APR` projection job은 생성되지 않습니다. UI, status response, next-action response가 Approval 요청 commit 전에 이 payload를 표시한다면 이를 candidate 표시로 label해야 하며 `APR` projection이라고 부르면 안 됩니다.

`dry_run=false`이고 `decision=allowed`일 때 response는 non-null `write_authorization_ref`를 포함해야 합니다. Caller가 expanded payload를 request하거나 implementation이 지원하면 `write_authorization` summary도 반환할 수 있습니다. `authorization_effect`는 Core가 새 authorization을 create하면 `created`입니다.

`WriteAuthorizationSummary.basis_state_version`은 Core가 allowed write attempt의 compatibility basis로 사용한 affected-scope state version입니다. MVP prepare-write product writes에서는 `task_id`의 Task State Version입니다. Replay와 최신성 감지 audit metadata이며, response의 resulting `base.state_version`이 아닙니다.

`authorization_effect=returned`는 같은 idempotency key, request hash, `basis_state_version`을 가진 동일한 committed `prepare_write` request와 response의 idempotent replay에만 reserved됩니다. 서로 다른 compatible request는 서로 다른 Write Authorization을 생성합니다. Compatibility가 authorization을 reusable하게 만들지는 않습니다. Compatibility basis가 바뀌면 Core는 오래된 unconsumed authorization을 `stale`, expire, revoke할 수 있습니다.

`dry_run=true`이고 write가 otherwise allowed라면 Core는 `decision=allowed`와 `authorization_effect=would_create`를 반환합니다. 하지만 `write_authorization_ref`와 `write_authorization`은 반드시 `null`이어야 하고, Write Authorization record, event, artifact, projection job은 create되지 않습니다.

`decision=blocked`, `decision=approval_required`, `decision=decision_required`, `decision=state_conflict`에서는 두 authorization fields가 모두 `null`이고 `authorization_effect=none`이어야 합니다.

Write Authorization은 intended operation과 현재 state, baseline, active Change Unit scope, Approval ref, Decision Packet ref, sensitive categories, guarantee level에 한정됩니다. 이는 `write_authorization_id`를 통해 `harness.record_run`이 consume하며 재사용 가능한 grant가 아닙니다.

`active_decision_packet_refs`는 intended write와 relevant한 모든 Decision Packets를 포함합니다. Pending, deferred, blocked, recently resolved packets가 포함됩니다.

`decision_packet_candidate`는 `decision=decision_required`이고 compatible Decision Packet이 아직 없을 때 present합니다. Field는 envelope 이후의 `RequestUserDecisionRequest`와 일치합니다. 이는 나중에 `harness.request_user_decision`을 호출하기 위한 non-mutating candidate payload입니다. `prepare_write`가 이를 반환해도 Decision Packet이 생성되거나 update되지는 않습니다.

상태 전이 요약: Task를 `executing`, `waiting_user`, `blocked`로 옮길 수 있습니다. Allowed일 때 Write Authorization을 생성하거나 idempotent replay에 대해 already committed response를 반환할 수 있습니다. `scope_gate=pending/blocked`, `decision_gate=required/pending/blocked`, `approval_gate=required/expired`, `stale` evidence/Approval marker를 set할 수 있습니다. `approval_gate=pending`은 `harness.request_user_decision(decision_kind=approval)`이 Approval 형태 Decision Packet과 연결된 pending Approval 기록을 생성할 때 시작됩니다.

반환될 수 있는 stable EventRef values: `prepare_write_allowed`, `write_authorization_created`, `write_authorization_returned`, `prepare_write_blocked`, `scope_required`, `decision_required`, `autonomy_boundary_exceeded`, `approval_required`, `baseline_stale_detected`, `capability_insufficient_detected`.

Projection job 대기열 추가: `TASK`. `prepare_write`는 `decision=approval_required` 또는 `approval_request_candidate`를 반환했다는 이유만으로 `APR`을 대기열에 넣으면 안 됩니다. `APR`은 기록된 Approval 기록과 Approval 형태 Decision Packet lifecycle에만 reserved됩니다.

ValidatorResults emitted: `autonomy_boundary_check`, `decision_gate_check`, `decision_quality_check`, `feedback_loop_check`, `tdd_trace_required`, `codebase_stewardship_check`, applicable design-quality validators, `surface_capability_check`.

`tdd_trace_required`가 적용되면 `prepare_write`는 actual RED evidence와 valid TDD waiver가 없는 non-test implementation write에 design-policy blocker를 보고할 수 있습니다. Intended operation이 failing RED check를 만드는 test-path write라면 scope, baseline, approval, Autonomy Boundary, other required check가 통과할 때 계속 진행할 수 있습니다. RED target 또는 plan은 그 test-path write를 뒷받침할 수 있지만, non-test implementation write를 위한 RED-evidence precondition이나 Evidence Manifest coverage를 충족하면 안 됩니다. Blocker는 validator 결과, blocked reason, 필요한 경우 secondary error/details로 표현합니다. Primary `ToolError.code`는 API precedence table에 따라 선택합니다.

Core checks/preconditions: `state_envelope`, `active_task`, `active_change_unit`, `scope_coverage`, `changed_paths_intent`, `baseline_freshness`, `approval_scope`, write 전 applicable한 design preconditions.

Possible errors: `STATE_CONFLICT`, `DECISION_REQUIRED`, `DECISION_UNRESOLVED`, `NO_ACTIVE_TASK`, `NO_ACTIVE_CHANGE_UNIT`, `SCOPE_REQUIRED`, `SCOPE_VIOLATION`, `AUTONOMY_BOUNDARY_EXCEEDED`, `APPROVAL_REQUIRED`, `APPROVAL_DENIED`, `APPROVAL_EXPIRED`, `BASELINE_STALE`, `CAPABILITY_INSUFFICIENT`, `MCP_UNAVAILABLE`, `VALIDATOR_FAILED`.

Idempotency behavior: 같은 payload로 repeated allowed/blocked decision은 original decision과 event refs를 반환합니다. 같은 key에 changed payload를 사용하면 `STATE_CONFLICT`입니다.

#### Approval Lifecycle

Sensitive-change Approval은 다음 절차를 따릅니다.

1. `harness.prepare_write`가 intended product write의 sensitive categories를 감지합니다.
2. Scope, baseline, sensitive categories, paths, tools, commands, network targets, secret access, capability 요구사항을 포괄하는 compatible granted Approval이 없으면 `prepare_write`는 `decision=approval_required`를 반환하고, `approval_request_candidate`를 포함하며, 두 Write Authorization field를 `null`로 두고 `authorization_effect=none`을 사용하며, Task blocker를 update하고 `TASK`를 대기열에 넣을 수 있습니다. 이 상태를 변경하지 않는 candidate 때문에 Approval 기록, Decision Packet, Write Authorization, `APR` projection job을 생성하면 안 됩니다.
3. Caller는 candidate와 current intended write에서 파생한 `approval_scope`로 `harness.request_user_decision`을 `decision_kind=approval`과 함께 호출합니다.
4. Core는 Approval 형태의 사용자 판단을 위한 기준 Decision Packet과 pending Approval 기록을 생성합니다. Response는 `decision_packet_ref`와 `approval_id`를 모두 포함하며, 이 커밋된 Approval 요청이 `APR`을 대기열에 넣습니다.
5. User 또는 operator는 해당 Decision Packet에 대해 `harness.record_user_decision`을 호출합니다.
6. Core는 Decision Packet 해소를 기록하고 연결된 Approval 기록을 업데이트하며 `approval_gate`를 granted, denied, expired 중 하나로 다시 계산하고, 업데이트된 Approval 결정을 위해 `APR`을 다시 대기열에 넣습니다.
7. Approval이 granted이면 caller는 fresh idempotency key와 current `expected_state_version`으로 `harness.prepare_write`를 다시 호출합니다.
8. 그 retry만 Write Authorization을 만들 수 있습니다. Approved scope, baseline, sensitive categories, paths, tools, commands, network targets, secret scope, Decision Packet refs, Approval refs, capability checks가 current intended write와 compatible할 때만 성공합니다.

Approval은 정해진 scope 안의 sensitive categories를 허가합니다. Approval은 제품 장단점, 설계 방향, 아키텍처 판단이나 중요한 기술 판단, 해결되지 않은 security 또는 product-security 판단, verification risk, QA 면제, final acceptance, Residual Risk 수용 같은 사용자 소유 판단을 해소하지 않습니다. Sensitive action이 사용자 소유의 제품 판단, 중요한 기술 판단이나 아키텍처 판단, 또는 해결되지 않은 security/product-security 판단도 포함하면 Core는 `prepare_write`가 `allowed`를 반환하기 전에 별도의 compatible Decision Packet을 요구해야 합니다. Approval은 Write Authorization이 아닙니다. 실제 제품 쓰기에는 여전히 allowed `prepare_write` result와 반환된 Write Authorization을 compatible하게 consume하는 `harness.record_run`이 필요합니다.

### `harness.record_run`

Purpose: artifacts와 evidence updates를 포함해 shaping, implementation, direct-result, verification-input run data를 기록합니다.

Allowed actor: `lead_agent`, `evaluator`, `operator`.

Request schema:

```yaml
RecordRunRequest:
  envelope: ToolEnvelope
  kind: shaping_update | implementation | direct | verification_input
  task_id: string
  change_unit_id: string | null
  run_id: string | null
  baseline_ref: string | null
  write_authorization_id: string | null
  summary: string
  artifact_inputs: ArtifactInput[]
  payload: RecordRunPayload

RecordRunPayload:
  shaping_update: ShapingUpdatePayload | null
  implementation: ImplementationPayload | null
  direct: DirectPayload | null
  verification_input: VerificationInputPayload | null

ShapingUpdatePayload:
  task_summary_update: string | null
  acceptance_criteria_updates:
    - criteria_id: string | null
      operation: add | update | remove
      statement: string
  change_unit_updates:
    - operation: create | update | select_active | complete | defer | supersede
      change_unit_id: string | null
      title: string | null
      purpose: string | null
      non_goals: string[]
      slice_type: vertical | enabling | cleanup | horizontal-exception | null
      horizontal_exception_reason: string | null
      follow_up_vertical_change_unit_id: string | null
      allowed_paths: string[]
      allowed_tools: string[]
      allowed_commands: string[]
      allowed_network_targets: string[]
      secret_scope: string[]
      sensitive_categories: string[]
      autonomy_profile: human_in_loop | afk_eligible | evaluator_only | read_only_advisor | null
      agent_may_do: string[]
      user_judgment_required: string[]
      afk_stop_conditions: string[]
      end_to_end_path: EndToEndPath | null
      validator_profile: string[]
      completion_conditions: string[]
      evaluator_focus: string[]
  design_record_refs: StateRecordRef[]
  pending_decision_refs: StateRecordRef[]
  feedback_loop_updates: FeedbackLoopUpdate[]

ImplementationPayload:
  observed_changes: ObservedChanges
  command_results: CommandResult[]
  evidence_updates: EvidenceUpdates
  tdd_trace_update: TddTraceUpdate | null

DirectPayload:
  observed_changes: ObservedChanges
  command_results: CommandResult[]
  evidence_updates: EvidenceUpdates
  self_check_summary: string
  escalation:
    value: none | escalate_to_work
    reason: string | null

VerificationInputPayload:
  evaluator_bundle_input: ArtifactInput | null
  evaluator_focus: string[]
  observed_changes: ObservedChanges
  command_results: CommandResult[]

ObservedChanges:
  changed_paths: string[]
  created_paths: string[]
  deleted_paths: string[]

CommandResult:
  command: string
  exit_code: integer
  artifact_inputs: ArtifactInput[]
  summary: string

EvidenceUpdates:
  acceptance_criteria:
    - criteria_id: string
      status: supported | unsupported | not_applicable
      supporting_refs: StateRecordRef[]
      artifact_inputs: ArtifactInput[]
  feedback_loop_updates: FeedbackLoopUpdate[]

FeedbackLoopUpdate:
  feedback_loop_id: string | null
  operation: create | update
  change_unit_id: string | null
  loop_kind: test | typecheck | lint | build | browser_smoke | manual_qa | tdd | eval | operational | alternate | null
  loop_profile: string | null
  planned_loop: string | null
  selected_loop_refs: StateRecordRef[]
  execution_refs: StateRecordRef[]
  artifact_inputs: ArtifactInput[]
  tdd_trace_refs: StateRecordRef[]
  manual_qa_record_refs: StateRecordRef[]
  evidence_manifest_refs: StateRecordRef[]
  status: defined | executed | waived | blocked | stale | null
  waiver_reason: string | null
  alternate_loop: string | null

TddTraceUpdate:
  tdd_trace_id: string | null
  status: required | recorded | waived | not_required
  red_inputs: ArtifactInput[]
  green_inputs: ArtifactInput[]
  refactor_inputs: ArtifactInput[]
  non_tdd_justification: string | null
```

`payload` branch는 `kind`와 일치해야 하며, 다른 branch는 `null`이거나 absent여야 합니다. `ArtifactInput` 값은 같은 Core transaction에서 찾고, response field에는 committed `ArtifactRef` 값이 들어갑니다. MVP에서 Change Unit creation과 update는 `kind=shaping_update`와 `change_unit_updates`를 통해 이뤄집니다. `operation=create`는 `change_units` record를 만들고, `operation=select_active`는 Task의 `active_change_unit_id`를 update합니다. `allowed_paths`, `allowed_tools`, `allowed_commands`, `allowed_network_targets`, `secret_scope`, `sensitive_categories`는 scope field입니다. `autonomy_profile`, `agent_may_do`, `user_judgment_required`, `afk_stop_conditions`는 Autonomy Boundary judgment latitude만 설명합니다.

`secret_omitted` artifact를 attach하는 Evidence update는 남아 있는 보이는 nonsecret evidence로 증명되는 acceptance criteria 또는 completion condition만 지원할 수 있습니다. `blocked` artifact를 attach하는 Evidence update는 attempted capture를 committed metadata-only notice로 보존하지만, 금지된 원본 payload가 필요한 evidence를 충족하지 않습니다. 관련 Evidence Manifest 또는 gate는 documented resolution이 valid path를 제공할 때까지 unsupported, partial, blocked, insufficient 중 적절한 상태로 남습니다.

Feedback Loop creation과 definition은 `ShapingUpdatePayload.feedback_loop_updates`를 통해 이뤄집니다. Execution evidence와 status update는 `EvidenceUpdates.feedback_loop_updates` 또는 Manual QA가 selected loop일 때 `harness.record_manual_qa`를 통해 이뤄집니다. `operation=create`는 기준 `feedback_loops` row를 만들고 `record_kind=feedback_loop`인 `StateRecordRef`를 반환합니다. Public caller는 일반적으로 Core 할당을 위해 `feedback_loop_id`를 null로 두며, executable fixture/import runner는 deterministic collision-free `FBL-*` ID를 supply할 수 있습니다. `operation=update`는 `feedback_loop_id`가 같은 Task와 compatible Change Unit에 속한 existing feedback-loop row를 가리켜야 합니다. Update에서는 null scalar field가 stored value를 unchanged로 두고, ref array와 artifact input은 additive입니다. TDD가 selected되면 TDD Trace를 `tdd_trace_refs`에 둘 수 있지만, 이는 execution evidence로 남으며 Feedback Loop row를 대체하지 않습니다. TDD waiver가 기록되면 `TddTraceUpdate.non_tdd_justification`은 reason을 기록하고, 관련 `FeedbackLoopUpdate.alternate_loop` 또는 selected-loop ref는 evidence를 제공할 alternate feedback loop를 기록합니다.

`write_authorization_id`는 `harness.prepare_write`가 반환한 compatible Write Authorization을 reference합니다. `kind=implementation`과 `kind=direct`에서는 Run이 product write를 기록하지 않고 Core가 read-only evidence 또는 shaping으로 classify하는 경우를 제외하면 `write_authorization_id`가 required입니다. `kind=shaping_update`에서는 `write_authorization_id`가 `null`이어야 합니다. MVP는 observed product write도 함께 기록하는 shaping update를 지원하지 않으므로, 그런 write는 compatible authorization과 함께 `kind=implementation` 또는 `kind=direct`로 record해야 합니다. `kind=verification_input`에서는 `write_authorization_id`를 `null`로 둡니다. Product write를 만드는 verification input은 MVP에서 보통 disallowed여야 합니다.

`runs.write_authorization_id`는 Run이 compatible Write Authorization을 성공적으로 consume할 때만 populated됩니다. Invalid, `stale`, missing, consumed, scope-exceeded authorization을 사용하려 한 violation 또는 audit Run은 `runs.write_authorization_id`를 consumed authorization으로 populate하면 안 됩니다. Audit에 유용한 attempted authorization 참조는 validator finding, run violation payload, 또는 `task_events.payload_json`에 기록해야 합니다. Observed product write가 이미 발생했다면 audit 또는 recovery를 위해 이런 violation Run을 record할 수 있지만, evidence sufficiency, detached verification, QA, acceptance, close readiness를 충족하면 안 됩니다. Corresponding Write Authorization은 unconsumed로 남아야 하며 violation과 compatibility basis에 따라 `stale`, revoked, expired로 표시될 수 있습니다.

Response schema:

```yaml
RecordRunResponse:
  base: ToolResponseBase
  run_id: string | null
  state: StateSummary
  write_authorization_ref: StateRecordRef | null
  evidence_manifest_ref: StateRecordRef | null
  updated_feedback_loop_refs: StateRecordRef[]
  run_summary_ref: StateRecordRef | null
  direct_result_ref: StateRecordRef | null
  registered_artifacts: ArtifactRef[]
  next_action: string
```

`run_id`는 Core가 Run을 record했을 때 committed Run ID입니다. Core가 어떤 Run도 commit하기 전에 request를 거부하면, 예를 들어 write-capable implementation 또는 direct Run에 Write Authorization이 없으면 `run_id`는 `null`입니다. 이런 pre-commit rejection response에서는 `write_authorization_ref`, `evidence_manifest_ref`, `run_summary_ref`, `direct_result_ref`가 `null`로 남고, `registered_artifacts`와 `updated_feedback_loop_refs`는 비어 있습니다.

`write_authorization_ref`는 committed Run이 compatible Write Authorization을 성공적으로 consume할 때만 non-null입니다.

Violation 또는 audit Run은 Core가 그런 Run을 deliberate하게 record할 때, 예를 들어 observed product write가 이미 happened한 뒤에만 non-null `run_id`를 가질 수 있습니다. Rejected pre-commit cases는 Run ID를 fabricate하면 안 됩니다.

State transition summary: shaping updates는 `shaping`을 유지하거나 `ready` 또는 `waiting_user`로 이동할 수 있습니다. Implementation은 `verifying` 쪽으로 이동합니다. Direct는 close-eligible이 되거나 work로 escalate할 수 있습니다. Verification input은 detached verification을 증명하지 않고 evaluator bundle context를 기록합니다.

반환될 수 있는 stable EventRef values: `run_recorded`, `write_authorization_consumed`, `write_authorization_violation_detected`, `write_authorization_staled`, `write_authorization_revoked`, `write_authorization_expired`, `scope_violation_detected`, `evidence_manifest_updated`.

implementation-local detail/audit를 위해 반환될 수 있는 non-stable EventRef values: `shaping_updated`, `implementation_recorded`, `direct_result_recorded`, `verification_input_recorded`, `artifact_registered`, `feedback_loop_updated`, `tdd_trace_updated`.

Violation 또는 audit Run은 audit 및 recovery를 위해 `write_authorization_violation_detected`, `write_authorization_staled`, `write_authorization_revoked`, `write_authorization_expired`, `scope_violation_detected`를 내보낼 수 있습니다. 그런 Run은 evidence sufficiency, detached verification, QA, acceptance, close readiness를 충족할 수 없습니다. Pre-commit rejection response는 `record_run`에서 stable EventRef value를 반환하지 않습니다.

Committed Run response에서 대기열에 들어가는 projection job: `TASK`, `RUN-SUMMARY`, `EVIDENCE-MANIFEST`; `kind=direct`일 때 `DIRECT-RESULT`; TDD trace가 update되면 `TDD-TRACE`. Pre-commit rejection response는 projection job을 대기열에 넣지 않습니다.

ValidatorResults emitted: `decision_quality_check`, `autonomy_boundary_check`, `feedback_loop_check`, `tdd_trace_required`, `codebase_stewardship_check`, applicable design-quality validators, `surface_capability_check`.

Core checks/preconditions: `state_envelope`, `changed_paths`, `scope_coverage`, `approval_scope`, `baseline_freshness`, `artifact_integrity`, `evidence_sufficiency`.

Possible errors: `STATE_CONFLICT`, `NO_ACTIVE_TASK`, `NO_ACTIVE_CHANGE_UNIT`, `WRITE_AUTHORIZATION_REQUIRED`, `WRITE_AUTHORIZATION_INVALID`, `SCOPE_VIOLATION`, `APPROVAL_REQUIRED`, `APPROVAL_EXPIRED`, `ARTIFACT_MISSING`, `BASELINE_STALE`, `EVIDENCE_INSUFFICIENT`, `VALIDATOR_FAILED`, `CAPABILITY_INSUFFICIENT`, `MCP_UNAVAILABLE`.

Idempotency behavior: repeated request는 같은 run, artifact record, evidence update, event, projection job을 반환합니다. Artifact input과 resolved artifact ref는 original payload와 일치해야 합니다.

### `harness.request_user_decision`

Purpose: progress, write, close, risk acceptance, waiver, reconcile을 block하는 user judgment를 위한 structured Decision Packet을 create합니다.

Allowed actor: `lead_agent`, `evaluator`, `operator`.

Request schema:

```yaml
RequestUserDecisionRequest:
  envelope: ToolEnvelope
  task_id: string
  change_unit_id: string | null
  decision_kind: approval | scope_confirmation | design_choice | architecture_choice | product_tradeoff | autonomy_boundary | verification_waiver | qa_waiver | acceptance | residual_risk_acceptance | reconcile
  context:
    why_now: string
    source_refs: StateRecordRef[]
    evidence_refs: EvidenceRefs
  state_summary_at_request: StateSummary | null
  what_user_is_deciding: string
  what_agent_may_decide_without_user: string[]
  affected_gates:
    - scope_gate | decision_gate | approval_gate | design_gate | evidence_gate | verification_gate | qa_gate | acceptance_gate
  affected_acceptance_criteria:
    - criteria_id: string
      statement: string
  options: DecisionPacketOption[]
  recommendation: DecisionPacketRecommendation | null
  deferral_consequence: string
  user_context: DecisionPacketUserContext
  expires_at: string | null
  approval_scope: ApprovalScope | null
  reconcile_item_id: string | null
```

Core는 기준 `DecisionPacket`을 저장합니다. Minimal MVP 구현은 `decision_requests`를 생략할 수 있으며, public request와 response schema는 Decision Request가 아니라 Decision Packet을 중심으로 유지됩니다. 구현이 `decision_requests`도 만들거나 업데이트한다면 해당 row는 routing, interaction, idempotency replay, legacy handoff metadata일 뿐이며 gate aggregation이 그 metadata를 고려하려면 먼저 기준 `decision_packet_id`로 다시 연결되어야 합니다. `decision_request` row만으로는 `decision_gate`, Approval, acceptance, waiver, Residual Risk 수용, close를 절대 만족하지 않습니다. `state_summary_at_request`가 `null`이면 Core가 같은 transaction 안에서 current state로부터 파생합니다. Stored `state_summary_at_request`는 request-time snapshot이며 이후 Task transition으로 업데이트되지 않습니다. `approval_scope`는 `decision_kind=approval`일 때 required이며, 다른 `decision_kind` value에서는 `null` 또는 omitted여야 합니다. `decision_kind=approval`은 Approval 형태의 sensitive-change context일 뿐이며, 별도의 compatible Decision Packet과 gate update 없이 제품 장단점, 설계 방향, 아키텍처 판단이나 중요한 기술 판단, 해결되지 않은 security 또는 product-security 판단, QA 면제, verification risk, final acceptance, Residual Risk 수용 같은 사용자 소유 판단을 해소할 수 없습니다. `decision_kind=approval`에서 Core는 Approval 범위를 사용해 연결된 pending Approval 기록도 생성합니다. Approval은 `harness.record_user_decision`이 Decision Packet을 해소하기 전에는 granted가 아닙니다. `residual_risk_acceptance` packet은 `user_context.minimum_context`에 risk visibility context를 포함하고 `context.source_refs`에 relevant risk ref를 포함해야 합니다.

Response schema:

```yaml
RequestUserDecisionResponse:
  base: ToolResponseBase
  decision_packet_id: string
  decision_packet_ref: StateRecordRef
  decision_packet: DecisionPacket
  approval_id: string | null
  reconcile_item_id: string | null
  state: StateSummary
  user_visible_summary: string
```

Status와 next-action response가 반환하는 `pending_decisions`는 `record_kind=decision_packet`인 해소되지 않은 user-action `StateRecordRef` entry를 포함합니다. `active_decision_packet_refs` field는 pending, deferred, blocked, recently resolved packet을 포함해 current phase 또는 requested action과 relevant한 모든 Decision Packet을 포함합니다.

상태 전이 요약: pending Decision Packet을 record하고 보통 Task를 `waiting_user`로 옮깁니다. 사용자 소유 제품 장단점 판단 또는 중요한 기술/아키텍처 선택 같은 `decision_gate` 대상 판단은 `decision_gate=pending`을 set합니다. Approval 요청은 pending Approval 기록을 생성하고 `approval_gate=pending`을 set하며, scope confirmation은 `scope_gate=pending`을 set합니다. Acceptance와 Residual Risk 수용은 acceptance가 required일 때 `acceptance_gate=pending`을 set하거나 유지합니다.

implementation-local detail/audit를 위해 반환될 수 있는 non-stable EventRef values: `decision_packet_created`, `user_decision_requested`, `approval_requested`, `scope_confirmation_requested`, `design_choice_requested`, `architecture_choice_requested`, `autonomy_boundary_decision_requested`, `verification_waiver_requested`, `qa_waiver_requested`, `acceptance_requested`, `residual_risk_acceptance_requested`, `reconcile_decision_requested`.

Projection job 대기열 추가: `TASK`; Core가 기준 Approval 형태 Decision Packet과 연결된 pending Approval 기록을 만든 뒤 `decision_kind=approval`에 대해서만 `APR`; reconcile에는 affected projection.

Standalone Decision Packet projection이 켜져 있을 때만 optional `DEC` job을 대기열에 넣습니다.

ValidatorResults emitted: `decision_quality_check`, `autonomy_boundary_check` when the packet affects the active Change Unit boundary, `residual_risk_visibility_check` for risk-acceptance decisions.

Core checks/preconditions: `state_envelope`, `decision_packet_validity`, Approval decision에 대한 `approval_scope`, reconcile decision에 대한 `reconcile_required`.

Possible errors: `STATE_CONFLICT`, `DECISION_REQUIRED`, `NO_ACTIVE_TASK`, `NO_ACTIVE_CHANGE_UNIT`, `SCOPE_REQUIRED`, `AUTONOMY_BOUNDARY_EXCEEDED`, `APPROVAL_REQUIRED`, `RECONCILE_REQUIRED`, `RESIDUAL_RISK_NOT_VISIBLE`, `PROJECTION_STALE`, `VALIDATOR_FAILED`, `MCP_UNAVAILABLE`.

Idempotency behavior: repeated request는 같은 Decision Packet, related record, event, projection job을 반환합니다. 같은 key에 다른 packet payload를 사용하면 `STATE_CONFLICT`입니다.

### `harness.record_user_decision`

Purpose: pending Decision Packet에 대한 user's answer를 record하고 optional accepted residual risk를 기록합니다.

Allowed actor: `user`, `operator`.

Request schema:

```yaml
RecordUserDecisionRequest:
  envelope: ToolEnvelope
  decision_packet_id: string
  decision_kind: approval | scope_confirmation | design_choice | architecture_choice | product_tradeoff | autonomy_boundary | verification_waiver | qa_waiver | acceptance | residual_risk_acceptance | reconcile
  selected_option_id: string
  decision: RecordUserDecisionPayload
  note: string
  waiver_reason: string | null
  accepted_risks: AcceptedRiskInput[]

RecordUserDecisionPayload:
  approval:
    value: granted | denied | expired
  scope_confirmation:
    value: confirmed | rejected | revise_scope
  design_choice:
    value: selected | rejected | defer
  architecture_choice:
    value: selected | rejected | defer
  product_tradeoff:
    value: selected | rejected | defer
  autonomy_boundary:
    value: accepted | rejected | revise_boundary | defer
  verification_waiver:
    value: waived | rejected
  qa_waiver:
    value: waived | rejected
  acceptance:
    value: accepted | rejected
  residual_risk_acceptance:
    value: accepted | rejected | defer
  reconcile:
    value: merge | reject | convert_to_note | create_decision | defer

AcceptedRiskInput:
  residual_risk_ref: StateRecordRef | null
  risk_summary: string
  accepted_scope: string[]
  acceptance_consequence: string
  follow_up_required: boolean
  follow_up: string | null
  evidence_refs: EvidenceRefs
```

Payload branch는 `decision_kind`와 일치해야 하며, 다른 branch는 absent여야 합니다. `accepted_risks`는 Decision Packet과 current Judgment Context가 user decision 전에 close-relevant residual risk를 보이게 만든 경우에만 allowed입니다. `decision_kind=acceptance`에서 Core는 close-relevant residual risk가 보이거나 `ResidualRiskSummary.status=none`이 no known close-relevant risk를 confirm한 경우에만 acceptance를 record할 수 있습니다. Core는 `decision_packet_id`가 식별하는 기준 `DecisionPacket`에 answer를 record합니다. 모든 `decision_requests` row는 routing/replay metadata로만 update되며 linked compatible Decision Packet과 owner-record update 없이는 `decision_gate`, approval, acceptance, waiver, Residual Risk 수용, close를 충족할 수 없습니다. Core는 Residual Risk record를 update하고 residual-risk 상태 참조를 반환하여 accepted risk를 기록하며, risk acceptance를 detached verification으로 취급하지 않습니다. `AcceptedRiskInput.residual_risk_ref=null`은 current Decision Packet과 Judgment Context가 해당 close-relevant risk를 이미 사용자에게 보이게 만들고, Core가 같은 기록된 transition 안에서 Residual Risk record를 생성하거나 associate할 수 있을 만큼 충분한 source/evidence context를 포함할 때만 allowed입니다. Visibility 또는 context가 없으면 Core는 hidden risk를 조용히 create하고 accept하지 말고 reject 또는 block해야 합니다.

Response schema:

```yaml
RecordUserDecisionResponse:
  base: ToolResponseBase
  decision_packet_id: string
  decision_packet_ref: StateRecordRef
  state: StateSummary
  updated_records: StateRecordRef[]
  accepted_risk_refs: StateRecordRef[]
  next_action: string
```

`RecordUserDecisionResponse.accepted_risk_refs`는 `record_kind=residual_risk`인 `StateRecordRef` entries만 포함합니다. Standalone accepted-risk record kind는 없습니다.

상태 전이 요약: targeted Decision Packet은 해소, defer, reject, block 상태로 처리합니다. Affected gate 또는 reconcile item을 업데이트합니다. Approval grant/deny는 연결된 Approval 기록과 `approval_gate`를 업데이트하지만 Write Authorization을 생성하지 않습니다. Accepted scope는 `scope_gate`를 업데이트하고, 사용자 소유 제품 장단점 판단 또는 중요한 기술/아키텍처 선택 같은 사용자가 해소한 `decision_gate` 대상 판단은 `decision_gate`를 업데이트합니다. Accepted Autonomy Boundary decision은 active Change Unit의 경계를 업데이트할 수 있습니다. Verification 면제는 `verification_gate=waived_by_user`를 업데이트하고, QA 면제는 `qa_gate`를 업데이트합니다. Acceptance는 user decision을 Decision Packet에 기록하고 `acceptance_gate`를 업데이트합니다. Accepted Residual Risk는 assurance를 높이지 않고 Residual Risk record를 업데이트하며 그 참조를 반환합니다. Reconcile은 accepted state 기록을 생성할 수 있습니다.

implementation-local detail/audit를 위해 반환될 수 있는 non-stable EventRef values: `user_decision_recorded`, `decision_packet_resolved`, `decision_packet_deferred`, `decision_packet_rejected`, `approval_granted`, `approval_denied`, `scope_confirmed`, `scope_rejected`, `design_choice_recorded`, `architecture_choice_recorded`, `autonomy_boundary_decision_recorded`, `verification_waiver_recorded`, `qa_waiver_recorded`, `acceptance_recorded`, `residual_risk_accepted`, `reconcile_resolved`.

Projection job 대기열 추가: `TASK`; targeted Decision Packet이 Approval 형태이고 연결된 Approval 기록이 update될 때 `APR`; QA 면제가 QA 기록으로 represented될 때 `MANUAL-QA`; reconcile에는 affected design/task projection. Decision Packet visibility는 여전히 `TASK` projection, status/next response, judgment-context resource, decision-packet resource를 통해 나타납니다.

Standalone Decision Packet projection이 켜져 있을 때만 optional `DEC` job을 대기열에 넣습니다.

ValidatorResults emitted: `decision_quality_check`, `autonomy_boundary_check`, `residual_risk_visibility_check`.

Core checks/preconditions: `state_envelope`, `pending_decision_packet_exists`, `approval_scope`, `qa_waiver_reason`, `reconcile_target_validity`.

Possible errors: `STATE_CONFLICT`, `DECISION_UNRESOLVED`, `NO_ACTIVE_TASK`, `AUTONOMY_BOUNDARY_EXCEEDED`, `APPROVAL_DENIED`, `APPROVAL_EXPIRED`, `SCOPE_VIOLATION`, `QA_REQUIRED`, `ACCEPTANCE_REQUIRED`, `RESIDUAL_RISK_NOT_VISIBLE`, `RECONCILE_REQUIRED`, `PROJECTION_STALE`, `VALIDATOR_FAILED`, `MCP_UNAVAILABLE`.

Idempotency behavior: repeated decision은 같은 Decision Packet 해소, accepted-risk refs, updated records, events를 반환합니다. 같은 key로 이미 recorded decision을 바꾸려 하면 `STATE_CONFLICT`를 반환합니다.

### `harness.launch_verify`

Purpose: detached verification run 또는 manual evaluator bundle을 create합니다.

Allowed actor: `lead_agent`, `operator`.

Request schema:

```yaml
LaunchVerifyRequest:
  envelope: ToolEnvelope
  task_id: string
  change_unit_id: string | null
  verification_mode: fresh_session | fresh_worktree | sandbox | manual_bundle
  evaluator_surface_id: string | null
  baseline_ref: string
  include_artifacts: ArtifactRef[]
  bundle_artifact_input: ArtifactInput | null
  evaluator_focus: string[]
```

`include_artifacts`는 bundle에 포함하거나 link할 already registered evidence를 reference합니다. `bundle_artifact_input`은 optional입니다. `null`이면 Core가 verification bundle을 assemble하고 등록합니다. 값이 있으면 Core가 supplied staged bundle을 검증하고 등록합니다. `secret_omitted` entry는 ref와 omission note 또는 handle로 포함하고, `blocked` entry는 unavailable-input notice로만 포함합니다. Verification path가 replacement, waiver, Decision Packet outcome, accepted risk, 또는 다른 documented resolution을 기록하지 않는 한 이는 `EVIDENCE_INSUFFICIENT`로 이어질 수 있습니다.

Returned `bundle_ref`는 보통 `kind=bundle` 또는 `kind=manifest`를 가진 `ArtifactRef`입니다. Artifact link는 Task, launching Run, Evidence Manifest, Eval, 렌더링된 Task-scoped projection 같은 existing owner 기록을 가리켜야 하며 `verification_bundle` state 기록을 만들지 않습니다.

Response schema:

```yaml
LaunchVerifyResponse:
  base: ToolResponseBase
  evaluator_run_id: string | null
  bundle_ref: ArtifactRef
  state: StateSummary
  evaluator_instructions: string
  independence_expected:
    context: fresh_session | fresh_worktree | sandbox | manual_bundle
    write_capable: boolean
```

State transition summary: verification launch를 record하고, `verification_gate=pending`을 set 또는 keep하며, evaluator run/bundle reference를 생성합니다.

implementation-local detail/audit를 위해 반환될 수 있는 non-stable EventRef values: `verification_launched`, `verification_bundle_created`, `evaluator_run_created`.

Projection job 대기열 추가: `TASK`; optional `EVIDENCE-MANIFEST`.

ValidatorResults emitted: `surface_capability_check`.

Core checks/preconditions: `state_envelope`, `evidence_sufficiency`, `baseline_freshness`, `artifact_integrity`, `same_session_verify_guard`.

Possible errors: `STATE_CONFLICT`, `NO_ACTIVE_TASK`, `EVIDENCE_INSUFFICIENT`, `BASELINE_STALE`, `ARTIFACT_MISSING`, `CAPABILITY_INSUFFICIENT`, `MCP_UNAVAILABLE`, `VALIDATOR_FAILED`.

Idempotency behavior: repeated request는 같은 evaluator run과 bundle ref를 반환합니다. Included artifact 참조와 bundle artifact input은 original payload와 일치해야 하며, 같은 key에서 staged bundle content는 byte-identical이어야 합니다.

### `harness.record_eval`

Purpose: verification result를 record하고 independence가 valid할 때 verification gate/assurance를 update합니다.

Allowed actor: `evaluator`, `operator`.

Request schema:

```yaml
RecordEvalRequest:
  envelope: ToolEnvelope
  task_id: string
  change_unit_id: string | null
  evaluator_run_id: string | null
  target_run_id: string | null
  verdict: passed | failed | blocked | inconclusive
  checks_performed:
    - check_id: string
      result: passed | failed | skipped | blocked
      summary: string
  evidence_reviewed:
    state_refs: StateRecordRef[]
    artifact_refs: ArtifactRef[]
  independence:
    context: same_session | subagent_context | fresh_session | fresh_worktree | sandbox | manual_bundle
    write_capable: boolean
    baseline_reverified: boolean
    evaluator_surface_id: string
    parent_run_id: string | null
  blockers: string[]
  artifact_inputs: ArtifactInput[]
```

`change_unit_id`가 omitted되면 Core가 `target_run_id` 또는 evidence bundle에서 도출할 수 있습니다. 하지만 Eval이 Change Unit에 적용되는 경우 explicit `change_unit_id`를 제공하면 projection과 template alignment가 더 좋아집니다.

Eval evidence review는 artifact redaction semantics를 보존해야 합니다. `secret_omitted` artifact는 보이는 nonsecret fact에 대해서만 Eval finding을 뒷받침할 수 있습니다. `blocked` artifact는 원본 evidence가 아니라 unavailable input notice로 검토됩니다. Blocked payload에 의존하는 Eval은 valid replacement 또는 documented resolution이 생길 때까지 `blocked` 또는 `inconclusive`여야 하거나 `EVIDENCE_INSUFFICIENT`를 반환해야 합니다.

Response schema:

```yaml
RecordEvalResponse:
  base: ToolResponseBase
  eval_id: string
  state: StateSummary
  assurance_updated: boolean
  eval_ref: StateRecordRef
  registered_artifacts: ArtifactRef[]
  next_action: string
```

State transition summary: Eval을 record합니다. Passed detached verification은 `verification_gate=passed`와 `assurance_level=detached_verified`를 set할 수 있습니다. Failed 또는 blocked Eval은 gate를 failed/blocked로 옮깁니다. Same-session 또는 invalid independence는 assurance를 높일 수 없습니다.

반환될 수 있는 stable EventRef values: `eval_recorded`, `verification_passed`, `verify_not_detached_detected`.

implementation-local detail/audit를 위해 반환될 수 있는 non-stable EventRef values: `verification_failed`, `verification_blocked`, `assurance_updated`.

Projection job 대기열 추가: `TASK`, `EVAL`; optional `EVIDENCE-MANIFEST`.

ValidatorResults emitted: `surface_capability_check`.

Core checks/preconditions: `state_envelope`, `same_session_verify_guard`, `baseline_freshness`, `artifact_integrity`, `evidence_sufficiency`, `approval_scope`.

Possible errors: `STATE_CONFLICT`, `NO_ACTIVE_TASK`, `VERIFY_NOT_DETACHED`, `EVIDENCE_INSUFFICIENT`, `BASELINE_STALE`, `ARTIFACT_MISSING`, `VALIDATOR_FAILED`, `CAPABILITY_INSUFFICIENT`, `MCP_UNAVAILABLE`.

Idempotency behavior: repeated request는 같은 Eval과 assurance decision을 반환합니다. 같은 key에서 changed verdict, independence payload, artifact input이 들어오면 `STATE_CONFLICT`입니다.

### `harness.record_manual_qa`

Purpose: individual human QA outcome을 record하고 required QA가 satisfied, failed, waived될 때 `qa_gate`를 update합니다.

Allowed actor: `user`, `operator`, `evaluator`.

Request schema:

```yaml
RecordManualQaRequest:
  envelope: ToolEnvelope
  task_id: string
  change_unit_id: string | null
  qa_profile: ui_quality | workflow | copy | accessibility | browser_smoke | performance_smoke | other
  performed_by: string
  result: passed | failed | waived
  findings:
    - severity: info | warning | error | blocker
      summary: string
      path: string | null
  artifact_inputs: ArtifactInput[]
  waiver_reason: string | null
  waiver_decision_packet_ref: StateRecordRef | null
  feedback_loop_ref: StateRecordRef | null
  next_action: rework | accept | waive | block | none
```

Manual QA가 Change Unit에 적용되는 경우 `change_unit_id`를 제공해야 합니다. 단일 Change Unit에 scoped되지 않는 Task-level QA에서는 `null`일 수 있습니다.

`RecordManualQaRequest.result`는 실제 Manual QA record의 record-level result이며 `passed`, `failed`, `waived`로 제한됩니다. Pending required QA는 `RecordManualQaRequest.result=pending`이 아니라 aggregate `qa_gate=pending`으로 표현합니다.

`result=waived`에서 product/user risk 또는 policy-required judgment가 있으면 `waiver_decision_packet_ref`가 reference하는 `qa_waiver` Decision Packet이 필요합니다. `waiver_reason`만으로 가능한 경우는 policy가 허용한 low-risk waiver에 한정됩니다.

Manual QA가 selected Feedback Loop인 경우 `feedback_loop_ref`는 `record_kind=feedback_loop`인 기준 `feedback_loops` row를 reference해야 합니다. Core는 Manual QA row를 record하고, resulting Manual QA 참조와 registered artifact를 그 Feedback Loop에 추가하며, QA result에 따라 status를 `executed`, `blocked`, 또는 `waived`로 업데이트합니다. 이 link는 execution evidence만 업데이트하며 selected-loop definition을 생성하지 않습니다.

Manual QA artifact ref도 다른 evidence와 같은 이후 규칙을 따릅니다. `secret_omitted` QA artifact는 생략된 value를 증명하지 않고 보이는 workflow 또는 UI finding을 뒷받침할 수 있습니다. `blocked` QA capture artifact는 screenshot, log, trace, recording input을 사용할 수 없다는 표시입니다. Replacement capture, waiver, Decision Packet outcome, accepted risk, 또는 documented fallback이 QA path를 해소하지 않는 한 QA record 또는 aggregate `qa_gate`는 blocked, failed, pending, waived, 또는 unresolved impact를 보여야 합니다.

Response schema:

```yaml
RecordManualQaResponse:
  base: ToolResponseBase
  manual_qa_record_id: string
  state: StateSummary
  manual_qa_ref: StateRecordRef
  updated_feedback_loop_refs: StateRecordRef[]
  registered_artifacts: ArtifactRef[]
  next_action: string
```

State transition summary: Manual QA를 record합니다. `passed`는 `qa_gate=passed`를 set할 수 있습니다. `failed`는 `qa_gate=failed`를 set하고 rework/blocked로 route합니다. `waived`는 compatible `qa_waiver` Decision Packet 또는 policy-permitted low-risk waiver reason을 요구하고 `qa_gate=waived`를 set합니다. Required QA가 충족 기록을 아직 만들지 못했거나 latest relevant 기록이 policy를 충족하지 못하면 aggregate gate는 `qa_gate=pending`으로 남습니다.

implementation-local detail/audit를 위해 반환될 수 있는 non-stable EventRef values: `manual_qa_recorded`, `qa_passed`, `qa_failed`, `qa_waived`, `artifact_registered`, `feedback_loop_updated`.

Projection job 대기열 추가: `TASK`, `MANUAL-QA`; optional `EVIDENCE-MANIFEST`. Waiver Decision Packet visibility는 여전히 `TASK` projection, status/next response, judgment-context resource, decision-packet resource를 통해 나타납니다.

Standalone Decision Packet projection이 켜져 있고 waiver Decision Packet이 visibility에 영향을 줄 때만 optional `DEC` job을 대기열에 넣습니다.

ValidatorResults emitted: `manual_qa_required`, `decision_quality_check`, `residual_risk_visibility_check`.

Core checks/preconditions: `state_envelope`, `qa_waiver_reason`, `artifact_integrity`, `evidence_sufficiency`.

Possible errors: `STATE_CONFLICT`, `DECISION_REQUIRED`, `DECISION_UNRESOLVED`, `NO_ACTIVE_TASK`, `QA_REQUIRED`, `RESIDUAL_RISK_NOT_VISIBLE`, `ARTIFACT_MISSING`, `EVIDENCE_INSUFFICIENT`, `VALIDATOR_FAILED`, `MCP_UNAVAILABLE`.

Idempotency behavior: repeated request는 같은 Manual QA 기록과 gate update를 반환합니다. Waiver reason과 artifact input은 일치해야 합니다.

### `harness.close_task`

Purpose: Core가 모든 close-relevant gates를 check한 뒤 Task를 close, cancel, supersede합니다.

Allowed actor: `user`, `lead_agent`, `operator`.

Request schema:

```yaml
CloseTaskRequest:
  envelope: ToolEnvelope
  task_id: string
  intent: complete | cancel | supersede
  requested_close_reason: completed_verified | completed_self_checked | completed_with_risk_accepted | cancelled | superseded
  user_note: string | null
  superseded_by_task_id: string | null
```

`CloseTaskRequest`는 accepted-risk refs를 전달하지 않습니다. `completed_with_risk_accepted`에서는 Core가 close-relevant Residual Risk records에 이미 기록된 수용 상태를 읽으며, visible accepted residual-risk 상태가 없으면 block합니다.

Response schema:

```yaml
CloseTaskResponse:
  base: ToolResponseBase
  closed: boolean
  state: StateSummary
  blockers:
    - code: ErrorCode
      message: string
      required_next_action: string
      related_refs: StateRecordRef[]
  final_report_refs: StateRecordRef[]
  artifact_refs: ArtifactRef[]
```

Close blockers에는 해소되지 않았거나 missing, deferred-without-coverage, blocked, rejected, `stale`, incompatible 상태인 blocking Decision Packets와, successful close 전에 표시되지 않은 known close-relevant residual risk가 포함됩니다. Known close-relevant residual risk가 없으면 `ResidualRiskSummary.status=none`이 residual-risk visibility를 충족하며 close blocker가 아닙니다. Risk-accepted close에는 표시되고 수용된 Residual Risk refs가 추가로 필요합니다. Acceptance가 required인 경우 close-relevant residual risk가 표시되거나 `ResidualRiskSummary.status=none`으로 confirmed된 뒤에만 acceptance를 record할 수 있습니다.

State transition summary: successful completion은 Task를 result와 close reason이 있는 `completed`로 옮깁니다. Cancellation/supersession은 Task를 `cancelled`로 옮깁니다. Failed close는 Task를 non-terminal로 남기고 blockers를 보고합니다.

반환될 수 있는 stable EventRef values: `close_requested`, `task_closed`, `task_cancelled`, `task_superseded`, `risk_accepted_close_recorded`, `close_blocked`.

Projection job 대기열 추가: `TASK`; final freshness에 필요한 latest required 보고서.

ValidatorResults emitted: `decision_gate_check`, `decision_quality_check`, `autonomy_boundary_check`, `feedback_loop_check`, `tdd_trace_required`, `codebase_stewardship_check`, `manual_qa_required`, `residual_risk_visibility_check`, `context_hygiene_check` when projection or context hygiene must be emitted as a ValidatorResult.

Core checks/preconditions: `state_envelope`, `active_run_absent`, `active_change_unit_complete`, `scope_coverage`, `approval_scope`, `design_gate_close`, `evidence_sufficiency`, `same_session_verify_guard`, `acceptance_required`, `projection_freshness`.

Possible errors: `STATE_CONFLICT`, `DECISION_REQUIRED`, `DECISION_UNRESOLVED`, `NO_ACTIVE_TASK`, `NO_ACTIVE_CHANGE_UNIT`, `SCOPE_REQUIRED`, `SCOPE_VIOLATION`, `AUTONOMY_BOUNDARY_EXCEEDED`, `APPROVAL_REQUIRED`, `APPROVAL_DENIED`, `APPROVAL_EXPIRED`, `EVIDENCE_INSUFFICIENT`, `VERIFY_NOT_DETACHED`, `QA_REQUIRED`, `ACCEPTANCE_REQUIRED`, `RESIDUAL_RISK_NOT_VISIBLE`, `PROJECTION_STALE`, `RECONCILE_REQUIRED`, `ARTIFACT_MISSING`, `BASELINE_STALE`, `VALIDATOR_FAILED`, `MCP_UNAVAILABLE`.

Idempotency behavior: repeated successful close는 같은 terminal state와 보고서 refs를 반환합니다. 다른 intent 또는 close reason으로 두 번째 close를 시도하면 `STATE_CONFLICT`입니다.
