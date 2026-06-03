# Storage와 DDL

## 이 문서가 담당하는 것

이 문서는 향후 local Harness Server를 위한 reference 문서입니다. 이 저장소에는 아직 database, migration runner, server, runtime이 없습니다. 현재 저장소 단계와 구현 인계 상태는 [구현 개요](../build/implementation-overview.md#문서-수락-상태)에 있습니다.

Storage authority, Runtime Home identity, staged SQLite table 필요 범위, event semantics, artifact registration, migration/validation constraint를 검토할 때 이 문서를 사용합니다. Stage 순서와 exit criteria는 [Build: MVP 계획](../build/mvp-plan.md)과 [첫 실행 가능한 조각](../build/first-runnable-slice.md)을 봅니다.

## 이런 때 읽기

- v0.1 또는 v0.2에 어떤 storage table이 필요한지 확인할 때.
- Core-owned state와 chat, Markdown projection, connector output, tool output을 분리할 때.
- Runtime Home risk, artifact poisoning control, event/audit behavior, JSON validation, enum hardening, future schema candidate를 확인할 때.
- later profile table이 첫 server batch를 부풀리지 않게 점검할 때.

## 관련 Owner

| 관심사 | Owner |
|---|---|
| Public MCP request/response shape, `ArtifactRef`, `ValidatorResult`, idempotency와 state conflict behavior | [MCP API와 스키마](mcp-api-and-schemas.md) |
| Task lifecycle, gate, `prepare_write`, `record_run`, `close_task`, stable event | [커널 참조](kernel.md) |
| Core process model, transaction order, lock, projection/reconcile placement | [런타임 아키텍처 참조](runtime-architecture.md) |
| Projection authority, freshness, managed block, rendered template | [문서 Projection 참조](document-projection.md)와 [Template 참조](templates/README.md) |
| Operator behavior, doctor/recover/export/reconcile/conformance entrypoint | [운영과 Conformance 참조](operations-and-conformance.md) |
| Fixture format과 assertion semantics | [Conformance Fixtures 참조](conformance-fixtures.md) |
| Stage sequence와 implementation readiness | [Build: MVP 계획](../build/mvp-plan.md), [구현 개요](../build/implementation-overview.md) |

## Storage Role And Authority Model

Harness storage는 local Core-owned operational state를 보관합니다. Active stage가 필요로 할 때 scope, write authorization, user-owned judgment, evidence reference, close readiness, acceptance, residual risk를 durable record로 남깁니다. 유용한 future feature가 문서에 있다는 이유만으로 첫 구현 조각에 모두 들어가지는 않습니다.

Authority boundary:

- Core-owned state table이 현재 Harness state의 authority입니다.
- `task_events`는 append-only audit와 ordering trail입니다. 일반 동작에서 current state를 재구성하는 source가 아닙니다.
- Artifact file은 Core가 등록하고 compatible owner record에 link하기 전까지 evidence authority가 아닙니다.
- Chat, Markdown projection, generated report, connector manifest, tool output, operator output은 Core mutation이 owner-valid state row 또는 artifact link를 기록하기 전까지 authority가 아닙니다.
- Projection과 status card는 readable derived view입니다. Stale, failed, absent 상태일 수 있고 canonical state를 바꾸지 않습니다.
- Future/profile table은 owning profile 또는 tool path가 활성 상태이거나 사용될 때만 required가 됩니다.

첫 server batch는 좁은 local authority loop를 증명하면 됩니다. Project identity, Task 하나, scoped boundary 하나, `prepare_write`, single-use Write Authorization 하나, Run 하나, artifact/evidence reference 하나, task event, structured blocker가 핵심입니다. Later profile contract가 문서화되어 있다는 이유로 수십 개 table을 만들 필요는 없습니다.

## Runtime Home Identity And Risks

Harness는 local Runtime Home 하나와 registered project별 state database 하나를 둡니다. 기본 reference location은 `~/.harness`이지만 구현은 configured equivalent를 선택할 수 있습니다.

### Runtime home layout

기준 layout:

```text
~/.harness/
  registry.sqlite
  projects/
    PRJ-0001/
      project.yaml
      state.sqlite
      artifacts/
        bundles/
        diffs/
        logs/
        screenshots/
        checkpoints/
        manifests/
        qa/
        tdd/
        designs/
        prototypes/
        architecture/
        decisions/
        exports/
        tmp/
```

`registry.sqlite`는 Runtime Home identity와 project registration을 저장합니다. `project.yaml`은 static project configuration만 저장합니다. `state.sqlite`는 project-local Core state를 저장합니다. Artifact directory는 Core가 artifact registration boundary를 적용한 뒤 등록된 file을 보관합니다.

Runtime Home identity는 path에만 의존하면 안 됩니다. 복사되거나 이동된 Runtime Home은 같은 stored `runtime_home_id`를 유지할 수 있고, 새 Runtime Home은 새 id를 가져야 합니다. `doctor`와 recovery flow는 이 identity를 사용해 의심스러운 copy, duplicate registration, path drift를 보고할 수 있습니다. 다만 이 id가 tamper-proofing을 제공하지는 않습니다.

### `project.yaml`

`project.yaml`은 static project configuration입니다. Current Task state, current gate, write authority, evidence sufficiency, acceptance, residual risk를 저장하면 안 됩니다.

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

### Runtime home permissions and tampering

Runtime Home은 local operational authority와 민감한 support data를 담습니다. Broad write access는 tampering과 artifact poisoning risk입니다. Broad read access는 secret, PII, token, log, screenshot, diff, verification bundle, export를 노출할 수 있습니다.

v0.1과 v0.2 storage는 다른 profile이 더 강한 control을 증명하기 전까지 cooperative/detective입니다. File permission, owner check, hash, `doctor` finding은 방어적 보강입니다. 그 자체로 OS-level sandboxing, arbitrary-tool control, tamper-proof storage, pre-execution blocking을 만들지 않습니다.

| 관찰 사항 | Storage 의미 |
|---|---|
| Runtime Home 또는 project storage의 owner/mode를 확인할 수 없습니다. | Unknown 또는 weak local file posture를 보고합니다. OS-level guarantee를 주장하지 않습니다. |
| Runtime Home, `state.sqlite`, `registry.sqlite`, artifact directory가 unrelated user, shared group, shared container, broad local process에게 writable입니다. | Tampering과 artifact-poisoning risk를 보고합니다. Core는 row, owner link, hash, artifact registration을 검증한 뒤에만 의미를 신뢰해야 합니다. |
| Artifact storage 또는 export가 unrelated user, shared group, shared container, broad local process에게 readable입니다. | 민감한 값을 그대로 출력하지 않고 confidentiality risk를 보고합니다. |
| Registered artifact hash, size, owner link, path가 storage metadata와 더 이상 맞지 않습니다. | Projection drift가 아니라 artifact integrity failure 또는 recovery input으로 취급합니다. |

## Table-To-Stage Matrix

이 matrix가 main table list입니다. 작은 v0.1/v0.2 storage와 later profile candidate를 분리합니다.

Public API ref는 [MCP API와 스키마](mcp-api-and-schemas.md#artifactref)가 담당합니다. Minimum v0.2 storage slice에서는 `evidence_summaries.evidence_summary_id`를 `StateRecordRef.record_kind=evidence_summary`로, `close_readiness.close_readiness_id`를 `StateRecordRef.record_kind=close_readiness`로 가리킬 수 있습니다. Approval 형태 민감 동작 승인은 `StateRecordRef.record_kind=decision_packet`으로 참조합니다. `StateRecordRef.record_kind=approval`은 `approvals` table이 명시적으로 승격되기 전까지 later-profile입니다. `change_unit_dependencies`는 future/diagnostic storage로 남으므로 `record_kind=change_unit_dependency`는 v0.2 active public ref가 아닙니다.

| Table | Purpose | First active stage | Authority or auxiliary | User-facing or internal | Later status |
|---|---|---|---|---|---|
| `registry_meta` | Runtime Home id와 registry schema version | v0.1 | auxiliary identity | internal | active early |
| `projects` | Registered project identity와 state location | v0.1 | registration authority | project selection에서 user-facing | active early |
| `project_surfaces` | Surface/capability declaration과 guarantee display, surface profile이 설치된 경우 | v0.3/v0.4 또는 profile-promoted | auxiliary capability state | internal/user-facing diagnostics | future/later |
| `project_state` | Project-local clock과 active Task pointer | v0.1 | authority | internal | active early |
| `tasks` | Current Task record와 task state clock | v0.1 | authority | user-facing summary | active early |
| `change_units` | Write를 위한 minimal scoped work boundary | v0.1 | authority | scope 설명 시 user-facing | active early |
| `write_authorizations` | Durable single-use `prepare_write` allow record | v0.1 | authority | internal, blocker는 user-visible | active early |
| `runs` | Committed observed Run record | v0.1 | authority | evidence/status ref로 user-facing | active early |
| `artifacts` | Registered artifact/evidence file metadata | v0.1 | artifact metadata authority | ref를 통해 surfaced | active early |
| `artifact_links` | Artifact와 Task/Run/owner record의 compatible link | v0.1 | artifact owner-link authority | internal | active early |
| `task_blockers` | Structured status/blocker row | v0.1 | stored blocker authority | user-facing | active early |
| `task_events` | Append-only audit와 event-order trail | v0.1 | audit trail, projection support | mostly internal | active early |
| `tool_invocations` | Committed idempotency replay row | v0.1 | replay support | internal | active early |
| `task_intake` | Ordinary-language intake와 tracked clarification state | v0.2 | auxiliary shaping state | user-facing | not v0.1 |
| `decision_packets` | Simplified user judgment record와 recorded answer | v0.2 | user judgment authority | user-facing | not v0.1 |
| `decision_requests` | Decision Packet에 link되는 optional prompt routing, replay, handoff metadata | v0.2 optional | auxiliary routing state | internal/user-facing prompt support | optional, not authority by itself |
| `residual_risks` | Minimal visible residual-risk row | v0.2 | stored residual risk authority | user-facing | not v0.1 |
| `evidence_summaries` | Artifact/run ref 위의 minimal evidence summary | v0.2 | authority ref 위의 auxiliary summary | user-facing | not v0.1 |
| `close_readiness` | Minimal close readiness와 close-blocker snapshot | v0.2 | auxiliary display/check snapshot | user-facing | not v0.1 |
| `projection_status_cards` | Projection job system 없는 optional freshness/status card state | v0.2 optional | auxiliary derived display state | user-facing | optional, not authority |
| `approvals` | Sensitive-action approval lifecycle | v0.3 또는 profile-promoted | profile active 시 authority | user-facing | future/later |
| `baselines` | Repository baseline capture | v0.3 또는 profile-promoted | assurance support | internal | future/later |
| `evidence_manifests` | Full criteria-to-evidence coverage | v0.3 또는 profile-promoted | full evidence profile authority | user-facing summary | future/later |
| `evals` | Detached verification/eval record | v0.3 또는 profile-promoted | profile active 시 authority | user-facing summary | future/later |
| `manual_qa_records` | Manual QA profile, result, finding | v0.3 또는 profile-promoted | profile active 시 authority | user-facing summary | future/later |
| `validator_runs` | Persisted validator result | v0.3 또는 profile-promoted | diagnostic state | internal/user-facing finding | future/later |
| `feedback_loops` | Feedback-loop policy record | v0.3 또는 profile-promoted | policy support | internal/user-facing summary | future/later |
| `tdd_traces` | TDD trace record | v0.3 또는 profile-promoted | policy/evidence support | internal/user-facing summary | future/later |
| `projection_jobs` | Durable projection outbox와 rendered-output freshness | v0.4 또는 profile-promoted | auxiliary derived-view job state | internal/user-facing freshness | future/later |
| `reconcile_items` | Human-editable projection drift/proposal handling | v0.4 또는 profile-promoted | Core에 accept되기 전까지 auxiliary | user-facing | future/later |
| `connector_manifests` | Connector-managed file manifest와 drift state | v0.4 또는 profile-promoted | diagnostic/support | internal | future/later |
| `persistent_locks` | Process lock만으로 부족할 때 durable lock/recovery metadata | v0.4 또는 profile-promoted | auxiliary | internal | future/later |
| `export_manifests` | Export/recover package manifest | v0.4 또는 profile-promoted | auxiliary support | internal/user-facing report | future/later |
| `recover_items` | Recovery finding과 repair plan state | v0.4 또는 profile-promoted | diagnostic/support | internal/user-facing report | future/later |
| `task_spine_entries` | Journey/spine continuity record | future/diagnostic | supplemental | user-facing | non-stage-required |
| `journey_cards` | Journey view를 위한 render/cache support가 필요할 때 | future/diagnostic | derived display support | user-facing | non-stage-required |
| `shared_designs` | Design-support profile이 승격될 때 shared design basis record | future/diagnostic | policy support | user-facing summary | non-stage-required |
| `change_unit_dependencies` | Change Unit 사이 dependency/ordering visibility | future/diagnostic | policy support | internal/user-facing summary | non-stage-required |
| `domain_terms` | Domain language/stewardship term | future/diagnostic | policy support | user-facing summary | non-stage-required |
| `module_map_items` | Module map/stewardship record | future/diagnostic | policy support | internal/user-facing summary | non-stage-required |
| `interface_contracts` | Interface contract/stewardship record | future/diagnostic | policy support | internal/user-facing summary | non-stage-required |

## v0.1 Physical Schema

v0.1은 Core Authority Smoke입니다. 의도적으로 작습니다. Project 등록, Task 하나 생성 또는 load, scoped work boundary 하나 정의, write 하나 authorize, Run 하나 기록, artifact/evidence ref 하나 등록, event append, structured blocker return에 충분해야 합니다.

아래 DDL은 planning을 위한 reference fragment입니다. Migration runner가 이미 존재한다는 증거가 아닙니다.

### Schema profile metadata

| Profile | Stage | Required for | 이 profile에 required가 아닌 것 |
|---|---|---|---|
| Core Authority Smoke schema | v0.1 | 좁은 local authority loop | Decision Packet, Evidence Manifest, Manual QA, Eval, residual-risk acceptance, projection job, reconcile, validator, Journey, stewardship map |
| First User-Value Slice schema | v0.2 | 첫 user-value record와 readable status | detached verification, full Manual QA, full projection job system, export/recover, broad operations |
| Agency Assurance schema | v0.3 또는 promoted profile | verification, QA, approval, feedback/TDD, validator support | promoted되지 않은 v0.1/v0.2 exit |
| Operations schema | v0.4 또는 promoted profile | projection job, reconcile, connector manifest, recover/export | promoted되지 않은 v0.1/v0.2 exit |
| Future / diagnostic schema | future/diagnostic | journey/spine, domain/module/interface diagnostic | promoted되지 않은 모든 current stage exit |

### Core Authority Smoke schema

Main v0.1 table count: total 12 tables입니다. `registry.sqlite`에 2개, project `state.sqlite`에 10개입니다. 첫 구현 조각에 맞게 작게 유지한 수입니다.

#### `registry.sqlite`

```sql
CREATE TABLE registry_meta (
  key TEXT PRIMARY KEY,
  value TEXT NOT NULL,
  updated_at TEXT NOT NULL
);

CREATE TABLE projects (
  project_id TEXT PRIMARY KEY,
  display_name TEXT NOT NULL,
  repo_root TEXT NOT NULL,
  project_dir TEXT NOT NULL,
  config_path TEXT NOT NULL,
  registered_at TEXT NOT NULL,
  updated_at TEXT NOT NULL
);
```

v0.1에서 required `registry_meta` key는 `runtime_home_id`와 `schema_version`입니다. 이후 구현이 더 formal한 metadata table을 선택할 수 있지만, v0.1은 durable identity와 version fact만 필요합니다.

#### `state.sqlite`

```sql
CREATE TABLE project_state (
  project_id TEXT PRIMARY KEY,
  schema_version INTEGER NOT NULL,
  state_version INTEGER NOT NULL DEFAULT 0,
  active_task_id TEXT,
  created_at TEXT NOT NULL,
  updated_at TEXT NOT NULL
);

CREATE TABLE tasks (
  task_id TEXT PRIMARY KEY,
  project_id TEXT NOT NULL,
  title TEXT NOT NULL,
  mode TEXT NOT NULL,
  lifecycle_phase TEXT NOT NULL,
  result TEXT,
  active_change_unit_id TEXT,
  state_version INTEGER NOT NULL DEFAULT 0,
  status_summary_json TEXT NOT NULL DEFAULT '{}',
  created_at TEXT NOT NULL,
  updated_at TEXT NOT NULL,
  closed_at TEXT
);

CREATE TABLE change_units (
  change_unit_id TEXT PRIMARY KEY,
  task_id TEXT NOT NULL REFERENCES tasks(task_id),
  summary TEXT NOT NULL,
  status TEXT NOT NULL,
  allowed_paths_json TEXT NOT NULL DEFAULT '[]',
  denied_paths_json TEXT NOT NULL DEFAULT '[]',
  touched_paths_json TEXT NOT NULL DEFAULT '[]',
  created_at TEXT NOT NULL,
  updated_at TEXT NOT NULL
);

CREATE TABLE write_authorizations (
  write_authorization_id TEXT PRIMARY KEY,
  task_id TEXT NOT NULL REFERENCES tasks(task_id),
  change_unit_id TEXT NOT NULL REFERENCES change_units(change_unit_id),
  status TEXT NOT NULL,
  basis_state_version INTEGER NOT NULL,
  allowed_paths_json TEXT NOT NULL DEFAULT '[]',
  denied_paths_json TEXT NOT NULL DEFAULT '[]',
  blocker_refs_json TEXT NOT NULL DEFAULT '[]',
  consumed_by_run_id TEXT,
  expires_at TEXT,
  created_at TEXT NOT NULL,
  updated_at TEXT NOT NULL
);

CREATE TABLE runs (
  run_id TEXT PRIMARY KEY,
  task_id TEXT NOT NULL REFERENCES tasks(task_id),
  change_unit_id TEXT REFERENCES change_units(change_unit_id),
  write_authorization_id TEXT REFERENCES write_authorizations(write_authorization_id),
  kind TEXT NOT NULL,
  status TEXT NOT NULL,
  summary TEXT NOT NULL,
  observed_changes_json TEXT NOT NULL DEFAULT '[]',
  command_results_json TEXT NOT NULL DEFAULT '[]',
  artifact_refs_json TEXT NOT NULL DEFAULT '[]',
  created_at TEXT NOT NULL
);

CREATE TABLE artifacts (
  artifact_id TEXT PRIMARY KEY,
  task_id TEXT NOT NULL REFERENCES tasks(task_id),
  kind TEXT NOT NULL,
  uri TEXT NOT NULL,
  sha256 TEXT NOT NULL,
  size_bytes INTEGER NOT NULL,
  content_type TEXT,
  redaction_state TEXT NOT NULL,
  retention_class TEXT NOT NULL,
  created_at TEXT NOT NULL
);

CREATE TABLE artifact_links (
  artifact_link_id TEXT PRIMARY KEY,
  artifact_id TEXT NOT NULL REFERENCES artifacts(artifact_id),
  task_id TEXT NOT NULL REFERENCES tasks(task_id),
  record_kind TEXT NOT NULL,
  record_id TEXT NOT NULL,
  relation TEXT NOT NULL,
  created_at TEXT NOT NULL
);

CREATE TABLE task_blockers (
  blocker_id TEXT PRIMARY KEY,
  task_id TEXT NOT NULL REFERENCES tasks(task_id),
  blocked_action TEXT NOT NULL,
  blocker_kind TEXT NOT NULL,
  status TEXT NOT NULL,
  message TEXT NOT NULL,
  owner_ref_json TEXT NOT NULL DEFAULT '{}',
  created_at TEXT NOT NULL,
  resolved_at TEXT
);

CREATE TABLE task_events (
  event_id TEXT PRIMARY KEY,
  task_id TEXT REFERENCES tasks(task_id),
  event_seq INTEGER NOT NULL,
  event_type TEXT NOT NULL,
  state_version INTEGER NOT NULL,
  actor TEXT NOT NULL,
  payload_json TEXT NOT NULL DEFAULT '{}',
  created_at TEXT NOT NULL,
  UNIQUE(event_seq)
);

CREATE TABLE tool_invocations (
  invocation_id TEXT PRIMARY KEY,
  idempotency_key TEXT NOT NULL,
  request_hash TEXT NOT NULL,
  tool_name TEXT NOT NULL,
  task_id TEXT REFERENCES tasks(task_id),
  basis_state_version INTEGER NOT NULL,
  response_json TEXT NOT NULL,
  status TEXT NOT NULL,
  created_at TEXT NOT NULL,
  UNIQUE(tool_name, idempotency_key)
);
```

Recommended v0.1 indexes:

```sql
CREATE INDEX idx_tasks_project_phase ON tasks(project_id, lifecycle_phase);
CREATE INDEX idx_change_units_task_status ON change_units(task_id, status);
CREATE INDEX idx_write_authorizations_task_status ON write_authorizations(task_id, status);
CREATE UNIQUE INDEX uq_runs_write_authorization_consumed
  ON runs(write_authorization_id)
  WHERE write_authorization_id IS NOT NULL;
CREATE INDEX idx_artifact_links_record ON artifact_links(record_kind, record_id);
CREATE INDEX idx_task_blockers_task_status ON task_blockers(task_id, status);
CREATE INDEX idx_task_events_task_seq ON task_events(task_id, event_seq);
```

v0.1은 full natural-language intake system 없이 narrow owner-valid setup path로 initial task creation을 저장할 수 있습니다. Status/blocker output은 `tasks`, `change_units`, `write_authorizations`, `runs`, `artifacts`, `artifact_links`, `task_blockers`에서 직접 반환할 수 있습니다.

### Artifact directory layout

Directory layout은 staged입니다. v0.1은 실제로 쓰는 directory만 필요합니다. 보통 `artifacts/tmp/`, `artifacts/diffs/`, `artifacts/logs/`, 필요하면 `artifacts/bundles/` 정도입니다. Reference layout의 나머지 directory는 허용되지만 v0.1 requirement는 아닙니다.

### Artifact Kind Storage Notes

Artifact kind name은 registered file을 설명할 뿐 그 자체로 authority가 아닙니다. `diff`, `log`, `screenshot`, `bundle`, `manifest`, `checkpoint`, `qa`, `tdd`, `design`, `architecture`, `decision`, `export_component` file은 `artifacts` row와 compatible `artifact_links` row가 commit된 뒤에만 의미가 있습니다.

### Artifact Registration Contract

Artifact registration은 artifact poisoning을 막는 storage boundary입니다. Staged path, captured file, declared content type, requested owner relation은 Core가 path를 validate하고, traversal 또는 symlink escape를 reject하고, stored-byte integrity를 계산하고, redaction 또는 omission rule을 적용하고, `artifacts` row를 쓰고, compatible owner record에 link하기 전까지 untrusted입니다.

State를 support하는 committed artifact에는 다음이 필요합니다.

- [MCP API와 스키마](mcp-api-and-schemas.md#artifactref)가 담당하는 registered `ArtifactRef` shape와 active stage value set
- `sha256`, `size_bytes`, `redaction_state`, `retention_class`를 가진 `artifacts` row
- Task-scoped owner record를 위한 compatible `artifact_links` row 하나 이상
- committed artifact registration 또는 이를 등록한 state mutation을 나타내는 `task_events` row

Compatible owner link가 없는 `artifacts` row만으로는 evidence, QA, verification, projection, export, close-related check를 만족할 수 없습니다.

## v0.2 Additions

v0.2는 첫 사용자 가치 조각(First User-Value Slice)입니다. 사람이 작업을 이해하는 데 필요한 record를 추가합니다. Intake state, simplified user judgment, Approval 형태 민감 동작 Decision Packet, visible residual risk, evidence summary, close blocker/readiness, optional status-card freshness가 핵심입니다. 그래도 committed Approval lifecycle storage, full assurance, projection job, reconciliation, operations system은 피합니다.

### First User-Value Slice schema

Main v0.2 addition count: 5 tables입니다. Optional `decision_requests`와 `projection_status_cards` table을 추가할 수 있습니다. 이 table들은 v0.1 schema 위에 놓입니다.

```sql
CREATE TABLE task_intake (
  intake_id TEXT PRIMARY KEY,
  task_id TEXT NOT NULL REFERENCES tasks(task_id),
  user_request TEXT NOT NULL,
  clarified_summary TEXT,
  open_questions_json TEXT NOT NULL DEFAULT '[]',
  status TEXT NOT NULL,
  created_at TEXT NOT NULL,
  updated_at TEXT NOT NULL
);

CREATE TABLE decision_packets (
  decision_packet_id TEXT PRIMARY KEY,
  task_id TEXT NOT NULL REFERENCES tasks(task_id),
  judgment_route TEXT NOT NULL,
  judgment_category TEXT NOT NULL,
  display_depth TEXT NOT NULL,
  judgment_payload_json TEXT NOT NULL DEFAULT '{}',
  status TEXT NOT NULL,
  question TEXT NOT NULL,
  options_json TEXT NOT NULL DEFAULT '[]',
  selected_option_json TEXT,
  affected_scope_json TEXT NOT NULL DEFAULT '{}',
  affected_gates_json TEXT NOT NULL DEFAULT '[]',
  context_refs_json TEXT NOT NULL DEFAULT '[]',
  artifact_refs_json TEXT NOT NULL DEFAULT '[]',
  expires_at TEXT,
  resolved_at TEXT,
  created_at TEXT NOT NULL,
  updated_at TEXT NOT NULL
);

CREATE TABLE residual_risks (
  residual_risk_id TEXT PRIMARY KEY,
  task_id TEXT NOT NULL REFERENCES tasks(task_id),
  status TEXT NOT NULL,
  visibility_status TEXT NOT NULL,
  summary TEXT NOT NULL,
  impact TEXT,
  mitigation TEXT,
  related_decision_packet_id TEXT REFERENCES decision_packets(decision_packet_id),
  accepted_at TEXT,
  accepted_by TEXT,
  accepted_risk_json TEXT NOT NULL DEFAULT '{}',
  created_at TEXT NOT NULL,
  updated_at TEXT NOT NULL
);

CREATE TABLE evidence_summaries (
  evidence_summary_id TEXT PRIMARY KEY,
  task_id TEXT NOT NULL REFERENCES tasks(task_id),
  status TEXT NOT NULL,
  summary TEXT NOT NULL,
  run_refs_json TEXT NOT NULL DEFAULT '[]',
  artifact_refs_json TEXT NOT NULL DEFAULT '[]',
  gaps_json TEXT NOT NULL DEFAULT '[]',
  updated_at TEXT NOT NULL
);

CREATE TABLE close_readiness (
  close_readiness_id TEXT PRIMARY KEY,
  task_id TEXT NOT NULL REFERENCES tasks(task_id),
  status TEXT NOT NULL,
  blocker_refs_json TEXT NOT NULL DEFAULT '[]',
  evidence_summary_id TEXT REFERENCES evidence_summaries(evidence_summary_id),
  residual_risk_refs_json TEXT NOT NULL DEFAULT '[]',
  checked_state_version INTEGER NOT NULL,
  updated_at TEXT NOT NULL
);
```

Optional v0.2 prompt routing table:

이 v0.2 addition의 public ref는 의도적으로 작게 유지합니다. `evidence_summaries`와 `close_readiness`는 `StateRecordRef`의 `evidence_summary`, `close_readiness`로 surface될 수 있습니다. 둘은 authority ref를 요약하거나 close check를 보조할 뿐이며, full `evidence_manifests`, verification, 수동 QA, projection, report/export profile이 active라는 뜻이 아닙니다.

```sql
CREATE TABLE decision_requests (
  decision_request_id TEXT PRIMARY KEY,
  task_id TEXT NOT NULL REFERENCES tasks(task_id),
  decision_packet_id TEXT REFERENCES decision_packets(decision_packet_id),
  status TEXT NOT NULL,
  request_payload_json TEXT NOT NULL DEFAULT '{}',
  created_at TEXT NOT NULL,
  updated_at TEXT NOT NULL,
  expires_at TEXT
);
```

`decision_requests`는 그 자체로 judgment, gate, waiver, residual-risk acceptance, close condition을 satisfy하지 않습니다. Compatible `decision_packets` row를 위한 routing 또는 replay metadata일 뿐입니다.

`judgment_route=approve-sensitive-action`에서 minimum v0.2는 요청된 `judgment_payload.approval_scope`를 `decision_packets.judgment_payload_json`에 저장하고, 사용자의 grant, denial, expiry를 Decision Packet에서 해소합니다. `approvals` row, Approval `StateRecordRef`, `approval_id`, `approval_refs`, `APR` projection은 요구하지 않습니다.

Optional v0.2 status-card freshness table:

```sql
CREATE TABLE projection_status_cards (
  card_id TEXT PRIMARY KEY,
  task_id TEXT NOT NULL REFERENCES tasks(task_id),
  card_kind TEXT NOT NULL,
  source_state_version INTEGER NOT NULL,
  rendered_state_version INTEGER,
  status TEXT NOT NULL,
  summary_json TEXT NOT NULL DEFAULT '{}',
  updated_at TEXT NOT NULL
);
```

`projection_status_cards`는 projection job system이 아닙니다. Status 또는 next-action card를 위한 optional display/freshness cache입니다. 생략한다면 v0.2는 current `tasks.state_version`과 read response의 source version을 비교해 freshness를 직접 계산할 수 있습니다.

## Future / Later Profile Schema Candidates

이 절은 유용한 future schema candidate를 보존하되 v0.1/v0.2 implementation path에 올리지 않습니다. 이 inventory를 required DDL bundle로 취급하지 않습니다.

### Agency Assurance schema

Agency Assurance profile storage는 v0.3 또는 profile-promoted 범위입니다. v0.1/v0.2가 아닙니다. Candidate table:

| Candidate table | 나중에 필요한 이유 | Required가 아닌 범위 |
|---|---|---|
| `approvals` | Sensitive-action approval lifecycle과 drift handling | v0.1 authority loop, Approval 형태 Decision Packet을 포함한 ordinary v0.2 judgment display |
| `baselines` | Assurance, approval, verification freshness를 위한 repository baseline capture | promoted profile이 baseline check를 요구하지 않는 v0.1/v0.2 |
| `evidence_manifests` | Full criteria-to-evidence coverage | v0.1 single artifact/evidence ref, v0.2 evidence summary |
| `evals` | Detached verification 또는 evaluator review | v0.1/v0.2 |
| `manual_qa_records` | Manual QA result, finding, setup, evidence refs | v0.1/v0.2 |
| `validator_runs` | Persisted `ValidatorResult` row | narrow owner가 validator를 승격하지 않은 v0.1/v0.2 |
| `feedback_loops` | Selected feedback loop를 위한 policy support | v0.1/v0.2 |
| `tdd_traces` | TDD profile이 선택될 때 red/green/refactor evidence | v0.1/v0.2 |

### Operations schema

Operations profile storage는 v0.4 또는 profile-promoted 범위입니다. Candidate table:

| Candidate table | 나중에 필요한 이유 | Required가 아닌 범위 |
|---|---|---|
| `projection_jobs` | Rendered Markdown 또는 managed output을 위한 durable outbox | v0.1/v0.2, optional status card는 이 table을 요구하지 않음 |
| `reconcile_items` | Human edit 또는 projection drift를 Core decision으로 라우팅 | v0.1/v0.2 |
| `connector_manifests` | Connector-managed file과 drift 추적 | v0.1/v0.2 |
| `persistent_locks` | Process lock만으로 부족할 때 durable lock/recovery metadata | v0.1/v0.2 |
| `export_manifests` | Release handoff 또는 export package metadata | v0.1/v0.2 |
| `recover_items` | Recovery finding, repair plan, operator follow-up | v0.1/v0.2 |

### Future / diagnostic schema

Future 또는 diagnostic schema candidate는 owner가 승격하기 전까지 non-stage-required입니다.

- Journey/spine: `task_spine_entries`, `journey_cards`
- Domain and stewardship: `domain_terms`, `module_map_items`, `interface_contracts`
- Rich design support: `shared_designs`, `change_unit_dependencies`
- Diagnostics and polish: metrics, dashboard, context index, connector analytics, export/recover detail table, richer projection cache

이 record들은 유용할 수 있지만 v0.1 Core Authority Smoke나 v0.2 First User-Value Slice의 전제 조건이 되어서는 안 됩니다.

### Baseline capture format

Baseline capture는 future assurance/profile feature입니다. 승격되면 approval, verification, evidence freshness에 사용한 repository state를 증명할 만큼 기록해야 합니다. 그 profile이 active가 되기 전까지 v0.1/v0.2에는 `baselines` table이나 baseline capture runner가 필요하지 않습니다.

### Verification Bundle Shape

Verification bundle은 future assurance/profile artifact입니다. Verification profile이 활성 상태일 때 baseline ref, run ref, artifact ref, evaluator input, validation output을 묶을 수 있습니다. v0.1 Run 또는 v0.2 evidence summary를 기록하기 위한 requirement가 아닙니다.

### Projection job table

`projection_jobs`는 Operations profile storage입니다. Full projection support가 enabled일 때 projection rendering을 위한 durable outbox입니다. v0.1의 일부가 아니며 v0.2 status 또는 next-action card에도 required가 아닙니다.

승격되면 projection job은 `projection_kind`, `target_ref`, `source_state_version`, job status, output location 또는 artifact ref, failure information을 기록해야 합니다. 이 field들은 derived output freshness를 설명합니다. Rendered Markdown을 authoritative하게 만들지 않습니다.

### Projection Worker Execution

Projection worker는 committed Core state를 읽고 derived file 또는 card를 만듭니다. Projection failure는 committed Core state를 rollback하면 안 됩니다. Worker가 freshness/job state를 Core-compatible ordering으로 update할 수는 있지만, stale 또는 failed projection output은 write를 authorize하거나, evidence를 satisfy하거나, acceptance를 record하거나, residual risk를 accept하거나, Task를 close할 수 없습니다.

### Validator runner skeleton

Persisted `validator_runs`는 owner가 좁은 validator를 명시적으로 earlier stage에 승격하지 않는 한 Agency Assurance profile behavior입니다. v0.1/v0.2는 persisted validator-run storage를 만들지 않고 structured blocker를 반환할 수 있습니다.

### Evidence and Verification Profile Implementation Notes

Full evidence sufficiency, detached verification, Manual QA, validator-backed assurance는 installed profile의 committed state와 registered artifact를 읽습니다. Markdown, chat, unregistered tool output으로 이를 simulate하면 안 됩니다.

## Event Semantics

### `task_events`

`task_events`는 append-only audit trail과 event-order support table입니다. Core가 무엇을 어떤 순서로 commit했는지 기록합니다. 일반 동작에서 current state의 authority source가 아니며, v0.1/v0.2 state는 보통 event replay로 재구성하지 않습니다.

Current state table이 authoritative합니다.

- `tasks`, `change_units`, `write_authorizations`, `runs`, `artifacts`, `artifact_links`, `task_blockers`는 v0.1 authority record입니다.
- `decision_packets`, `residual_risks`와 그 밖의 v0.2 row는 해당 profile이 활성 상태일 때 자신의 record family에 대해서만 authority가 됩니다.
- Event는 audit, debugging, idempotency explanation, projection freshness, recovery history를 support합니다.

Deterministic event order는 `state.sqlite` 안에서 ascending `event_seq`입니다. Task-scoped reader는 `task_id`로 filter합니다. `created_at`은 audit metadata이며 여러 event가 같은 timestamp를 가질 때 ordering에 충분하지 않습니다.

Required event emission:

| Stage | Mutation | Event expectation |
|---|---|---|
| v0.1 | Project registration 또는 project path/config update | Project state가 바뀌면 project 또는 task-scoped event를 emit합니다. Registry-only event는 `task_id=NULL`을 사용할 수 있습니다. |
| v0.1 | Task create/update/close state change | New state version과 함께 event를 emit합니다. |
| v0.1 | Change Unit 또는 task boundary create/update | Event를 emit하고 affected state version을 update합니다. |
| v0.1 | `prepare_write` allow가 Write Authorization을 create 또는 refresh | authorization-created 또는 authorization-updated event를 emit합니다. |
| v0.1 | `prepare_write`가 block하고 structured blocker를 store/update | blocker-opened 또는 blocker-updated event를 emit합니다. |
| v0.1 | `record_run`이 Run을 commit | run-recorded event를 emit합니다. Write Authorization을 consume한다면 같은 transaction 또는 payload에서 authorization-consumed relation을 남깁니다. |
| v0.1 | Artifact registration/link commit | artifact-registered event를 emit하거나 owning mutation event에 artifact refs를 포함합니다. |
| v0.1 | Blocker resolved 또는 superseded | blocker-resolved 또는 blocker-superseded event를 emit합니다. |
| v0.1 | Idempotent replay가 existing committed response를 return | 새 semantic event를 append하지 않습니다. Original event가 committed audit fact로 남습니다. |
| v0.2 | Intake state create/update | Persisted라면 intake-updated event를 emit합니다. |
| v0.2 | User judgment requested, answered, expired, superseded | `decision_packets` row와 연결된 decision event를 emit합니다. |
| v0.2 | Residual risk opened, changed, accepted, mitigated, deferred, superseded | residual-risk event를 emit합니다. |
| v0.2 | Evidence summary 또는 close readiness changes | Persisted라면 evidence-summary-updated 또는 close-readiness-updated event를 emit합니다. |
| v0.2 optional | Projection/status-card freshness changes | Optional table이 installed된 경우에만 freshness/status-card event를 emit합니다. |

Malformed request, dry run, pre-commit state conflict, state를 mutate하지 않는 invalid request에는 `task_events` row가 필요하지 않습니다. Blocked request가 stored blocker를 create/update한다면 그 blocker mutation이 event-worthy state change입니다.

### Projection freshness without projection authority

Projection freshness는 readable output의 `source_state_version`과 current Core state를 비교한 값입니다. Readable output을 state authority로 만들지 않습니다.

- v0.1은 projection freshness table이 없어도 됩니다. Read는 current state를 직접 반환할 수 있습니다.
- v0.2는 status 또는 next-action card를 위해 optional `projection_status_cards`를 저장할 수 있습니다.
- v0.4 또는 promoted profile은 durable rendering을 위해 `projection_jobs`를 추가할 수 있습니다.

모든 stage에서 stale Markdown 또는 stale card는 owner path를 통해 warning 또는 trust blocker가 될 수 있습니다. 하지만 write를 authorize하거나, evidence를 satisfy하거나, acceptance를 record하거나, residual risk를 accept하거나, Task를 close할 수는 없습니다.

## Migration And Validation Notes

이 저장소에는 migration runner가 없습니다. 아래 note는 향후 구현이 migration mechanism을 선택할 때 지켜야 할 constraint입니다.

### 권한 경계로서의 Storage hardening

SQLite는 Core와 migration이 막지 않으면 malformed row도 저장할 수 있습니다. Row는 owner schema, owner value set, state-version basis, idempotency key, artifact owner-link contract에 맞을 때만 authoritative합니다.

`doctor`, `recover`, artifact check, conformance runner는 malformed JSON, unknown owner-bound value, mismatched replay row, stale state-version claim, artifact hash mismatch, invalid owner link를 projection drift가 아니라 storage integrity finding으로 보고해야 합니다.

### JSON TEXT validation

JSON `TEXT` column은 storage flexibility를 위한 것이지 arbitrary JSON을 저장하라는 뜻이 아닙니다. Core가 JSON `TEXT` value를 commit하기 전에는 값을 parse하고 parsed shape를 owner에 맞게 validate해야 합니다.

- API-shaped payload는 [MCP API와 스키마](mcp-api-and-schemas.md)에 맞게 validate합니다.
- Storage-only JSON은 이 문서 또는 이 문서가 이름 붙인 owner document에 맞게 validate합니다.
- `'{}'`, `'[]'` 같은 SQLite default는 storage representation rule입니다. Public API field를 optional로 만들지 않습니다.

Malformed JSON과 schema-incompatible JSON은 invalid state입니다. SQLite build가 JSON check를 지원한다면 migration은 `CHECK (json_valid(column_name))`을 방어적 보강으로 추가할 수 있습니다. 그래도 commit 전 Core shape validation이 의미를 소유합니다.

### Canonical enum hardening

Status-like `TEXT` column은 open string이 아닙니다. Allowed value는 Core validation이 소유합니다. Database `CHECK` constraint나 lookup table은 방어적 보강입니다.

Early hardening 대상:

| Field(s) | Owner/value source |
|---|---|
| `tasks.mode`, `tasks.lifecycle_phase`, `tasks.result` | [커널 참조](kernel.md) |
| `change_units.status` | Kernel/Change Unit owner rules |
| `write_authorizations.status` | [Kernel `prepare_write`](kernel.md#prepare_write)와 [`harness.prepare_write`](mcp-api-and-schemas.md#harnessprepare_write) |
| `runs.kind`, `runs.status` | [`harness.record_run`](mcp-api-and-schemas.md#harnessrecord_run)와 storage compatibility notes |
| `task_blockers.status`, `blocked_action`, `blocker_kind` | Kernel/API blocker owners |
| `tool_invocations.status` | storage idempotency replay semantics |
| `decision_packets.status`, `judgment_route`, `judgment_category`, `display_depth` | user-judgment API/kernel owners |
| `residual_risks.status`, `visibility_status` | close와 residual-risk owners |
| `evidence_summaries.status`, `close_readiness.status` | evidence/close-readiness owner behavior |
| Future `projection_jobs.status`, `projection_jobs.projection_kind` | Operations profile active 시 Projection/API owners |
| Future `validator_runs.status` | assurance profile active 시 `ValidatorResult` owner |
| Future `project_surfaces.guarantee_level`, `write_authorizations.guarantee_level`, `validator_runs.guarantee_level` | Relevant profile이 active일 때 security threat model과 agent-integration guarantee-level owners |
| Future `approvals.status` | Approval profile이 active일 때 Approval lifecycle owner |
| Future `evidence_manifests.status` | Full Evidence Manifest profile이 active일 때 Evidence profile owner |
| Future `feedback_loops.loop_kind`, `feedback_loops.status`, `tdd_traces.status` | Feedback/TDD profile이 active일 때 design-quality/API owners |
| Future `connector_manifests.status`, `baselines.status`, `decision_requests.status`, `task_spine_entries.status`, `change_unit_dependencies.status`, `shared_designs.status`, `reconcile_items.status`, `domain_terms.status`, `module_map_items.status`, `interface_contracts.review_status` | 아래 storage compatibility values. Optional/future table이 retained되거나 seeded되거나 활성 상태인 경우에만 적용합니다. |

Unknown owner-bound value는 fixture가 invalid-state recovery를 명시적으로 다루지 않는 한 invalid state입니다. Migration은 unknown value가 있을 때 tightening 전에 멈춰야 합니다. Owner가 정의하지 않은 fallback meaning으로 조용히 mapping하면 안 됩니다.

Storage-owned compatibility value:

| Field | Durable values | Meaning |
|---|---|---|
| `runs.status` | `completed`, `interrupted`, `blocked`, `violation` | Committed Run row입니다. `completed`만 normal owner ref를 통해 evidence를 support할 수 있습니다. 다른 값은 audit/recovery record이며 그 자체로 evidence, QA, verification, acceptance, close readiness를 satisfy하지 않습니다. |
| `change_units.status` | `planned`, `active`, `completed`, `deferred`, `superseded` | Scope lifecycle입니다. Active compatible Change Unit만 new write를 scope합니다. |
| `write_authorizations.status` | `active`, `consumed`, `expired`, `revoked`, `blocked` | Durable authorization lifecycle입니다. `active`이고 compatible한 row만 `record_run`이 consume할 수 있습니다. |
| `task_blockers.status` | `open`, `resolved`, `superseded` | Stored blocker lifecycle입니다. Open blocker는 Core가 resolve 또는 supersede할 때까지 visible 상태로 남습니다. |
| `tool_invocations.status` | `committed` | Committed replayable response에 대해서만 row가 존재합니다. |
| `residual_risks.status` | `open`, `accepted`, `mitigated`, `deferred`, `superseded` | Residual-risk lifecycle입니다. Accepted risk는 work acceptance와 분리됩니다. |

Future seed loader와 optional profile implementation을 위해 보존하는 profile-only compatibility value:

| Field | Durable values | Meaning |
|---|---|---|
| `baselines.status` | `captured`, `stale` | Assurance profile의 baseline freshness입니다. |
| `connector_manifests.status` | `current`, `drifted` | Connector-managed file state입니다. Drift는 owning reconcile/operations path로 라우팅해야 합니다. |
| `decision_requests.status` | `open`, `linked`, `closed`, `expired`, `cancelled`, `superseded` | Prompt routing lifecycle일 뿐입니다. Authority는 linked `decision_packets`를 통해 생깁니다. |
| `task_spine_entries.status` | `current`, `superseded` | Journey/spine continuity support이며 current state authority가 아닙니다. |
| `change_unit_dependencies.status` | `open`, `satisfied`, `blocked`, `deferred`, `superseded` | Dependency visibility입니다. Scheduler나 parallel-lane authority가 아닙니다. |
| `shared_designs.status` | `proposed`, `active`, `stale`, `deferred`, `superseded` | Design-support basis입니다. Approval, work acceptance, residual-risk acceptance가 아닙니다. |
| `reconcile_items.status` | `pending`, `merged`, `rejected`, `converted_to_note`, `decision_created`, `deferred` | Reconcile outcome state입니다. Accepted Core mutation만 authority를 바꿉니다. |
| `domain_terms.status` | `active`, `conflict` | Domain-language support입니다. Conflict는 owner path가 해소할 때까지 visible 상태로 남습니다. |
| `module_map_items.status` | `active` | Profile이 active일 때 current usable module-map support record입니다. |
| `interface_contracts.review_status` | `pending`, `reviewed` | Interface review support입니다. Risk를 waive하거나 gate를 override하지 않습니다. |

Future table value set은 해당 table의 owner profile이 active이거나, fixture가 optional table을 명시적으로 seed하거나, owner document가 값을 명시적으로 승격할 때만 사용해야 합니다.

### Migrations

Future migration은 다음을 지켜야 합니다.

- `registry_meta`와 `project_state` 또는 선택한 equivalent metadata mechanism에 schema/profile version을 기록합니다.
- Constraint를 tighten하기 전에 JSON과 owner-bound status value를 validate합니다.
- `task_events.event_seq` order를 보존하고 historical ordering을 rewrite하지 않습니다.
- Artifact hash와 owner link를 보존하거나, 영향을 받는 artifact를 recovery 대상으로 invalid 표시합니다.
- Unknown owner-bound enum/status value가 있으면 owner가 소유하지 않은 fallback meaning을 만들지 말고 stop합니다.
- Projection/card/job freshness는 derived state로 취급하고 canonical state로 취급하지 않습니다.

이 note는 v0.1에 특정 migration runner, migration file format, CLI command를 요구하지 않습니다.

### Lock policy

Runtime mutation은 [런타임 아키텍처](runtime-architecture.md#state-transaction-flow)가 담당하는 Core transaction order를 통해 serialize해야 합니다. v0.1은 ordinary SQLite transaction과 필요 시 process/project lock으로 충분할 수 있습니다. `persistent_locks`는 later Operations candidate이지 v0.1 table이 아닙니다.

Lock은 concurrent write를 보호합니다. OS sandboxing, artifact integrity, tamper-proof storage를 제공하지 않습니다.
