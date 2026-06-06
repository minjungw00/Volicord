# Storage

## 이 문서가 담당하는 것

이 문서는 향후 로컬 하네스 서버를 위한 참조 문서입니다. 이 저장소에는 아직
database, migration runner, server, runtime state, generated artifact, generated
projection이 없습니다. 현재 저장소 단계와 인계 상태는 [MVP 계획](../build/mvp-plan.md#문서-수락-상태)에서
확인합니다.

이 문서는 활성 첫 구현 조각의 초기 storage design을 담당합니다. 범위는 Runtime
Home identity, project-local persisted record, artifact storage metadata, artifact
link, 최소 evidence coverage record, storage-owned JSON `TEXT` 규칙, storage-owned
enum hardening, active persistence와 later/profile storage의 경계입니다.

Lifecycle 의미, gate 의미, public API payload, 사용자에게 보이는 decision은 각 담당
문서에 남습니다. 이 문서는 그 계약을 구현하는 데 필요한 record와 필수 persisted
field를 이름 붙입니다. Core 또는 API state machine을 다시 정의하지 않습니다.

## 이런 때 읽기

- 첫 executable authority loop와 MVP-1 사용자 작업 루프에 필요한 가장 작은
  저장 범위를 확인할 때.
- Core-owned state와 chat, Markdown projection, connector output, tool output,
  report text를 분리할 때.
- Write Authorization, evidence linkage, blocker, status, close readiness에 어떤
  persisted record가 필요한지 확인할 때.
- later/profile table이 MVP 선결 조건처럼 보이지 않게 점검할 때.

## 관련 Owner

| 관심사 | Owner |
|---|---|
| Public MCP request/response shape | [MVP API](api/mvp-api.md)와 [API Schema Core](api/schema-core.md) |
| 활성 MVP method row mutation, dry-run/failure side effect, response ref | [MVP API: 활성 MVP 전이 매트릭스](api/mvp-api.md#활성-mvp-전이-매트릭스) |
| `ArtifactRef`, staged active ref kind, idempotency, state conflict behavior | [API Schema Core](api/schema-core.md#artifactref), [API Schema Core: Stage-Specific Active Value Sets](api/schema-core.md#stage-specific-active-value-sets), [API Errors](api/errors.md) |
| Task lifecycle, gate, `prepare_write`, Write Authorization, `record_run`, `close_task`, stable event | [Core Model 참조](core-model.md) |
| Core process model, transaction order, lock, projection/reconcile placement | [런타임 아키텍처 참조](runtime-architecture.md) |
| Projection authority, freshness, managed block, rendered template | [Projection과 Template 참조](projection-and-templates.md)와 [Template 참조](templates/README.md) |
| Operator behavior, doctor/recover/export/reconcile/conformance entrypoint | [운영과 Conformance 참조](operations-and-conformance.md) |
| Fixture format과 assertion semantics | [Conformance Fixtures 참조](conformance-fixtures.md) |
| Stage sequence와 implementation readiness | [Build: MVP-1 사용자 작업 루프](../build/mvp-plan.md#user-work-loop), [내부 엔지니어링 점검](../build/mvp-plan.md#first-internal-smoke-target), [MVP 계획](../build/mvp-plan.md) |

## 활성 첫 구현 저장 범위

활성 첫 구현 저장 범위는 첫 executable authority loop와 MVP-1 사용자 작업 루프에
필요한 가장 작은 지속 기록 묶음입니다. 이는 초기 schema design입니다. 수락된
migration plan도 아니고 runtime data가 존재한다는 증거도 아닙니다.

활성 persisted record는 다음뿐입니다.

- `project_state`
- `surfaces`, 또는 같은 역할을 하는 reference surface registration record
- `tasks`
- `task_events`
- `change_units`
- `user_judgments`
- `write_authorizations`
- `runs`
- `artifacts`
- `artifact_links`
- `evidence_summaries`, 또는 같은 역할의 minimal evidence coverage record
- `blockers`
- `tool_invocations`

다른 persisted table family는 MVP 선결 조건이 아닙니다. 향후 구현은 이 record를
위해 physical index, lookup table, JSON 분해를 선택할 수 있습니다. 하지만 그런 선택이
later/profile record를 활성 범위로 끌어오면 안 됩니다.

활성 저장 범위는 아래 authority boundary를 지킵니다.

- Core-owned state row가 현재 Harness state의 기준입니다.
- `task_events`를 유지하더라도 audit와 ordering trail입니다. 일반 동작에서 current
  state를 재구성하는 기준 source가 아닙니다.
- `tool_invocations`는 committed idempotency replay를 지원합니다. 별도 user-facing
  domain record가 아닙니다.
- 요구사항 구체화 출력은 필요에 따라 active `tasks`, `change_units`,
  `user_judgments`, `evidence_summaries`, `blockers`에 저장합니다. MVP-1에는
  committed `shared_designs` table, Discovery Brief table, Question Queue table,
  Assumption Register table, First Safe Change Unit Candidate table, required
  Shared Design projection cache가 없습니다.
- `artifacts`는 integrity, redaction, producer, retention, availability fact를 가진
  등록된 증거 바이트 또는 안전한 메타데이터를 저장합니다. Core가 owner-valid row로
  연결하기 전까지 sufficiency를 증명하지 않습니다.
- `artifact_links`는 artifact를 owner record에 연결합니다. Link 자체는 report,
  projection, QA result, Eval, final acceptance, residual-risk acceptance가 아닙니다.
- `evidence_summaries`는 MVP-1을 위한 최소 coverage와 gap record입니다. Full Evidence
  Manifest report table이 아닙니다.
- Raw secret, token, full sensitive log는 valid 증거 바이트가 아닙니다. 대신 redacted
  byte, `secret_omitted` / `blocked` notice, safe handle, 또는 owner가 허용한 safe
  representation을 저장합니다.
- Chat, Markdown projection, generated report, connector manifest, tool output,
  operator output은 Core mutation이 owner-valid state row, artifact, artifact link를
  기록하기 전까지 권한 근거가 아닙니다.
- Status card, close result, next action, run/evidence summary, compact view는 파생
  output입니다. Stale, failed, absent이거나 다시 계산되어도 persisted authority
  record를 바꾸지 않습니다.
- Future/profile table은 owning profile 또는 tool path가 active이거나 사용될 때만
  required가 됩니다.

## Runtime Home Identity와 위험

Harness는 local Runtime Home 하나와 registered project별 project-local state database
하나를 둡니다. 기본 reference location은 `~/.harness`이지만 구현은 configured
equivalent를 선택할 수 있습니다.

기준 layout:

```text
~/.harness/
  registry.sqlite
  projects/
    PRJ-0001/
      project.yaml
      state.sqlite
      artifacts/
        tmp/
        diffs/
        logs/
        screenshots/
        checkpoints/
```

`registry.sqlite`는 Runtime Home identity와 minimal project registration metadata를
저장합니다. `project.yaml`은 static project configuration만 저장합니다.
`state.sqlite`는 project-local Core state를 저장합니다. Artifact directory는 Core가
artifact registration boundary를 적용한 뒤 등록된 파일 또는 안전한 메타데이터를
보관합니다.

`project.yaml`은 current Task state, current gate, Write Authorization state, evidence
sufficiency, final acceptance, residual-risk acceptance를 저장하면 안 됩니다.

```yaml
project_id: PRJ-0001
display_name: my-app
repo_root: /abs/path/to/my-app
default_agent_surface: reference

default_checks:
  lint: []
  test: []
  build: []
```

Runtime Home identity는 path에만 의존하면 안 됩니다. 복사되거나 이동된 Runtime Home은
같은 stored `runtime_home_id`를 유지할 수 있고, 새 Runtime Home은 새 id를 가져야
합니다. `doctor`와 recovery flow는 이 identity를 사용해 의심스러운 copy, duplicate
registration, path drift를 보고할 수 있습니다. 다만 이 id가 tamper-proofing을
제공하지는 않습니다.

Runtime Home은 local operational authority와 민감한 support data를 담습니다. Broad
write access는 tampering과 evidence poisoning risk입니다. Broad read access는 secret,
PII, token, log, screenshot, diff, verification bundle, export를 노출할 수 있습니다.

내부 엔지니어링 점검과 MVP-1 storage는 다른 profile이 더 강한 control을 증명하기 전까지
cooperative/detective입니다. File 승인, owner check, hash, `doctor` finding은
방어적 보강입니다. 그 자체로 OS-level sandboxing, arbitrary-tool control,
tamper-proof storage, pre-execution blocking을 만들지 않습니다.

| 관찰 사항 | Storage 의미 |
|---|---|
| Runtime Home 또는 project storage의 owner/mode를 확인할 수 없습니다. | Unknown 또는 weak local file posture를 보고합니다. OS-level guarantee를 주장하지 않습니다. |
| Runtime Home, `state.sqlite`, `registry.sqlite`, artifact directory가 unrelated user, shared group, shared container, broad local process에게 writable입니다. | Tampering과 evidence-poisoning risk를 보고합니다. Core는 row, owner link, hash, artifact registration을 검증한 뒤에만 의미를 신뢰해야 합니다. |
| Artifact storage 또는 export가 unrelated user, shared group, shared container, broad local process에게 readable입니다. | 민감한 값을 그대로 출력하지 않고 confidentiality risk를 보고합니다. |
| Registered artifact `sha256`, `size_bytes`, `content_type`, `redaction_state`, owner link, resolved storage location이 storage metadata와 더 이상 맞지 않습니다. | Projection drift가 아니라 evidence integrity failure 또는 recovery input으로 취급합니다. Missing bytes나 `hash_mismatch` 같은 diagnostic은 related evidence를 stale 또는 blocked로 만듭니다. |

## 활성 기록 계약

아래 표는 활성 범위의 contract-level persisted field를 정리합니다. 정확한 lifecycle
의미와 API response 의미는 [Core Model 참조](core-model.md), [MVP API](api/mvp-api.md),
[API Schema Core](api/schema-core.md)가 담당합니다.
각 API call이 어떤 row를 만들거나 업데이트하는지에 대한 method별 index는
[MVP API: 활성 MVP 전이 매트릭스](api/mvp-api.md#활성-mvp-전이-매트릭스)에 있습니다.

| Record | 최소 persisted 역할 | 필수 field |
|---|---|---|
| `project_state` | Project-local state header, state clock, active Task pointer, active/default surface pointer. | `project_id`, `schema_version`, `storage_profile`, `state_version`, `active_task_id`, `default_surface_id`, `created_at`, `updated_at`. |
| `surfaces` | Local caller/display path를 위한 reference surface registration입니다. Core가 어떤 surface와 대화한다고 보는지를 기록합니다. Broad connector ecosystem table이 아닙니다. | `surface_id`, `project_id`, `surface_kind`, `display_name`, `registration_source`, `local_access_posture`, `capability_profile_json`, `guarantee_level`, `status`, `created_at`, `updated_at`. |
| `tasks` | User-value work unit, task-scoped state clock, active requirements-shaping summary. | `task_id`, `project_id`, `title`, `user_request`, `current_goal_summary`, `mode`, `lifecycle_phase`, `result`, `summary`, `success_criteria_json`, `non_goals_json`, `affected_areas_json`, `affected_path_candidates_json`, `constraints_json`, `confirmed_facts_json`, `remaining_uncertainties_json`, `blocking_question`, `next_safe_action`, `active_change_unit_id`, `state_version`, `created_at`, `updated_at`, `closed_at`. |
| `task_events` | Committed Core mutation의 append-only audit/order trail. | `event_id`, `task_id` 또는 project scope, `event_seq`, `event_type`, `state_version`, `actor_kind`, `surface_id`, `payload_json`, `created_at`. |
| `change_units` | Product write와 close basis를 위한 proposed 또는 current scoped work boundary. | `change_unit_id`, `task_id`, `scope_summary`, `affected_areas_json`, `affected_path_candidates_json`, `non_goals_json`, `success_criteria_json`, `allowed_paths_json`, `denied_paths_json`, `sensitive_categories_json`, `baseline_ref`, `autonomy_boundary_json`, `status`, `created_at`, `updated_at`. |
| `user_judgments` | Product decision, technical decision, scope decision, sensitive approval, QA waiver, verification-risk acceptance, final acceptance, residual-risk acceptance, cancellation을 위한 사용자 소유 판단 기록. | `user_judgment_id`, `task_id`, `change_unit_id`, `judgment_kind`, `presentation`, `status`, `state_summary_at_request_json`, `question`, `judgment_context_json`, `boundary_text_json`, `options_json`, `recommendation_json`, `selected_option_json`, `judgment_payload_json`, `affected_scope_json`, `affected_gates_json`, `affected_acceptance_criteria_json`, `context_refs_json`, `artifact_refs_json`, `resolution_json`, `expires_at`, `resolved_at`, `created_at`, `updated_at`. |
| `write_authorizations` | `dry_run=false`인 `prepare_write.decision=allowed`일 때만 생기는 durable single-use cooperative record입니다. Row는 Core 비교에 쓰는 full active MVP `AuthorizedAttemptScope`를 보존합니다. | `write_authorization_id`, `task_id`, `change_unit_id`, `surface_id`, `status`, `basis_state_version`, `attempt_scope_json`, `consumed_by_run_id`, `expires_at`, `created_at`, `updated_at`, `consumed_at`. |
| `runs` | Product write가 있었다면 compatible write consumption까지 포함하는 committed execution 또는 observation record. | `run_id`, `task_id`, `change_unit_id`, `write_authorization_id`, `surface_id`, `kind`, `status`, `product_write`, `baseline_ref`, `summary`, `observed_attempt_json`, `observed_changes_json`, `command_results_json`, `tool_invocations_json`, `network_accesses_json`, `secret_accesses_json`, `evidence_updates_json`, `observation_capability_json`, `created_at`. |
| `artifacts` | Integrity와 redaction fact를 가진 registered durable 증거 바이트 또는 안전한 메타데이터. | `artifact_id`, `project_id`, `task_id`, `run_id`, `kind`, `uri`, `sha256`, `size_bytes`, `content_type`, `redaction_state`, `retention_class`, `produced_by`, `status`, `created_at`, `updated_at`. |
| `artifact_links` | Artifact가 지원하는 Core/API owner record로 가는 owner relation. | `artifact_link_id`, `artifact_id`, `task_id`, `owner_record_kind`, `owner_record_id`, `relation`, `created_at`. |
| `evidence_summaries` | MVP-1 status와 close에 필요한 최소 증거 coverage와 gap record입니다. 활성 범위에서는 full Evidence Manifest table을 대체합니다. | `evidence_summary_id`, `task_id`, `change_unit_id`, `status`, `coverage_items_json`, `summary`, `supporting_run_ids_json`, `supporting_artifact_link_ids_json`, `gap_blocker_ids_json`, `updated_at`. |
| `blockers` | Next action, write compatibility, evidence gap, close readiness, recovery를 위한 structured blocker. | `blocker_id`, `task_id`, `blocked_action`, `blocker_kind`, `status`, `message`, `owner_ref_json`, `related_refs_json`, `required_next_action`, `created_at`, `resolved_at`. |
| `tool_invocations` | `dry_run=false`인 state-changing tool response의 committed idempotency replay row. | `invocation_id`, `project_id`, `tool_name`, `idempotency_key`, `request_hash`, `task_id`, `basis_state_version`, `response_json`, `status`, `created_at`. |

`tool_invocations` row는 committed replayable `dry_run=false` response에 대해서만 존재합니다.
Dry run과 pre-commit conflict는 storage에서 `idempotency_key`를 예약하지 않습니다.

`tasks.user_request`는 사용자의 원래 표현을 저장합니다. Shaping update는 그 원문을
바꾸지 않고 current goal, success criteria, non-goals, affected areas, path
candidates, constraints, confirmed facts, remaining uncertainties, blocking question,
next safe action을 구체화합니다. `tasks.constraints_json`은 활성
`TaskShapingUpdate.constraints` 내용을 보존합니다. 현재는 allowed paths와 sensitive
categories입니다. Blocking question이 user-owned이면 `UserJudgmentCandidate`가 되고,
요청/기록되면 `user_judgments` row가 됩니다. User judgment가 아닌 blocker는 active
`blockers` path를 사용합니다.

`change_units.status`는 Core/API owner rule에 따라 proposed candidate 또는
active/superseded scope를 표현할 수 있습니다. First Safe Change Unit Candidate는 이
record family가 담는 proposed Change Unit boundary이지 별도 active table이나 ref kind가
아닙니다. `change_units`는 활성 `ChangeUnitShapingUpdate` scope content를 저장합니다.
여기에는 affected areas와 path candidates, allowed/denied paths, non-goals, success
criteria, sensitive categories, `baseline_ref`, compact `autonomy_boundary_json`이
포함됩니다.

`user_judgments.judgment_kind`가 저장되는 판단 identity입니다. Display label은 read/render 시
`judgment_kind`와 locale에서 파생합니다. Active storage는 canonical `display_label` column을
두지 않으며, 표시 text를 compatibility check, validator, gate, close aggregation, owner ref에
사용하지 않습니다.

`write_authorizations.attempt_scope_json`은 [API Schema Core](api/schema-core.md#evidence-and-pre-write-scope-schemas)의
`AuthorizedAttemptScope`를 storage에 serialized한 값입니다. 이 값은 intended operation,
intended paths, intended tools, intended commands와 command classes,
product-file-write intent, intended network targets, intended secret handles/scope,
sensitive categories, `baseline_ref`, `task_id`, `change_unit_id`,
`basis_state_version`, `surface_id`, related user judgment refs, `guarantee_level`을
보존해야 합니다. Top-level `task_id`, `change_unit_id`, `surface_id`,
`basis_state_version` column은 query/index field입니다. Core comparison은 stored
attempt scope를 authoritative authorization boundary로 사용합니다.

`runs.observed_attempt_json`은 `record_run` compatibility comparison을 위한 normalized
storage bundle입니다. Product-write flag, baseline, observed changed paths, command와
command-class observation, tool use, network observation, secret-access observation,
관찰된 경우의 sensitive categories, Task/Change Unit/surface context, comparison outcome을
보존합니다. 더 구체적인 JSON column은 active `RecordRunPayload` branch를 evidence와 read
response에서 사용할 수 있게 남깁니다. `observation_capability_json`은 active surface가
정직하게 관찰하거나 attest할 수 없었던 field를 기록합니다. Unsupported 또는 absent
observation은 verified success가 아니라 unsupported/unknown으로 저장합니다. 필요한 비교
fact가 unsupported이면 Core는 authorization을 fully compatible하게 소비하지 말고 claim을
좁히거나, block하거나, 명시적으로 지원되는 경우 violation/audit path를 기록하거나,
insufficient surface capability를 반환/보고해야 합니다.

State clock은 global clock이 아니라 scope별 clock입니다. Task-scoped mutation은
`tasks.state_version`을 사용합니다. Core-resolved primary Task가 없는 project-scoped
mutation은 `project_state.state_version`을 사용합니다.
`tool_invocations.basis_state_version`은 committed mutation 전에 compatibility basis로
사용한 affected-scope version을 저장합니다. `response_json`은 resulting
`ToolResponseBase.state_version`을 포함한 original response를 저장합니다.

## Artifact와 Evidence 경계

MVP-1 storage는 evidence linkage를 위해 작은 record 세 가지를 사용합니다.

| Record | Active responsibility | 담당하지 않는 것 |
|---|---|---|
| `artifacts` | Durable byte 또는 안전한 메타데이터와 integrity fact를 등록합니다. | Evidence sufficiency, QA, Eval, final acceptance, residual-risk acceptance, export bundle, report prose. |
| `artifact_links` | Artifact를 Task, Change Unit, Run, user judgment, blocker, minimal evidence summary에 연결합니다. | Owner record 생성, gate 자체 충족, report rendering. |
| `evidence_summaries` | Status, run/evidence summary, close에 필요한 최소 coverage/gap result를 저장합니다. | Full criteria matrix, Evidence Manifest report table, detached verification, Manual QA matrix, long-term analytics. |

Evidence에 사용할 수 있는 artifact 계약은 `artifacts`와 `artifact_links`를 합친 것입니다.
두 record가 함께 `artifact_id`, Task 또는 equivalent owner scope, kind, `uri`, `sha256`,
`size_bytes`, `content_type`, `redaction_state`, `produced_by`, relation owner,
`retention_class`를 제공해야 합니다. Critical 또는 close-relevant evidence에 required
metadata, owner link, availability fact, integrity match 중 하나라도 빠지면
`sufficient`가 될 수 없습니다. Core는 affected coverage를 `stale` 또는 `blocked`로
표시하고, required evidence가 affected이면 close를 계속 blocked로 둡니다.

구현은 같은 역할과 owner link를 보존하는 경우에만 minimal coverage record 이름을 다르게
정할 수 있습니다. 활성 범위는 `evidence_summaries` 또는 같은 역할의 minimal coverage
record로 충분할 때 full Evidence Manifest storage를 요구하면 안 됩니다.

## 지속 상태와 파생 상태 보기

Persisted active state는 Core가 commit한 row set입니다. Derived status/view는 Core 또는
renderer가 그 row에서 계산해 사용자, agent, operator에게 보여주는 값입니다.

| 파생 output | Source records | Active storage rule |
|---|---|---|
| Status card / task summary | `project_state`, `surfaces`, `tasks`, `change_units`, `user_judgments`, `write_authorizations`, `runs`, `evidence_summaries`, `blockers` | 파생된 상태 보기입니다. Read 시 다시 계산할 수 있습니다. `projection_status_cards`나 `projection_jobs` table은 필요하지 않습니다. |
| Next safe actions | Open blockers, pending user judgments, write-check state, evidence summaries, Task lifecycle | 파생된 상태 보기입니다. Task, judgment, Run, evidence summary, artifact, Write Authorization을 만들지 않습니다. |
| Run/evidence summary | `runs`, `artifacts`, `artifact_links`, `evidence_summaries`, `blockers` | Active record에서 파생한 보기입니다. Full Evidence Manifest나 report projection이 아닙니다. |
| Close readiness | Task lifecycle, scope state, pending user judgments, evidence coverage, artifact availability, open blockers, final-acceptance와 residual-risk user judgment | 파생 check입니다. Active storage는 check에 쓰는 owner record와 blocker를 저장하며 별도 `close_readiness` 기준 source를 만들지 않습니다. |
| Projection freshness | Current state version과 read/view response가 반환한 source version 비교 | 파생 diagnostic입니다. Full `projection_jobs` storage는 운영 프로필 또는 profile-promoted storage입니다. |

## Close-Blocker 계산에 필요한 필드

MVP-1 close-blocker calculation은 current persisted record를 읽고 close result를
파생합니다. Journey, Spine, full Evidence Manifest report table, Eval, full Manual QA
matrix, export/recover table, projection job, broad validator-run archive, long-term
metrics, connector ecosystem table이 필요하지 않습니다.

| Blocker 또는 close fact | Minimum source fields |
|---|---|
| Active Task exists and is closeable | `project_state.active_task_id`, `tasks.lifecycle_phase`, `tasks.result`, `tasks.closed_at` |
| Scope is present and current | `tasks.active_change_unit_id`, `change_units.status`, `change_units.scope_summary`, `change_units.non_goals_json`, `change_units.success_criteria_json` |
| User-owned judgment is unresolved | `user_judgments.judgment_kind`, `user_judgments.status`, `user_judgments.affected_scope_json`, `user_judgments.context_refs_json` |
| 민감 동작 승인이 없거나 거부됨 | `judgment_kind=sensitive_approval`인 `user_judgments` row와 write가 관련될 때 current `write_authorizations.attempt_scope_json.related_user_judgment_refs` |
| Write Authorization is missing, expired, stale, revoked, consumed, or incompatible | `write_authorizations.status`, `write_authorizations.basis_state_version`, `write_authorizations.attempt_scope_json`, `write_authorizations.consumed_by_run_id`, current `tasks.state_version`, current `tasks.active_change_unit_id`, current surface/profile facts |
| Run 또는 artifact support가 missing 또는 stale입니다 | `runs.status`, `runs.product_write`, `runs.baseline_ref`, `runs.observed_attempt_json`, `runs.observation_capability_json`, `runs.observed_changes_json`, `runs.command_results_json`, `runs.tool_invocations_json`, `runs.network_accesses_json`, `runs.secret_accesses_json`, `artifacts.status`, `artifacts.sha256`, `artifacts.size_bytes`, `artifacts.content_type`, `artifacts.redaction_state`, `artifact_links.owner_record_kind`, `artifact_links.owner_record_id` |
| Evidence coverage가 missing, insufficient, stale 중 하나입니다 | `evidence_summaries.status`, `evidence_summaries.coverage_items_json`, `evidence_summaries.supporting_artifact_link_ids_json`, `evidence_summaries.gap_blocker_ids_json` |
| Final acceptance is required but missing | `judgment_kind=final_acceptance`인 `user_judgments` row와 compatible `status` / `selected_option_json` |
| Residual risk is not visible or not accepted | Residual-risk blocker kind를 가진 `blockers` row와, acceptance가 required일 때 `judgment_kind=residual_risk_acceptance`인 `user_judgments` row |
| A blocker is still open | `blockers.status`, `blockers.blocker_kind`, `blockers.blocked_action`, `blockers.related_refs_json`, `blockers.required_next_action` |
| Readable status is stale | Current `tasks.state_version`과 read/card response가 반환한 source version 비교. Later `projection_jobs`는 운영 프로필이 active일 때만 사용합니다. |

Close response는 compact close-readiness summary, evidence summary, next action을
보여줄 수 있습니다. 이것들은 active record에서 파생한 output입니다.
`close_readiness`, status-card cache, projection cache, full report table을 저장하는 일은
owner profile이 승격하기 전까지 optional/later입니다.

MVP-1에서 `evidence_summaries.status`는 정확히 `not_required`,
`none`, `partial`, `sufficient`, `stale`, `blocked`를 사용합니다.
`coverage_items_json`이 있으면 각 item의 `coverage_state`는 정확히
`supported`, `unsupported`, `partial`, `not_applicable`, `stale`, `blocked`를
사용합니다. Evidence가 close에 required일 때 close를 만족할 수 있는 evidence state는
`status=sufficient`뿐입니다. Full Evidence Manifest row, detached Eval row,
Manual QA matrix는 해당 owner profile이 active일 때가 아니면 이 active storage slice에
필요하지 않습니다.
Close-required coverage item이 missing artifact, 없는 `sha256` / `size_bytes` /
`content_type` / `redaction_state` metadata, unresolved relation owner, unavailable
bytes, 또는 `hash_mismatch` 같은 integrity failure에 의존하면 storage는 그 사실을 Core에
노출합니다. Evidence state는 `sufficient`가 아니라 `stale` 또는 `blocked`로 남습니다.

Write Authorization row는 close-readiness row가 아닙니다. Stale, missing, expired,
revoked, consumed, incompatible authorization이나 authorization row를 만들지 않은
blocked `prepare_write` decision은 그 영향이 닿는 현재 Run, scope, artifact,
evidence summary, blocker record를 통해서만 close에 영향을 줍니다. Storage는
authorization lifecycle value나 차단된 write-check response를 close result로 바꾸면 안
되며, attempted invalid authorization ref를 evidence support로 쓰면 안 됩니다.

## Later/Profile 저장

Later/profile storage는 유용한 design inventory입니다. MVP DDL bundle이나 첫 구현
선결 조건으로 읽으면 안 됩니다.

| Later/profile table family | 나중에 필요한 이유 | Active-slice replacement |
|---|---|---|
| Full Eval system, including `evals` and evaluator bundles | Detached verification과 independence hardening | Run, artifact, artifact link, evidence summary, blocker. Owner profile이 active가 아니면 detached assurance claim은 하지 않습니다. |
| Full Manual QA matrix, including `manual_qa_records` | Human inspection workflow, finding, setup, QA evidence ref | 활성 사용자 소유 QA 면제/위험 질문은 `user_judgments`, `blockers`, `evidence_summaries`로 표시합니다. 수동 QA 통과 기록, 세부 매트릭스, close blocker는 owner profile이 active일 때만 사용합니다. |
| Full Evidence Manifest report tables, including detailed `evidence_manifests` | Criteria-to-evidence matrix와 rich report | `evidence_summaries` 또는 같은 역할의 minimal evidence coverage와 artifact link |
| Shared Design/design-support records, including `shared_designs` and full design artifacts | 풍부한 요구사항/설계 history와 later-profile design review | Active Task shaping field, proposed 또는 active Change Unit, user judgment 후보/기록, blocker, 필요한 경우 evidence summary |
| Projection job system, including `projection_jobs` and durable projection caches | Rendered Markdown 또는 managed output을 위한 durable outbox | Read-time compact view와 source-version freshness display |
| Export/recover tables, including `export_manifests`, `recover_items`, release-handoff bundles | Operations, handoff, recovery, export package | 운영 프로필이 active가 아니면 active artifact와 blocker만 사용합니다. |
| Broad validator run archive, including `validator_runs` | Persisted validator history와 diagnostic trend analysis | Current blocker, API `ValidatorResult` response data, owner-field validation |
| Long-term metrics tables | Trend analysis, latency, turnaround, operational diagnostic | Active storage에는 없음. Status는 current record에서 파생합니다. |
| Connector ecosystem tables, including connector manifests, marketplace records, connector analytics, remote-surface inventories | Broad connector operation과 marketplace behavior | Local active surface를 위한 `surfaces` 또는 equivalent reference-surface registration만 사용합니다. |

아래 later/profile candidate도 owner가 승격하기 전까지 활성 범위 밖에 남습니다.

- 더 풍부한 Approval lifecycle을 위한 committed `approvals` table
- blocker와 user judgment에서 분리된 rich residual-risk lifecycle을 위한 `residual_risks`
  table
- Active compatibility basis를 넘어서는 repository baseline capture용 `baselines`
- Human edit 또는 projection drift를 라우팅하는 `reconcile_items`
- Process/project lock을 넘어서는 durable lock/recovery metadata용 `persistent_locks`
- `task_spine_entries`, `journey_cards` 같은 Journey/spine continuity record
- `domain_terms`, `module_map_items`, `interface_contracts`, `change_unit_dependencies`
  같은 design/stewardship record

## Event와 Idempotency 의미

`task_events`는 구현이 유지할 때 append-only audit trail과 event-order support table입니다.
Core가 무엇을 어떤 순서로 commit했는지 기록합니다. 일반 동작에서 current state의
authority source가 아니며, active state는 보통 event replay로 재구성하지 않습니다.

Current state table이 authoritative합니다.

- `project_state`, `surfaces`, `tasks`, `change_units`, `user_judgments`,
  `write_authorizations`, `runs`, `artifacts`, `artifact_links`,
  `evidence_summaries`, `blockers`는 active current record입니다.
- `task_events`는 audit, debugging, idempotency explanation, projection freshness,
  recovery history를 support합니다.
- `tool_invocations`는 `dry_run=false`인 state-changing tool response의 exact
  committed replay를 support합니다.

Required event emission은 committed state mutation에만 적용됩니다. Malformed request,
dry run, pre-commit state conflict, state를 mutate하지 않는 invalid request에는
`task_events` row가 필요하지 않습니다. Blocked request가 stored blocker를 create/update한다면
그 blocker mutation이 event-worthy state change입니다. `dry_run=true`는 current record,
`task_events` row, artifact, consumable Write Authorization, projection job,
`tool_invocations` replay row를 만들지 않습니다.

`harness.record_run`에서 pre-commit rejection은 Run row, artifact row,
artifact link, evidence summary, authorization consumption, blocker/gate update,
`task_events` row, projection job, state-version advance, `tool_invocations` row를
만들지 않습니다. 활성 계약의 유일한 예외는 observed after-the-fact behavior를 위한
명시적 committed violation/audit path입니다. 이 path는 `runs.status=violation` row와
recovery/blocker/event state를 저장할 수 있지만 invalid authorization을 소비하거나
evidence, QA, verification, final acceptance, residual-risk acceptance, close readiness를
충족하면 안 됩니다.

<a id="canonical-enum-hardening"></a>

## Storage Validation과 Enum Hardening

SQLite는 Core와 migration이 막지 않으면 malformed row도 저장할 수 있습니다. Row는
owner schema, owner value set, state-version basis, idempotency key, artifact
owner-link contract에 맞을 때만 authoritative합니다.

JSON `TEXT` column은 storage flexibility를 위한 것이지 arbitrary JSON을 저장하라는 뜻이
아닙니다. Core가 JSON `TEXT` value를 commit하기 전에는 값을 parse하고 parsed shape를
owner에 맞게 validate해야 합니다.

- API-shaped payload는 [MVP API](api/mvp-api.md)와 [API Schema Core](api/schema-core.md)에
  맞게 validate합니다.
- Storage-only JSON은 이 문서 또는 이 문서가 이름 붙인 owner document에 맞게 validate합니다.
- `'{}'`, `'[]'` 같은 SQLite default는 storage representation rule입니다. Public API
  field를 optional로 만들지 않습니다.

Status-like `TEXT` column은 open string이 아닙니다. Allowed value는 Core validation이
소유합니다. Database `CHECK` constraint나 lookup table은 방어적 보강입니다.

Early hardening 대상:

| Field(s) | Owner/value source |
|---|---|
| `tasks.mode`, `tasks.lifecycle_phase`, `tasks.result` | [Core Model 참조](core-model.md) |
| `surfaces.status`, `surfaces.guarantee_level`, `surfaces.local_access_posture` | [Agent 통합 참조](agent-integration.md), [보안 참조](security.md), 이 문서의 storage registration rule |
| `change_units.status` | Core Model / Change Unit owner rules |
| `user_judgments.status`, `judgment_kind`, `presentation` | user-judgment API/Core owners |
| `write_authorizations.status` | [Core Model `prepare_write`](core-model.md#prepare_write), [`harness.prepare_write`](api/mvp-api.md#harnessprepare_write), [`harness.record_run`](api/mvp-api.md#harnessrecord_run) |
| `runs.kind`, `runs.status` | [`harness.record_run`](api/mvp-api.md#harnessrecord_run)와 storage compatibility notes |
| `artifacts.kind`, `artifacts.redaction_state`, `artifacts.retention_class`, `artifacts.status` | `ArtifactRef`/artifact owners와 storage compatibility notes |
| `artifact_links.owner_record_kind`, `artifact_links.relation` | API `StateRecordRef`, `ArtifactInput.relation`, storage owner-link rules |
| `evidence_summaries.status` | Core evidence gate와 API evidence summary owners |
| `blockers.status`, `blocked_action`, `blocker_kind` | Core Model과 API blocker owners |
| `task_events.event_type` | Core stable event semantics |
| `tool_invocations.status` | storage idempotency replay semantics |
| Future `projection_jobs.status`, `projection_jobs.projection_kind` | Operations Profile active 시 Projection/API owners |
| Future `validator_runs.status` | assurance 또는 conformance profile active 시 `ValidatorResult` owner |
| Future `evidence_manifests.status` | Full Evidence Manifest profile이 active일 때 Evidence profile owner |

Storage-owned compatibility values:

| Field | Durable values | Meaning |
|---|---|---|
| `runs.status` | `completed`, `interrupted`, `blocked`, `violation` | Committed Run row입니다. `completed`만 normal owner ref를 통해 evidence를 support할 수 있습니다. 다른 값은 audit/recovery record이며 그 자체로 evidence, QA, verification, acceptance, close readiness를 satisfy하지 않습니다. |
| `change_units.status` | `planned`, `active`, `completed`, `deferred`, `superseded` | Scope lifecycle입니다. Active compatible scope row만 new write를 scope합니다. |
| `user_judgments.status` | `proposed`, `pending_user`, `resolved`, `deferred`, `rejected`, `blocked`, `superseded` | User judgment lifecycle입니다. Resolved judgment는 기록한 judgment type과 payload에만 영향을 줍니다. |
| `write_authorizations.status` | `active`, `consumed`, `expired`, `stale`, `revoked` | Core/API owner value set과 일치하는 durable authorization lifecycle입니다. `active`이고 compatible한 row만 `record_run`이 consume할 수 있습니다. |
| `artifacts.status` | `available`, `missing`, `stale`, `blocked` | Artifact availability입니다. Storage와 integrity fact이지 full evidence sufficiency가 아닙니다. |
| `evidence_summaries.status` | `not_required`, `none`, `partial`, `sufficient`, `stale`, `blocked` | MVP-1 status와 close에 쓰는 최소 증거 coverage state입니다. Evidence가 close-required이면 `sufficient`가 필요합니다. |
| `blockers.blocker_kind` | `task`, `open_run`, `scope`, `user_judgment`, `sensitive_approval`, `design_policy`, `write_compatibility`, `baseline`, `surface_capability`, `evidence`, `artifact_availability`, `final_acceptance`, `residual_risk_visibility`, `residual_risk_acceptance`, `cancellation`, `supersession`, `recovery` | Status, write compatibility, run recording, close, recovery에서 쓰는 active blocker category입니다. Close response는 MVP API가 소유하는 close-category subset을 보여줍니다. Verification, Manual QA, projection/report freshness, export, operations, full Approval, full Residual Risk, Evidence Manifest, Eval, detached-verification blocker category는 later/profile-only입니다. |
| `blockers.status` | `open`, `resolved`, `superseded` | Stored blocker lifecycle입니다. Open blocker는 Core가 resolve 또는 supersede할 때까지 visible 상태로 남습니다. |
| `tool_invocations.status` | `committed` | Committed replayable `dry_run=false` response에 대해서만 row가 존재합니다. |

`prepare_write.decision`은 durable authorization lifecycle column과 별도입니다. Canonical
`prepare_write.decision` 값은 `allowed`, `blocked`, `approval_required`,
`decision_required`, `state_conflict`입니다. `dry_run=false`인 `decision=allowed`만
durable authorization row를 만듭니다. Exact idempotent replay는 original committed
response를 반환합니다.

새 row는 `write_authorizations.status=active`로 시작합니다. `blocked`는 persisted Write
Authorization lifecycle status가 아닙니다. `dry_run=false`인 `blocked`,
`approval_required`, `decision_required` decision은 response decision, blocker, validator
finding, error, 필요한 경우 committed idempotency replay state로 표현합니다.
`state_conflict`는 새 replay row를 merge하지 않고 conflict state를 반환합니다.
`dry_run=true`의 `decision=allowed` response는 `authorization_effect=would_create`를
설명할 수 있지만 `write_authorizations` row를 insert하지 않고 `record_run`이 소비할 수도
없습니다.

Future table value set은 해당 table의 owner profile이 active이거나, fixture가 optional
table을 명시적으로 seed하거나, owner document가 값을 명시적으로 승격할 때만 사용해야
합니다.

## 초기 Schema 경계

이 저장소에는 migration runner가 없고 migrate할 runtime data도 없습니다. 이 문서는
기존 runtime data를 migration하는 전략을 정의하지 않습니다. Runtime implementation
전에는 maintainer가 actual DDL, migration mechanism, storage profile, tightening
behavior를 별도로 수락해야 합니다.

향후 구현 계획에서도 아래 storage rule은 보존해야 합니다.

- `project_state`와 Runtime Home metadata 또는 선택한 equivalent mechanism에
  schema/profile version을 기록합니다.
- Constraint를 tighten하기 전에 JSON과 owner-bound status value를 validate합니다.
- `task_events`를 유지한다면 `task_events.event_seq` order를 보존합니다.
- Artifact hash와 owner link를 보존하거나, 영향을 받는 ref를 recovery 대상으로 invalid
  표시합니다.
- Unknown owner-bound enum/status value가 있으면 owner가 소유하지 않은 fallback meaning을
  만들지 말고 stop합니다.
- Status card, compact view, projection/card/job freshness, close readiness, full
  report text는 derived output으로 취급하고 canonical state로 취급하지 않습니다.

## Lock Policy

Runtime mutation은 [런타임 아키텍처](runtime-architecture.md#state-transaction-flow)가
담당하는 Core transaction order를 통해 serialize해야 합니다. 활성 범위는 ordinary
SQLite transaction과 필요 시 process/project lock을 사용할 수 있습니다.
`persistent_locks`는 later Operations candidate이지 active storage 선결 조건이 아닙니다.

Lock은 concurrent write를 보호합니다. OS sandboxing, evidence integrity,
tamper-proof storage를 제공하지 않습니다.
