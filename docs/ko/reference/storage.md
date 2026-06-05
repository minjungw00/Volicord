# Storage

## 이 문서가 담당하는 것

이 문서는 향후 local Harness Server를 위한 reference 문서입니다. 이 저장소에는
아직 database, migration runner, server, runtime이 없습니다. 현재 저장소 단계와
구현 인계 상태는 [구현 개요](../build/implementation-overview.md#문서-수락-상태)에
있습니다.

이 문서는 MVP-1에 필요한 최소 저장 모델, Runtime Home identity, SQLite schema
sketch, storage-owned JSON `TEXT` 규칙, enum hardening, artifact/evidence 참조
저장, 지속 기록과 파생 상태 보기의 경계를 담당합니다. Stage 순서와 exit
criteria는 [Build: MVP-1 사용자 작업 루프](../build/mvp-user-work-loop.md)와
[내부 엔지니어링 점검](../build/engineering-checkpoint.md)을 봅니다.

## 이런 때 읽기

- MVP-1에 필요한 가장 작은 storage model을 확인할 때.
- Core-owned state와 chat, Markdown projection, connector output, tool output,
  report text를 분리할 때.
- close blocker 계산에 필요한 field를 확인할 때.
- later-profile table이 MVP-1 requirement처럼 보이지 않게 점검할 때.

## 관련 Owner

| 관심사 | Owner |
|---|---|
| Public MCP request/response shape | [MVP API](api/mvp-api.md)와 [API Schema Core](api/schema-core.md) |
| `ArtifactRef`, staged active ref kind, idempotency, state conflict behavior | [API Schema Core](api/schema-core.md#artifactref), [API Schema Core: Stage-Specific Active Value Sets](api/schema-core.md#stage-specific-active-value-sets), [API Errors](api/errors.md) |
| Task lifecycle, gate, `prepare_write`, `record_run`, `close_task`, stable event | [Core Model 참조](core-model.md) |
| Core process model, transaction order, lock, projection/reconcile placement | [런타임 아키텍처 참조](runtime-architecture.md) |
| Projection authority, freshness, managed block, rendered template | [Projection과 Template 참조](projection-and-templates.md)와 [Template 참조](templates/README.md) |
| Operator behavior, doctor/recover/export/reconcile/conformance entrypoint | [운영과 Conformance 참조](operations-and-conformance.md) |
| Fixture format과 assertion semantics | [Conformance Fixtures 참조](conformance-fixtures.md) |
| Stage sequence와 implementation readiness | [Build: MVP-1 사용자 작업 루프 계획](../build/mvp-user-work-loop.md), [구현 개요](../build/implementation-overview.md) |

## MVP-1 저장 목표

MVP-1 storage는 사용자가 chat memory나 generated Markdown을 신뢰하지 않아도
현재 작업을 이해할 수 있게 하는 가장 작은 로컬 기준 기록을 보관합니다.
Project identity, tracked Task, task scope, 사용자 소유 판단, 협력형 쓰기 확인
결과, Run, evidence pointer, blocker가 핵심입니다.

MVP-1 storage는 Journey system, report system, projection job system,
conformance runner, QA database, Eval store, export pipeline, dashboard data model이
아닙니다. 이런 record는 나중에 유용할 수 있지만, owner profile이 명시적으로
승격하기 전까지 MVP-1 밖에 둡니다.

MVP-1 storage는 다음 authority boundary를 지켜야 합니다.

- Core-owned state row가 현재 Harness state의 기준입니다.
- `task_events`를 유지하더라도 audit와 ordering trail입니다. 일반 동작에서 current
  state를 재구성하는 기준 source가 아닙니다.
- Evidence pointer는 Core가 기록하고 compatible owner record에 연결하기 전까지
  evidence authority가 아닙니다.
- Chat, Markdown projection, generated report, connector manifest, tool output,
  operator output은 Core mutation이 owner-valid state row 또는 evidence ref를
  기록하기 전까지 권한 근거가 아닙니다.
- Status card, task summary, close readiness, evidence summary, next action,
  projection freshness는 파생된 상태 보기입니다. Stale, failed, absent이거나 다시
  계산되어도 persisted state record를 바꾸지 않습니다.
- Future/profile table은 owning profile 또는 tool path가 활성 상태이거나 사용될 때만
  required가 됩니다.

## Runtime Home Identity와 위험

Harness는 local Runtime Home 하나와 registered project별 state database 하나를 둡니다.
기본 reference location은 `~/.harness`이지만 구현은 configured equivalent를 선택할 수
있습니다.

기준 layout:

```text
~/.harness/
  registry.sqlite
  projects/
    PRJ-0001/
      project.yaml
      state.sqlite
      evidence/
        tmp/
        diffs/
        logs/
        screenshots/
        checkpoints/
```

`registry.sqlite`는 Runtime Home identity와 project registration을 저장합니다.
`project.yaml`은 static project configuration만 저장합니다. `state.sqlite`는
project-local Core state를 저장합니다. Evidence directory는 Core가 evidence
registration boundary를 적용한 뒤 등록된 file이나 pointer를 보관합니다.

`project.yaml`은 current Task state, current gate, 쓰기 전 범위 확인이나 Write Authorization,
evidence sufficiency, 작업 수락, 잔여 위험 수용을 저장하면 안 됩니다.

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
cooperative/detective입니다. File permission, owner check, hash, `doctor` finding은
방어적 보강입니다. 그 자체로 OS-level sandboxing, arbitrary-tool control,
tamper-proof storage, pre-execution blocking을 만들지 않습니다.

| 관찰 사항 | Storage 의미 |
|---|---|
| Runtime Home 또는 project storage의 owner/mode를 확인할 수 없습니다. | Unknown 또는 weak local file posture를 보고합니다. OS-level guarantee를 주장하지 않습니다. |
| Runtime Home, `state.sqlite`, `registry.sqlite`, evidence directory가 unrelated user, shared group, shared container, broad local process에게 writable입니다. | Tampering과 evidence-poisoning risk를 보고합니다. Core는 row, owner link, hash, evidence registration을 검증한 뒤에만 의미를 신뢰해야 합니다. |
| Evidence storage 또는 export가 unrelated user, shared group, shared container, broad local process에게 readable입니다. | 민감한 값을 그대로 출력하지 않고 confidentiality risk를 보고합니다. |
| Registered evidence hash, size, owner link, path가 storage metadata와 더 이상 맞지 않습니다. | Projection drift가 아니라 evidence integrity failure 또는 recovery input으로 취급합니다. |

## Core Records

MVP-1에는 작은 persisted record set만 있습니다. 향후 구현은 약간 다른 physical table
layout을 선택할 수 있습니다. 그래도 later-profile record를 MVP-1 requirement로 만들면
안 됩니다.

| Record | 최소 persisted 목적 | 메모 |
|---|---|---|
| `project` | Local project identity, Runtime Home registration, state database location, active Task pointer. | 아래 sketch에서는 `registry_meta`, `projects`, `project_state`에 나뉘어 저장됩니다. |
| `task` | Tracked work item입니다. User request, current summary, lifecycle, result, active scope, state clock을 저장합니다. | Task는 user-value unit입니다. Report, Journey, projection이 아닙니다. |
| `task_scope` / `change_unit` | Current scope, non-goals, success criteria, allowed paths, denied paths, scoped-write status. | 기존 Core/API 이름은 `Change Unit`과 `record_kind=change_unit`을 씁니다. MVP-1 storage에는 active task-scope row 하나 또는 같은 의미의 Task scope field면 충분합니다. DAG는 필요하지 않습니다. |
| `user_judgment` | 사용자 소유 제품/UX 판단, 기술 판단, 민감 동작 승인, 작업 수락, 잔여 위험 수용. | Full-format Decision Packet은 presentation이지 별도 authority table이 아닙니다. Committed `approvals`는 later-profile입니다. |
| `write_check` / `write_authorization` | 정확한 proposed write에 대한 cooperative `prepare_write` decision과, `dry_run=false`이며 `decision=allowed`일 때만 생기는 durable Write Authorization row입니다. Dry-run result는 diagnostic 또는 candidate effect만 반환합니다. `dry_run=false`인 `blocked`, `approval_required`, `decision_required` result는 경우에 따라 blocker, validator finding, error, committed replayable response를 만들 수 있지만 authorization row를 만들지 않습니다. `state_conflict` result는 conflict를 보고하되 새 authorization row를 만들거나 새 replay row를 merge하지 않습니다. Exact idempotent replay는 original committed response를 반환합니다. | Core path에 대한 Harness 기록/확인입니다. OS-level permission이나 arbitrary-tool prevention이 아닙니다. |
| `run` | Task, scope, optional Write Authorization, evidence refs에 연결되는 agent work run 또는 observed execution result. | Run은 registered ref를 통해서만 evidence를 support할 수 있습니다. 그 자체로 verification, QA, acceptance, close를 증명하지 않습니다. |
| `evidence_ref` | Diff, log, screenshot, checkpoint, existing artifact ref 같은 evidence의 pointer와 short summary. | MVP-1에는 detailed Evidence Manifest가 필요하지 않습니다. 큰 byte는 state에 embed하지 않고 참조합니다. |
| `blocker` | Owner refs와 smallest required next action을 가진 close blocker 또는 next-action blocker. | Close readiness는 open blocker와 owner record에서 파생됩니다. MVP-1에 별도 `close_readiness` table이 필요하지 않습니다. |

`tool_invocations`, `task_events` 같은 support row는 replay, idempotency, audit, ordering을
돕습니다. User-facing domain record가 아니며 MVP-1 product surface를 넓히지 않습니다.
`tool_invocations` row는 committed replayable `dry_run=false` response에 대해서만 존재합니다.
Dry run과 pre-commit conflict는 storage에서 `idempotency_key`를 예약하지 않습니다.

## Persisted State와 파생된 상태 보기

Persisted MVP-1 state는 Core가 commit한 row set입니다. Derived status/view는 Core 또는
renderer가 그 row에서 계산해 사용자, agent, operator에게 보여주는 값입니다.

| 파생 output | Source records | MVP-1 storage rule |
|---|---|---|
| Status card / task summary | `tasks`, `change_units`, `user_judgments`, `write_authorizations`, `runs`, `evidence_refs`, `blockers` | 파생된 상태 보기입니다. Read 시 다시 계산할 수 있습니다. `projection_status_cards`나 `projection_jobs` table은 필요하지 않습니다. |
| Next safe actions | Open blockers, pending user judgments, write-check state, evidence refs, Task lifecycle | 파생된 상태 보기입니다. Task, judgment, Run, evidence, Write Authorization을 만들지 않습니다. |
| Evidence summary | `runs`와 `evidence_refs` | 파생 summary입니다. MVP-1은 ref와 short summary를 저장하며 full `evidence_summaries` 또는 `evidence_manifests` authority table을 요구하지 않습니다. |
| Close readiness | Task lifecycle, scope state, pending user judgments, evidence refs, open blockers, work-acceptance와 residual-risk user judgment | 파생 check입니다. MVP-1은 check에 쓰는 blocker와 owner record를 저장하고, 별도 `close_readiness` 기준 source를 만들지 않습니다. |
| Projection freshness | Current state version과 read/view가 반환한 source version 비교 | 파생 diagnostic입니다. Full `projection_jobs` storage는 운영 프로필 또는 profile-promoted scope입니다. |

## Minimal DDL 또는 Schema Sketch

아래 DDL은 planning을 위한 reference sketch입니다. Migration runner가 이미 존재한다는
증거가 아닙니다. MVP-1을 위의 최소 record에 맞춰 좁게 유지합니다.

<a id="core-authority-smoke-schema"></a>
<a id="mvp-1-minimal-storage-schema"></a>

### `registry.sqlite`

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

MVP-1에서 required `registry_meta` key는 `runtime_home_id`와 `schema_version`입니다.

### `state.sqlite`

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
  user_request TEXT NOT NULL,
  mode TEXT NOT NULL,
  lifecycle_phase TEXT NOT NULL,
  result TEXT NOT NULL DEFAULT 'none',
  summary TEXT NOT NULL DEFAULT '',
  active_change_unit_id TEXT,
  state_version INTEGER NOT NULL DEFAULT 0,
  created_at TEXT NOT NULL,
  updated_at TEXT NOT NULL,
  closed_at TEXT
);

CREATE TABLE change_units (
  change_unit_id TEXT PRIMARY KEY,
  task_id TEXT NOT NULL REFERENCES tasks(task_id),
  scope_summary TEXT NOT NULL,
  non_goals_json TEXT NOT NULL DEFAULT '[]',
  success_criteria_json TEXT NOT NULL DEFAULT '[]',
  allowed_paths_json TEXT NOT NULL DEFAULT '[]',
  denied_paths_json TEXT NOT NULL DEFAULT '[]',
  status TEXT NOT NULL,
  created_at TEXT NOT NULL,
  updated_at TEXT NOT NULL
);

CREATE TABLE user_judgments (
  user_judgment_id TEXT PRIMARY KEY,
  task_id TEXT NOT NULL REFERENCES tasks(task_id),
  change_unit_id TEXT REFERENCES change_units(change_unit_id),
  judgment_type TEXT NOT NULL,
  presentation TEXT NOT NULL,
  display_label TEXT NOT NULL,
  status TEXT NOT NULL,
  question TEXT NOT NULL,
  options_json TEXT NOT NULL DEFAULT '[]',
  selected_option_json TEXT,
  judgment_payload_json TEXT NOT NULL DEFAULT '{}',
  affected_scope_json TEXT NOT NULL DEFAULT '{}',
  affected_gates_json TEXT NOT NULL DEFAULT '[]',
  context_refs_json TEXT NOT NULL DEFAULT '[]',
  artifact_refs_json TEXT NOT NULL DEFAULT '[]',
  expires_at TEXT,
  resolved_at TEXT,
  created_at TEXT NOT NULL,
  updated_at TEXT NOT NULL
);

CREATE TABLE write_authorizations (
  write_authorization_id TEXT PRIMARY KEY,
  task_id TEXT NOT NULL REFERENCES tasks(task_id),
  change_unit_id TEXT NOT NULL REFERENCES change_units(change_unit_id),
  status TEXT NOT NULL,
  basis_state_version INTEGER NOT NULL,
  intended_operation TEXT NOT NULL,
  allowed_paths_json TEXT NOT NULL DEFAULT '[]',
  denied_paths_json TEXT NOT NULL DEFAULT '[]',
  related_user_judgment_refs_json TEXT NOT NULL DEFAULT '[]',
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
  evidence_ref_ids_json TEXT NOT NULL DEFAULT '[]',
  created_at TEXT NOT NULL
);

CREATE TABLE evidence_refs (
  evidence_ref_id TEXT PRIMARY KEY,
  task_id TEXT NOT NULL REFERENCES tasks(task_id),
  run_id TEXT REFERENCES runs(run_id),
  owner_record_kind TEXT NOT NULL,
  owner_record_id TEXT NOT NULL,
  kind TEXT NOT NULL,
  uri TEXT NOT NULL,
  summary TEXT NOT NULL,
  sha256 TEXT,
  size_bytes INTEGER,
  content_type TEXT,
  redaction_state TEXT NOT NULL,
  status TEXT NOT NULL,
  created_at TEXT NOT NULL,
  updated_at TEXT NOT NULL
);

CREATE TABLE blockers (
  blocker_id TEXT PRIMARY KEY,
  task_id TEXT NOT NULL REFERENCES tasks(task_id),
  blocked_action TEXT NOT NULL,
  blocker_kind TEXT NOT NULL,
  status TEXT NOT NULL,
  message TEXT NOT NULL,
  owner_ref_json TEXT NOT NULL DEFAULT '{}',
  related_refs_json TEXT NOT NULL DEFAULT '[]',
  required_next_action TEXT NOT NULL,
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

Recommended MVP-1 indexes:

```sql
CREATE INDEX idx_tasks_project_phase ON tasks(project_id, lifecycle_phase);
CREATE INDEX idx_change_units_task_status ON change_units(task_id, status);
CREATE INDEX idx_user_judgments_task_status ON user_judgments(task_id, status);
CREATE INDEX idx_write_authorizations_task_status ON write_authorizations(task_id, status);
CREATE UNIQUE INDEX uq_runs_write_authorization_consumed
  ON runs(write_authorization_id)
  WHERE write_authorization_id IS NOT NULL;
CREATE INDEX idx_evidence_refs_owner ON evidence_refs(owner_record_kind, owner_record_id);
CREATE INDEX idx_blockers_task_status ON blockers(task_id, status);
CREATE INDEX idx_task_events_task_seq ON task_events(task_id, event_seq);
```

구현이 MVP-1에서 separate `artifacts`와 `artifact_links` table을 유지한다면, 그 table은
`evidence_ref` storage의 physical representation일 뿐입니다. Full artifact-integrity,
export, projection-linking, Evidence Manifest profile을 만든다는 뜻이 아닙니다.

## Close-Blocker 계산에 필요한 필드

MVP-1 close-blocker calculation은 current persisted record를 읽고 close result를
파생합니다. Journey, Spine, detailed Evidence Manifest, Eval, Manual QA, export/report
table, projection job, validator-run storage가 필요하지 않습니다.

| Blocker 또는 close fact | Minimum source fields |
|---|---|
| Active Task exists and is closeable | `project_state.active_task_id`, `tasks.lifecycle_phase`, `tasks.result`, `tasks.closed_at` |
| Scope is present and current | `tasks.active_change_unit_id`, `change_units.status`, `change_units.scope_summary`, `change_units.non_goals_json`, `change_units.success_criteria_json` |
| User-owned judgment is unresolved | `user_judgments.judgment_type`, `user_judgments.status`, `user_judgments.affected_gates_json`, `user_judgments.context_refs_json` |
| Sensitive-action permission is missing or denied | `judgment_type=sensitive_action_approval`인 `user_judgments` row와 write가 관련될 때 current `write_authorizations.related_user_judgment_refs_json` |
| Write authority is missing, expired, stale, revoked, consumed, or incompatible | `write_authorizations.status`, `write_authorizations.basis_state_version`, `write_authorizations.consumed_by_run_id`, current `tasks.state_version` |
| Run or evidence support is missing | `runs.status`, `runs.evidence_ref_ids_json`, `evidence_refs.status`, `evidence_refs.owner_record_kind`, `evidence_refs.owner_record_id` |
| Work acceptance is required but missing | `judgment_type=work_acceptance`인 `user_judgments` row와 compatible `status` / `selected_option_json` |
| Residual risk is not visible or not accepted | Residual-risk blocker kind를 가진 `blockers` row와, acceptance가 required일 때 `judgment_type=residual_risk_acceptance`인 `user_judgments` row |
| A blocker is still open | `blockers.status`, `blockers.blocker_kind`, `blockers.blocked_action`, `blockers.related_refs_json`, `blockers.required_next_action` |
| Readable status is stale | Current `tasks.state_version`과 read/card response가 반환한 source version 비교. Later `projection_jobs`는 운영 프로필이 active일 때만 사용합니다. |

Close response는 compact close-readiness summary, evidence summary, next action을 보여줄
수 있습니다. 이것들은 파생 output입니다. `close_readiness`, `evidence_summary`, status-card
cache를 저장하는 일은 owner profile이 승격하기 전까지 optional/later입니다.

## Later-Profile Storage

Later-profile storage는 유용한 design inventory입니다. MVP-1 DDL bundle로 읽으면 안
됩니다.

### 보증 프로필

보증 프로필 또는 profile-promoted storage는 다음을 추가할 수 있습니다.

| Candidate table | 나중에 필요한 이유 | MVP-1에 required가 아닌 이유 |
|---|---|---|
| `approvals` | Committed sensitive-action approval lifecycle과 drift handling | MVP-1 sensitive-action approval user judgment |
| `baselines` | Assurance, approval, verification freshness를 위한 repository baseline capture | Promoted profile이 baseline check를 요구하지 않는 MVP-1 write check |
| `residual_risks` | Blocker row와 분리된 rich residual-risk lifecycle | MVP-1 residual-risk visibility 또는 acceptance prompt |
| `evidence_manifests` | Full criteria-to-evidence coverage | MVP-1 evidence ref와 ref에서 파생한 evidence summary |
| `evals` | Detached verification 또는 evaluator review | MVP-1 status, close blocker, self-checked evidence |
| `manual_qa_records` | Manual QA result, finding, setup, evidence refs | Profile 또는 user request가 Manual QA support를 요구하기 전까지 MVP-1 밖 |
| `validator_runs` | Persisted `ValidatorResult` row | Narrow owner가 validator storage를 명시적으로 승격하지 않은 MVP-1 blocker |
| `feedback_loops` | Feedback-loop policy support | Profile이 선택되기 전까지 MVP-1 밖 |
| `tdd_traces` | TDD profile이 선택될 때 red/green/refactor evidence | MVP-1 밖 |

### 운영 프로필

운영 프로필 또는 profile-promoted storage는 다음을 추가할 수 있습니다.

| Candidate table | 나중에 필요한 이유 | MVP-1에 required가 아닌 이유 |
|---|---|---|
| `projection_jobs` | Rendered Markdown 또는 managed output을 위한 durable outbox | MVP-1 status card 또는 next-action summary |
| `reconcile_items` | Human edit 또는 projection drift를 Core decision으로 라우팅 | MVP-1 밖 |
| `connector_manifests` | Connector-managed file과 drift 추적 | MVP-1 밖 |
| `persistent_locks` | Process lock만으로 부족할 때 durable lock/recovery metadata | MVP-1 밖 |
| `export_manifests` | Release handoff 또는 export package metadata | MVP-1 밖 |
| `recover_items` | Recovery finding, repair plan, operator follow-up | MVP-1 밖 |

### Future 또는 Diagnostic

Future 또는 diagnostic candidate는 owner가 승격하기 전까지 MVP-1 밖에 남습니다.

- Journey/spine continuity: `task_spine_entries`, `journey_cards`
- Domain and stewardship: `domain_terms`, `module_map_items`, `interface_contracts`
- Rich design support: `shared_designs`, `change_unit_dependencies`
- Diagnostics and polish: metrics, dashboard, context index, connector analytics,
  richer projection cache, export/recover detail table

## MVP-1에서 제거하거나 옮긴 Future Records

다음 record는 의도적으로 MVP-1 storage path 밖에 둡니다.

| Record family | MVP-1 replacement | Later location |
|---|---|---|
| Journey, Journey Card, Journey Spine, Spine entries | Task, scope, judgment, evidence ref, blocker에서 파생한 MVP-1의 작은 보기 | Future/diagnostic projection 또는 owner-promoted continuity support |
| Detailed Evidence Manifest | `evidence_refs`와 파생 evidence summary | 보증 프로필 full evidence coverage |
| Eval / detached verification records | Applicable한 경우 Run/evidence refs와 self-check label | 보증 프로필 verification |
| Manual QA detailed records | QA가 required지만 구현되지 않았을 때 user judgment, blocker, profile-specific prompt | 보증 프로필 Manual QA |
| Export, report, bundle tables | Evidence ref와 optional artifact pointer만 사용 | 운영/export profile |
| Projection job tables | Ephemeral 또는 read-time 작은 보기 freshness | 운영 프로필 projection rendering |
| Future validation/conformance tables | Direct blocker와 owner-field validation | Executable runtime suite가 생긴 뒤 assurance/conformance profile |
| Committed Approval table | `judgment_type=sensitive_action_approval`인 `user_judgment` | Approval/보증 프로필 |
| Separate `task_intake` table | `tasks.user_request`, `tasks.summary`, `change_units` scope fields | 필요할 경우 later intake workflow profile |
| Separate `evidence_summaries` / `close_readiness` / `projection_status_cards` tables | `runs`, `evidence_refs`, `blockers`, current state version에서 파생한 상태 보기 | Optional profile-promoted cache only |

## Event Semantics

`task_events`는 구현이 유지할 때 append-only audit trail과 event-order support table입니다.
Core가 무엇을 어떤 순서로 commit했는지 기록합니다. 일반 동작에서 current state의
authority source가 아니며, MVP-1 state는 보통 event replay로 재구성하지 않습니다.

Current state table이 authoritative합니다.

- `tasks`, `change_units`, `user_judgments`, `write_authorizations`, `runs`,
  `evidence_refs`, `blockers`는 MVP-1 current record입니다.
- Event는 audit, debugging, idempotency explanation, projection freshness, recovery
  history를 support합니다.

Required event emission은 committed state mutation에만 적용됩니다. Malformed request,
dry run, pre-commit state conflict, state를 mutate하지 않는 invalid request에는
`task_events` row가 필요하지 않습니다. Blocked request가 stored blocker를 create/update한다면
그 blocker mutation이 event-worthy state change입니다.
`dry_run=true`는 current record, `task_events` row, artifact, consumable Write Authorization,
projection job, `tool_invocations` replay row를 만들지 않습니다.

## Migration과 Validation Notes

이 저장소에는 migration runner가 없습니다. 아래 note는 향후 구현이 migration mechanism을
선택할 때 지켜야 할 constraint입니다.

### 권한 경계로서의 Storage Hardening

SQLite는 Core와 migration이 막지 않으면 malformed row도 저장할 수 있습니다. Row는
owner schema, owner value set, state-version basis, idempotency key, evidence
owner-link contract에 맞을 때만 authoritative합니다.

`doctor`, `recover`, evidence check, conformance runner는 malformed JSON, unknown
owner-bound value, mismatched replay row, stale state-version claim, evidence hash
mismatch, invalid owner link를 projection drift가 아니라 storage integrity finding으로
보고해야 합니다.

### JSON `TEXT` Validation

JSON `TEXT` column은 storage flexibility를 위한 것이지 arbitrary JSON을 저장하라는 뜻이
아닙니다. Core가 JSON `TEXT` value를 commit하기 전에는 값을 parse하고 parsed shape를
owner에 맞게 validate해야 합니다.

- API-shaped payload는 [MVP API](api/mvp-api.md)와 [API Schema Core](api/schema-core.md)에
  맞게 validate합니다.
- Storage-only JSON은 이 문서 또는 이 문서가 이름 붙인 owner document에 맞게 validate합니다.
- `'{}'`, `'[]'` 같은 SQLite default는 storage representation rule입니다. Public API
  field를 optional로 만들지 않습니다.

Malformed JSON과 schema-incompatible JSON은 invalid state입니다. SQLite build가 JSON
check를 지원한다면 migration은 `CHECK (json_valid(column_name))`을 방어적 보강으로
추가할 수 있습니다. 그래도 commit 전 Core shape validation이 의미를 소유합니다.

### Canonical Enum Hardening

Status-like `TEXT` column은 open string이 아닙니다. Allowed value는 Core validation이
소유합니다. Database `CHECK` constraint나 lookup table은 방어적 보강입니다.

Early hardening 대상:

| Field(s) | Owner/value source |
|---|---|
| `tasks.mode`, `tasks.lifecycle_phase`, `tasks.result` | [Core Model 참조](core-model.md) |
| `change_units.status` | Core Model / Change Unit owner rules |
| `user_judgments.status`, `judgment_type`, `presentation` | user-judgment API/kernel owners |
| `write_authorizations.status` | [Core Model `prepare_write`](core-model.md#prepare_write), [`harness.prepare_write`](api/mvp-api.md#harnessprepare_write), [`harness.record_run`](api/mvp-api.md#harnessrecord_run) |
| `runs.kind`, `runs.status` | [`harness.record_run`](api/mvp-api.md#harnessrecord_run)와 storage compatibility notes |
| `evidence_refs.kind`, `evidence_refs.redaction_state`, `evidence_refs.status` | `ArtifactRef`/evidence owners와 storage compatibility notes |
| `blockers.status`, `blocked_action`, `blocker_kind` | Core Model과 API blocker owners |
| `tool_invocations.status` | storage idempotency replay semantics |
| Future `projection_jobs.status`, `projection_jobs.projection_kind` | Operations Profile active 시 Projection/API owners |
| Future `validator_runs.status` | assurance profile active 시 `ValidatorResult` owner |
| Future `approvals.status` | Approval profile이 active일 때 Approval lifecycle owner |
| Future `evidence_manifests.status` | Full Evidence Manifest profile이 active일 때 Evidence profile owner |

Unknown owner-bound value는 fixture가 invalid-state recovery를 명시적으로 다루지 않는 한
invalid state입니다. Migration은 unknown value가 있을 때 tightening 전에 멈춰야 합니다.
Owner가 정의하지 않은 fallback meaning으로 조용히 mapping하면 안 됩니다.

Storage-owned compatibility value:

| Field | Durable values | Meaning |
|---|---|---|
| `runs.status` | `completed`, `interrupted`, `blocked`, `violation` | Committed Run row입니다. `completed`만 normal owner ref를 통해 evidence를 support할 수 있습니다. 다른 값은 audit/recovery record이며 그 자체로 evidence, QA, verification, acceptance, close readiness를 satisfy하지 않습니다. |
| `change_units.status` | `planned`, `active`, `completed`, `deferred`, `superseded` | Scope lifecycle입니다. Active compatible scope row만 new write를 scope합니다. |
| `user_judgments.status` | `proposed`, `pending_user`, `resolved`, `deferred`, `rejected`, `blocked`, `superseded` | User judgment lifecycle입니다. Resolved judgment는 기록한 judgment type과 payload에만 영향을 줍니다. |
| `write_authorizations.status` | `active`, `consumed`, `expired`, `stale`, `revoked` | Core/API owner value set과 일치하는 durable authorization lifecycle입니다. `active`이고 compatible한 row만 `record_run`이 consume할 수 있습니다. |
| `evidence_refs.status` | `available`, `missing`, `stale`, `blocked` | Evidence pointer availability입니다. Full evidence sufficiency가 아닙니다. |
| `blockers.status` | `open`, `resolved`, `superseded` | Stored blocker lifecycle입니다. Open blocker는 Core가 resolve 또는 supersede할 때까지 visible 상태로 남습니다. |
| `tool_invocations.status` | `committed` | Committed replayable `dry_run=false` response에 대해서만 row가 존재합니다. |

`prepare_write.decision`은 durable authorization lifecycle column과 별도입니다. Canonical `prepare_write.decision` 값은 `allowed`, `blocked`, `approval_required`, `decision_required`, `state_conflict`입니다. `dry_run=false`인 `decision=allowed`만 durable authorization row를 만듭니다. Exact idempotent replay는 original committed response를 반환합니다.

새 row는 `write_authorizations.status=active`로 시작합니다. `dry_run=false`인 `blocked`, `approval_required`, `decision_required` decision은 response decision, blocker, validator finding, error, 필요한 경우 committed idempotency replay state로 표현합니다. `state_conflict`는 새 replay row를 merge하지 않고 conflict state를 반환합니다. `dry_run=true`의 `decision=allowed` response는 `authorization_effect=would_create`를 설명할 수 있지만 `write_authorizations` row를 insert하지 않고 `record_run`이 소비할 수도 없습니다.

Future table value set은 해당 table의 owner profile이 active이거나, fixture가 optional
table을 명시적으로 seed하거나, owner document가 값을 명시적으로 승격할 때만 사용해야
합니다.

### Migrations

Future migration은 다음을 지켜야 합니다.

- `registry_meta`와 `project_state` 또는 선택한 equivalent metadata mechanism에
  schema/profile version을 기록합니다.
- Constraint를 tighten하기 전에 JSON과 owner-bound status value를 validate합니다.
- `task_events`를 유지한다면 `task_events.event_seq` order를 보존합니다.
- Evidence hash와 owner link를 보존하거나, 영향을 받는 ref를 recovery 대상으로 invalid
  표시합니다.
- Unknown owner-bound enum/status value가 있으면 owner가 소유하지 않은 fallback meaning을
  만들지 말고 stop합니다.
- Status card, projection/card/job freshness, evidence summary, close readiness는
  derived state로 취급하고 canonical state로 취급하지 않습니다.

이 note는 MVP-1에 특정 migration runner, migration file format, CLI command를 요구하지
않습니다.

### Lock Policy

Runtime mutation은 [런타임 아키텍처](runtime-architecture.md#state-transaction-flow)가
담당하는 Core transaction order를 통해 serialize해야 합니다. MVP-1은 ordinary SQLite
transaction과 필요 시 process/project lock으로 충분할 수 있습니다. `persistent_locks`는
later Operations candidate이지 MVP-1 table이 아닙니다.

Lock은 concurrent write를 보호합니다. OS sandboxing, evidence integrity,
tamper-proof storage를 제공하지 않습니다.
