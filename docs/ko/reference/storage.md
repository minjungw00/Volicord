# Storage

이 문서는 향후 하네스 저장소를 위한 참조 문서입니다. 이 저장소에 하네스 서버,
Runtime Home, database, artifact store, migration runner, generated projection,
runtime state, 구현 완료된 DDL이 있다는 뜻이 아닙니다. 현재 저장소 상태는
[MVP 계획](../build/mvp-plan.md#문서-수락-상태)이 담당합니다.

## 1. 담당 / 담당하지 않음

이 문서는 현재 활성 MVP의 영속 경계를 담당합니다.

- Runtime Home identity와 project-local 저장소 layout.
- 활성 영속 레코드와 table-level 저장 역할.
- 저장소가 소유하는 JSON `TEXT` 규칙.
- Artifact persistence와 artifact owner link.
- Event와 멱등성 storage 의미.
- 상태 버전 저장 규칙.
- 잠금 정책과 마이그레이션 경계.
- 현재 활성 MVP storage와 later/profile storage의 경계.

이 문서는 아래 항목을 담당하지 않습니다.

- Core lifecycle, gate, blocker, Write Authorization, `record_run`, close 의미.
  [Core Model 참조](core-model.md)를 봅니다.
- Public MCP request/response, shared schema, active enum value, error, replay
  behavior. [MVP API](api/mvp-api.md), [API Schema Core](api/schema-core.md),
  [API Errors](api/errors.md)를 봅니다.
- Projection rendering, template body, report format, dashboard, export,
  reconcile behavior, operations entrypoint, conformance runner, future fixture
  storage.
- OS permission, sandboxing, tamper-proof file, pre-tool blocking, security
  isolation 주장. [보안 참조](security.md)를 봅니다.

저장소는 Core가 commit하고 담당 Core/API/storage 계약에 맞게 validate한 row에
대해서만 현재 하네스 레코드의 기준이 됩니다. Chat, generated Markdown, status card,
projection, connector output, operator output, report prose는 storage authority가
아닙니다.

## 2. Runtime Home identity

하네스는 local Runtime Home 하나와 등록된 project별 project-local state database 하나를
사용합니다. 기본 reference root는 `~/.harness`입니다. 구현은 같은 역할의 configured
root를 선택할 수 있습니다.

기준 layout은 다음과 같습니다.

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

`registry.sqlite`는 Runtime Home identity와 최소 project registration data를
저장합니다. `project.yaml`은 static project configuration만 저장합니다.
`state.sqlite`는 project-local Core state를 저장합니다. Artifact directory는 Core가
artifact registration boundary를 적용한 뒤 등록된 증거 바이트 또는 안전한 메타데이터를
저장합니다.

`project.yaml`은 current Task state, gate, Write Authorization state, evidence
sufficiency, final acceptance, residual-risk acceptance, close state를 저장하면 안 됩니다.

Runtime Home identity는 filesystem path에만 의존하면 안 됩니다. 복사되거나 이동된
Runtime Home은 같은 stored `runtime_home_id`를 가질 수 있습니다. 새 Runtime Home은 새 id를
가져야 합니다. 이 id는 의심스러운 copy, duplicate registration, path drift를 감지하는 데
도움이 됩니다. 하지만 storage를 tamper-proof하게 만들지는 않습니다.

Runtime Home file은 local operational control data이고 민감한 support data를 담을 수
있습니다. Broad read access는 secret, PII, token, log, screenshot, diff, artifact content를
노출할 수 있습니다. Broad write access는 tampering과 evidence poisoning risk입니다. File
permission, owner check, hash, diagnostic은 방어적 확인입니다. 그 자체로 OS-level
sandboxing, arbitrary-tool control, tamper-proof storage, pre-execution blocking을 만들지
않습니다.

## 3. 활성 영속 레코드

현재 활성 MVP는 활성 method set에 필요한 레코드만 영속화합니다.
`harness.intake`, `harness.status`, `harness.prepare_write`,
`harness.record_run`, `harness.request_user_judgment`,
`harness.record_user_judgment`, `harness.close_task`가 그 범위입니다.

활성 영속 레코드는 다음뿐입니다.

- `registry.sqlite`의 Runtime Home identity.
- `registry.sqlite`의 minimal project registration.
- `project.yaml`의 static project configuration.
- `project_state`.
- `surfaces`. 단, active API envelope, capability display, local-access posture에
  필요한 registered local/reference surface fact로 제한합니다.
- `tasks`.
- `change_units`.
- `user_judgments`.
- `write_authorizations`.
- `runs`.
- `artifacts`.
- `artifact_links`.
- `evidence_summaries`.
- `blockers`.
- `task_events`.
- `tool_invocations`.

다른 persisted table family는 현재 활성 MVP 범위가 아닙니다. 요구사항 구체화는
`tasks`, `change_units`, `user_judgments`, `evidence_summaries`, `blockers`를 통해
저장합니다. 별도 committed Discovery Brief, Shared Design, Question Queue, Assumption
Register, First Safe Change Unit Candidate table을 만들지 않습니다. Evidence는 compact
evidence summary와 artifact ref를 통해 저장합니다. Full Evidence Manifest storage를
요구하지 않습니다.

## 4. Tables

아래 표는 active storage table과 최소 저장 역할을 이름 붙입니다. Full DDL이 아니며 API
schema를 복사하지 않습니다.

| Table 또는 file | 위치 | Active role | Essential stored fields |
|---|---|---|---|
| Runtime Home identity | `registry.sqlite` | Local Runtime Home과 schema/storage profile을 식별합니다. | `runtime_home_id`, `schema_version`, `storage_profile`, `created_at`, `updated_at`. |
| Project registration | `registry.sqlite` | Registered project를 project-local storage에 연결합니다. | `project_id`, `repo_root`, `project_home`, `display_name`, `status`, `created_at`, `updated_at`. |
| `project.yaml` | Project directory | Static project configuration. | `project_id`, `repo_root`, display/config default. |
| `project_state` | `state.sqlite` | Project-local state header, state clock, active Task pointer, default surface pointer. | `project_id`, `schema_version`, `storage_profile`, `state_version`, `active_task_id`, `default_surface_id`, `created_at`, `updated_at`. |
| `surfaces` | `state.sqlite` | `surface_id`, capability profile, local access posture, guarantee display에 필요한 registered local/reference surface fact. | `surface_id`, `project_id`, `surface_kind`, `capability_profile_json`, `local_access_posture`, `guarantee_level`, `status`, `created_at`, `updated_at`. |
| `tasks` | `state.sqlite` | User-value work unit, task-scoped state clock, current shaping summary, lifecycle, result, close field. | `task_id`, `project_id`, `title`, `user_request`, `current_goal_summary`, `mode`, `lifecycle_phase`, `result`, `summary`, shaping JSON columns, `blocking_question`, `next_safe_action`, `active_change_unit_id`, `state_version`, `created_at`, `updated_at`, `closed_at`. |
| `change_units` | `state.sqlite` | Write compatibility와 close basis를 위한 current 또는 proposed scoped work boundary. | `change_unit_id`, `task_id`, `scope_summary`, scope JSON columns, `baseline_ref`, `autonomy_boundary_json`, `status`, `created_at`, `updated_at`. |
| `user_judgments` | `state.sqlite` | Active `UserJudgment.judgment_kind` 값에 대한 사용자 소유 판단 기록. | `user_judgment_id`, `task_id`, `change_unit_id`, `judgment_kind`, `presentation`, `status`, request/context JSON columns, `question`, `resolution_json`, `expires_at`, `resolved_at`, `created_at`, `updated_at`. |
| `write_authorizations` | `state.sqlite` | `dry_run=false`인 `prepare_write`가 `decision=allowed`일 때만 생성되는 durable single-use cooperative Write Authorization. | `write_authorization_id`, `task_id`, `change_unit_id`, `surface_id`, `status`, `basis_state_version`, `attempt_scope_json`, `consumed_by_run_id`, `expires_at`, `created_at`, `updated_at`, `consumed_at`. |
| `runs` | `state.sqlite` | Product write가 있었다면 compatible authorization consumption까지 포함하는 committed execution 또는 observation record. | `run_id`, `task_id`, `change_unit_id`, `write_authorization_id`, `surface_id`, `kind`, `status`, `product_write`, `baseline_ref`, `summary`, observed/evidence JSON columns, `created_at`, `completed_at`. |
| `artifacts` | `state.sqlite`와 artifact store | Integrity, redaction, producer, retention, availability fact를 가진 registered durable evidence byte 또는 safe metadata. | `artifact_id`, `project_id`, `task_id`, `run_id`, `kind`, `uri`, `sha256`, `size_bytes`, `content_type`, `redaction_state`, `retention_class`, `produced_by`, `status`, `created_at`, `updated_at`. |
| `artifact_links` | `state.sqlite` | Artifact가 지원하는 active Core/API record로 가는 owner relation. | `artifact_link_id`, `artifact_id`, `task_id`, `owner_record_kind`, `owner_record_id`, `relation`, `created_at`. |
| `evidence_summaries` | `state.sqlite` | Status, run/evidence summary, blocker, close에 쓰는 compact evidence coverage와 gap record. | `evidence_summary_id`, `task_id`, `change_unit_id`, `status`, `coverage_items_json`, `summary`, `supporting_run_ids_json`, `supporting_artifact_link_ids_json`, `gap_blocker_ids_json`, `updated_at`. |
| `blockers` | `state.sqlite` | Next action, write compatibility, evidence gap, close readiness, recovery를 위한 structured blocker. | `blocker_id`, `task_id`, `blocked_action`, `blocker_kind`, `status`, `message`, `owner_ref_json`, `related_refs_json`, `required_next_action`, `created_at`, `resolved_at`. |
| `task_events` | `state.sqlite` | Committed Core mutation의 append-only audit와 ordering trail. | `event_id`, `task_id`, `event_seq`, `event_type`, `state_version`, `actor_kind`, `surface_id`, `payload_json`, `created_at`. |
| `tool_invocations` | `state.sqlite` | `dry_run=false`인 state-changing tool response의 committed idempotency replay row. | `invocation_id`, `project_id`, `tool_name`, `idempotency_key`, `request_hash`, `task_id`, `basis_state_version`, `response_json`, `status`, `created_at`. |

`surfaces`는 connector marketplace나 broad connector ecosystem table이 아닙니다.
`surface_id`, capability, local access posture, guarantee display를 해석하는 데 필요한
active local/reference surface registration입니다.

`display_label`은 active storage identity column이 아닙니다. Display label은
`judgment_kind` 같은 stable identifier와 locale에서 파생합니다.

## 5. JSON TEXT columns

JSON을 저장하는 SQLite `TEXT` column은 storage representation 선택입니다. Arbitrary JSON을
저장하라는 뜻이 아닙니다. Core는 commit 전에 JSON을 parse하고 validate해야 합니다.

API-shaped stored JSON은 [MVP API](api/mvp-api.md)와
[API Schema Core](api/schema-core.md)에 맞게 validate합니다. Storage-only JSON은 이 문서
또는 이 문서가 이름 붙인 owner 문서에 맞게 validate합니다. `'{}'`, `'[]'` 같은 SQLite
default는 storage default일 뿐입니다. API field를 optional로 만들지 않습니다.

활성 JSON `TEXT` column은 active record에 필요한 compact owner-shaped data로 제한합니다.
예시는 다음과 같습니다.

- `surfaces.capability_profile_json`.
- `success_criteria_json`, `non_goals_json`, `affected_areas_json`,
  `affected_path_candidates_json`, `constraints_json`, `autonomy_boundary_json` 같은
  Task와 Change Unit shaping column.
- `user_judgments`의 request, context, option, affected-ref, artifact-ref,
  `resolution_json` column.
- `AuthorizedAttemptScope`를 저장하는 `write_authorizations.attempt_scope_json`.
- `runs`의 observed-attempt와 evidence-update JSON column.
- `evidence_summaries.coverage_items_json`과 supporting/gap ref array.
- `blockers.owner_ref_json`과 `blockers.related_refs_json`.
- `task_events.payload_json`.
- `tool_invocations.response_json`.

Status-like `TEXT` value는 open string이 아니라 닫힌 owner value set입니다. Active value는
Core/API owner와 이 문서의 storage note가 담당합니다. Defensive `CHECK` constraint나
lookup table을 사용할 수 있지만 Core validation은 계속 필요합니다.

## 6. Artifact references

`ArtifactRef`는 등록된 durable evidence byte 또는 safe metadata를 위한 public API shape입니다.
Storage는 `artifacts`와 `artifact_links`로 이를 구현합니다. 자세한 shape는
[API Schema Core: ArtifactRef](api/schema-core.md#artifactref)를 봅니다.

Artifact가 evidence-eligible하려면 storage가 아래 사실을 가져야 합니다.

- artifact store 아래 등록된 byte 또는 safe metadata notice,
- `sha256`, `size_bytes`, `content_type` 같은 integrity fact,
- `redaction_state`,
- producer와 retention fact,
- availability `status`,
- `task`, `change_unit`, `run`, `user_judgment`, `evidence_summary`, `blocker` 같은 active
  record로 가는 owner link.

`uri`는 보통 `harness-artifact://{project_id}/{artifact_id}` 형태로 Harness storage를 통해
resolve됩니다. Caller가 임의로 준 filesystem path가 아닙니다. Raw secret, token, full
sensitive log를 evidence byte로 저장하면 안 됩니다. 대신 redacted byte,
`secret_omitted` 또는 `blocked` notice, safe handle, owner가 허용한 safe representation을
저장합니다.

Artifact link는 owner record를 만들지 않습니다. 그 자체로 gate를 충족하거나, evidence
sufficiency를 증명하거나, QA를 수행하거나, final acceptance를 만들거나, residual risk를
수락하거나, Task를 close하지 않습니다.

## 7. 멱등성과 event 의미

`task_events`는 committed Core mutation을 순서대로 기록합니다. Audit와 ordering trail이지,
일반 동작에서 current state를 재구성하는 source가 아닙니다. `tasks`, `change_units`,
`user_judgments`, `write_authorizations`, `runs`, `artifacts`, `artifact_links`,
`evidence_summaries`, `blockers` 같은 current row가 현재 상태입니다.

`tool_invocations`는 committed non-dry-run state-changing response의 exact replay를
저장합니다. Key scope는 [API Errors: Idempotency](api/errors.md#idempotency)가 담당합니다.
같은 key와 request hash가 replay되면 Core는 event를 append하거나, artifact를 등록하거나,
authorization을 consume하거나, state를 다시 바꾸지 않고 original committed response를
반환합니다. 같은 key가 다른 request hash로 재사용되면 Core는 API owner가 정의한 state
conflict behavior를 반환합니다.

Dry run, malformed request, pre-commit validation failure, pre-commit state conflict,
mutation을 만들지 않는 rejected `record_run` attempt는 current row, `task_events`,
artifact, evidence summary, Write Authorization, close state, `tool_invocations` replay
row를 만들지 않습니다.

Blocked response는 method owner가 허용한 blocker 또는 다른 mutation만 저장할 수 있습니다.
Blocker가 없다고 말하는 authority를 만들면 안 됩니다. 예를 들어 blocked `prepare_write`
response는 consumable `write_authorizations`를 만들지 않습니다.

## 8. 상태 버전

상태 버전은 scope별 clock입니다. Task-scoped mutation은 `tasks.state_version`을 올립니다.
Core-resolved primary Task가 없는 project-scoped mutation은 `project_state.state_version`을
올립니다.

State-changing API call은 commit 전에 `ToolEnvelope.expected_state_version`을 affected
scope와 비교합니다. `ToolResponseBase.state_version`은 committed mutation에서는 affected
scope의 resulting version이고, read-only와 dry-run response에서는 current readable version
또는 would-be affected version입니다.

`write_authorizations.basis_state_version`은 Core가 attempt를 allow할 때 사용한 상태 버전을
저장합니다. `write_authorizations.attempt_scope_json`은 나중에 `record_run`이 observed fact와
비교할 authorized attempt boundary를 저장합니다. Top-level `task_id`, `change_unit_id`,
`surface_id`, `basis_state_version` column은 query field입니다. Stored attempt scope가
compatibility boundary로 남습니다.

`tool_invocations.basis_state_version`은 committed mutation 전에 compatibility basis로 사용한
affected-scope version을 저장합니다. `task_events.state_version`은 committed event의 resulting
affected-scope version을 저장합니다.

## 9. 잠금 정책

Runtime mutation은 Core-owned state-changing path를 통해 serialize합니다. Ordinary SQLite
transaction과 필요한 경우 process/project lock을 사용합니다. Authority placement는
[런타임 경계 참조](runtime-boundaries.md)가 담당합니다.

현재 활성 MVP는 `persistent_locks` table을 요구하지 않습니다. Durable lock/recovery
metadata는 owner가 승격하기 전까지 later Operations/Profile material입니다.

Lock은 concurrent state write를 보호합니다. OS sandboxing, artifact integrity,
tamper-proof storage, permission isolation, pre-tool blocking을 제공하지 않습니다.

## 10. 마이그레이션 경계

이 저장소에는 migration runner가 없고 migrate할 runtime data도 없습니다. 이 문서는 기존
runtime data를 migration하는 step을 정의하지 않습니다. Runtime implementation 전에는
maintainer가 실제 DDL, migration mechanism, storage profile, tightening behavior를 별도로
수락해야 합니다.

현재 활성 마이그레이션 경계는 다음과 같습니다.

- Runtime Home metadata와 `project_state`, 또는 수락된 equivalent mechanism에
  schema/profile version을 저장합니다.
- Commit 전과 constraint tightening 전에 owner-shaped JSON을 validate합니다.
- Unknown owner-bound status 또는 enum value는 owner가 정의하기 전까지 invalid로
  취급합니다.
- `task_events`를 유지한다면 `task_events.event_seq` ordering을 보존합니다.
- Artifact hash와 owner link를 보존하거나 affected ref를 recovery 대상으로 invalid
  표시합니다.
- Status card, compact view, projection freshness, close readiness, report prose는 current
  record에서 파생합니다. Migration authority가 아닙니다.

이 문서는 inactive DDL bundle, migration catalog, profile-specific migration detail을
의도적으로 제외합니다.

## 11. 현재 활성 MVP에서 제외되는 later storage

Later/profile storage는 owner 문서가 scope, fallback behavior, proof expectation과 함께 좁은
동작을 승격하기 전까지 현재 활성 MVP 밖에 있습니다. Reference schema에 존재한다는
사실만으로 storage가 active가 되지 않습니다.

현재 활성 MVP는 아래 storage를 제외합니다.

- Projection job, durable projection cache, managed-output outbox, projection dashboard.
- Validator-run record, conformance-runner state, fixture execution history, generated
  conformance artifact.
- Doctor suite, recover, export, release handoff, artifact dashboard, reconcile queue,
  operational report를 위한 operations-profile storage.
- Full Evidence Manifest table, detailed evidence catalog, detached Eval, detached
  verification, full Manual QA matrix, rich QA/waiver machinery.
- `user_judgments`와 `blockers`에서 분리된 rich Approval table과 rich residual-risk lifecycle
  table.
- Dashboard, metrics, analytics, team workflow, hosted connector registry, connector
  marketplace, connector analytics, cross-surface orchestration storage.
- Shared Design, Journey/Spine, Domain Language, Module Map, Interface Contract,
  stewardship, long-term design-support storage.

Active status, close readiness, run/evidence summary, next action, readable card, guarantee
display는 위 active persisted record에서 파생합니다. 이 output은 stale, absent, failed일 수
있고 다시 계산될 수 있습니다. 그래도 storage authority를 바꾸지 않습니다.
