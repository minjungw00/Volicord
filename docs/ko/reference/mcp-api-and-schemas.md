# MCP API와 스키마

## 이 문서가 도와주는 일

이 참조 문서는 하네스의 public MCP resource와 tool 계약을 구현·테스트·검토할 때 사용합니다. 이 문서는 읽기 전용 resource와 public tool, 공통 envelope, request/response schema를 다룹니다. 또한 shared ref, error taxonomy, idempotency, state conflict 동작, `ValidatorResult`, `ArtifactRef`의 public API shape를 정리합니다.

SQLite DDL과 storage layout, 전체 kernel transition table, projection template text, CLI command semantics, connector cookbook detail은 이 문서의 담당 범위가 아닙니다. Storage-owned JSON과 DDL 규칙은 [Storage와 DDL](storage-and-ddl.md)이 담당합니다.

이 문서는 향후 Harness 동작을 위한 참조 문서입니다. 현재 저장소 단계와 구현 인계 상태는 [구현 개요](../build/implementation-overview.md#문서-수락-상태)에 있습니다.

## 이런 때 읽기

- MCP client 또는 server 접점을 Harness Core에 연결할 때.
- 하네스 tool의 정확한 public request 또는 response shape가 필요할 때.
- API response에 어떤 error, validator result, 아티팩트 참조, projection 참조가 나타날 수 있는지 확인할 때.
- Public API behavior를 검증하는 conformance fixture를 작성할 때.

## 읽기 전에

[Runtime Architecture](runtime-architecture.md#state-transaction-flow)는 Core transaction order를, [커널 참조](kernel.md)는 상태 전이 의미를, [Storage와 DDL](storage-and-ddl.md)은 storage layout과 durable replay row를, [운영과 Conformance 참조](operations-and-conformance.md)는 operator command semantics를 담당합니다.

## 핵심 생각

MCP resource는 읽기 전용 보기로 동작합니다. 현재 상태, projection support가 있을 때의 projection 최신성, 사용자에게 보이는 요약을 보고할 수 있지만, 상태를 만들거나 복구하면 안 됩니다.

모든 상태 변경은 public tool과 Core를 거칩니다. MCP envelope는 Core가 검증할 caller claim을 담을 뿐 두 번째 상태 모델이 아닙니다. Tool response에는 projection support가 범위에 있을 때 projection path가, artifact가 범위에 있을 때 아티팩트 참조가 포함될 수 있습니다. 하지만 이 값들은 기준 상태 기록이나 durable evidence file을 가리키는 참조일 뿐, 기준 상태를 대체하지 않습니다.

Status와 next-action 표시는 public MCP 개념을 사용자에게 보이기 전에 평범한 말로 바꿔 보여줘야 합니다. 사용자는 무엇이 막고 있는지, 가장 작은 해소 방법이 무엇인지, 중요한 추가 막힘이 무엇인지 볼 수 있어야 합니다. Raw `ToolError`, `ErrorCode`, schema field name은 구현자, log, conformance output을 위한 선택 세부 정보로는 보일 수 있지만, 사용자 설명이 그것만으로 끝나면 안 됩니다.

이 문서의 public request와 response schema는 API payload의 검증 기준입니다. 여기에는 Core가 나중에 저장하는 API-shaped payload도 포함됩니다. Core는 commit 전에 모든 storage JSON 값을 이 문서의 API-owned shape 또는 [Storage와 DDL](storage-and-ddl.md)의 storage-owned shape에 맞게 검증해야 합니다. 잘못된 JSON이나 schema와 맞지 않는 JSON은 유효하지 않은 상태입니다.

Idempotency와 state conflict 동작은 Core state 위에 놓인 API-owned surface입니다. Exact replay는 original committed response를 반환하고, changed-payload replay는 `STATE_CONFLICT`를 반환하며, stale `expected_state_version`은 Core가 commit하기 전에 new mutation을 차단합니다. Durable storage detail은 [Storage와 DDL](storage-and-ddl.md)에 남습니다.

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

Conformance fixture에서는 이 public request schema가 정확한 기준입니다. [Conformance Fixtures 참조](conformance-fixtures.md#catalog-only-fixture-skeleton-guidance)의 catalog-only skeleton guidance와 [향후 Fixture Catalog](future-fixture-catalog.md)의 scenario row는 scenario가 어떤 action을 실행해야 하는지 말할 수 있지만, request field, alternate payload branch, fixture-only API shortcut을 추가하지 않습니다. Executable fixture의 `input`은 documented `ToolEnvelope` expansion 이후 선택한 action의 public request schema를 통과해야 합니다.

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

## 계약 위치 지도

| 필요한 것 | 먼저 볼 곳 | 관련 owner |
|---|---|---|
| Read-only resource contract | [Read-only resources](#read-only-resources) | Projection rendering rule은 [문서 Projection 참조](document-projection.md)에 남습니다. |
| 공통 request envelope와 response shape | [Tool envelope](#tool-envelope), [Common response](#common-response) | State-version transition 의미는 [커널 참조](kernel.md)에 남습니다. Core transaction order는 [Runtime Architecture](runtime-architecture.md#state-transaction-flow)에 남습니다. |
| Shared public schema와 ref | [Shared schemas](#shared-schemas), [ArtifactRef](#artifactref), [ValidatorResult](#validatorresult) | Storage-only JSON과 DDL은 [Storage와 DDL](storage-and-ddl.md)에 남습니다. |
| Markdown schema 표기 | [Schema notation convention](#schema-notation-convention) | Fixture assertion mode는 [Conformance Fixtures 참조](conformance-fixtures.md#fixture-assertion-semantics)에 남습니다. |
| Sensitive category label | [Sensitive Categories](#sensitive-categories) | 민감 동작 승인과 write-state behavior는 [커널 참조](kernel.md#prepare_write)에 남습니다. |
| Error code와 primary-error 선택 | [Error taxonomy](#error-taxonomy), [Primary Error Code Precedence](#primary-error-code-precedence), [`harness.close_task` Close Blockers](#harnessclose_task-close-blockers) | Operator diagnostic은 [운영과 Conformance 참조](operations-and-conformance.md)에 남습니다. |
| Public tool request와 response schema | [Public Tool Schema Map](#public-tool-schema-map), 그리고 해당 tool section | Fixture `action`과 `input` rule은 [Conformance Fixtures 참조](conformance-fixtures.md#conformance-fixture-format)에 남습니다. |
| Idempotency와 stale-state behavior | [Idempotency](#idempotency), [State Conflict 동작](#state-conflict-동작) | Durable replay row와 index는 [Storage와 DDL](storage-and-ddl.md)에 남습니다. |

## Schema notation convention

이 문서의 Markdown YAML-like block은 surrounding text가 example이라고 명시하지 않는 한 normative schema notation입니다. 구현자는 다음 rule에 따라 validation code로 옮겨야 합니다.

- `field: Type`은 field가 required이고 value가 non-null이어야 함을 뜻합니다.
- `field: Type | null`은 field가 여전히 required이지만 value가 JSON `null`일 수 있음을 뜻합니다. Omission은 expected `null`과 다릅니다.
- Field의 prose, branch rule, 또는 explicit extension rule이 omitted 가능하다고 말할 때만 optional입니다. Nullable은 optional을 뜻하지 않습니다.
- 이 표기 convention은 TypeScript 스타일의 question-mark optional-field 표기를 정의하지 않습니다. Optional 여부는 prose, branch rule, 또는 explicit extension rule로 말해야 합니다.
- `Type[]`은 item이 `Type`과 일치하는 JSON array입니다. 명시적 empty array `[]`는 present empty collection이며 omission과 다릅니다. Field prose가 one or more items를 요구하지 않는 한 empty array는 valid합니다.
- `one_of:`는 branch object를 정의합니다. 나열된 child field 중 정확히 하나만 present이고, 그 value는 해당 child field의 stated type과 일치해야 합니다. Branch prose는 selected branch가 `judgment_route` 또는 `display_depth` 같은 discriminator field와 일치해야 한다고 추가로 요구할 수 있습니다. 선택되지 않은 모든 branch는 `null`로 present가 아니라 absent여야 합니다.
- `object`는 JSON object입니다. Nested field가 표시되면 child field에도 같은 required, nullable, array, enum rule이 적용됩니다. Object map은 `field: { [key_name]: ValueType }` 또는 "keyed by validator ID"처럼 keyed object로 쓰거나 설명합니다. Key는 string이고 value는 stated value type과 일치해야 합니다. Object-map field의 명시적 `{}`는 present empty map입니다.
- `a | b | c`는 literal value enum입니다. 해당 section이 enum을 extensible이라고 label하거나 field를 display/routing string이라고 설명하지 않는 한 closed enum입니다. Extensible enum은 알려진 supported value와 enabled extension tier를 정의하며, public request validator는 supported 또는 enabled value만 accept합니다. Payload에 unknown value가 나타난다고 canonical authority가 생기지 않습니다.
- Prose의 branch rule은 어떤 field를 non-null로, 다른 field를 `null`로, 또는 다른 branch를 absent로 요구할 수 있습니다. 이런 branch rule도 schema의 일부입니다.
- 나열되지 않은 field는 section이 extension container 또는 optional extension field를 명시적으로 정의하지 않는 한 public contract field가 아닙니다. Public request validator는 이런 extension point 밖의 unlisted field를 reject해야 합니다. Optional extension field는 기본적으로 omitted이며, profile 또는 owner scope를 가져야 하고, owner document가 그 의미를 승격하기 전까지 gate, state authority, storage ownership에 영향을 주면 안 됩니다.

Storage validation은 별도의 ownership boundary입니다. Public API payload와 API-shaped stored JSON은 먼저 이 문서에 맞게 validate합니다. Storage-only JSON `TEXT`, DDL nullability, column default, storage hardening은 [Storage와 DDL](storage-and-ddl.md)에 맞게 validate합니다. Owner document가 명시적으로 연결하지 않는 한 SQLite column에서 public API field를 추론하거나 public response display field에서 storage column rule을 추론하면 안 됩니다.

## Stage Profile Manifest

이 문서는 향후 Harness 동작을 설명합니다. 이 저장소에는 아직 MCP server 구현이 없습니다. 아래에서 "Active"는 future implementation이 해당 profile을 선택했을 때 계약에 들어간다는 뜻입니다. 현재 문서 저장소가 구현했다는 뜻이 아닙니다.

Harness는 local authority-record와 user-judgment-routing layer입니다. Core-owned local state가 authority입니다. Chat, Markdown projection, connector output, tool output은 authority가 아닙니다.

| Stage/profile | Active API slice | 해당 slice에서 active가 아닌 것 |
|---|---|---|
| v0.1 Core authority smoke | Current status/state read, owner-valid setup path 하나, `harness.prepare_write`, compatible `harness.record_run` 하나, artifact/evidence ref 하나, structured status/blocker response. Optional smoke read는 `harness.next` 또는 좁은 `harness.close_task` blocker response를 쓸 수 있습니다. | Full natural-language intake, stored user-judgment packet, full Evidence Manifest, detached verification, Manual QA, work acceptance, residual-risk acceptance, rich projection, export/recover, advanced connector, operations surface. |
| v0.2 First user-value slice | Ordinary-language intake, next-safe-action output이 있는 status, `harness.prepare_write`, `harness.record_run`, `harness.request_user_judgment`, `harness.record_user_judgment`, blocker summary가 있는 `harness.close_task`. | Detached verification launch/eval, full Manual QA matrix, user-facing approval route를 넘는 approval hardening, export/recover, advanced connector API, broad operations, detailed diagnostic projection. |
| v0.3+ Later profiles/future | Assurance, verification, Manual QA, waiver, full residual-risk acceptance, reconcile, validator emission, projection/report/export/recover, operations, advanced connector profile. Owner doc이 승격할 때만 active입니다. | v0.1 smoke 또는 minimum v0.2 user-value slice가 아닙니다. |

### First Implementable Calls

v0.1 implementer는 이 set 안에 머물 수 있어야 합니다.

1. `harness.status` 또는 v0.1 read-only resource로 status를 읽습니다.
2. Owner-valid setup path 하나로 local project, Task, scoped work boundary를 만들거나 선택합니다. `harness.intake`를 쓰면 minimal setup shape만 씁니다.
3. 정확한 intended product write에 대해 `harness.prepare_write`를 호출합니다.
4. Authorized implementation/direct Run 뒤에 `harness.record_run`을 호출하고 safe artifact/evidence ref 하나를 등록합니다.
5. `harness.status`로 status/blocker를 반환합니다. Blocker를 더 분명히 보여줄 때만 `harness.next` 또는 좁은 `harness.close_task` smoke response를 선택할 수 있습니다.

v0.2 implementer는 later assurance/operations를 떠안지 않고 user value를 추가합니다.

1. `harness.intake`로 ordinary language를 받습니다.
2. `harness.status.next_actions`로 current state와 next safe action을 보여줍니다. `harness.next`는 optional expanded/compatibility read입니다.
3. User-owned judgment가 progress, write, acceptance, risk, close를 막으면 `harness.request_user_judgment`를 씁니다.
4. Committed Decision Packet에 대한 사용자 답은 `harness.record_user_judgment`로 기록합니다.
5. Write/evidence authority path는 계속 `harness.prepare_write`와 `harness.record_run`입니다.
6. Close state와 structured blocker는 `harness.close_task`로 다룹니다.

### Method Activation Table

| Method or capability | v0.1 | v0.2 | v0.3+ / later profiles |
|---|---|---|---|
| `harness.status` | Active: current state/status/blocker/write authority. | Active: ordinary-language status, `next_actions`, pending judgment, evidence summary, close readiness. | Assurance, operations, projection, diagnostic ref는 profile이 켜질 때만 추가됩니다. |
| `harness.intake` | 선택한 경우 optional minimal setup. | Active ordinary-language intake/resume. | 더 풍부한 discovery와 design-support routing을 추가할 수 있습니다. |
| `harness.next` | Optional smoke read. | Optional expanded/compatibility read. Preferred v0.2 path는 `harness.status.next_actions`입니다. | Assurance, QA, reconcile, operations action kind는 active profile에서만 포함합니다. |
| `harness.prepare_write` | Active. | Active. | Approval/assurance/profile-specific blocker는 profile이 active일 때만 추가됩니다. |
| `harness.record_run` | Compatible implementation/direct Run 하나와 artifact/evidence ref 하나에 대해 active. | Evidence/artifact summary에 active. | Evidence Manifest, feedback-loop, TDD, verification-input payload는 profile이 active일 때만 추가됩니다. |
| `harness.request_user_judgment` | Not active. | User-owned judgment와 work-acceptance prompt에 active. | Approval hardening, waiver, full residual-risk acceptance, reconcile profile을 추가할 수 있습니다. |
| `harness.record_user_judgment` | Not active. | Decision Packet에 대한 사용자 답 기록에 active. | Approval lifecycle, waiver, risk-acceptance, reconcile record update를 추가할 수 있습니다. |
| `harness.close_task` | Optional narrow blocker/status smoke. | Close state와 blocker summary에 active. | Full assurance, QA, accepted-risk, projection/report freshness, operations blocker는 profile이 active일 때만 추가됩니다. |
| `harness.launch_verify`, `harness.record_eval`, `harness.record_manual_qa` | Not active. | 기본 v0.2에는 not active. | Agency Assurance 또는 later owner profile에서만 active입니다. |
| Export/recover/operator/advanced connector API | Not active. | Not active. | Operations 또는 future profile입니다. 이 문서는 아직 public MCP tool을 정의하지 않습니다. |

### Field Activation Table

아래 schema block은 method/profile이 active일 때 exact합니다. 이 표가 stage filter입니다. Future field는 reference target일 수 있지만 profile이 켜지기 전에는 inactive입니다.

| Field 또는 schema family | v0.1 | v0.2 | v0.3+ / later profiles |
|---|---|---|---|
| `ToolEnvelope`, `ToolResponseBase`, `ToolError`, `StateSummary`, `EventRef` | Active. | Active. | Active. |
| `StatusResponse.active_task`, `status_card`, `write_authority_summary`, `guarantee_display`, primary blocker detail | Active. | Active. | Active. |
| `StatusResponse.next_actions` | Optional minimal blocker/action. | Active preferred next-safe-action output. | Later action kind는 profile이 active일 때만 포함합니다. |
| `DecisionPacket`, `JudgmentContext`, judgment category/route/depth fields | Owner path가 이미 가진 seeded/display ref를 제외하면 inactive. | User-facing judgment에 active. | Approval, waiver, risk, reconcile profile이 확장합니다. |
| `approval_request_candidate`, `approval_refs`, `ApprovalScope` | `prepare_write`가 approval을 요구할 때 candidate display는 가능하지만 committed Approval lifecycle은 inactive. | User-facing sensitive-action approval route에만 active. | Hardened approval/profile behavior. |
| `EvidenceRefs`, `ArtifactInput`, `ArtifactRef`, minimal `registered_artifacts` | Safe artifact/evidence ref 하나에 active. | Evidence summary에 active. | Full Evidence Manifest, feedback-loop, TDD, Eval, Manual QA, export, richer relation semantics. |
| `ResidualRiskSummary`, `AcceptanceVisibilityContext` | Smoke blocker에 필요한 explicit `none`/`not_required` status를 제외하면 inactive. | Visibility와 close/acceptance display에 active. | Full accepted-risk close semantics에 active. |
| Verification, Eval, Manual QA, waiver, validator, projection/report/export/recover fields | Inactive. | Owner profile이 작은 subset을 명시적으로 승격하지 않는 한 inactive. | 각 owner profile에서만 active입니다. |

### Naming And Compatibility Decisions

Preferred v0.2 public method name은 `harness.request_user_judgment`와 `harness.record_user_judgment`입니다. 기존 `harness.request_user_decision`과 `harness.record_user_decision`은 maintainer가 docs와 fixture 전체 rename을 수락할 때까지 compatibility alias로 남습니다. Alias는 one-to-one mapping입니다. 추가 method, state path, authority를 만들지 않습니다.

`DecisionPacket`은 canonical owner record name으로 남습니다. Kernel record가 user judgment와 route를 저장하기 때문입니다. Public method name은 broad approval처럼 보이지 않도록 "judgment"를 씁니다.

`harness.status.next_actions`가 preferred v0.2 next-safe-action output입니다. `harness.next`는 expanded next-action payload와 기존 문서 compatibility를 위한 optional read method입니다. v0.2 implementation은 `harness.status.next_actions`만으로도 minimum next-action requirement를 만족할 수 있습니다. 단 caller가 next safe action과 smallest unblocker를 찾을 수 있어야 합니다.

API surface는 stage가 모든 범주를 activate하지 않아도 close-support category를 분리해 보존해야 합니다. Evidence, verification, Manual QA, work acceptance, residual-risk visibility, residual-risk acceptance는 서로 다른 의미입니다. Status, next-action, judgment, close response는 어떤 category가 `not_required`라고 말할 수 있습니다. 하지만 test pass, Eval verdict, QA waiver, work-acceptance note, accepted residual risk를 다른 category의 generic substitute로 쓰면 안 됩니다.

Capability는 first-class kernel gate가 아닙니다. Surface capability는 validator emission이 active일 때 `surface_capability_check`, `harness.prepare_write.response.blocked_reasons`, status/write decision의 guarantee display로 나타납니다. Core precondition과 mechanical check는 validator보다 앞서 또는 함께 실행될 수 있습니다. `ValidatorResult`로 emit되고 `validator_runs`에 저장되는 stable ID만 validator ID입니다. `scope_coverage`, `changed_paths`, `changed_paths_intent`, `approval_scope`, `baseline_freshness`, `qa_waiver_reason`, `projection_freshness` 같은 check는 owner section이 승격하지 않는 한 Core check로 남습니다.

## Read-only resources

Resource 조회는 상태를 변경하지 않고 현재 상태와 projection 중심 요약을 보여줍니다. Public tool처럼 resource도 active delivery profile에 따라 단계가 나뉩니다. Reference contract가 later resource URI를 미리 이름 붙일 수는 있지만, 그렇다고 모든 `harness://` resource surface를 기본으로 읽어야 한다는 뜻은 아닙니다.

Resource 조회는 Task record, decision, projection job, reconcile item을 만들거나 상태 변경을 일으키면 안 됩니다. Resource 조회 중 최신이 아닌 projection을 감지하면 freshness만 보고하고 복구하지 않습니다.

Read-only resource는 source record가 이미 그 summary를 뒷받침할 때 가장 먼저 해소할 막힘, 추가 막힘, 가장 작은 해소 방법을 렌더링할 수 있습니다. 그래도 이 렌더링은 읽기 전용 보기입니다. 권한을 만들거나, gate를 clear하거나, projection repair를 enqueue하거나, 기준 상태를 변경하면 안 됩니다.

앞 단계의 resource는 뒤 단계에서도 재사용할 수 있습니다. 뒤 단계 resource는 담당 profile이 켜졌을 때만 사용할 수 있습니다.

### 코어 권한 조각(v0.1 Core Authority Slice) resources

v0.1 resource subset은 의도적으로 작습니다. 첫 authority loop에 필요한 current project, active/current Task, current state와 blocker, write-authority status, Core가 chat memory나 generated Markdown보다 권한 있는 기록임을 보여 주는 Run/artifact/evidence ref만 지원합니다.

| Resource | Profile 의미 |
|---|---|
| `harness://project/current` | 현재 등록된 project identity, 관련될 때의 current project state version, Core read/write 주소 지정에 필요한 local MCP availability fact. |
| `harness://task/active` | active Task pointer, 또는 명시적인 `none` / `unknown`. Task를 만들지 않습니다. |
| `harness://task/{task_id}` | 좁은 authority loop를 위한 현재 Task state입니다. Lifecycle, scoped work boundary ref, blocker, write-authority status, latest Run ref, 존재할 때 최소 artifact/evidence ref를 포함할 수 있습니다. |
| `harness://task/{task_id}/summary` | Connector가 `harness.status` / `harness.next` output만 쓰지 않고 resource를 사용할 때의 optional compact Task status/blocker summary입니다. |
| `harness://status/card` | Current Core state에서 파생된 optional compact current-position/status card입니다. Primary blocker, secondary blocker, smallest unblocker를 보여줄 수 있지만 여전히 읽기 전용입니다. |

v0.1은 Journey, Journey Spine, Decision Packet storage, Evidence Manifest, bundle, design map, module map, interface contract, report, projection job, projection renderer를 요구하지 않습니다.

### 사용자 대상 하네스 MVP(v0.2 User-Facing Harness MVP) resources

v0.2는 v0.1 current-status resource를 유지하고, 일반 사용자가 현재 작업 상태, 대기 중인 사용자 판단, 근거 요약, 닫기 준비 상태, close blocker, 관련될 때의 작업 수락 필요/상태와 잔여 위험 표시를 이해할 수 있게 그 summary를 확장할 수 있습니다. Minimum v0.2 path에는 detailed report resource가 필요하지 않습니다.

| Resource | Profile 의미 |
|---|---|
| `harness://task/{task_id}/decision-packets` | 해당 Task의 active, resolved, deferred, blocked Decision Packet summary입니다. 사용자 판단 표시를 위해 사용합니다. |
| `harness://task/{task_id}/judgment-context` | 사용자 판단에 필요한 최소 current context입니다. Optional pull ref는 required context와 분리합니다. 사용자 판단이 필요할 때만 사용합니다. |

v0.2의 근거와 닫기 준비 상태 resource path는 current Core state와 ref에서 파생된 `harness://task/{task_id}/summary`, `harness://status/card`, `harness.status`, 또는 `harness.next`로 충족할 수 있습니다. `harness://task/{task_id}/evidence-manifest`를 요구하지 않습니다.

### 에이전시 보증 팩(v0.3 Agency Assurance Pack) resources

이 resource들은 profile-gated assurance read입니다. 첫 조각이나 minimum user-facing MVP 요구사항이 아닙니다.

| Resource | Profile 의미 |
|---|---|
| `harness://policy/sensitive-categories` | Approval, waiver, risk, assurance profile을 위한 read-only sensitive-action category policy입니다. Approval을 부여하거나 action을 승인된 것으로 분류하지 않습니다. |
| `harness://task/{task_id}/evidence-manifest` | Assurance/evidence profile이 켜졌을 때 evidence coverage와 manifest-oriented read입니다. v0.2 evidence summary에는 필요하지 않으며, detailed manifest projection body는 pull-on-demand 또는 diagnostic 범위에 남습니다. |

### 운영과 인계 팩(v0.4 Operations & Handoff Pack) resources

이 resource들은 operations profile이 켜졌을 때 operator visibility, connector freshness, report/export, handoff workflow를 지원합니다.

| Resource | Profile 의미 |
|---|---|
| `harness://project/surfaces` | Registered surface/profile inventory, MCP exposure posture, capability freshness, connector-operational status입니다. 좁은 구현은 current-surface fact만 더 이른 단계에 노출할 수 있지만, 넓은 surface inventory는 operations 범위입니다. |
| `harness://task/{task_id}/reports/latest` | Operations/readiness를 위한 latest readable report ref와 freshness입니다. Report는 derived view이며 Core state를 대체하지 않습니다. |
| `harness://task/{task_id}/bundle/current` | Handoff 또는 recovery profile을 위한 current bundle/export-oriented ref입니다. Bundle은 derived/export artifact이지 authority가 아닙니다. |

### Future/diagnostic resources

이 resource들은 projection-oriented, design/reference, 또는 diagnostic read입니다. Owner가 승격한 profile을 통해서만 켜거나 진단을 위해 필요할 때만 가져옵니다. v0.1 또는 minimum v0.2 요구사항으로 취급하면 안 됩니다.

| Resource | Profile 의미 |
|---|---|
| `harness://task/{task_id}/spine` | 해당 diagnostic view가 켜졌을 때 owner record와 event에서 재구성하는 Journey Spine-style read입니다. |
| `harness://task/{task_id}/journey` | Journey view가 켜졌을 때 current-position, Journey Card, 또는 Journey Spine-oriented read입니다. |
| `harness://task/{task_id}/change-unit-dag` | Broad dependency view가 켜졌을 때 Change Unit dependency ref와 ordering summary입니다. Scheduler나 authorization surface가 아닙니다. |
| `harness://design/domain-language` | Design/domain profile이 켜졌을 때 design owner record에서 읽는 domain-language read입니다. |
| `harness://design/module-map` | Module-map profile이 켜졌을 때 design owner record에서 읽는 module-map read입니다. |
| `harness://design/interface-contracts` | Interface-contract profile이 켜졌을 때 design owner record에서 읽는 interface-contract read입니다. |

Resource로 노출되는 projection은 읽기용 파생 보기이지 authority가 아닙니다. Projection-backed resource를 읽는 행위는 projection을 refresh하거나, projection job을 enqueue하거나, missing owner record를 만들거나, 근거를 accept하거나, verification을 수행하거나, 수동 QA를 기록하거나, 결과를 수락하거나, 잔여 위험을 받아들이거나, write를 authorize하거나, Task를 close하면 안 됩니다.

## Tool envelope

모든 public tool request는 envelope를 가집니다. State-changing tool에는 non-null `idempotency_key`와 `expected_state_version`이 필요합니다. Read-only tool도 tracing을 위해 같은 envelope를 받을 수 있으며, `expected_state_version`을 `null`로 둘 수 있습니다. Envelope는 [state transaction flow](runtime-architecture.md#state-transaction-flow)의 시작에서 Core가 검증합니다. Envelope가 surface에 Core 밖에서 상태를 변경할 권한을 주지는 않습니다.

Core는 operation이 가리키는 primary Task를 기준으로 State version scope를 결정합니다. Resolved primary Task는 `ToolEnvelope.task_id`, tool-specific `task_id`, 또는 active Task resolution으로 정해질 수 있습니다. Exact idempotent replay가 아닌 것으로 확인된 뒤, Task 범위의 상태 변경은 `expected_state_version`을 해당 Task의 `tasks.state_version`과 비교합니다. Core가 primary Task를 찾지 못하고 operation이 project-scoped이면 `expected_state_version`을 `project_state.state_version`과 비교합니다.

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

### MCP 경계와 호출자 신뢰

Public MCP 계약의 v0.1/default reference posture는 등록된 project surface에 대한 local-only 노출입니다. Local-only란 기대되는 local user/profile에 대한 로컬 프로세스, 로컬 socket, 또는 localhost loopback 연결을 뜻합니다. 인증되지 않은 shared endpoint, non-loopback bind, forwarded/tunneled endpoint, cloud/CI relay, cross-user socket 또는 directory, 등록된 connector profile로 설명되지 않는 remote caller는 제외됩니다. MCP server를 이 로컬 경계 밖에 노출하면 위협 모델이 달라지며, owner documentation과 conformance가 특정 connector posture를 승격하기 전까지 v0.1/default reference posture 밖에 남습니다. 그런 더 강한 profile이 없다면 MCP endpoint에 닿을 수 있는 호출자도 Core가 검증해야 하는 claim을 보낸 출처일 뿐, 자동으로 신뢰되는 권한이 아닙니다.

접근 제어 계약은 localhost-only binding, local file permission으로 제한된 Unix-domain socket 또는 다른 local socket, in-process 또는 stdio transport, per-project token handle, process-scoped configuration material, 또는 이에 준하는 로컬 제어 같은 여러 방식으로 구현될 수 있습니다. 이 예시는 access-control material class이지 schema enum, raw secret value, 필수 CLI syntax가 아닙니다. Public schema와 diagnostic detail은 material class, bind/reachability posture, freshness state, profile ref, conformance 또는 operator-check ref, display-safe handle/fingerprint를 담을 수 있지만 raw token, secret, private configuration value, omitted secret, blocked payload bytes를 담으면 안 됩니다. Public API 계약에서 중요한 점은 호출자의 access mode가 등록된 surface profile과 맞아야 하고, 변경 요청 전에 Core가 모든 envelope claim을 계속 검증한다는 것입니다.

권한이 없거나 profile에 맞지 않는 호출자는 endpoint에 닿을 수 있다는 이유만으로 권한으로 승격되면 안 됩니다. 호출이 Core에 닿을 수 없으면 authoritative Core response는 존재하지 않으며 `MCP_UNAVAILABLE` 또는 diagnostic `MCP_SERVER_UNAVAILABLE`입니다. Core 또는 operator가 reachable local caller나 access path가 registered local profile 밖이라고 분류할 수 있으면 response는 `LOCAL_ACCESS_MISMATCH`를 사용하고 display-safe detail만 담습니다. Project, Task, surface, Run, actor claim mismatch는 addressed tool에 대한 일반 record-compatibility, state-conflict, scope, capability, validator checks로 해석합니다.

Envelope field는 routing과 감사용 claim입니다.

- `project_id`, `task_id`, `surface_id`, `run_id`는 addressed operation과 호환되는 record로 해석되어야 합니다. 호출자가 다른 project, Task, surface, Run을 이름으로 지정한다고 권한이 생기지 않습니다.
- `actor_kind`는 routing과 policy check를 위한 claimed actor role입니다. 그 자체만으로 민감 동작 승인, 작업 수락, Decision Packet resolution, 수동 QA judgment, 분리 검증 독립성를 충족하면 안 됩니다.
- `idempotency_key`는 committed mutation의 중복을 막습니다. Authorization token이 아니며, 같은 `(project_id, tool_name, idempotency_key)` scope에서 같은 canonical request payload일 때만 replay로 유효합니다. 같은 key를 변경된 payload, artifact input set, envelope authority basis와 함께 재사용하면 `STATE_CONFLICT`를 반환하며, 새 effect를 original committed response에 merge하면 안 됩니다.
- `expected_state_version`은 새 mutation attempt에 대한 호출자의 freshness와 concurrency claim입니다. 최신이 아니거나 잘못된 version은 mutation 전에 `STATE_CONFLICT`를 반환합니다. 이는 오래된 Task 또는 project view, Approval basis, evidence context, artifact relation, projection summary, user-judgment context가 write authority가 되는 것을 막기 위한 장치입니다.
- `dry_run=true`는 진단 정보만 반환합니다. Idempotency key를 예약하거나, Write Authorization을 만들거나, artifact를 attach하거나, 이후 write가 안전하다는 증거를 만들지 않습니다.

Public tool response는 local-security claim failure를 기존 response shape로 보여줘야 합니다.

| Condition | Response guidance |
|---|---|
| `project_id`, `task_id`, `surface_id`가 resolve되지 않거나, addressed project 밖으로 resolve되거나, tool-specific Task 또는 owner record와 충돌합니다. | Mutation 전에 거부합니다. Primary `ErrorCode`는 기존 precedence table에서 선택하고, 구체적인 claim mismatch는 `ToolError.details`, blocked reason, state summary, validator/check output에 둡니다. Public spoofing-specific code를 추가하지 않습니다. |
| `actor_kind`가 `user`, `operator`, `evaluator`라고 claim하지만 request path가 `acceptance_gate`에 필요한 사용자 판단, 수동 QA, 민감 동작 승인, 분리 검증 독립성을 충족할 수 없습니다. | 관련 gate를 충족되지 않은 상태로 유지하고, tool에 따라 `ACCEPTANCE_REQUIRED`, `QA_REQUIRED`, `APPROVAL_REQUIRED`, `DECISION_REQUIRED`, `VERIFY_NOT_DETACHED`, `CAPABILITY_INSUFFICIENT`, 또는 validator result 같은 기존 blocker를 사용합니다. Actor claim은 audit context이지 judgment의 증거가 아닙니다. |
| MCP endpoint가 off-profile, weak, stale, forwarded, tunneled, cross-user, unauthorized, 또는 unknown access mode를 통해서만 도달할 수 있습니다. | Core 또는 operator가 condition을 분류할 수 있으면 `LOCAL_ACCESS_MISMATCH`를 사용하고 display-safe access-mode fact만 `details` 또는 보장 수준 표시에 포함합니다. Core에 닿을 수 없다면 authoritative Core response나 mutation을 주장할 수 없습니다. |

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

Fixture assertions를 위한 event stability는 [Kernel Stable Event Catalog](kernel.md#stable-event-catalog)가 담당합니다. 아래 tool sections는 response가 반환하거나 implementation이 저장할 수 있는 `EventRef.event_type` 값을 설명하지만, 두 번째 event taxonomy를 정의하지 않습니다. Stable로 label된 names는 catalog names입니다. Stable catalog에 없는 이름은 implementation-local detail 또는 audit events로 나타날 수 있지만 fixture-stable이 아니며 staged/reference `expected_events` fixtures가 요구하면 안 됩니다. ValidatorResult IDs, Core check names, projection status shorthands, fixture seed shorthand는 kernel catalog가 명시적으로 나열하지 않는 한 event names가 아닙니다.

`ProjectionKind`는 API가 단계별 지원 계층을 담당하는 extensible enum입니다.

| 지원 계층 | Values | Requirement |
|---|---|---|
| Core status output | required 없음 | v0.1 status/blocker output은 persisted Markdown projection job 없이 read/freshness fact를 노출할 수 있습니다. |
| User-facing MVP summaries | persisted projection support를 사용할 때 `TASK` minimal task-scoped readable summary; active direct-work compact-result display에만 `DIRECT-RESULT` | 읽기용 status/judgment/evidence/close summary path를 제공합니다. 동등한 status/next card로도 full `TASK` template rendering이나 persisted projection job 없이 MVP output을 충족할 수 있습니다. |
| Agency assurance reports | `APR`, `MANUAL-QA` | 해당 approval, 수동 QA, waiver, verification, assurance profile이 active일 때만 구현합니다. Compact verification은 card output을 사용할 수 있으며 detailed `EVAL` Markdown은 later diagnostic scope로 남습니다. |
| Operations/export reports | `EXPORT` | Export, release-handoff, operations report profile이 범위에 있을 때만 켭니다. Export report projection은 읽기용 snapshot이지 authority가 아닙니다. |
| Future/diagnostic projections | `RUN-SUMMARY`, `EVIDENCE-MANIFEST`, `EVAL`, `TDD-TRACE`, `DOMAIN-LANGUAGE`, `MODULE-MAP`, `INTERFACE-CONTRACT`, `DEC`, `DESIGN`, `JOURNEY-CARD` | Detailed report, trace, map, standalone Decision Packet, persisted Journey Card, Journey Spine-style, detailed Evaluation, diagnostic view입니다. Owner가 승격한 later profile이 범위에 있을 때만 켭니다. |

지원 계층 label은 enum value가 아닙니다. v0.1에는 owner path가 이미 만든 freshness/read fact를 보존하는 것 외의 projection rendering exit requirement가 없습니다. 사용자 대상 하네스 MVP(v0.2 User-Facing Harness MVP)는 사용자가 현재 작업, 사용자 판단, 근거, 닫기 막힘을 이해할 만큼의 파생 output을 제공하지만 broad template polish를 요구하지 않습니다. 작업 수락과 잔여 위험 사실은 관련 있을 때 distinct하게 남지만 필수 projection kind를 늘리지는 않습니다. Agency assurance와 operations/export support는 later profile class입니다. Future/diagnostic projections는 자동으로 v1+ 전용이라는 뜻은 아니며 owner가 승격한 profile에서 켤 수 있지만, v0.1이나 최소 v0.2 요구사항은 아닙니다.

ProjectionKind extensibility가 projection을 기준 상태로 만들지는 않습니다. 모든 projection job은 여전히 owner 기록 및 아티팩트 참조에서 파생된 보기를 렌더링합니다. 어떤 ProjectionKind 지원 계층도 state, 근거, 수동 QA, 검증, 작업 수락, 잔여 위험 수용, close authority, Write Authorization을 만들지 않습니다. `DEC`는 해당 기능이 켜졌을 때 standalone Decision Packet Markdown에만 유효합니다. Standalone `DEC` job이 없어도 active stage/profile이 요구하는 Decision Packet visibility가 줄어들면 안 되며, 이 visibility는 status/next responses, judgment-context resources, decision-packet resources, 최소 `TASK` 또는 card display를 통해 제공되어야 합니다. 사용자 대상 MVP에 필요한 것은 standalone `DEC` `ProjectionKind`가 아니라 Decision Packet 사용자 판단 요청 display shape입니다. Persisted `JOURNEY-CARD` Markdown과 Journey Spine-style output은 future/diagnostic 범위입니다. `harness.status`, `harness.next`, significant resume flow의 현재 위치 맥락은 간결한 상태 출력으로 충족할 수 있습니다.

`EXPORT`는 export 기능이 켜졌을 때 Release Handoff 같은 보고서 profile을 포함할 수 있습니다. 이런 profile은 projection/보고서 접점일 뿐입니다. Deployment 권한, merge 권한, production-monitoring 권한, 작업 수락, 잔여 위험 수용, assurance 향상, Task close 권한을 만들지 않습니다.

`StateSummary.mode`, `IntakeRequest.requested_mode`, 그 밖의 schema-owned mode field는 `advisor`, `direct`, `work` enum 값을 그대로 유지합니다. 사용자에게 보이는 접점은 이를 읽기/조언, 작은 변경, 추적되는 작업으로 렌더링할 수 있습니다. 이 label은 파생된 표시 text일 뿐 schema field, enum value, canonical record type, projection kind, gate value, authority path가 아닙니다. 이 표시 번역은 schema rename이 아니며 public enum 값을 추가하지 않고, 쓰기 권한, 민감 동작 승인, 사용자 소유 판단, evidence, QA, verification, 잔여 위험, 작업 수락, close requirement를 숨기면 안 됩니다.

```yaml
ToolError:
  code: ErrorCode
  message: string
  retryable: boolean
  details: object

ToolErrorMcpUnavailableDetails:
  mcp_unavailable_kind: server_unavailable | surface_mcp_unavailable | stale_connection | unknown

ToolErrorLocalAccessDetails:
  local_access_issue: off_profile | stale_profile | weak_binding | forwarded_or_tunneled | cross_user | unauthorized_local_caller | unknown
  surface_id: string | null
  profile_ref: string | null
  safe_handle: string | null

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

주요 schema object는 담당 section과 아래 표를 통해 stage/profile 의미를 가집니다. `introduced_in/profile` label은 문서 metadata일 뿐 payload field나 새 schema member가 아닙니다.

| Schema object 또는 family | introduced_in/profile | Requiredness 의미 |
|---|---|---|
| `ToolEnvelope`, `ToolResponseBase`, `ToolError`, `StateSummary`, `EventRef` | v0.1 Core Authority Slice | 공통 contract shape입니다. Required field는 이 object를 사용하는 모든 emitted 또는 accepted payload에서 active contract field입니다. |
| `WriteAuthorizationSummary`, `WriteAuthoritySummary`, `ApprovalScope` | v0.1 core write authority. Approval record는 v0.3 Agency Assurance | Write Authorization summary는 allowed write가 만들어질 때 v0.1 범위입니다. Approval-specific refs와 lifecycle은 Approval profile이 켜지기 전까지 later-profile입니다. |
| `ArtifactRef`, `ArtifactInput`, `EvidenceRefs`, `StateRecordRef` | v0.1 minimal artifact/evidence ref. 더 풍부한 owner relation은 later profile | v0.1은 registered artifact/evidence ref 하나와 compatible owner link 하나가 필요합니다. Later owner record kind는 해당 storage/API profile이 있을 때만 valid합니다. |
| `DecisionPacket`, `DecisionPacketCandidate`, `JudgmentContext`, `AcceptanceVisibilityContext` | v0.2 User-Facing Harness MVP. Full approval/waiver/risk/reconcile profile은 v0.3/v0.4 | Decision Packet 또는 candidate가 만들어질 때 required field가 적용됩니다. v0.1은 Decision Packet storage를 요구하지 않습니다. |
| `ResidualRiskSummary`와 residual-risk refs | v0.2 visibility. v0.3 full residual-risk acceptance semantics | `status=none`은 summary claim일 수 있습니다. Accepted risk는 standalone record kind가 아니라 `residual_risk` ref의 state입니다. |
| `ValidatorResult` | v0.3 Agency Assurance와 v0.4 Operations. Owner가 승격한 capability/status check는 예외 | Validator result가 emitted될 때 required field가 적용됩니다. Core check는 validator ID가 되지 않고도 block할 수 있습니다. |
| `ProjectionJobRef`, `ProjectionKind`, projection freshness object | v0.1에서 required인 `ProjectionKind`는 없음. v0.2는 minimal `TASK`/card output을 사용할 수 있고, assurance, operations/export, future/diagnostic class는 profile-gated | Projection ref와 job은 derived-view metadata이며 authority가 아닙니다. Kind는 해당 projection support를 enabled한 stage/profile에서만 valid reference target입니다. |
| `JourneyCardSummary`, `RecommendedPlaybook`, Role Lens routing field | profile에 따라 v0.2 display guidance 또는 later diagnostic view | 읽기 전용 display/routing schema입니다. 그 자체로 write를 authorize하거나, gate를 충족하거나, risk를 accept하거나, Task를 close하지 않습니다. |

### Sensitive Categories

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

Sensitive category는 명령어처럼 외우는 체계가 아니라 민감 동작 승인이 필요한 민감 위험을 설명하는 label입니다. 하나의 intended write에는 여러 category가 함께 붙을 수 있습니다. Category는 민감 동작 승인이 왜 필요한지 설명하지만 제품, 아키텍처, 보안, QA, verification, 작업 수락, 잔여 위험 관련 판단, policy 판단을 대신 해결하지 않습니다. 정확한 write-state 동작은 [커널 참조](kernel.md#prepare_write)가 담당하고, public request와 lifecycle shape은 [`harness.prepare_write`](#harnessprepare_write)와 [Approval Lifecycle](#approval-lifecycle)이 담당합니다.

| Category | 보통 뜻하는 것 | Approval, Decision Packet, evidence, redaction 지침 |
|---|---|---|
| `auth_change` | 로그인, session, password, OAuth, account recovery, lockout, authentication policy 변경. | Approval은 민감한 auth 단계를 포괄합니다. Auth model, lockout behavior, recovery UX, user notice, 해소되지 않은 security trade-off에는 Decision Packet을 사용합니다. Evidence는 test 또는 review 결과를 보여주되 credential, token, secret value를 노출하지 않아야 합니다. |
| `permission_model_change` | Role, ACL/RBAC rule, authorization check, admin capability, ownership check, access boundary 변경. | Approval은 permission-sensitive mutation을 포괄합니다. Role design, migration, audit expectation, default access, compatibility에는 Decision Packet을 사용합니다. Evidence는 보호 대상 subject data를 노출하지 않고 covered path와 policy check를 식별해야 합니다. |
| `schema_change` | Database, state, API, event, fixture, data-model shape 변경과 migration. | Approval은 민감한 schema 또는 migration side effect를 포괄합니다. Additive path와 breaking path 선택, backfill, rollback, compatibility window, maintenance cost에는 Decision Packet을 사용합니다. Evidence에는 migration/test coverage를 포함하고 production-like record는 redaction해야 합니다. |
| `dependency_change` | Runtime/build dependency 또는 dependency lock의 install, upgrade, removal, 변경. | Approval은 install, lockfile edit, dependency-file write를 포괄합니다. Dependency 채택이 architecture, compatibility, cost, license posture, rollback, maintenance를 바꾸면 Decision Packet을 사용합니다. Evidence에는 lockfile diff, test output, security 또는 license scan ref를 둘 수 있습니다. |
| `public_api_change` | CLI flag, HTTP endpoint, SDK contract, exported function, public config, documented behavior, module-boundary commitment 변경. | Approval은 민감한 write를 포괄할 수 있지만 compatibility와 breaking-change 판단에는 Decision Packet이 필요합니다. Evidence에는 caller-impact check, docs update, migration note, relevant test를 포함해야 합니다. |
| `destructive_write` | Delete, overwrite, irreversible migration step, data loss, reset operation, history/state removal. | Approval은 destructive side effect와 affected scope를 이름 붙여야 합니다. Rollback, backup, user impact, irreversibility, 잔여 위험 수용에는 Decision Packet을 사용합니다. Evidence에는 applicable한 dry-run, backup, diff, recovery ref가 있어야 합니다. |
| `network_write` | POST/PUT/PATCH/DELETE 또는 그에 준하는 네트워크 write operation. | Approval은 network target, method/class, payload class, expiry를 포괄합니다. External user impact, rollback, data selection, target ownership이 불확실한 경우 Decision Packet을 사용합니다. Evidence는 request를 안전하게 요약하고 secret 또는 PII payload를 생략해야 합니다. |
| `external_service_write` | Third-party service 또는 external account의 resource 생성, 변경, 삭제, configuration. | Approval은 external service action과 account/tenant boundary를 포괄합니다. Ownership, lifecycle, retention, cost, rollback, user notice, support impact에는 Decision Packet을 사용합니다. Evidence는 token이나 private raw payload 대신 service ref, id, redacted log를 사용해야 합니다. |
| `secret_access` | Credential, token, certificate, key, secret handle의 read, write, rotation, copy, use. | Approval은 named secret scope와 access kind를 포괄합니다. Secret choice, rotation plan, retention, audit trail, exposure risk에는 Decision Packet을 사용합니다. Evidence는 handle, `secret_omitted`, `blocked` artifact를 사용해야 하며 secret value를 log, projection, export, screenshot, summary에 넣으면 안 됩니다. |
| `production_config_change` | Production flag, environment variable, config file, runtime limit, operational default, safety switch 변경. | Approval은 production-sensitive config write를 포괄합니다. Rollout, rollback, user impact, monitoring, operational trade-off에는 Decision Packet을 사용합니다. Evidence에는 config diff 또는 plan ref를 두고 secret과 tenant-specific value는 redaction 또는 omission해야 합니다. |
| `ci_cd_change` | CI workflow, release pipeline, deployment automation, runner permission, signing, publishing, test gate 변경. | Approval은 pipeline 또는 automation mutation을 포괄합니다. Release policy, required gate, runner trust, rollback, deployment authority에는 Decision Packet을 사용합니다. Evidence에는 workflow diff와 secret value가 omitted된 run log를 포함해야 합니다. |
| `infra_or_deployment_change` | Cloud, container, Kubernetes, Terraform, provisioning, routing, scaling, deployment, operational topology 변경. | Approval은 infrastructure 또는 deployment side effect를 포괄합니다. Topology, rollout, rollback, availability, security boundary, cost, ownership choice에는 Decision Packet을 사용합니다. Evidence에는 plan/apply summary, affected resource, redacted provider output을 포함해야 합니다. |
| `privacy_or_pii_change` | PII 또는 privacy-sensitive data의 collection, storage, display, transformation, retention, deletion. | Approval은 privacy-sensitive action을 포괄합니다. Data minimization, field, retention, user notice, consent, access, redaction strategy에는 Decision Packet을 사용합니다. Evidence는 sanitized sample을 사용하고 PII는 artifact 등록 전에 redacted, omitted, blocked 상태가 되어야 합니다. |
| `data_export` | Report, file, external system, support bundle, user/operator download 등으로 현재 boundary 밖에 data를 내보내는 작업. | Approval은 어떤 data가 어디로 나가는지, destination, retention, expiry를 포괄합니다. Field selection, recipient authority, redaction, omitted value, audit trail, export와 관련해 받아들이는 위험에는 Decision Packet을 사용합니다. Evidence에는 redaction state를 보존하는 Evidence Manifest 또는 export ref를 포함해야 합니다. |
| `telemetry_or_logging_change` | Event, log, metric, trace, sampling, correlation id, log retention 추가, 삭제, 변경. | Approval은 behavior, user data, cost, operational risk를 노출할 수 있는 telemetry를 포괄합니다. Event semantics, privacy posture, retention, opt-out, observability trade-off, support burden에는 Decision Packet을 사용합니다. Evidence는 sanitized sample을 보여주고 raw secret 또는 PII를 피해야 합니다. |
| `license_or_compliance_change` | License file, notice, compliance control, audit evidence, legal commitment, policy-governed obligation 변경. | Approval은 compliance-sensitive write를 포괄합니다. Acceptable license posture, obligation handling, exception path, 위험을 받아들이는 판단에는 Decision Packet을 사용합니다. Evidence는 scan, notice, policy ref를 가리키되 restricted audit material을 불필요하게 노출하지 않아야 합니다. |
| `billing_or_cost_change` | Paid resource use, quota, billing configuration, plan, cost-bearing model call, usage limit 변경. | Approval은 비용이 발생하는 action과 budget boundary를 포괄합니다. Cost/performance trade-off, quota policy, user impact, rollback, 비용 지출과 관련해 받아들이는 위험에는 Decision Packet을 사용합니다. Evidence에는 estimate, limit, observed usage ref를 포함해야 합니다. |
| `model_or_prompt_policy_change` | Model selection, system/developer prompt, safety policy, tool policy, routing, evaluation policy, generated-output policy 변경. | Approval은 민감한 policy 또는 prompt write를 포괄합니다. Product tone, safety trade-off, data exposure, model cost, eval threshold, user-facing behavior에는 Decision Packet을 사용합니다. Evidence에는 eval ref와 필요한 경우 redacted prompt/policy artifact를 포함해야 합니다. |
| `policy_override` | Harness, project, security, QA, verification, compliance policy를 우회, 약화, waiver, exception 처리하는 작업. | Approval은 scope 안의 민감한 override step만 허가할 수 있습니다. 왜 exception이 받아들일 만한지, 어떤 위험을 받아들이는지, 어떤 follow-up이 남는지, close에 어떤 영향이 있는지에는 Decision Packet을 사용합니다. Evidence는 policy, waiver, Residual Risk, follow-up ref를 연결해야 합니다. |

Approval prompt는 일반 사용자가 이해하는 side effect를 먼저 말하고 identifier를 뒤에 붙여야 합니다. 예: "redaction된 billing CSV를 vendor X로 export해도 될까요? (`data_export`, `external_service_write`)." 같은 단계가 사용자 소유 제품 판단, 기술 구조 판단, 보안, QA, verification, 작업 수락, 잔여 위험 관련 판단, policy 판단도 결정한다면 그 판단은 compatible Decision Packet으로 연결해야 합니다. 그 판단이 write authority를 막고 있다면 `prepare_write`가 `allowed`를 반환하기 전에 applicable한 owner gate 의미에 따라 resolved, deferred, waived 또는 그 밖의 호환되는 방식으로 기록되어야 합니다.

## ArtifactRef

아티팩트 참조는 artifact store에 등록되어 지속 보관되는 근거 파일을 가리킵니다. Report projection과 record projection은 근거 파일 참조가 필요할 때 아티팩트 참조를 사용합니다. Projection 자체는 근거 파일이 아닙니다.

Artifact 등록은 임의 파일을 쌓아 두는 느슨한 파일 덤프가 아닙니다. Staged file은 Core가 staging 또는 capture source, stored-byte integrity, `redaction_state`, Task-scoped owner relation을 검증한 뒤에만 public `ArtifactRef`가 됩니다.

Reference implementation에서 artifact 등록은 Task-scoped입니다. `ArtifactRef.task_id`와 `ArtifactInput.relation.task_id`는 required이며 `artifacts.task_id`와 `artifact_links.task_id`에 대응합니다. `retention_class=project`는 retention policy에 영향을 줄 뿐 artifact ownership scope를 바꾸지 않습니다.

향후 Browser QA Capture는 새 reference schema가 아니라 이 artifact 경계를 사용합니다. 화면 capture는 보통 `screenshot`을 사용하고, 묶음 QA output은 `qa_capture`를 사용할 수 있습니다. Console log와 network trace는 `log` 또는 `qa_capture`를 사용할 수 있고, accessibility snapshot과 workflow recording은 명확한 description과 함께 `qa_capture` 또는 `other`를 사용할 수 있습니다. 이러한 artifact는 모두 redaction, secret/PII handling, Task-scoped ownership, 수동 QA record 또는 Feedback Loop attachment rules를 따라야 합니다. Capture artifact는 근거를 보강할 수 있지만 작업 수락을 만들거나, 수동 QA judgment를 대체하거나, 분리 검증을 충족하거나, 코어 권한 조각(v0.1 Core Authority Slice)에 필요한 capture schema를 추가하지 않습니다.

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
| `secret_omitted` | Secret value 또는 PII를 의도적으로 생략하거나 handle로 대체했습니다. Secret이 아닌 근거가 남아 있는 주장에는 도움이 될 수 있지만, 생략된 값 자체를 증명하는 evidence는 아닙니다. |
| `blocked` | 금지된 내용 때문에 capture 또는 원본 payload 저장이 차단되었습니다. Core가 blocked artifact ref를 기록했다면 metadata notice만 노출될 수 있으며 evidence, QA, verification, projection, export display는 원본 artifact를 사용할 수 있는 것처럼 보이지 않도록 차단 상태를 표시해야 합니다. |

`redacted`, `secret_omitted`, `blocked`에서 `sha256`, `size_bytes`, `content_type`은 숨겨진 원본이 아니라 커밋된 안전 저장 bytes를 설명합니다. `blocked`의 경우 이 bytes는 Core가 audit과 이후 표시를 위해 commit한 metadata-only notice이며, 금지된 원본 payload가 아닙니다. 이 notice artifact 자체는 차단된 capture의 사용 가능한 원본 근거가 아닙니다.

Evidence를 만들거나 연결하는 request는 `ArtifactInput`을 사용합니다. Request는 기존 committed artifact를 참조하거나, Core가 검증하고 등록한 뒤 `ArtifactRef`로 반환할 staged file을 제공할 수 있습니다.

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
- Existing artifact를 새 record에 연결할 때 Core는 artifact의 task relation을 검증하고 incompatible reuse를 거부합니다.
- `staged_uri`는 Harness staging location 또는 등록된 capture adapter output을 가리키는 locator이지, 임의 파일을 읽어도 된다는 권한이 아닙니다. Absolute path, parent traversal, symlink escape, repo-local path, 호출자 제공 URI는 staging 또는 capture adapter가 신뢰된 소스로 정규화하기 전까지 신뢰하지 않습니다.
- `staged_uri`, `display_name`, supplied `content_type`은 Core가 staging 또는 capture source, stored bytes, redaction state, owner relation을 검증하기 전까지 신뢰할 수 있는 input이 아닙니다.
- `expected_sha256` 또는 `expected_size_bytes`가 있으면 Core는 commit 전에 stored bytes를 확인합니다. 이 field가 제공되었는지와 무관하게 Core는 redaction, omission, blocking이 적용된 뒤의 safe stored bytes에서 committed `sha256`, `size_bytes`, `content_type`을 기록합니다.
- Core는 final storage 전에 redaction, omission, blocking policy를 적용하고 committed artifact를 `ArtifactRef`로 기록합니다.
- Secret 또는 PII를 포함할 수 있는 log, screenshot, network trace, export snapshot, 기타 captured evidence는 policy가 요구할 때 등록 전에 redacted, omitted, 또는 blocked 상태가 되어야 합니다.
- Policy가 omission 또는 blocking을 요구하면 committed ref는 `redaction_state=secret_omitted` 또는 `redaction_state=blocked`를 기록합니다. 호출자는 생략되었거나 차단된 bytes를 사용할 수 있는 evidence, QA material, verification input, projection body text, export payload로 취급하면 안 됩니다.
- Core가 기록한 `blocked` metadata-only notice는 여전히 committed registered artifact record입니다. Artifact ref, hash, size, content type, owner relation, retention class는 metadata-only notice bytes에 적용되며, 금지된 원본 bytes를 사용할 수 있게 만들지 않고 audit/display continuity를 보존합니다.
- Tool response는 기록된 `ArtifactRef` 값을 `registered_artifacts`, `bundle_ref`, 기타 response field로 반환합니다. Response는 `staged_uri`를 권한이나 durable evidence URI처럼 다시 노출하면 안 됩니다.
- `relation.record_kind`는 Core가 검증할 수 있는 기존 기준 owner 기록 또는 렌더링된 projection 참조를 이름으로 지정해야 합니다. Current Task-scoped artifact API의 non-projection owner에서는 concrete owner row가 `relation.task_id`와 같은 Task scope여야 합니다. 같은 owner kind의 project-scoped row는 future extension이 project-scoped artifact storage/API를 추가하기 전까지 artifact-link target이 아닙니다. Verification bundle은 `ArtifactRef.kind=bundle` 또는 `manifest`를 사용합니다. Export output은 `ArtifactRef.kind=export_component` 또는 `retention_class=export`를 사용합니다. `verification_bundle`과 `export`는 current reference artifact relation record kind가 아닙니다.
- `relation.record_kind=projection`은 Core가 `projection_jobs`를 통해 찾을 수 있는, 이미 렌더링되었거나 기록된 Task-scoped projection output에만 valid합니다. Current Task-scoped artifact API에서 `record_id_hint`는 `projection_jobs.projection_job_id`를 이름으로 지정하고, job의 `task_id`는 `relation.task_id`와 일치해야 합니다. Core는 hint를 검증할 때 `target_ref`와 `output_path`를 사용할 수 있지만, 이 값들이 identity에서 job id를 대체하지 않습니다. Project-level projection job은 owner docs가 허용하는 곳에서 존재할 수 있지만, current Task-scoped artifact API는 이를 위한 project-scoped artifact link를 등록하지 않습니다.

이후 consumer도 같은 의미를 유지해야 합니다. Evidence Manifest, 수동 QA, Eval, projection, export, Release Handoff, doctor, artifact integrity display는 ref, hash, 안전한 omission note, handle, blocked notice를 보여줄 수 있지만 생략되었거나 차단된 원본 값을 inline 표시하거나 재구성하거나 요약하거나 export하면 안 됩니다. `secret_omitted`는 secret이 아닌 근거가 보이는 주장만 충족할 수 있으며, 생략된 값이 필요한 주장은 unsupported 또는 insufficient로 남겨야 합니다. `blocked`는 replacement artifact, compatible waiver, Decision Packet outcome, Residual Risk record의 accepted-risk 상태, 또는 다른 documented resolution이 그 경로를 해소하기 전까지 evidence, QA, verification, projection, export, Release Handoff에서 시도된 input을 사용할 수 없는 것으로 취급한다는 뜻입니다.

Record 또는 projection references는 `ArtifactRef`가 아니라 `StateRecordRef`를 사용합니다.

```yaml
StateRecordRef:
  record_kind: task | change_unit | change_unit_dependency | run | approval | write_authorization | decision_packet | journey_spine_entry | shared_design | domain_term | module_map_item | interface_contract | feedback_loop | residual_risk | evidence_manifest | eval | manual_qa_record | tdd_trace | reconcile_item | projection
  record_id: string
  projection_path: string | null
```

`record_kind=projection`에서 `record_id`는 projection job identity인 `projection_jobs.projection_job_id`입니다. `projection_path`는 선택적 표시 및 복구 metadata입니다. 값이 있으면 job의 `output_path`를 mirror하거나 좁혀야 하며 같은 job 아래에서 찾을 수 있어야 합니다. Alternate key가 아니며 별도의 `projections` table을 뜻하지 않습니다.

Current reference API에는 `accepted_risk` `StateRecordRef.record_kind`가 없습니다. `accepted_risk_refs`, `accepted_refs`, 또는 accepted-risk equivalent로 이름 붙은 public fields는 `record_kind=residual_risk`인 `StateRecordRef` entries를 사용해야 합니다. Accepted risk는 그 Residual Risk records의 metadata/state입니다.

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

`WriteAuthorizationSummary`와 `WriteAuthoritySummary`는 API payload shape일 뿐입니다. 이 문서는 Write Authorization 기록에 대한 SQLite DDL을 정의하지 않습니다. `WriteAuthorizationSummary`는 `harness.prepare_write`가 반환한 durable single-use authorization을 나타냅니다. 같은 Run request의 idempotent replay를 제외하면 하나의 committed implementation 또는 direct `harness.record_run` consumption과만 호환됩니다. `WriteAuthoritySummary`는 client가 Write Authority Summary를 Autonomy Boundary 판단 재량 옆에 표시하기 위해 사용하는 display/read shape입니다.

Client가 guard, freeze, careful-mode control을 렌더링할 때는 권한 field를 추가하지 않고 이 기존 display shape를 사용합니다. `guarantee_display.level`과 `guarantee_display.notes`는 실제 연결된 capability와 현재 적용 경로를 설명해야 합니다. `blocked_reasons[].message`는 scope, MCP availability, Approval, baseline, capability 같은 구체적인 보류 또는 차단 조건을 이름 붙여야 하며, "guard"나 "freeze" 같은 command label만으로 더 강한 guarantee를 암시하면 안 됩니다.

`DEC`, `DESIGN`, `EXPORT`, `JOURNEY-CARD`, `RUN-SUMMARY`, `EVIDENCE-MANIFEST`, `EVAL`, `TDD-TRACE`, `MODULE-MAP`, `INTERFACE-CONTRACT`처럼 active stage/profile 밖에 있는 `ProjectionKind` 값은 해당 projection 기능 또는 profile이 켜졌을 때만 projection job kind로 유효합니다. Active stage/profile이 요구하는 Decision Packet visibility는 status/next responses, judgment-context resources, decision-packet resources, 최소 `TASK` 또는 card display를 통해 제공됩니다. 이는 standalone `DEC` `ProjectionKind`가 아니라 Decision Packet 사용자 판단 요청 display shape에 대한 요구사항입니다. Persisted `JOURNEY-CARD` Markdown과 Journey Spine-style output은 future/diagnostic 범위이며 status, next, significant resume flow의 현재 위치 맥락은 간결한 상태 출력으로 충족할 수 있습니다. 전체 projection template text는 [Template 참조](templates/README.md)에 있으며, 이 API schema file이 담당하지 않습니다.

Decision Packet, Write Authorization, Write Authority Summary, Journey Card, Judgment Context, Autonomy Boundary, Recommended Playbook, acceptance visibility, residual-risk summaries는 public MCP schemas입니다. 이 schemas는 API payload만 설명합니다. 기준 kernel records는 owner docs가 정의합니다. 이 목록에서 `RecommendedPlaybook`은 표시 전용 예외입니다. 자체 기준 kernel record, DDL table, task event, projection job이 없습니다.

Role Lens behavior는 이 기존 표시 및 routing schema를 사용합니다. Role lens는 `RecommendedPlaybook`으로 나타날 수 있고, existing Decision Packet으로 route할 수 있으며, `DecisionPacketCandidate`를 propose할 수 있습니다. 별도의 public payload schema, 권한 기록, 상태 전이를 도입하지 않습니다.

Judgment 품질은 아래 public field와 [Decision Packet](kernel.md#decision-packet), [Decision Gate](kernel.md#decision-gate)의 kernel authority rule로 판단합니다. 충분한 packet은 사용자가 무엇을 판단하는지, category, internal route, display depth, relevant scope/ref, 기록될 option 또는 outcome을 보여줘야 합니다. 선택, source, evidence, impact, agent latitude를 솔직히 보여줄 수 없으면 broad approval처럼 제시하지 말고 blocked 또는 narrowed로 다룹니다.

v0.2 judgment model은 세 축입니다.

| Field | Values | 의미 |
|---|---|---|
| `judgment_category` | `product_ux`, `technical_architecture`, `security_privacy`, `scope_autonomy`, `qa_verification`, `work_acceptance`, `residual_risk`, `mixed` | 사용자에게 보이는 judgment category입니다. |
| `judgment_route` | `choose`, `defer`, `approve-sensitive-action`, `waive`, `accept-result`, `accept-risk`, `reconcile` | 어떤 owner path와 resolution rule을 쓸지 정하는 internal route입니다. |
| `display_depth` | `simple`, `tradeoff`, `high-risk`, `close-affecting` | Prompt depth와 필요한 context 수준입니다. |

이전 field와의 compatibility mapping은 임시로 명시합니다.

| Older field/value family | Current field/value family |
|---|---|
| `judgment_domain` | `judgment_category`. 기존 `qa_acceptance`는 실제 판단에 따라 `qa_verification` 또는 `work_acceptance`로 나눕니다. |
| `decision_kind` | `judgment_route`. Product, architecture, design, scope choice는 `choose`; sensitive-action approval은 `approve-sensitive-action`; QA/verification waiver는 `waive`; work acceptance는 `accept-result`; residual-risk acceptance는 `accept-risk`; reconcile은 `reconcile`입니다. |
| `decision_profile` | `display_depth`. 간단한 prompt는 `simple`, trade-off prompt는 `tradeoff`, approval/security/privacy/waiver/risk prompt는 보통 `high-risk`, work acceptance 또는 close-affecting risk prompt는 `close-affecting`입니다. |
| `RequestUserDecisionRequest` / `RecordUserDecisionRequest` | `RequestUserJudgmentRequest` / `RecordUserJudgmentRequest`의 compatibility alias입니다. |

`judgment_category`는 authority를 만들거나 gate를 충족하지 않습니다. Payload branch도 고르지 않습니다. `display_depth`는 prompt가 보여줘야 할 context 양을 정합니다. Approval, waiver, result acceptance, risk acceptance, close authority를 만들지 않습니다. 하나의 answer가 실제로 여러 concerns를 함께 다룰 때만 `mixed`를 씁니다. 아니면 Decision Packet을 분리합니다.

`DecisionPacketCandidate`, `RecommendedPlaybook.route`, `judgment_packet_route`, optional implementation `decision_requests`는 request/display/routing metadata입니다. [`harness.request_user_judgment`](#harnessrequest_user_judgment)로 caller를 안내할 수 있지만, owner path가 compatible `DecisionPacket`과 linked owner record를 commit 또는 update하기 전에는 authority가 아닙니다. Stored Decision Packet이 committed `judgment_category`, `judgment_route`, `display_depth`, `judgment_payload`를 소유합니다.

아래 schema block은 이 문서의 YAML-like notation만 사용합니다. `JudgmentPayload`의 route-specific field는 `judgment_route`로 validate합니다. `approval_scope`는 `approve-sensitive-action`에서만 필요합니다. `waiver`는 `waive`에서만 필요합니다. `acceptance`는 `accept-result`에서만 필요합니다. `residual_risk_acceptance`는 `accept-risk`에서만 필요합니다. `reconcile`은 `reconcile`에서만 필요합니다. `display_depth=tradeoff`, `high-risk`, `close-affecting`은 informed judgment에 충분한 option, ref, risk, consequence, "does not cover" text를 요구합니다. `display_depth=simple`은 extra trade-off text가 중요하지 않으면 concise option과 `null` detail을 쓸 수 있습니다.

```yaml
DecisionPacket:
  decision_packet_id: string
  task_id: string
  change_unit_id: string | null
  status: proposed | pending_user | resolved | deferred | rejected | blocked | superseded
  judgment_category: product_ux | technical_architecture | security_privacy | scope_autonomy | qa_verification | work_acceptance | residual_risk | mixed
  judgment_route: choose | defer | approve-sensitive-action | waive | accept-result | accept-risk | reconcile
  display_depth: simple | tradeoff | high-risk | close-affecting
  context:
    why_now: string
    source_refs: StateRecordRef[]
    evidence_refs: EvidenceRefs
  state_summary_at_request: StateSummary
  what_user_is_judging: string
  what_agent_may_decide_without_user: string[]
  affected_scope: DecisionPacketScope
  affected_gates: DecisionPacketGateRef[]
  affected_acceptance_criteria: DecisionPacketCriterionRef[]
  judgment_payload: JudgmentPayload
  resolution: JudgmentResolution | null
  expires_at: string | null
  created_at: string
  updated_at: string
  resolved_at: string | null

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
  option_id: string | null  # null이면 recommended option이 없다는 뜻이며, reason이 이유를 설명해야 한다.
  reason: string
  uncertainty: string | null
  when_to_revisit: string | null

JudgmentUserContext:
  minimum_context: string[]
  optional_pull_refs: StateRecordRef[]

DecisionPacketScope:
  scope_refs: StateRecordRef[]
  product_areas: string[]
  files_or_paths: string[]
  acceptance_criteria_refs: StateRecordRef[]
  note: string | null

DecisionPacketGateRef:
  gate: scope_gate | decision_gate | approval_gate | design_gate | evidence_gate | verification_gate | qa_gate | acceptance_gate
  blocked_action: string | null

DecisionPacketCriterionRef:
  criteria_id: string
  statement: string

DecisionPacketResolution:
  selected_option_id: string | null
  judgment: RecordUserJudgmentPayload | null
  note: string | null

JudgmentPayload:
  options: JudgmentOption[]
  recommendation: JudgmentRecommendation | null
  uncertainty: string | null
  deferral_consequence: string | null
  user_context: JudgmentUserContext | null
  approval_scope: ApprovalScope | null
  covers: string[]
  does_not_cover: string[]
  waiver: WaiverJudgment | null
  acceptance: AcceptanceJudgment | null
  residual_risk_acceptance: ResidualRiskAcceptanceJudgment | null
  reconcile: ReconcileJudgment | null
  separate_decisions_required: string[]

WaiverJudgment:
  skipped_check: string
  waiver_reason: string
  gate_or_close_impact: string
  residual_risk_refs: StateRecordRef[]
  follow_up: string | null

AcceptanceJudgment:
  result_ref: StateRecordRef | null
  result_summary: string
  evidence_status_refs: StateRecordRef[]
  verification_status_refs: StateRecordRef[]
  qa_status_refs: StateRecordRef[]
  residual_risk_visibility: ResidualRiskSummary
  does_not_replace: string[]

ResidualRiskAcceptanceJudgment:
  residual_risk_refs: StateRecordRef[]
  accepted_scope: string[]
  acceptance_consequence: string
  follow_up_required: boolean
  follow_up: string | null
  evidence_refs: EvidenceRefs

ReconcileJudgment:
  reconcile_item_id: string
  target_refs: StateRecordRef[]
  options: JudgmentOption[]

DecisionPacketCandidate:
  # Candidate 공통 field는 canonical id/status/timestamp를 제외하고 DecisionPacket 공통 field와 맞춘다.
  task_id: string
  change_unit_id: string | null
  judgment_category: product_ux | technical_architecture | security_privacy | scope_autonomy | qa_verification | work_acceptance | residual_risk | mixed
  judgment_route: choose | defer | approve-sensitive-action | waive | accept-result | accept-risk | reconcile
  display_depth: simple | tradeoff | high-risk | close-affecting
  context:
    why_now: string
    source_refs: StateRecordRef[]
    evidence_refs: EvidenceRefs
  state_summary_at_request: StateSummary
  what_user_is_judging: string
  what_agent_may_decide_without_user: string[]
  affected_scope: DecisionPacketScope
  affected_gates: DecisionPacketGateRef[]
  affected_acceptance_criteria: DecisionPacketCriterionRef[]
  judgment_payload: JudgmentPayload
  expires_at: string | null

NextActionSummary:
  action_kind: ask_user | prepare_write | implement | launch_verify | record_eval | record_manual_qa | request_acceptance | close_task | reconcile | idle
  summary: string
  required_tool: string | null
  related_refs: StateRecordRef[]
  blocker_code: ErrorCode | null

RecommendedPlaybook:
  playbook_id: string
  label: string
  reason: string
  applies_to:
    focus: status | shaping | judgment | implementation | verification | qa | acceptance | reconcile
    state_refs: StateRecordRef[]
  route:
    display_route: continue_guidance | show_existing_decision_packet | propose_decision_packet_request | write_readiness_guidance | evidence_guidance | verification_guidance | manual_qa_guidance | close_readiness_guidance | reconcile_guidance
    decision_packet_ref: StateRecordRef | null
    judgment_packet_route: none | existing_decision_packet | decision_packet_candidate_or_request_path
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

`ResidualRiskSummary.status=none`은 현재 Task와 requested action에 대해 Core가 알고 있는 close-relevant Residual Risk가 없다는 뜻입니다. 이는 작업 수락과 ordinary successful close에서 잔여 위험 표시를 충족하며, 이때 `close_relevant_count=0`이고 risk-ref array는 비어 있습니다. Core가 hidden, blocked, 또는 표시되지 않은 close-relevant risk를 알고 있다면 이 status를 반환하면 안 되며, 그런 경우 `not_visible` 또는 `blocked`를 사용합니다.

`ResidualRiskSummary.visible_refs`, `not_visible_refs`, `unaccepted_refs`, `accepted_refs`, related acceptance visibility risk-ref array는 `record_kind=residual_risk`인 `StateRecordRef` entry를 포함합니다. `visible_refs`는 현재 judgment context에서 visible한 close-relevant Residual Risk record를 나열하며, 잔여 위험 수용이 아직 필요하면 `unaccepted_refs`가 visible risk와 overlap될 수 있습니다. Accepted risk는 Residual Risk record의 metadata/state로 남습니다.

표시에서는 "none"과 "not visible"을 반드시 구분해야 합니다. `status=none`은 requested action에 대해 알려진 close-relevant Residual Risk가 없다는 current-state claim입니다. `status=not_visible`은 알려진 close-relevant risk가 있지만 작업 수락 또는 close에 필요한 맥락으로 아직 보이지 않았다는 blocker 또는 작업 수락 전 경고입니다. 사용자에게 보이는 summary는 status와 관련 risk refs, 또는 명시적인 empty ref set을 함께 렌더링해야 합니다.

Autonomy Boundary summary는 범위 권한이 아니라 판단 재량을 설명합니다. Active Change Unit scope와 필요한 민감 동작 승인 밖의 paths, tools, commands, network targets, secret access, sensitive categories를 허가하지 않습니다.

`judgment_route=approve-sensitive-action`은 민감 동작 승인을 위한 judgment만 뜻합니다. 제품 장단점, 설계 방향, 아키텍처 판단이나 기술 구조 판단, 해결되지 않은 security/privacy judgment, QA waiver, verification waiver, verification risk, work acceptance, residual-risk acceptance는 별도의 compatible Decision Packet과 owner-record update가 있어야 해소됩니다.

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

에이전시 보증 팩(v0.3 Agency Assurance Pack)과 운영과 인계 팩(v0.4 Operations & Handoff Pack)의 ValidatorResult ID는 다음과 같습니다.

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

이 smaller subset에서 빠진 design-quality validator, 즉 `shared_design_alignment`, `vertical_slice_shape`, `domain_language_consistency`, `module_interface_review`는 위 full stable ValidatorResult-emitting set에 계속 포함됩니다.

아래 tool description은 `ValidatorResults emitted`와 Core check/precondition을 구분합니다. Core check는 transition을 막거나, gate를 갱신하거나, blocked reason을 채우거나, fixture assertion에 나타날 수 있지만 위에 나열되지 않는 한 validator ID가 아닙니다.

## Error taxonomy

| Code | Meaning |
|---|---|
| `VALIDATION_FAILED` | request payload, enum value, activation rule, profile-specific schema validation이 mutation 전에 실패함 |
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
| `APPROVAL_REQUIRED` | sensitive action requires Approval before proceeding |
| `APPROVAL_DENIED` | the relevant Approval was denied |
| `APPROVAL_EXPIRED` | Approval expired or drifted from baseline/scope |
| `CAPABILITY_INSUFFICIENT` | 연결된 surface는 valid하지만 required validator, feature, enforcement condition을 충족할 수 없음 |
| `MCP_UNAVAILABLE` | required MCP access가 unavailable, `stale`, 또는 unreachable임 |
| `LOCAL_ACCESS_MISMATCH` | Core 또는 operator가 caller의 local access mode를 off-profile, stale, weak, forwarded/tunneled, cross-user, unauthorized for registered local profile, 또는 mismatch 상태로 분류할 수 있음 |
| `EVIDENCE_INSUFFICIENT` | required 근거 coverage가 absent, partial, `stale`, 또는 blocked임 |
| `VERIFY_NOT_DETACHED` | verification cannot count as 분리 검증 |
| `QA_REQUIRED` | required 수동 QA is pending, failed, or missing |
| `ACCEPTANCE_REQUIRED` | required 작업 수락이 pending 또는 rejected임 |
| `PROJECTION_STALE` | requested action에 필요한 projection 최신성이 `stale` 또는 `failed`임 |
| `RECONCILE_REQUIRED` | human-editable or managed-block drift requires reconcile |
| `RESIDUAL_RISK_NOT_VISIBLE` | known close-relevant residual risk가 작업 수락 또는 successful close 전에 표시되지 않음 |
| `ARTIFACT_MISSING` | a referenced artifact file is missing or integrity check failed |
| `BASELINE_STALE` | baseline no longer matches the repository state required by the operation |
| `VALIDATOR_FAILED` | 하나 이상의 required validators 또는 close/blocker checks가 failed이고 더 specific한 typed `ErrorCode`가 적용되지 않을 때 사용하는 generic fallback |

`WRITE_AUTHORIZATION_REQUIRED`와 `WRITE_AUTHORIZATION_INVALID`는 missing 또는 invalid Write Authorization에만 사용합니다. Observed paths, tools, commands, network targets, secrets, sensitive categories가 authorized 또는 active scope를 넘는 경우 scope violations는 계속 `SCOPE_VIOLATION`을 사용합니다.

MCP availability, local access/profile mismatch, capability insufficiency는 서로 다릅니다.

- `MCP_UNAVAILABLE` with diagnostic `MCP_SERVER_UNAVAILABLE`: tool 호출이 Core에 닿을 수 없어 authoritative Core response가 불가능합니다. 호출자는 상태 변경을 주장하기 전에 진단하거나 reconnect해야 합니다.
- `MCP_UNAVAILABLE` with diagnostic `SURFACE_MCP_UNAVAILABLE`: Core 또는 operator는 연결된 surface에 usable MCP가 없거나, MCP configuration이 stale이거나, required MCP tool을 호출할 수 없는 상태를 관찰할 수 있습니다.
- `LOCAL_ACCESS_MISMATCH`: Core 또는 operator가 reachable local endpoint 또는 caller path를 registered local access/profile boundary 밖으로 분류할 수 있습니다. Detail은 `ToolErrorLocalAccessDetails`를 사용하며 raw tokens, private config, sensitive file contents를 노출하면 안 됩니다.
- `CAPABILITY_INSUFFICIENT`: caller가 recognized surface/profile 위에 있지만 그 profile이 required capability, validator, enforcement condition을 충족할 수 없습니다.

MCP availability problem에 대해 `ToolError` object를 사용할 수 있는 경우 `details.mcp_unavailable_kind`는 `server_unavailable`, `surface_mcp_unavailable`, `stale_connection`, `unknown` 중 하나일 수 있습니다.

`LOCAL_ACCESS_MISMATCH`를 반환할 때 `details.local_access_issue`는 `off_profile`, `stale_profile`, `weak_binding`, `forwarded_or_tunneled`, `cross_user`, `unauthorized_local_caller`, `unknown` 중 하나로 문제를 구분해야 합니다. Diagnostic detail에는 safe handle, fingerprint, profile ref, surface id 같은 값을 넣을 수 있습니다. Raw secrets, tokens, private logs, private configuration values, sensitive file contents는 넣으면 안 됩니다.

사용자 표시에서는 `ErrorCode` 값을 raw code로만 되풀이하지 말고 표시 라벨과 next-action 문장으로 바꿔 보여줘야 합니다. 아래 라벨은 표시 지침이며 새 public `ErrorCode`가 아닙니다.

| API condition | 사용자 표시 라벨 | 가장 작은 해소 방법 문장 |
|---|---|---|
| `VALIDATION_FAILED` | 잘못된 request | Retry 전에 payload, enum value, activation rule, profile-specific field set을 고칩니다. |
| `STATE_CONFLICT` | 상태 충돌 | 현재 Task 또는 project status를 새로 읽은 뒤 현재 state version으로 다시 시도하거나 original idempotent request를 그대로 재실행합니다. |
| `MCP_UNAVAILABLE`(`details.mcp_unavailable_kind=server_unavailable`) 또는 진단상 `MCP_SERVER_UNAVAILABLE` | MCP 사용 불가: Core에 닿을 수 없음 | 기준 상태 변경, Approval, gate update, projection repair, close를 주장하기 전에 Core 연결을 복구하거나 진단합니다. |
| `MCP_UNAVAILABLE` 또는 `CAPABILITY_INSUFFICIENT`(`details.mcp_unavailable_kind=surface_mcp_unavailable`) 또는 진단상 `SURFACE_MCP_UNAVAILABLE` | 현재 접점에서 MCP 사용 불가 | 필요한 MCP tool을 호출할 수 있는 접점으로 전환하거나 현재 접점을 복구합니다. 더 강한 guard가 실제로 실행을 막는 경우가 아니면 product write는 지시로 보류합니다. |
| `LOCAL_ACCESS_MISMATCH` | local access profile mismatch | Core state change를 주장하기 전에 registered local surface/profile로 다시 연결하거나 local binding/profile을 고칩니다. |
| `CAPABILITY_INSUFFICIENT` | 접점 capability 부족 | 필요한 enforcement 또는 validator capability가 있는 접점이나 profile을 쓰거나, 작업을 줄이거나, 그 기능이 필요 없는 경로를 선택합니다. |
| `NO_ACTIVE_TASK` | active Task 없음 | Task-scoped action을 사용하기 전에 Task를 선택하거나 만듭니다. |
| `WRITE_AUTHORIZATION_REQUIRED`, `WRITE_AUTHORIZATION_INVALID` | 쓰기 권한 없음 또는 최신 아님 | 제품 파일 쓰기 전에 정확한 의도한 작업, 현재 범위, 현재 상태로 `harness.prepare_write`를 호출하거나 다시 시도합니다. |
| `NO_ACTIVE_CHANGE_UNIT`, `SCOPE_REQUIRED`, `SCOPE_VIOLATION`, `AUTONOMY_BOUNDARY_EXCEEDED`, `BASELINE_STALE` | 범위, 사용자 판단 경계 또는 baseline 문제 | Scope를 확인하거나 줄이고, Change Unit 또는 baseline을 갱신하거나, 진행 전에 필요한 Decision Packet을 요청합니다. |
| `DECISION_REQUIRED`, `DECISION_UNRESOLVED` | 사용자 판단 필요 | 관련 Decision Packet 또는 judgment prompt를 `display_depth`에 맞는 options 또는 chosen outcome, refs, 필요할 때 recommendation, uncertainty, deferral effect와 함께 보여줍니다. |
| `APPROVAL_REQUIRED`, `APPROVAL_DENIED`, `APPROVAL_EXPIRED` | Approval 필요 또는 사용할 수 없음 | Sensitive-action Approval을 요청, 해소, 갱신합니다. Approval을 Write Authorization이나 product judgment처럼 다루면 안 됩니다. |
| `EVIDENCE_INSUFFICIENT`, `VERIFY_NOT_DETACHED`, `QA_REQUIRED`, `ACCEPTANCE_REQUIRED`, `RESIDUAL_RISK_NOT_VISIBLE` | 근거, 검증, QA, 작업 수락 또는 위험 표시 필요 | 빠진 check를 기록하거나 다시 실행하고, Residual Risk를 보여주고, 작업 수락을 요청하거나, 담당 경로로 유효한 면제 판단을 기록합니다. |
| `PROJECTION_STALE` | 오래된 읽기용 상태 보기 | 읽기용 보기에 의존하기 전에 projection을 refresh 또는 reconcile합니다. 기준 상태를 직접 읽을 수 있으면 기준 상태가 권한 있는 출처로 남습니다. |
| `RECONCILE_REQUIRED` | reconcile 필요 | 영향받는 projection 또는 close path를 사용하기 전에 human-editable 또는 managed-block drift를 reconcile합니다. |
| `ARTIFACT_MISSING` | artifact 문제 | 해당 artifact를 evidence로 의존하기 전에 missing 또는 failed artifact를 다시 첨부하거나, 생성하거나, 교체합니다. |
| `VALIDATOR_FAILED` | check 실패 | 가능하면 구체적인 validator finding을 보여주고 가장 작은 구체적 수정을 이름 붙입니다. 더 specific한 typed blocker가 없을 때만 이 fallback을 사용합니다. |

`DECISION_REQUIRED`, `DECISION_UNRESOLVED`, `WRITE_AUTHORIZATION_REQUIRED`, `WRITE_AUTHORIZATION_INVALID`, `AUTONOMY_BOUNDARY_EXCEEDED`, `RESIDUAL_RISK_NOT_VISIBLE`, `MCP_UNAVAILABLE`, `LOCAL_ACCESS_MISMATCH`, `VALIDATION_FAILED`는 stable public `ErrorCode` values입니다. Validator-specific detail은 여전히 `ValidatorResult.findings`에 속합니다.

### Primary Error Code Precedence

Public tool response는 Core가 여러 blocker를 동시에 관찰해도 하나의 primary `ToolError.code`만 가집니다. `ToolResponseBase.errors`가 비어 있지 않으면 `errors[0]`가 이 precedence table로 선택된 primary `ToolError`이고, 나머지 entry는 secondary blocker를 나타낼 수 있습니다. Tool subsection이 더 좁은 order를 정의하지 않는 한 primary code는 아래 precedence list에서 처음 적용되는 code입니다. Secondary blocker는 `blocked_reasons`, `CloseTaskResponse.blockers`, validator result, `ToolError.details`, state summary 같은 tool-specific field에 유지합니다.

표시에서는 primary error가 "다음 단계를 가장 먼저 막는 것은 무엇인가?"에 답합니다. 추가 막힘은 plan, close readiness, user judgment, evidence work를 바꿀 때 계속 보여야 하지만 가장 먼저 해소할 막힘과 경쟁시키지 말고 "추가로 막는 것" 또는 "그다음 막힘"으로 묶습니다. 가장 작은 해소 방법은 가장 먼저 해소할 막힘을 기준으로 제시하되, `CloseTaskResponse.blockers[].required_next_action` 같은 tool-specific field가 같은 blocker에 대해 더 정확한 action을 제공하면 그것을 사용합니다.

`Possible errors` list는 tool에서 사용할 수 있는 code를 나열합니다. 이는 per-tool precedence table이 아닙니다.

모든 public tool은 method-specific handling 전에 request/schema/activation validation failure에 대해 `VALIDATION_FAILED`를 반환할 수 있습니다. Core 또는 operator가 reachable caller/access path가 registered local profile 밖이라고 분류할 수 있으면 `LOCAL_ACCESS_MISMATCH`도 반환할 수 있습니다. 이 global errors는 모든 tool-specific `Possible errors` list에 반복하지 않습니다.

MCP server나 호출자가 Core에 전혀 닿을 수 없으면 접점 또는 operator는 `MCP_UNAVAILABLE`을 보고할 수 있습니다. 하지만 권한 있는 Core response나 상태 변경을 주장할 수는 없습니다. Core가 request를 평가할 수 있으면 다음 순서를 적용합니다.

| Precedence | Primary `ErrorCode` | Selection note |
|---:|---|---|
| 1 | `VALIDATION_FAILED` | request payload, enum, activation, profile-specific field validation이 mutation 전에 실패함 |
| 2 | `STATE_CONFLICT` | 최신이 아닌 `expected_state_version`, state lock conflict, 또는 같은 idempotency key가 다른 payload로 reused됨 |
| 3 | `MCP_UNAVAILABLE` | Core 또는 operator가 availability problem을 분류한 뒤 required MCP access가 사용할 수 없거나, `stale`이거나, unreachable임 |
| 4 | `LOCAL_ACCESS_MISMATCH` | reachable local caller/access mode가 off-profile, stale, weak, forwarded/tunneled, cross-user, 또는 registered local profile에 unauthorized임 |
| 5 | `NO_ACTIVE_TASK` | operation에 Task가 필요하지만 active 또는 addressed Task가 없음 |
| 6 | `NO_ACTIVE_CHANGE_UNIT` | operation이 write-capable 또는 close-relevant인데 active scoped Change Unit이 적용되지 않음 |
| 7 | `BASELINE_STALE` | requested operation이 최신이 아닌 baseline에 의존함 |
| 8 | `SCOPE_REQUIRED` | requested operation이 proceed하기 전에 scope confirmation이 필요함 |
| 9 | `SCOPE_VIOLATION` | intended 또는 observed paths, tools, commands, network, secrets, categories가 active 또는 authorized scope를 초과함 |
| 10 | `WRITE_AUTHORIZATION_REQUIRED` | write-capable Run에 required Write Authorization이 없음 |
| 11 | `WRITE_AUTHORIZATION_INVALID` | supplied Write Authorization이 `stale`, expired, revoked, replay 밖에서 consumed, 또는 incompatible함 |
| 12 | `APPROVAL_DENIED` | relevant 민감 동작 승인이 denied됨 |
| 13 | `APPROVAL_EXPIRED` | relevant 민감 동작 승인이 expired되었거나 scope 또는 baseline에서 drift됨 |
| 14 | `APPROVAL_REQUIRED` | sensitive change에 민감 동작 승인이 필요하지만 compatible granted 민감 동작 승인이 없음 |
| 15 | `DECISION_UNRESOLVED` | existing relevant Decision Packet이 pending, deferred without coverage, rejected, blocked, `stale`, 또는 incompatible함 |
| 16 | `AUTONOMY_BOUNDARY_EXCEEDED` | intended operation이 active Change Unit Autonomy Boundary를 초과하며, next step이 Decision Packet이어도 이 code를 사용함 |
| 17 | `DECISION_REQUIRED` | 사용자 소유 판단이 action 진행을 막고 있어 Decision Packet이 필요함 |
| 18 | `CAPABILITY_INSUFFICIENT` | 연결된 접점이 required capability 또는 enforcement condition을 충족할 수 없음 |
| 19 | `EVIDENCE_INSUFFICIENT` | required 근거 coverage가 absent, partial, `stale`, 또는 blocked임 |
| 20 | `VERIFY_NOT_DETACHED` | verification이 분리 검증으로 count될 수 없음 |
| 21 | `QA_REQUIRED` | required 수동 QA가 pending, failed, missing, 또는 validly waived되지 않음 |
| 22 | `RESIDUAL_RISK_NOT_VISIBLE` | known close-relevant 잔여 위험이 작업 수락 또는 close 전에 visible하지 않음. `ResidualRiskSummary.status=none`이 no known close-relevant risk를 confirm한 경우에는 선택하지 않음 |
| 23 | `ACCEPTANCE_REQUIRED` | 잔여 위험 표시가 satisfied된 뒤에도 required 작업 수락이 pending 또는 rejected임 |
| 24 | `PROJECTION_STALE` | requested action에 필요한 projection freshness가 `stale` 또는 `failed`임 |
| 25 | `RECONCILE_REQUIRED` | human-editable 또는 managed-block drift에 reconcile이 필요함 |
| 26 | `ARTIFACT_MISSING` | referenced artifact file이 missing이거나 integrity check에 failed함 |
| 27 | `VALIDATOR_FAILED` | 위의 더 specific한 typed blocker가 적용되지 않을 때만 선택되는 generic validator fallback |

<a id="harnessclose_task-close-blockers"></a>

#### `harness.close_task` Close Blockers

`harness.close_task`는 여러 close blocker를 반환할 수 있습니다. `CloseTaskResponse.base.errors`의 primary `ToolError`는 위 precedence를 사용합니다. Present하면 `CloseTaskResponse.base.errors[0].code`가 primary close error code입니다. `CloseTaskResponse.blockers`는 관찰된 close blocker를 같은 relative order의 structured result로 포함해야 합니다. Status, report, Journey view, agent summary의 prose는 blocker를 설명할 수 있지만, prose-only text는 close-blocker result가 아닙니다. Required 작업 수락은 close-relevant 잔여 위험이 visible한 뒤에만 record하거나 rely할 수 있으므로 close 및 작업 수락 flow에서 잔여 위험 표시는 `ACCEPTANCE_REQUIRED`보다 앞에 둡니다.

표시되었지만 수용되지 않은 close-relevant risk는 `RESIDUAL_RISK_NOT_VISIBLE`로 반환하지 않습니다. Requested close path가 잔여 위험 수용을 요구하면 public close/API response는 잔여 위험 수용 Decision Packet을 요청해야 할 때 primary `DECISION_REQUIRED`를 사용하고, 관련 잔여 위험 수용 Decision Packet이 pending, rejected, blocked, stale, deferred without coverage, incompatible일 때 `DECISION_UNRESOLVED`를 사용합니다. Structured close blocker category는 `residual_risk_acceptance`여야 하며, `related_refs`는 관련 `residual_risk` refs 또는 pending Decision Packet refs를 인용해야 합니다.

## Idempotency

Idempotency keys는 `(project_id, tool_name, idempotency_key)`에 scoped됩니다. 같은 key로 같은 payload를 반복하면 original committed response를 반환합니다. 같은 key를 다른 payload로 reuse하면 `STATE_CONFLICT`를 반환합니다.

`request_hash`는 UTF-8로 encode한 정규화된 JSON에서 계산합니다. 정규화된 input은 `tool_name`, schema-normalized request body, 그리고 `request_id`와 `idempotency_key`를 제외한 모든 `ToolEnvelope` field를 포함합니다. 포함되는 envelope fields는 `expected_state_version`, `project_id`, `task_id`, `surface_id`, `run_id`, `actor_kind`, `dry_run`입니다. Hashing 전에 optional fields는 request schema의 default 및 null/empty-field rule에 따라 normalize하고, object keys는 sort하며, arrays는 schema가 order-insignificant라고 명시한 경우가 아니면 schema-defined order를 유지하고, Unicode strings는 NFC를 사용해 일관되게 normalize합니다.

같은 key를 다른 canonical request payload와 함께 reuse하면 `STATE_CONFLICT` response는 민감한 request body를 노출하지 않으면서 replay 문제를 진단할 수 있게 해야 합니다. `ToolError.details`에는 idempotency scope, stored/received request hash 또는 그에 준하는 opaque comparison, 호출자가 original request를 replay하거나 fresh key로 retry해야 한다는 사실을 포함할 수 있습니다.

State-changing tool에서 Core는 call을 새 mutation attempt로 취급하기 전에 existing committed replay row를 확인합니다. `(project_id, tool_name, idempotency_key)`에 committed `tool_invocations` row가 있으면 Core는 canonical `request_hash`를 먼저 비교합니다. Hash가 일치하면 original committed response를 반환하며, current `expected_state_version` freshness check를 다시 실행하거나, event를 append하거나, artifact를 등록하거나, projection을 enqueue하거나, replay row를 update하지 않습니다. Hash가 다르면 idempotency replay mismatch로 `STATE_CONFLICT`를 반환하고 original replay row를 보존합니다. Committed replay row가 없을 때만 Core는 mutation 전에 resolved affected scope에 대해 `expected_state_version`을 평가합니다.

## State Conflict 동작

Supplied idempotency scope에 committed replay row가 없는 state-changing tool에서 Core는 mutation 전에 `expected_state_version`을 현재 project/Task state와 비교합니다. 일치하지 않으면 `STATE_CONFLICT`를 반환하고 `details`에 현재 state version과 status summary를 포함합니다. 이 conflicting new attempt에 대해서는 current record, event, artifact, projection job, replay row를 만들지 않습니다. 호출자는 상태를 새로 읽은 뒤 새 idempotency key로 retry하거나 exact previous request를 replay해야 합니다.

그 새 mutation attempt 경로에서 state conflict 비교는 scope-specific입니다. Core는 먼저 `ToolEnvelope.task_id`, tool-specific `task_id`, 또는 active Task resolution에서 operation이 가리키는 primary Task를 찾습니다. Task-scoped tool은 해당 Task의 `tasks.state_version`과 비교하고, 찾은 primary Task가 없는 project-scoped tool은 `project_state.state_version`과 비교합니다. `STATE_CONFLICT.details`에는 `scope`(`task` 또는 `project`), `current_state_version`, `expected_state_version`, relevant `project_id`, 그리고 `scope=task`일 때 `task_id`를 포함해야 합니다. Refresh guidance를 위한 compact status summary도 포함할 수 있습니다.

최신이 아닌 `expected_state_version`은 호출자 identity의 증거가 아니라 concurrency drift로 보고합니다. 진단 표시는 어떤 scope가 stale이었는지, Core가 관찰한 current version이 무엇인지, retry 전에 호출자가 refresh해야 한다는 점을 말해야 합니다. 호출자가 제공했다는 이유로 오래된 Task 또는 project view를 받아들이면 안 됩니다.

## Public tools

### Public Tool Schema Map

Public method는 staged surface별로 묶어 읽습니다. 같은 method가 later profile에서 payload 의미를 확장하면 여러 stage에 나타날 수 있습니다. 그렇다고 later profile이 더 이른 stage에서 active가 되는 것은 아닙니다. 아래 tool section은 method/profile이 범위에 들어왔을 때의 정확한 request/response contract입니다.

#### v0.1 Core Authority Slice surface

| Method 또는 capability | Activation | Scope boundary |
|---|---|---|
| `harness.status` | Active minimal status/blocker read. | Current Core state, write-authority summary, structured blocker를 반환합니다. Future-profile 값은 schema가 허용하는 `null`, empty array, `unknown`, `not_required`로 표현합니다. Projection rendering은 요구하지 않습니다. |
| `harness.prepare_write` | Active product-write authorization check. | 하나의 allowed/blocked path와, allowed일 때 durable single-use Write Authorization 하나를 만듭니다. Approval-shaped Decision Packet과 Approval record lifecycle은 later-profile입니다. |
| `harness.record_run` | Active Run recording and Write Authorization consumption. | Compatible implementation/direct Run 하나를 기록하고 authorization 하나를 한 번 consume하며, owner path를 통해 artifact/evidence ref 하나를 등록합니다. Full Evidence Manifest, TDD, Eval, Manual QA update는 later-profile입니다. |
| `harness.intake` 또는 owner-valid setup path | Active setup capability입니다. v0.1에서 method 자체는 optional입니다. | v0.1은 local project, Task, scoped work boundary가 필요합니다. 이 public method를 사용한다면 minimal setup shape만 사용하고, full natural-language intake/discovery는 v0.2입니다. |
| `harness.next` | Optional v0.1 read. | Smoke path에서 구현한다면 다음 minimal authority-loop action 또는 smallest unblocker만 반환합니다. Verification, QA, acceptance, reconcile action kind는 later-profile입니다. |
| `harness.close_task` | Optional narrow blocker/status smoke only. | Structured blocker를 보여주는 가장 단순한 방법일 때만 사용할 수 있습니다. v0.1은 작업 수락, 잔여 위험 수용, full close semantics를 증명하지 않습니다. |

#### v0.2 User-Facing Harness MVP surface

| Method 또는 capability | Activation | Scope boundary |
|---|---|---|
| `harness.status.next_actions`; optional `harness.next` | Active fuller user-facing status/next display. | 현재 위치, pending user judgments, evidence summary, close readiness/blocker, smallest unblocker를 사용자 언어로 보여줍니다. 작업 수락과 잔여 위험 사실은 관련 있을 때 distinct하게 남지만 필수 projection kind를 늘리지는 않습니다. |
| `harness.intake` | Active user-facing intake/resume path. | Ordinary user work를 schema mode로 분류하면서 scope, non-goals, acceptance criteria, user-owned judgment boundary를 보존합니다. |
| `harness.request_user_judgment`와 `harness.record_user_judgment` | User-owned judgment와 work acceptance가 progress 또는 close를 막을 때 active입니다. | Packet/candidate가 만들어질 때 Decision Packet field가 active입니다. `harness.request_user_decision`과 `harness.record_user_decision`은 compatibility alias입니다. Approval hardening, waiver, full residual-risk acceptance, reconcile profile은 owner profile이 켜질 때 들어옵니다. |
| `harness.record_run` | Active evidence/artifact summary path. | MVP path에서 evidence가 충분히 visible해야 합니다. Full Evidence Manifest projection과 assurance record는 명시적으로 enabled되기 전까지 later-profile입니다. |
| `harness.close_task` | Active close-readiness and blocker response. | Close는 Core-owned로 남습니다. Structured blocker는 evidence, user judgment, work acceptance, residual-risk visibility를 구분합니다. |
| Minimal `TASK` projection 또는 compact status/card output | Active display capability입니다. Persisted projection job일 필요는 없습니다. | Status/next/card output이 최소 사용자 읽기용 MVP summary를 충족하면 persisted Markdown rendering은 optional입니다. |

#### v0.3 Agency Assurance surface

| Method 또는 capability | Activation | Scope boundary |
|---|---|---|
| `harness.launch_verify`와 `harness.record_eval` | Detached verification profile이 enabled될 때 active입니다. | Launch는 detached candidate 또는 bundle을 만들 뿐입니다. Recorded qualifying Eval만 verification/assurance를 update할 수 있습니다. |
| `harness.record_manual_qa` | Manual QA profile이 enabled될 때 active입니다. | Human QA judgment와 valid waiver path는 screenshot, browser capture, work acceptance, verification과 구분됩니다. |
| `harness.request_user_judgment`와 `harness.record_user_judgment` assurance profiles | Approval-shaped judgment, QA/verification waiver, full residual-risk acceptance, work-acceptance separation이 enabled될 때 active입니다. | Approval은 Write Authorization이 아닙니다. Waiver와 accepted risk는 detached verification 또는 generic acceptance를 만들지 않습니다. |
| `harness.record_run` assurance payloads | Evidence manifest, feedback-loop, TDD profile이 enabled될 때 active입니다. | Rich evidence, feedback-loop, TDD record는 해당 enabled profile에서만 active입니다. |
| `ValidatorResult` emitting paths | Active profile이 enable한 agency assurance validators에 대해 active입니다. | Core check는 별도로 block할 수 있습니다. 나열된 stable validator ID만 validator result입니다. |

#### v0.4 Operations surface

| Method 또는 capability | Activation | Scope boundary |
|---|---|---|
| `harness.status`, `harness.next`, `harness.close_task`, response `projection_jobs`의 projection freshness | Operations projection support가 enabled될 때만 active입니다. | Projection job과 ref는 Core state에서 파생된 view입니다. Evidence, acceptance, risk acceptance, close authority, write authority를 만들지 않습니다. |
| `harness.request_user_judgment` / `harness.record_user_judgment`의 Reconcile profile | Reconcile storage/operations support가 enabled될 때만 active입니다. | Reconcile candidate는 existing Decision Packet/owner path가 outcome을 commit하기 전까지 state가 되지 않습니다. |
| Operator diagnostics, recover/export, artifact integrity, conformance commands | v0.4 Operations owner surface이며, 현재 이 문서의 public MCP tool은 아닙니다. | 정확한 command semantics는 [Operations And Conformance Reference](operations-and-conformance.md)가 담당합니다. 이 capability를 future MCP tool로 승격하면 matching storage profile support와 함께 이 stage에 추가합니다. |

#### v1+ / future candidates

Dashboard, hosted workflow UI, broad connectors, Browser QA Capture automation, Cross-Surface Verification automation, Context Index, team workflow, orchestration 같은 후보는 owner document가 승격하기 전까지 현재 public MCP method가 없습니다. 승격 뒤에는 current state, refs, artifacts, projections를 읽거나 보여줄 수 있지만 UI 또는 derived response에 나타난다는 이유만으로 authority가 되면 안 됩니다.

#### Method index

Current public MCP tool set에는 standalone `harness.record_evidence` method가 없습니다. Evidence는 `harness.record_run`의 artifact/evidence update를 통해 들어오고, later profile에서는 `harness.launch_verify`, `harness.record_eval`, `harness.record_manual_qa` owner record와 artifact refs를 통해 들어옵니다. Projection refresh, recover/export, artifact integrity check, conformance execution도 현재 이 문서의 public MCP tool section이 아닙니다. Owner document가 public MCP method로 승격하기 전까지 Operations가 해당 command surface를 담당합니다.

| Tool | 이 section에서 찾는 것 |
|---|---|
| [`harness.status`](#harnessstatus) | status, gate, 범위에 있을 때의 projection freshness, write authority, guarantee, residual risk, recommended playbook |
| [`harness.intake`](#harnessintake) | tracked work 시작 또는 resume, 초기 Task/Change Unit shaping |
| [`harness.next`](#harnessnext) | next-action과 smallest-unblocker display payload |
| [`harness.prepare_write`](#harnessprepare_write) | write precondition check, blocked reason, approval candidate, Write Authorization summary |
| [`harness.record_run`](#harnessrecord_run) | run recording, artifact/evidence update, feedback loop, TDD trace, Write Authorization consumption |
| [`harness.request_user_judgment`](#harnessrequest_user_judgment) | Decision Packet creation, approval-shaped judgment request, user-judgment prompt |
| [`harness.record_user_judgment`](#harnessrecord_user_judgment) | Decision Packet, 민감 동작 승인, 면제 판단, 작업 수락, 잔여 위험 judgment 해결 |
| [`harness.close_task`](#harnessclose_task) | close request/response, close state, blocker, close result, close projection ref |
| [`harness.launch_verify`](#harnesslaunch_verify) | verification launch request/response와 bundle ref |
| [`harness.record_eval`](#harnessrecord_eval) | Eval recording, verification verdict, independence qualifier, artifact ref |
| [`harness.record_manual_qa`](#harnessrecord_manual_qa) | 수동 QA result, waiver link, residual-risk ref, QA artifact |

### `harness.status`

Purpose: project, surface, active Task, 간결한 현재 위치 요약, gate, guarantee, 범위에 있을 때의 projection freshness, active Decision Packet, Autonomy Boundary, Write Authority Summary, residual-risk, pending-decision status를 반환합니다.

Stage/profile: v0.1은 minimal status/blocker profile을 사용합니다. v0.2는 더 풍부한 사용자 대상 status/card profile을 추가합니다. v0.3은 verification, QA, evidence, residual-risk, validator profile이 켜졌을 때 assurance summary를 추가합니다. v0.4는 Operations storage가 있을 때 projection/reconcile/operations freshness를 추가합니다. Response schema는 정확하게 유지하지만, future-profile field는 해당 capability를 v0.1에 강제하지 않고 `null`, empty array, `unknown`, `not_required` 값을 가질 수 있습니다.

v0.1 Core Authority Slice에서는 이를 내부 authority loop를 위한 minimal status/blocker output으로 사용할 수 있습니다. Future-profile field는 그 capability가 v0.1 범위에 없을 때 schema 의미에 따라 `null`, empty, `unknown`, 또는 `not_required`일 수 있습니다.

사용자에게 보이는 의미: 현재 위치를 보여줍니다. Status 표시는 active Task, 현재 phase, 가장 먼저 해소할 막힘이 있으면 그 막힘, 가장 작은 해소 방법, 쓰기 권한 상태, 보장 수준, projection freshness를 먼저 보여야 합니다. Ref와 추가 막힘을 함께 보여줄 수 있지만, 사용자가 계속 진행할 수 있는지 이해하려고 raw schema field만 읽게 만들면 안 됩니다.

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
  next_actions: NextActionSummary[]
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

Status response profiles:

| Profile | Profile-scoped response meaning |
|---|---|
| v0.1 minimal | Present하면 active Task/status, compact `status_card`, current state version, 요청되고 사용 가능할 때 write-authority summary, guarantee display, primary structured blocker를 반환합니다. `next_actions`는 empty이거나 minimal blocker/action 하나를 담을 수 있습니다. Future-profile array는 empty이고 future-profile object는 schema에 따라 `null` 또는 `unknown`입니다. |
| v0.2 user-facing | `next_actions`, pending-judgment visibility, evidence/status summary, close-readiness/blocker display를 추가합니다. 작업 수락과 잔여 위험 사실은 관련 있을 때 나타나지만 별도 projection kind를 요구하지 않습니다. |
| v0.3/v0.4 assurance and operations | Verification, Manual QA, validator, projection freshness, reconcile, diagnostic/export-related ref는 해당 profile과 storage가 enabled될 때만 추가합니다. |

State transition summary: state transition 없음.

반환될 수 있는 EventRef values: 없음.

Projection job 대기열 추가: 없음.

`pending_decisions`는 해소되지 않은 user-action Decision Packets를 포함합니다. `active_decision_packet_refs`는 pending, deferred, blocked, recently resolved packet을 포함해 현재 phase 또는 requested action과 relevant한 모든 Decision Packet을 포함합니다. 두 field는 모두 `record_kind=decision_packet`인 `StateRecordRef` entry를 사용합니다.

`recommended_playbooks`는 접점 또는 agent stage router를 위한 non-authoritative display guidance이며, status/next display를 위해 현재 상태와 policy/playbook context에서 계산됩니다. Shared design, review, TDD, QA, guard check, release handoff, browser-QA candidacy 같은 절차를 제안할 수 있습니다. `RecommendedPlaybook.playbook_id`는 stable display/routing string identifier이지 Core-owned closed enum이나 DDL-backed value set이 아닙니다. Known initial ID에는 `shared-design`, `product-review`, `eng-review`, `design-review`, `security-review`, `tdd-loop`, `spec-review`, `code-quality-review`, `qa-review`, `guard-check`, `release-handoff`, `browser-qa-candidate`가 포함되며, 이 목록은 future display/playbook documentation 전체를 포괄하지 않습니다. Recommended Playbook은 자체 기준 kernel 기록, DDL table, `task_events` entry, projection job이 없습니다. Recommendation을 따를 때 사용자 소유 판단이 필요하면 route는 affected write 또는 close가 진행되기 전에 existing Decision Packet 또는 normal Decision Packet candidate/request path를 가리켜야 합니다. `route.display_route` 값은 display route이지 public tool name이 아니며 상태 변경 tool call 지시도 아닙니다. Role Lens/playbook의 전체 권한 없음 경계는 [Agent Integration](agent-integration.md#role-lens-동작)이 담당합니다.

`StatusResponse.recommended_playbooks`와 `StatusResponse.journey_card.recommended_playbooks`가 둘 다 present이면, 둘은 같은 computed guidance를 다른 display level에 렌더링한 것입니다. Top-level field는 compact status만 렌더링하는 status 접점용이고, Journey Card field는 해당 future/diagnostic view가 켜졌을 때 같은 guidance를 현재 위치 요약과 함께 유지합니다.

`write_authority_summary`는 `include.write_authority=true`일 때 반환됩니다. `include.journey_card=true`이면 같은 current Write Authority Summary display가 `journey_card.write_authority_summary`에도 나타날 수 있습니다.

`projection_freshness.status`가 `stale`, `failed`, `unknown`이면 `status_card`가 사용자의 현재 위치 파악에는 도움을 줄 수 있습니다. 하지만 읽기용 보기가 stale, failed, unknown임을 표시해야 하며, 그 보기를 신뢰할 수 있는 context로 쓰기 전에는 refresh 또는 reconcile을 가장 작은 해소 방법으로 가리키는 것이 좋습니다.

ValidatorResults emitted: optional `surface_capability_check`, optional `decision_gate_check`, optional `autonomy_boundary_check`.

Core checks/preconditions: optional 잔여 위험 표시 read, optional projection freshness read.

Possible errors: `MCP_UNAVAILABLE`, `PROJECTION_STALE`.

Idempotency behavior: read-only입니다. Repeated request는 상태를 변경하지 않습니다.

### `harness.intake`

Purpose: user intent에서 Task를 create 또는 resume하고 schema mode `advisor`, `direct`, `work`로 분류합니다. 사용자에게 보이는 표시는 이를 읽기/조언, 작은 변경, 추적되는 작업으로 설명할 수 있습니다.

Stage/profile: v0.1은 implementation이 public API setup을 선택한 경우 minimal owner-valid Task/scope setup path로만 이 method를 사용할 수 있습니다. Full natural-language intake, discovery quality, procedural budget display, ordinary-user resume behavior는 v0.2 User-Facing Harness MVP 범위입니다.

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

Projection job 대기열 추가: source record를 변경하는 committed non-dry-run intake에서는 Task projection support가 범위에 있으면 `TASK`; intake가 design support record를 accepted했고 해당 projection kind가 범위에 있으면 optional `DOMAIN-LANGUAGE`, `MODULE-MAP`, `INTERFACE-CONTRACT`. Dry-run, pre-commit state-conflict, rejected pre-commit request는 projection job을 대기열에 넣지 않습니다.

ValidatorResults emitted: `surface_capability_check`.

Core checks/preconditions: `state_envelope`, `active_task_policy`.

Possible errors: `STATE_CONFLICT`, `MCP_UNAVAILABLE`, `VALIDATOR_FAILED`, `CAPABILITY_INSUFFICIENT`.

Idempotency behavior: 같은 key는 같은 Task/resume decision을 반환합니다. 같은 key에 다른 payload를 사용하면 `STATE_CONFLICT`입니다.

### `harness.next`

Purpose: 현재 Task의 next safe action, instruction bundle, 대기 중인 판단을 반환합니다.

Stage/profile: optional v0.1 minimal next-action read입니다. v0.2는 더 풍부한 사용자 대상 next action과 smallest-unblocker display를 제공합니다. v0.3/v0.4 assurance, QA, acceptance, verification, reconcile action kind는 owner profile이 enabled될 때만 사용합니다.

사용자에게 보이는 의미: 다음에 무엇을 해야 하는지 보여줍니다. `next_action.summary`는 사용자에게 질문하기, 이 쓰기 준비하기, evidence 기록하기, verification 실행하기, 수동 QA 기록하기, 작업 수락 요청하기, refresh 또는 reconcile하기, close하기처럼 평범한 행동 언어여야 합니다. `next_action.required_tool`은 caller hint이지, power-user detail이 유용한 경우가 아니라면 사용자가 반드시 봐야 하는 명령이 아닙니다.

Allowed actor: `user`, `lead_agent`, `evaluator`, `operator`.

Request schema:

```yaml
NextRequest:
  envelope: ToolEnvelope
  task_id: string | null
  focus: status | shaping | judgment | implementation | verification | qa | acceptance | reconcile
  include_instruction_bundle: boolean
```

Response schema:

```yaml
NextResponse:
  base: ToolResponseBase
  state: StateSummary | null
  next_action: NextActionSummary
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

Next response profiles:

| Profile | Profile-scoped response meaning |
|---|---|
| v0.1 minimal | 구현된 경우 scoped write 준비, authorized Run 기록, missing scope/write authority 보고, missing artifact/evidence support 보고 같은 다음 authority-loop action 또는 smallest blocker만 반환합니다. `recommended_playbooks`와 future-profile refs는 empty일 수 있고, owner state가 없으면 `judgment_context`와 `autonomy_boundary`는 `null`일 수 있습니다. |
| v0.2 user-facing | Ordinary-language next action, user-owned judgment가 필요할 때 pending Decision Packet refs와 judgment context, evidence/close-readiness context를 반환합니다. 작업 수락과 잔여 위험 사실은 관련 있을 때 나타나지만 별도 projection kind를 요구하지 않습니다. |
| v0.3/v0.4 assurance and operations | Verification, Eval, Manual QA, acceptance, reconcile, projection freshness, operations-oriented next action은 해당 profile이 enabled되고 matching storage가 있을 때만 반환합니다. |

v0.1 Core Authority Slice에서 `harness.next`는 optional입니다. 사용한다면 scoped write 준비, authorized Run 기록, missing scope 보고, missing artifact/evidence support 보고 같은 minimal authority-loop action만 반환해야 합니다. `verification`, `qa`, `acceptance`, `reconcile` focus value와 관련 action kind는 future-profile behavior입니다.

`next_action.action_kind` 의미:

| `action_kind` | 사용자에게 보이는 의미 | 권한 경계 |
|---|---|---|
| `ask_user` | 이름 붙은 경로를 계속하기 전에 사용자 소유 판단, 민감 동작 승인, QA 판단, 작업 수락, 잔여 위험 수용, 또는 다른 답변이 필요합니다. | Prompt는 사용자가 무엇을 판단하는지와 관련 refs를 말해야 하며, 그 자체로 write나 close를 허가하지 않습니다. |
| `prepare_write` | 정확히 의도한 제품 파일 쓰기를 지금 해도 되는지 확인합니다. | `harness.prepare_write`가 compatible Write Authorization을 반환하기 전에는 제품 파일 쓰기가 허가되지 않습니다. |
| `implement` | 이미 범위가 잡힌 작업을 수행합니다. 제품 파일 쓰기에는 current compatible Write Authorization만 사용합니다. | Scope를 넓히거나 오래된 authorization을 재사용하거나 사용자 소유 판단을 해결하지 않습니다. |
| `launch_verify` | Current evidence와 source refs에서 verification path를 준비하거나 시작합니다. | 많아야 detached candidate를 만들 뿐이며, passing Eval이나 assurance upgrade가 아닙니다. |
| `record_eval` | Evaluator 결과와 reviewed refs를 기록합니다. | Core가 qualifying Eval을 기록하고 gate 또는 assurance state를 갱신하기 전에는 verification passed가 아닙니다. |
| `record_manual_qa` | Human 수동 QA outcome 또는 valid waiver path를 기록합니다. | Browser artifact, smoke run, note만으로는 수동 QA가 아니며 수동 QA record 또는 valid waiver가 필요합니다. |
| `request_acceptance` | Evidence, verification, 수동 QA, 잔여 위험 표시를 보여준 뒤 작업 수락을 요청합니다. | 작업 수락은 evidence, verification, 수동 QA, 민감 동작 승인, scope, 잔여 위험 표시, 잔여 위험 수용을 대체하지 않습니다. |
| `close_task` | `harness.close_task`로 close, cancel, supersede를 시도합니다. | Close attempt는 여전히 blocker를 반환할 수 있으며, status text나 report만으로 Task가 닫히지 않습니다. |
| `reconcile` | 오래된 projection, managed-block drift, proposal text, state/display mismatch를 refresh 또는 reconcile합니다. | Reconcile 표시는 기존 reconcile/owner path가 받아들이기 전에는 상태가 아닙니다. |
| `idle` | 요청한 focus에 필요한 즉시 Harness action이 없습니다. | Task가 닫혔거나, 수락됐거나, verified됐거나, risk-free라는 뜻이 아닙니다. |

State transition summary: state transition 없음.

반환될 수 있는 EventRef values: 없음.

Projection job 대기열 추가: 없음.

`pending_decisions`는 해소되지 않은 user-action Decision Packets를 포함합니다. 현재 phase 또는 requested action에 아직 영향을 주는 deferred, blocked, recently resolved packet은 `judgment_context.active_decision_packet_refs`를 통해 나타납니다.

`recommended_playbooks`는 반환된 next safe action에 맞는 절차를 호출자가 선택하도록 돕습니다. 이는 현재 상태와 policy/playbook context에서 계산되는 API/display guidance일 뿐입니다. `playbook_id`는 display/routing string identifier로 남으며 기준 kernel enum이 아닙니다. 그 자체로 state transition, event, projection, gate, write, evidence, verification, QA, risk, 작업 수락, close 효과를 만들지 않습니다. 사용자 소유 판단을 새로 요구하는 playbook recommendation은 affected write 또는 close가 진행되기 전에 Decision Packet candidate/request path 또는 existing Decision Packet으로 라우팅해야 합니다. `route.display_route` 값은 display route이지 public tool name이 아니며 상태 변경 tool call 지시도 아닙니다. 전체 Role Lens/playbook 경계는 [Agent Integration](agent-integration.md#role-lens-동작)이 담당합니다.

`RecommendedPlaybook.route`, `judgment_packet_route`, 선택적 구현 `decision_requests` 같은 routing metadata는 그 자체로 authority가 아닙니다. Caller를 existing Decision Packet 또는 owner-record path로 안내할 수는 있지만 민감 동작 승인, 사용자 소유 판단, 작업 수락, 면제 판단, 잔여 위험 수용, Write Authorization, close를 충족할 수 없습니다.

`focus=acceptance`일 때 `judgment_context.acceptance_visibility`는 non-null이어야 합니다. 이 context는 residual-risk summary, unaccepted close-relevant risk refs, evidence summary refs, verification status, QA status, 작업 수락 상태, 작업 수락이 대체하지 않는 것을 포함해야 합니다. 이 context는 known close-relevant risk가 없다는 뜻의 `ResidualRiskSummary.status=none`과, known close-relevant risk가 아직 hidden이라는 뜻의 `not_visible`을 구분해야 합니다. 작업 수락 요청 전에 작업 수락이 evidence sufficiency, verification, 수동 QA, 민감 동작 승인, scope, 잔여 위험 표시, 잔여 위험 수용을 대체하지 않는다는 점을 명확히 보여줘야 합니다. 이 visibility context 없이 작업 수락을 요청하는 response는 incomplete display이며 작업 수락 authority로 취급하면 안 됩니다.

Next action이 blocked이면 가장 먼저 해소할 막힘과 가장 작은 해소 방법을 먼저 보여줍니다. 그 막힘이 해소된 뒤에도 같은 close, write, verification, QA, 작업 수락 경로를 막을 추가 막힘은 후속 context로 보여줍니다.

ValidatorResults emitted: optional `surface_capability_check`, optional `decision_gate_check`, optional `autonomy_boundary_check`, optional `context_hygiene_check`.

Possible errors: `DECISION_REQUIRED`, `DECISION_UNRESOLVED`, `NO_ACTIVE_TASK`, `MCP_UNAVAILABLE`, `PROJECTION_STALE`, `AUTONOMY_BOUNDARY_EXCEEDED`, `RECONCILE_REQUIRED`.

Idempotency behavior: read-only입니다. Repeated request는 상태를 변경하지 않습니다.

<a id="harnessprepare_write"></a>

### `harness.prepare_write`

Purpose: agent가 write하기 전에 intended product write가 allowed인지 결정합니다. 이는 public 제품 파일 쓰기 권한에 대한 유일한 decision point입니다.

Stage/profile: v0.1 core authority loop에서 active입니다. Approval candidate는 Approval state를 만들지 않고 반환할 수 있습니다. Committed Approval record와 approval-shaped Decision Packet lifecycle은 later Approval/Agency Assurance profile과 matching storage가 필요합니다.

사용자에게 보이는 의미: 지금 이 정확한 product write를 해도 되는지 답합니다. 이 답은 현재 active Task, active Change Unit scope, Autonomy Boundary, baseline freshness, 민감 동작 승인, Decision Packet coverage, design policy, surface capability에 기반합니다. `decision=allowed`이면 허용된 작업, 범위 근거, durable single-use Write Authorization ref 또는 summary, 보장 수준의 한계, detective/cooperative limitation을 보여줍니다. 쓰기가 blocked이면 가장 먼저 해소할 이유와 가장 작은 해소 방법을 보여줍니다. Approval 또는 Decision Packet candidate payload는 담당 tool path로 commit되기 전까지 candidate일 뿐입니다.

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

`approval_request_candidate`는 `decision=approval_required`이거나 Core가 새 Approval 요청을 제안할 수 있을 때만 포함합니다. 그 외에는 `null`입니다. 이는 이후 `harness.request_user_judgment(judgment_route=approve-sensitive-action)` 호출의 `approval_scope`에 사용할, 상태를 변경하지 않는 candidate입니다. `prepare_write`가 이를 반환해도 Approval 기록, Decision Packet, Write Authorization, `APR` projection job은 생성되지 않습니다. UI, status response, next-action response가 Approval 요청 commit 전에 이 payload를 표시한다면 이를 candidate 표시로 label해야 하며 `APR` projection이라고 부르면 안 됩니다.

`dry_run=false`이고 `decision=allowed`일 때 response는 non-null `write_authorization_ref`를 포함해야 합니다. 호출자가 expanded payload를 request하거나 implementation이 지원하면 `write_authorization` summary도 반환할 수 있습니다. `authorization_effect`는 Core가 새 authorization을 create하면 `created`입니다.

`WriteAuthorizationSummary.basis_state_version`은 Core가 allowed write attempt의 compatibility basis로 사용한 affected-scope state version입니다. Current reference prepare-write product write에서는 `task_id`의 Task State Version입니다. Replay와 최신성 감지 audit metadata이며, response의 resulting `base.state_version`이 아닙니다.

`authorization_effect=returned`는 같은 idempotency key, request hash, `basis_state_version`을 가진 동일한 committed `prepare_write` request와 response의 idempotent replay에만 reserved됩니다. 서로 다른 compatible request는 서로 다른 Write Authorization을 생성합니다. Compatibility가 authorization을 reusable하게 만들지는 않습니다. Compatibility basis가 바뀌면 Core는 오래된 unconsumed authorization을 `stale`, expire, revoke할 수 있습니다.

`dry_run=true`이고 write가 otherwise allowed라면 Core는 `decision=allowed`와 `authorization_effect=would_create`를 반환합니다. 하지만 `write_authorization_ref`와 `write_authorization`은 반드시 `null`이어야 하고, Write Authorization record, event, artifact, projection job은 create되지 않습니다.

`decision=blocked`, `decision=approval_required`, `decision=decision_required`, `decision=state_conflict`에서는 두 authorization fields가 모두 `null`이고 `authorization_effect=none`이어야 합니다.

Write Authorization은 intended operation과 현재 state, baseline, active Change Unit scope, Approval ref, Decision Packet ref, sensitive categories, 보장 수준에 한정됩니다. 이는 `write_authorization_id`를 통해 `harness.record_run`이 consume하며 재사용 가능한 grant가 아닙니다. 하나의 authorization은 같은 committed `record_run` request의 idempotent replay를 제외하면 하나의 committed implementation 또는 direct Run과만 호환됩니다.

`active_decision_packet_refs`는 intended write와 relevant한 모든 Decision Packets를 포함합니다. Pending, deferred, blocked, recently resolved packets가 포함됩니다.

`decision_packet_candidate`는 `decision=decision_required`이고 compatible Decision Packet이 아직 없을 때 present합니다. Field는 envelope 이후의 `RequestUserJudgmentRequest`와 일치합니다. 여기에는 `judgment_category`, `judgment_route`, `display_depth`, `judgment_payload`가 포함됩니다. 이는 나중에 `harness.request_user_judgment`를 호출하기 위한 non-mutating candidate payload입니다. `prepare_write`가 이를 반환해도 Decision Packet이 생성되거나 update되지는 않습니다.

상태 전이 요약: Task를 `executing`, `waiting_user`, `blocked`로 옮길 수 있습니다. Allowed일 때 Write Authorization을 생성하거나 idempotent replay에 대해 already committed response를 반환할 수 있습니다. `scope_gate=pending/blocked`, `decision_gate=required/pending/blocked`, `approval_gate=required/expired`, `stale` evidence/Approval marker를 set할 수 있습니다. `approval_gate=pending`은 `harness.request_user_judgment(judgment_route=approve-sensitive-action)`이 Approval 형태 Decision Packet과 연결된 pending Approval 기록을 생성할 때 시작됩니다.

반환될 수 있는 stable EventRef values: `prepare_write_allowed`, `write_authorization_created`, `write_authorization_returned`, `prepare_write_blocked`, `scope_required`, `decision_required`, `autonomy_boundary_exceeded`, `approval_required`, `baseline_stale_detected`, `capability_insufficient_detected`.

Projection job 대기열 추가: Task display나 blocker에 영향을 주는 committed non-dry-run state change에 대해, Task projection support가 범위에 있으면 `TASK`. `prepare_write`는 `decision=approval_required` 또는 `approval_request_candidate`를 반환했다는 이유만으로 `APR`을 대기열에 넣으면 안 됩니다. `APR`은 기록된 Approval 기록과 Approval 형태 Decision Packet lifecycle에만 reserved됩니다. Dry-run, state-conflict, pure pre-commit rejection path는 projection job을 대기열에 넣지 않습니다. Idempotent replay는 새 work를 enqueue하지 않고 original job refs를 반환합니다.

ValidatorResults emitted: `autonomy_boundary_check`, `decision_gate_check`, `decision_quality_check`, `feedback_loop_check`, `tdd_trace_required`, `codebase_stewardship_check`, applicable design-quality validators, `surface_capability_check`.

`tdd_trace_required`가 적용되면 `prepare_write`는 actual RED evidence와 valid TDD 면제 판단이 없는 non-test implementation write에 design-policy blocker를 보고할 수 있습니다. Intended operation이 failing RED check를 만드는 test-path write라면 scope, baseline, 민감 동작 승인, Autonomy Boundary, other required check가 통과할 때 계속 진행할 수 있습니다. RED target 또는 plan은 그 test-path write를 뒷받침할 수 있지만, non-test implementation write를 위한 RED-evidence precondition이나 Evidence Manifest coverage를 충족하면 안 됩니다. Blocker는 validator 결과, blocked reason, 필요한 경우 secondary error/details로 표현합니다. Primary `ToolError.code`는 API precedence table에 따라 선택합니다.

Core checks/preconditions: `state_envelope`, `active_task`, `active_change_unit`, `scope_coverage`, `changed_paths_intent`, `baseline_freshness`, `approval_scope`, write 전 applicable한 design preconditions.

Possible errors: `STATE_CONFLICT`, `DECISION_REQUIRED`, `DECISION_UNRESOLVED`, `NO_ACTIVE_TASK`, `NO_ACTIVE_CHANGE_UNIT`, `SCOPE_REQUIRED`, `SCOPE_VIOLATION`, `AUTONOMY_BOUNDARY_EXCEEDED`, `APPROVAL_REQUIRED`, `APPROVAL_DENIED`, `APPROVAL_EXPIRED`, `BASELINE_STALE`, `CAPABILITY_INSUFFICIENT`, `MCP_UNAVAILABLE`, `VALIDATOR_FAILED`.

Idempotency behavior: 같은 payload로 repeated allowed/blocked decision은 original decision과 event refs를 반환합니다. 같은 key에 changed payload를 사용하면 `STATE_CONFLICT`입니다.

#### Approval Lifecycle

Sensitive-action Approval은 다음 절차를 따릅니다.

1. `harness.prepare_write`가 intended product write의 sensitive categories를 감지합니다.
2. Scope, baseline, sensitive categories, paths, tools, commands, network targets, secret access, capability 요구사항을 포괄하는 compatible granted 민감 동작 승인이 없으면 `prepare_write`는 `decision=approval_required`를 반환하고, `approval_request_candidate`를 포함하며, 두 Write Authorization field를 `null`로 두고 `authorization_effect=none`을 사용하며, Task blocker를 update할 수 있습니다. Task display 또는 blocker에 영향을 주는 committed non-dry-run state change이고 Task projection support가 범위에 있을 때만 `TASK`를 대기열에 넣습니다. 이 상태를 변경하지 않는 candidate 때문에 Approval 기록, Decision Packet, Write Authorization, `APR` projection job을 생성하면 안 됩니다.
3. 호출자는 candidate와 current intended write에서 파생한 `approval_scope`로 `harness.request_user_judgment`를 `judgment_route=approve-sensitive-action`과 함께 호출합니다.
4. Core는 Approval 형태의 사용자 판단을 위한 기준 Decision Packet과 pending Approval 기록을 생성합니다. Response는 `decision_packet_ref`와 `approval_id`를 모두 포함하며, 이 커밋된 Approval 요청은 APR projection support가 범위에 있을 때 `APR`을 대기열에 넣습니다.
5. User 또는 operator는 해당 Decision Packet에 대해 `harness.record_user_judgment`를 호출합니다.
6. Core는 Decision Packet 해소를 기록하고 연결된 Approval 기록을 업데이트하며 `approval_gate`를 granted, denied, expired 중 하나로 다시 계산하고, APR projection support가 범위에 있을 때 업데이트된 Approval 결정을 위해 `APR`을 다시 대기열에 넣습니다.
7. Approval이 granted이면 호출자는 fresh idempotency key와 current `expected_state_version`으로 `harness.prepare_write`를 다시 호출합니다.
8. 그 retry만 Write Authorization을 만들 수 있습니다. Granted 민감 동작 승인의 scope, baseline, sensitive categories, paths, tools, commands, network targets, secret scope, Decision Packet refs, Approval refs, capability checks가 current intended write와 compatible할 때만 성공합니다.

Approval은 정해진 scope 안의 sensitive categories를 허가하는 민감 동작 승인입니다. Approval은 제품 장단점, 설계 방향, 아키텍처 판단이나 기술 구조 판단, 해결되지 않은 security 또는 product-security 판단, 검증 위험, QA 면제 판단, 검증 면제 판단, 작업 수락, 잔여 위험 수용 같은 사용자 소유 판단을 해소하지 않습니다. Sensitive action이 사용자 소유의 제품 판단, 기술 구조 판단이나 아키텍처 판단, 또는 해결되지 않은 security/product-security 판단도 포함하면 Core는 `prepare_write`가 `allowed`를 반환하기 전에 별도의 compatible Decision Packet을 요구해야 합니다. Approval은 Write Authorization이 아닙니다. 실제 제품 쓰기에는 여전히 allowed `prepare_write` result와 반환된 Write Authorization을 compatible하게 consume하는 `harness.record_run`이 필요합니다. Approval prompt와 record는 broad agreement를 묻지 말고 민감 동작, scope, expiry, 아직 결정되지 않은 것을 보여줘야 합니다.

<a id="harnessrecord_run"></a>

### `harness.record_run`

Purpose: artifacts와 evidence updates를 포함해 shaping, implementation, direct-result, verification-input run data를 기록합니다. Implementation 및 direct product-write Run에서는 compatible Write Authorization을 consume하며, write authorization을 판단하지 않습니다.

Stage/profile: v0.1은 Write Authorization 하나를 consume하는 implementation/direct Run 하나와 artifact/evidence ref 하나를 등록하는 데 active입니다. v0.2는 같은 method를 user-readable evidence summary에 사용합니다. Evidence Manifest, feedback-loop, TDD, verification-input, 더 풍부한 artifact relation은 v0.3+ owner profile이 enabled될 때만 active입니다.

사용자에게 보이는 의미: 무슨 일이 일어났고 evidence, artifact, next action이 어떻게 바뀌었는지 말합니다. Core가 Run을 commit하기 전에 request를 거부했다면 Run이 존재한다고 주장하면 안 됩니다. 관찰된 product write 뒤에 Core가 violation 또는 audit Run을 기록했다면 감사/복구 맥락으로 표시하고, evidence, 분리 검증, QA, 작업 수락, close 준비 상태를 충족한 것처럼 보여주면 안 됩니다.

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

`payload` branch는 `kind`와 일치해야 하며, 다른 branch는 `null`이거나 absent여야 합니다. `ArtifactInput` 값은 같은 Core transaction에서 찾고, response field에는 committed `ArtifactRef` 값이 들어갑니다. Current reference API에서 Change Unit creation과 update는 `kind=shaping_update`와 `change_unit_updates`를 통해 이뤄집니다. `operation=create`는 `change_units` record를 만들고, `operation=select_active`는 Task의 `active_change_unit_id`를 update합니다. `allowed_paths`, `allowed_tools`, `allowed_commands`, `allowed_network_targets`, `secret_scope`, `sensitive_categories`는 scope field입니다. `autonomy_profile`, `agent_may_do`, `user_judgment_required`, `afk_stop_conditions`는 Autonomy Boundary judgment latitude만 설명합니다.

`secret_omitted` artifact를 연결하는 Evidence update는 남아 있는 보이는 nonsecret evidence로 증명되는 acceptance criteria 또는 completion condition만 지원할 수 있습니다. `blocked` artifact를 연결하는 Evidence update는 시도된 capture를 커밋된 metadata-only notice로 보존하지만, 금지된 원본 payload가 필요한 근거를 충족하지 않습니다. 관련 Evidence Manifest 또는 gate는 documented resolution이 유효한 path를 제공할 때까지 unsupported, partial, blocked, insufficient 중 적절한 상태로 남습니다.

Feedback Loop creation과 definition은 `ShapingUpdatePayload.feedback_loop_updates`를 통해 이뤄집니다. Execution evidence와 status update는 `EvidenceUpdates.feedback_loop_updates` 또는 수동 QA가 selected loop일 때 `harness.record_manual_qa`를 통해 이뤄집니다. `operation=create`는 기준 `feedback_loops` row를 만들고 `record_kind=feedback_loop`인 `StateRecordRef`를 반환합니다. 호출자는 일반적으로 Core 할당을 위해 `feedback_loop_id`를 null로 두며, executable fixture/import runner는 deterministic collision-free `FBL-*` ID를 supply할 수 있습니다. `operation=update`는 `feedback_loop_id`가 같은 Task와 compatible Change Unit에 속한 existing feedback-loop row를 가리켜야 합니다. Update에서는 null scalar field가 stored value를 unchanged로 두고, ref array와 artifact input은 additive입니다. TDD가 selected되면 TDD Trace를 `tdd_trace_refs`에 둘 수 있지만, 이는 execution evidence로 남으며 Feedback Loop row를 대체하지 않습니다. TDD waiver가 기록되면 `TddTraceUpdate.non_tdd_justification`은 reason을 기록하고, 관련 `FeedbackLoopUpdate.alternate_loop` 또는 selected-loop ref는 근거를 제공할 alternate feedback loop를 기록합니다.

`write_authorization_id`는 `harness.prepare_write`가 반환한 compatible Write Authorization을 reference합니다. `kind=implementation`과 `kind=direct`에서는 Run이 product write를 기록하지 않고 Core가 read-only 근거 또는 shaping으로 분류하는 경우를 제외하면 `write_authorization_id`가 required입니다. `kind=shaping_update`에서는 `write_authorization_id`가 `null`이어야 합니다. 이 reference API는 observed product write도 함께 기록하는 shaping update를 지원하지 않으므로, 그런 write는 compatible authorization과 함께 `kind=implementation` 또는 `kind=direct`로 record해야 합니다. `kind=verification_input`에서는 `write_authorization_id`를 `null`로 둡니다. Product write를 만드는 verification input은 이 reference API에서 보통 허용하지 않아야 합니다.

Core는 consumed authorization을 observed changed paths, created/deleted paths, artifact inputs와 resolved artifact refs, command results, run summary, baseline, active Change Unit, Approval refs, Decision Packet refs, sensitive categories, surface guarantee와 비교해 validate합니다. Run summary는 Run을 설명하는 데 도움을 주지만, compatible observed paths, artifacts, authorization basis 없이는 authorized changes의 proof로 받아들이면 안 됩니다.

`runs.write_authorization_id`는 Run이 compatible Write Authorization을 성공적으로 consume할 때만 채워집니다. Invalid, `stale`, missing, consumed, scope-exceeded authorization을 사용하려 한 violation 또는 audit Run은 `runs.write_authorization_id`를 consumed authorization으로 채우면 안 됩니다. Audit에 유용한 attempted authorization 참조는 validator finding, run violation payload, 또는 `task_events.payload_json`에 기록해야 합니다. Observed product write가 이미 발생했다면 audit 또는 recovery를 위해 이런 violation Run을 record할 수 있지만, evidence sufficiency, 분리 검증, QA, 작업 수락, close readiness를 충족하면 안 됩니다. Corresponding Write Authorization은 unconsumed로 남아야 하며 violation과 compatibility basis에 따라 `stale`, revoked, expired로 표시될 수 있습니다.

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

Violation 또는 audit Run은 Core가 그런 Run을 의도적으로 record할 때, 예를 들어 observed product write가 이미 발생한 뒤에만 non-null `run_id`를 가질 수 있습니다. Rejected pre-commit cases는 Run ID를 만들어내면 안 됩니다.

State transition summary: shaping updates는 `shaping`을 유지하거나 `ready` 또는 `waiting_user`로 이동할 수 있습니다. Implementation은 `verifying` 쪽으로 이동합니다. Direct는 close-eligible이 되거나 work로 escalate할 수 있습니다. Verification input은 분리 검증을 증명하지 않고 verification bundle material을 기록합니다.

반환될 수 있는 stable EventRef values: `run_recorded`, `write_authorization_consumed`, `write_authorization_violation_detected`, `write_authorization_staled`, `write_authorization_revoked`, `write_authorization_expired`, `scope_violation_detected`, `evidence_manifest_updated`.

implementation-local detail/audit를 위해 반환될 수 있는 non-stable EventRef values: `shaping_updated`, `implementation_recorded`, `direct_result_recorded`, `verification_input_recorded`, `artifact_registered`, `feedback_loop_updated`, `tdd_trace_updated`.

Violation 또는 audit Run은 audit 및 recovery를 위해 `write_authorization_violation_detected`, `write_authorization_staled`, `write_authorization_revoked`, `write_authorization_expired`, `scope_violation_detected`를 내보낼 수 있습니다. 그런 Run은 evidence sufficiency, 분리 검증, QA, 작업 수락, close readiness를 충족할 수 없습니다. Pre-commit rejection response는 `record_run`에서 stable EventRef value를 반환하지 않습니다.

Committed non-dry-run Run response에서 relevant projection support가 범위에 있을 때 대기열에 들어가는 projection job: 최소 task summary를 위한 `TASK`; direct-result display가 켜져 있고 `kind=direct`일 때 `DIRECT-RESULT`; active projection profile이 해당 future/diagnostic output을 포함할 때만 `RUN-SUMMARY`, `EVIDENCE-MANIFEST`, `TDD-TRACE`. Dry-run과 pre-commit rejection response는 projection job을 대기열에 넣지 않습니다.

ValidatorResults emitted: `decision_quality_check`, `autonomy_boundary_check`, `feedback_loop_check`, `tdd_trace_required`, `codebase_stewardship_check`, applicable design-quality validators, `surface_capability_check`.

Core checks/preconditions: `state_envelope`, `changed_paths`, `scope_coverage`, `approval_scope`, `baseline_freshness`, `artifact_integrity`, `evidence_sufficiency`. Run summary는 주변 changed-path, artifact, authorization compatibility check의 일부로 비교하며, 새 gate가 아닙니다.

Possible errors: `STATE_CONFLICT`, `NO_ACTIVE_TASK`, `NO_ACTIVE_CHANGE_UNIT`, `WRITE_AUTHORIZATION_REQUIRED`, `WRITE_AUTHORIZATION_INVALID`, `SCOPE_VIOLATION`, `APPROVAL_REQUIRED`, `APPROVAL_EXPIRED`, `ARTIFACT_MISSING`, `BASELINE_STALE`, `EVIDENCE_INSUFFICIENT`, `VALIDATOR_FAILED`, `CAPABILITY_INSUFFICIENT`, `MCP_UNAVAILABLE`.

Idempotency behavior: repeated request는 같은 run, artifact record, evidence update, event, projection job을 반환합니다. Artifact input과 resolved artifact ref는 original payload와 일치해야 합니다.

<a id="harnessrequest_user_judgment"></a>
<a id="harnessrequest_user_decision"></a>

### `harness.request_user_judgment`

Compatibility alias: `harness.request_user_decision`.

Purpose: progress, write, close, residual-risk acceptance, waiver, reconcile을 막는 user judgment를 위한 structured Decision Packet을 create합니다.

Stage/profile: v0.1에서는 active가 아닙니다. v0.2는 user-owned judgment 또는 work acceptance가 progress나 close를 막을 때 사용자 대상 Decision Packet prompt를 도입합니다. Approval hardening, QA/verification waiver, full residual-risk acceptance, reconcile은 owner profile이 active일 때의 later-profile extension입니다.

Allowed actor: `lead_agent`, `evaluator`, `operator`.

Request schema:

```yaml
RequestUserJudgmentRequest:
  envelope: ToolEnvelope
  task_id: string
  change_unit_id: string | null
  judgment_category: product_ux | technical_architecture | security_privacy | scope_autonomy | qa_verification | work_acceptance | residual_risk | mixed
  judgment_route: choose | defer | approve-sensitive-action | waive | accept-result | accept-risk | reconcile
  display_depth: simple | tradeoff | high-risk | close-affecting
  context:
    why_now: string
    source_refs: StateRecordRef[]
    evidence_refs: EvidenceRefs
  state_summary_at_request: StateSummary | null
  what_user_is_judging: string
  what_agent_may_decide_without_user: string[]
  affected_scope: DecisionPacketScope
  affected_gates: DecisionPacketGateRef[]
  affected_acceptance_criteria: DecisionPacketCriterionRef[]
  judgment_payload: JudgmentPayload
  expires_at: string | null
```

이 method/profile이 active이면 Core는 canonical `DecisionPacket`을 저장합니다. v0.1은 이 storage path를 요구하지 않습니다. Public request와 response schema는 optional interaction row가 아니라 Decision Packet을 중심으로 유지됩니다. 구현이 `decision_requests`를 만들면 그 row는 routing, interaction, replay, compatibility metadata일 뿐입니다. 어떤 gate도 canonical `decision_packet_id`로 다시 연결되기 전에는 그것을 고려하면 안 됩니다.

`state_summary_at_request=null`이면 Core가 같은 transaction 안에서 request-time summary를 파생합니다. `expires_at`은 값이 있을 때 canonical Decision Packet state에 복사됩니다. `judgment_payload`는 [Shared schemas](#shared-schemas)의 설명처럼 `judgment_route`와 일치해야 합니다. "go ahead", "proceed", "looks good", "좋아", "진행해" 같은 넓은 자연어 답변은 schema branch가 아닙니다. Request는 사용자가 무엇을 판단하는지, route, category, display depth, relevant refs, 답변이 해결하지 않는 것을 이름 붙여야 합니다.

`judgment_route=approve-sensitive-action`에서는 `judgment_payload.approval_scope`가 required입니다. Core는 linked pending Approval record를 만들 수 있습니다. 하지만 Approval은 `harness.record_user_judgment`가 packet을 해소하기 전에는 granted가 아닙니다. 이 route는 별도 표현이 없는 product, architecture, security/privacy, QA/verification, work-acceptance, residual-risk judgment를 해소하지 않습니다.

`judgment_route=accept-result` 또는 `accept-risk`에서는 사용자가 close-relevant risk를 acceptance 전에 볼 수 있을 만큼 acceptance visibility 또는 residual-risk refs가 있어야 합니다. Context가 부족하면 request를 block하거나 좁혀야 합니다.

Response schema:

```yaml
RequestUserJudgmentResponse:
  base: ToolResponseBase
  decision_packet_id: string
  decision_packet_ref: StateRecordRef
  decision_packet: DecisionPacket
  approval_id: string | null
  reconcile_item_id: string | null
  state: StateSummary
  user_visible_summary: string
```

`RequestUserDecisionRequest`와 `RequestUserDecisionResponse`는 compatibility alias입니다. Alias payload는 `judgment_domain`을 `judgment_category`로, `decision_kind`를 `judgment_route`로, `decision_profile`을 `display_depth`로 map합니다. 새 example과 future fixture는 judgment 이름을 써야 합니다.

Status와 next-action response가 반환하는 `pending_decisions`는 `record_kind=decision_packet`인 해소되지 않은 user-action `StateRecordRef` entry를 포함합니다. `active_decision_packet_refs` field는 pending, deferred, blocked, recently resolved packet을 포함해 current phase 또는 requested action과 relevant한 모든 Decision Packet을 포함합니다.

상태 전이 요약: pending Decision Packet을 record하고 보통 Task를 `waiting_user`로 옮깁니다. Choice judgment는 `decision_gate=pending`을 set할 수 있습니다. Approval judgment는 `approval_gate=pending`을 set할 수 있습니다. Scope/autonomy judgment는 `scope_gate` 또는 active Change Unit boundary에 영향을 줄 수 있습니다. Work-acceptance judgment는 `acceptance_gate=pending`을 set하거나 유지할 수 있습니다. Risk acceptance는 이름 붙은 visible risk가 accepted될 때까지 `residual_risk_acceptance` close blocker category를 사용합니다.

implementation-local detail/audit를 위해 반환될 수 있는 non-stable EventRef values: `decision_packet_created`, `user_judgment_requested`, `approval_requested`, `scope_judgment_requested`, `product_judgment_requested`, `technical_judgment_requested`, `autonomy_boundary_judgment_requested`, `verification_waiver_requested`, `qa_waiver_requested`, `work_acceptance_requested`, `residual_risk_acceptance_requested`, `reconcile_judgment_requested`.

Projection job 대기열 추가: source record를 변경하는 committed non-dry-run Decision Packet creation에서는 Task projection support가 범위에 있으면 `TASK`; standalone Decision Packet projection이 enabled될 때만 `DEC`; `judgment_route=approve-sensitive-action`에서 Core가 canonical approval-shaped Decision Packet과 linked pending Approval record를 만들고 APR projection support가 active일 때만 `APR`. Dry-run, pre-commit state conflict, rejected pre-commit request는 projection job을 대기열에 넣지 않습니다.

ValidatorResults emitted: `decision_quality_check`, active Change Unit boundary에 영향을 주는 packet에서는 `autonomy_boundary_check`, risk-acceptance judgment에서는 `residual_risk_visibility_check`.

Core checks/preconditions: `state_envelope`, `decision_packet_validity`, approval judgment의 `approval_scope`, reconcile judgment의 `reconcile_required`.

Possible errors: `STATE_CONFLICT`, `NO_ACTIVE_TASK`, `NO_ACTIVE_CHANGE_UNIT`, `SCOPE_REQUIRED`, `DECISION_REQUIRED`, `AUTONOMY_BOUNDARY_EXCEEDED`, `APPROVAL_REQUIRED`, `RECONCILE_REQUIRED`, `RESIDUAL_RISK_NOT_VISIBLE`, `PROJECTION_STALE`, `VALIDATOR_FAILED`, `MCP_UNAVAILABLE`.

Idempotency behavior: repeated request는 같은 Decision Packet, related records, events, projection jobs를 반환합니다. 같은 key에 다른 packet payload를 사용하면 `STATE_CONFLICT`입니다.

<a id="harnessrecord_user_judgment"></a>
<a id="harnessrecord_user_decision"></a>

### `harness.record_user_judgment`

Compatibility alias: `harness.record_user_decision`.

Purpose: pending Decision Packet에 대한 사용자 답을 record하고, 필요한 경우 accepted residual risk를 기록합니다.

Stage/profile: v0.1에서는 active가 아닙니다. v0.2는 user-owned judgments와 work acceptance가 MVP scope에 있을 때 이를 기록합니다. Approval grant/deny, waiver, residual-risk acceptance metadata, reconcile outcome은 later owner profile이 active일 때 필요합니다. 이 method는 existing canonical Decision Packet에 대한 답을 기록합니다. Broad approval이나 write authority를 만들지 않습니다.

Allowed actor: `user`, `operator`.

이 request는 existing canonical Decision Packet에 답합니다. Caller가 대체 judgment context를 넘겨 저장된 packet을 바꿀 수 없습니다. Core는 stored packet의 `judgment_route`, `judgment_category`, `display_depth`, status, refs, `judgment_payload`에 맞춰 answer를 validate합니다.

Request schema:

```yaml
RecordUserJudgmentRequest:
  envelope: ToolEnvelope
  decision_packet_id: string
  judgment_route: choose | defer | approve-sensitive-action | waive | accept-result | accept-risk | reconcile
  selected_option_id: string | null
  judgment: RecordUserJudgmentPayload
  note: string
  waiver_reason: string | null
  accepted_risks: AcceptedRiskInput[]

RecordUserJudgmentPayload:
  route: choose | defer | approve-sensitive-action | waive | accept-result | accept-risk | reconcile
  value: selected | rejected | deferred | granted | denied | expired | waived | accepted | merge | convert_to_note | create_decision
  value_note: string | null

AcceptedRiskInput:
  residual_risk_ref: StateRecordRef | null
  risk_summary: string
  accepted_scope: string[]
  acceptance_consequence: string
  follow_up_required: boolean
  follow_up: string | null
  evidence_refs: EvidenceRefs
```

`RecordUserJudgmentPayload.route`는 `RecordUserJudgmentRequest.judgment_route`와 stored Decision Packet route와 일치해야 합니다. Allowed `value` 의미는 route별로 다릅니다. `choose`는 `selected`, `rejected`, `deferred`를 씁니다. `defer`는 `deferred`를 씁니다. `approve-sensitive-action`은 `granted`, `denied`, `expired`를 씁니다. `waive`는 `waived` 또는 `rejected`를 씁니다. `accept-result`는 `accepted` 또는 `rejected`를 씁니다. `accept-risk`는 `accepted`, `rejected`, `deferred`를 씁니다. `reconcile`은 `merge`, `rejected`, `convert_to_note`, `create_decision`, `deferred`를 씁니다.

Core는 existing canonical Decision Packet에 대해 answer를 validate합니다. Request는 `judgment_category`, `display_depth`, `judgment_payload`, affected refs, stored route를 바꿀 수 없습니다. Free-form note text는 답변을 sensitive-action Approval, work acceptance, waiver, residual-risk acceptance, write authority로 넓힐 수 없습니다.

`accepted_risks`는 Decision Packet과 current Judgment Context가 close-relevant residual risk를 이미 사용자에게 보이게 만든 경우에만 allowed입니다. `AcceptedRiskInput.residual_risk_ref=null`은 current visible context가 같은 transition에서 Core가 Residual Risk record를 만들거나 연결하기에 충분할 때만 allowed입니다. Visibility 또는 context가 없으면 Core는 hidden risk를 조용히 create하고 accept하지 말고 reject 또는 block해야 합니다.

Response schema:

```yaml
RecordUserJudgmentResponse:
  base: ToolResponseBase
  decision_packet_id: string
  decision_packet_ref: StateRecordRef
  decision_packet: DecisionPacket
  state: StateSummary
  updated_records: StateRecordRef[]
  accepted_risk_refs: StateRecordRef[]
  next_action: string
```

`RecordUserDecisionRequest`와 `RecordUserDecisionResponse`는 compatibility alias입니다.

`RecordUserJudgmentResponse.accepted_risk_refs`는 `record_kind=residual_risk`인 `StateRecordRef` entries만 포함합니다. Standalone accepted-risk record kind는 없습니다.

상태 전이 요약: targeted Decision Packet은 resolved, deferred, rejected, blocked 상태로 처리됩니다. Affected gate 또는 reconcile item을 업데이트합니다. Approval grant/deny는 linked Approval record와 `approval_gate`를 업데이트하지만 Write Authorization을 만들지 않습니다. User-resolved product, scope, autonomy, technical judgment는 `decision_gate` 또는 관련 owner state를 업데이트합니다. Verification waiver는 `verification_gate=waived_by_user`를 set할 수 있습니다. QA waiver는 policy가 waiver path를 accept할 때 `qa_gate=waived`를 set할 수 있습니다. Work acceptance는 `acceptance_gate`를 업데이트합니다. Accepted residual risk는 assurance를 높이지 않고 Residual Risk records를 업데이트합니다.

implementation-local detail/audit를 위해 반환될 수 있는 non-stable EventRef values: `user_judgment_recorded`, `decision_packet_resolved`, `decision_packet_deferred`, `decision_packet_rejected`, `approval_granted`, `approval_denied`, `scope_confirmed`, `scope_rejected`, `product_judgment_recorded`, `technical_judgment_recorded`, `autonomy_boundary_judgment_recorded`, `verification_waiver_recorded`, `qa_waiver_recorded`, `work_acceptance_recorded`, `residual_risk_accepted`, `reconcile_resolved`.

Projection job 대기열 추가: source record를 변경하는 committed non-dry-run user-owned judgment record에서는 Task projection support가 범위에 있으면 `TASK`; standalone Decision Packet projection이 enabled될 때만 `DEC`; targeted packet이 approval-shaped이고 linked Approval record가 update되며 APR projection support가 active일 때 `APR`; QA waiver가 QA record로 represented되고 Manual QA projection support가 active일 때 `MANUAL-QA`. Dry-run, pre-commit state conflict, rejected pre-commit request는 projection job을 대기열에 넣지 않습니다.

ValidatorResults emitted: `decision_quality_check`, `autonomy_boundary_check`, `residual_risk_visibility_check`.

Core checks/preconditions: `state_envelope`, `pending_decision_packet_exists`, `approval_scope`, `qa_waiver_reason`, `reconcile_target_validity`.

Possible errors: `STATE_CONFLICT`, `NO_ACTIVE_TASK`, `DECISION_UNRESOLVED`, `AUTONOMY_BOUNDARY_EXCEEDED`, `APPROVAL_DENIED`, `APPROVAL_EXPIRED`, `SCOPE_VIOLATION`, `QA_REQUIRED`, `ACCEPTANCE_REQUIRED`, `RESIDUAL_RISK_NOT_VISIBLE`, `RECONCILE_REQUIRED`, `PROJECTION_STALE`, `VALIDATOR_FAILED`, `MCP_UNAVAILABLE`.

Idempotency behavior: repeated judgment는 같은 Decision Packet resolution, accepted-risk refs, updated records, events를 반환합니다. 같은 key로 이미 recorded judgment를 바꾸려 하면 `STATE_CONFLICT`를 반환합니다.

<a id="harnessclose_task"></a>

### `harness.close_task`

Purpose: Core가 모든 close-relevant gates를 check한 뒤 Task를 close, cancel, supersede합니다. 이는 public completion decision point입니다.

Stage/profile: v0.1은 implementation이 그 경로를 선택한 경우 narrow structured blocker/status smoke로만 사용할 수 있습니다. v0.2는 close state와 blocker display를 요구합니다. v0.3+는 profile이 active일 때 full assurance, QA, waiver, accepted-risk, projection/report freshness semantics를 추가합니다.

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

`CloseTaskRequest`는 accepted-risk refs를 전달하지 않습니다. `completed_with_risk_accepted`에서는 Core가 close-relevant Residual Risk records에 이미 기록된 accepted state를 읽습니다. Visible accepted residual-risk state가 없으면 block합니다. `verification_gate=waived_by_user`이면 completion은 `completed_with_risk_accepted`를 request해야 합니다. Waiver는 detached verification이 아니므로 `completed_verified`는 계속 blocked입니다.

Response schema:

```yaml
CloseState:
  value: open | blocked | closed | cancelled | superseded

CloseBlockerCategory:
  enum:
    - open_run
    - scope
    - user_judgment
    - sensitive_action_approval
    - design_policy
    - evidence
    - verification
    - manual_qa
    - residual_risk_visibility
    - residual_risk_acceptance
    - work_acceptance
    - projection_freshness
    - artifact_availability

CloseTaskResponse:
  base: ToolResponseBase
  close_state: CloseState
  closed: boolean
  close_reason: none | completed_verified | completed_self_checked | completed_with_risk_accepted | cancelled | superseded
  assurance_level: none | self_checked | detached_verified
  residual_risk_state: ResidualRiskSummary
  acceptance_state:
    status: not_required | required | pending | accepted | rejected
    accepted_by_ref: StateRecordRef | null
    required_before_close: boolean
  profile_required_verification:
    active: boolean
    status: not_required | required | pending | passed | failed | waived_by_user | blocked
    required_profile: string | null
    related_refs: StateRecordRef[]
  state: StateSummary
  blockers:
    - code: ErrorCode
      category: CloseBlockerCategory
      message: string
      required_next_action: string
      related_refs: StateRecordRef[]
  final_report_refs: StateRecordRef[]
  artifact_refs: ArtifactRef[]
```

`profile_required_verification.active=false`는 current stage/profile이 이 close path에 profile-specific verification을 요구하지 않는다는 뜻입니다. Verification passed를 뜻하지 않습니다. Active일 때 status와 refs는 required verification profile과 blocker를 설명합니다.

Close blockers에는 unresolved, missing, deferred-without-coverage, blocked, rejected, stale, incompatible Decision Packets, missing evidence, active Runs, active profile에서만 요구되는 verification, active profile에서만 요구되는 Manual QA, active일 때 missing work acceptance, visible하지 않은 known close-relevant residual risk, requested close reason 때문에 acceptance가 아직 필요한 visible close-relevant risk가 포함됩니다. Known close-relevant residual risk가 없으면 `ResidualRiskSummary.status=none`이 residual-risk visibility를 충족하며 close blocker가 아닙니다.

사용자에게 보이는 의미: Task를 지금 끝내거나 취소하거나 supersede할 수 있는지 답합니다. Close가 blocked이면 첫 번째 close blocker를 primary close blocker로 보여주고, 있으면 `required_next_action`을 가장 작은 해소 방법으로 사용합니다. 나머지 blockers는 refs와 함께 secondary close blockers로 보여줍니다.

상태 전이 요약: successful completion은 Task를 result, close state, close reason, assurance level, residual-risk state, acceptance state가 있는 `completed`로 옮깁니다. Cancellation 또는 supersession은 Task를 `cancelled`로 옮깁니다. Failed close는 Task를 non-terminal로 남기고 blockers를 보고합니다.

반환될 수 있는 stable EventRef values: `close_requested`, `task_closed`, `task_cancelled`, `task_superseded`, `risk_accepted_close_recorded`, `close_blocked`.

Projection job 대기열 추가: committed non-dry-run successful close 또는 close-blocker state change에서는 task-summary projection support가 범위에 있으면 `TASK`; active projection/report profile이 해당 reports를 포함할 때만 latest profile-required reports. Projection text는 Core close state 또는 structured close blockers를 대체하지 않습니다.

ValidatorResults emitted: `decision_gate_check`, `decision_quality_check`, `autonomy_boundary_check`, `feedback_loop_check`, `tdd_trace_required`, `codebase_stewardship_check`, `manual_qa_required`, `residual_risk_visibility_check`, active profile이 해당 validator result를 emit할 때만 `context_hygiene_check`.

Core checks/preconditions: `state_envelope`, `active_run_absent`, `active_change_unit_complete`, `scope_coverage`, `approval_scope`, `design_gate_close`, `evidence_sufficiency`, verification이 active일 때 `same_session_verify_guard`, QA가 active일 때 `qa_required`, `residual_risk_visibility`, `residual_risk_acceptance`, `acceptance_required`, projection/report profile이 active일 때 `projection_freshness`.

Possible errors: `STATE_CONFLICT`, `NO_ACTIVE_TASK`, `NO_ACTIVE_CHANGE_UNIT`, `SCOPE_REQUIRED`, `SCOPE_VIOLATION`, `DECISION_REQUIRED`, `DECISION_UNRESOLVED`, `AUTONOMY_BOUNDARY_EXCEEDED`, `APPROVAL_REQUIRED`, `APPROVAL_DENIED`, `APPROVAL_EXPIRED`, `EVIDENCE_INSUFFICIENT`, `VERIFY_NOT_DETACHED`, `QA_REQUIRED`, `ACCEPTANCE_REQUIRED`, `RESIDUAL_RISK_NOT_VISIBLE`, `PROJECTION_STALE`, `RECONCILE_REQUIRED`, `ARTIFACT_MISSING`, `BASELINE_STALE`, `VALIDATOR_FAILED`, `MCP_UNAVAILABLE`.

Idempotency behavior: repeated successful close는 같은 terminal state와 report refs를 반환합니다. 다른 intent 또는 close reason으로 두 번째 close를 시도하면 `STATE_CONFLICT`입니다.

## Later-Profile Public Tool Details

다음 method들은 future/later-profile public API details입니다. v0.1 requirement가 아니며, owner profile이 명시적으로 active하지 않는 한 minimum v0.2 user-value slice에도 포함되지 않습니다.

<a id="harnesslaunch_verify"></a>

### `harness.launch_verify`

Purpose: 분리 검증 run 또는 manual evaluator bundle을 create합니다.

Stage/profile: v0.3 Agency Assurance behavior입니다. v0.1에는 required가 아니며, 그 자체로 detached assurance를 만들지 않습니다. `harness.record_eval`이 나중에 qualify할 수 있는 candidate/bundle을 만듭니다.

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

`include_artifacts`는 bundle에 포함하거나 link할 already registered 근거를 reference합니다. `bundle_artifact_input`은 optional입니다. `null`이면 Core가 verification bundle을 assemble하고 등록합니다. 값이 있으면 Core가 supplied staged bundle을 검증하고 등록합니다. `secret_omitted` entry는 ref와 omission note 또는 handle로 포함하고, `blocked` entry는 unavailable-input notice로만 포함합니다. Verification path가 replacement, waiver, Decision Packet outcome, Residual Risk record의 accepted-risk 상태, 또는 다른 documented resolution을 기록하지 않는 한 이는 `EVIDENCE_INSUFFICIENT`로 이어질 수 있습니다.

`verification_mode`는 detached-candidate launch path만 나열합니다. Self-check와 same-session review는 `harness.launch_verify`가 아니라 Run/Eval context로 기록합니다. `subagent_context`도 기본적으로 detached가 아니므로 launch mode가 아닙니다. Subagent가 실제로 더 엄격한 boundary를 충족한다면 launch는 그 stricter mode를 기록하고 `harness.record_eval`이 final independence context를 기록합니다.

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

Launch는 detached candidate를 만들 뿐 detached assurance를 만들지 않습니다. Eval이 기록되기 전에 source state version, baseline ref, included artifacts, Evidence Manifest, approval/Decision Packet refs, close-relevant Residual Risk refs가 drift하면 bundle은 assurance 관점에서 stale입니다. Stale bundle은 artifact로 등록된 채 남을 수 있지만, `harness.record_eval`에서 replacement 또는 compatible re-verification 없이는 `assurance_level=detached_verified`를 뒷받침할 수 없습니다.

implementation-local detail/audit를 위해 반환될 수 있는 non-stable EventRef values: `verification_launched`, `verification_bundle_created`, `evaluator_run_created`.

Projection job 대기열 추가: source record를 변경하는 committed non-dry-run verification launch에서는 Task projection support가 범위에 있으면 `TASK`; evidence-manifest projection support가 범위에 있으면 optional `EVIDENCE-MANIFEST`. Dry-run, pre-commit state-conflict, rejected pre-commit request는 projection job을 대기열에 넣지 않습니다.

ValidatorResults emitted: `surface_capability_check`.

Core checks/preconditions: `state_envelope`, `evidence_sufficiency`, `baseline_freshness`, `artifact_integrity`, `same_session_verify_guard`.

Possible errors: `STATE_CONFLICT`, `NO_ACTIVE_TASK`, `EVIDENCE_INSUFFICIENT`, `BASELINE_STALE`, `ARTIFACT_MISSING`, `CAPABILITY_INSUFFICIENT`, `MCP_UNAVAILABLE`, `VALIDATOR_FAILED`.

Idempotency behavior: repeated request는 같은 evaluator run과 bundle ref를 반환합니다. Included artifact 참조와 bundle artifact input은 original payload와 일치해야 하며, 같은 key에서 staged bundle content는 byte-identical이어야 합니다.

<a id="harnessrecord_eval"></a>

### `harness.record_eval`

Purpose: verification result를 record하고 independence가 valid할 때 verification gate/assurance를 update합니다.

Stage/profile: v0.3 Agency Assurance behavior입니다. Eval recording이 enabled될 때 required field가 적용됩니다. Same-session check, self-check summary, passed command는 이 method가 qualifying Eval을 기록하고 Core가 gate/assurance state를 update하기 전까지 detached verification이 아닙니다.

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

Eval evidence review는 artifact redaction semantics를 보존해야 합니다. `secret_omitted` artifact는 보이는 nonsecret fact에 대해서만 Eval finding을 뒷받침할 수 있습니다. `blocked` artifact는 원본 근거가 아니라 사용할 수 없는 입력 notice로 검토됩니다. Blocked payload에 의존하는 Eval은 valid replacement 또는 documented resolution이 생길 때까지 `blocked` 또는 `inconclusive`여야 하거나 `EVIDENCE_INSUFFICIENT`를 반환해야 합니다.

`independence.context` 값은 assurance와 별개입니다.

| Context | Assurance effect |
|---|---|
| `same_session` | Self-check 또는 same-session review입니다. Useful finding은 기록할 수 있지만 `verdict=passed`가 `detached_verified`를 set하면 안 됩니다. |
| `subagent_context` | 기본적으로 detached가 아닙니다. Recorded facts가 subagent가 더 엄격한 `fresh_session`, `fresh_worktree`, `sandbox`, `manual_bundle` boundary를 충족했음을 보여 줄 때만 detached assurance를 뒷받침할 수 있습니다. |
| `fresh_session` | Evaluator가 lead chat context를 이어받지 않고 task/evidence bundle에서 시작할 때 detached candidate입니다. |
| `fresh_worktree` | Evaluator가 separate worktree 또는 equivalent isolated repository state에서 work를 확인할 때 detached candidate입니다. |
| `sandbox` | Verification boundary와 artifact capture가 meaningful하고 write capability가 disclosed되면 detached 또는 isolated candidate입니다. |
| `manual_bundle` | Evaluator가 unreviewed lead-session context에 의존하지 않고 complete bundle을 검토할 때 detached candidate입니다. |

`verdict=passed`는 assurance upgrade에 필요하지만 충분하지 않습니다. Core는 Eval이 passed이고, independence가 valid이고, same-session guard가 통과하고, artifacts가 available and intact이고, baseline/bundle inputs가 current이거나 명시적으로 reverified된 경우에만 `assurance_updated=true`를 set할 수 있습니다. `baseline_reverified=true`는 evaluator의 baseline check를 기록하지만 known drift, stale bundle inputs, missing artifacts, invalid independence를 override하지 않습니다.

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

State transition summary: Eval을 record합니다. Valid independence와 current inputs가 있는 passed 분리 검증은 `verification_gate=passed`와 `assurance_level=detached_verified`를 set할 수 있습니다. Failed 또는 blocked Eval은 gate를 failed/blocked로 옮깁니다. Same-session, invalid independence, stale evaluator bundle, baseline drift, missing/failed-integrity artifacts는 assurance를 높일 수 없습니다.

반환될 수 있는 stable EventRef values: `eval_recorded`, `verification_passed`, `verify_not_detached_detected`.

implementation-local detail/audit를 위해 반환될 수 있는 non-stable EventRef values: `verification_failed`, `verification_blocked`, `assurance_updated`.

Projection job 대기열 추가: source record를 변경하는 committed non-dry-run Eval record에서는 task-summary projection support가 범위에 있으면 `TASK`; detailed Evaluation projection support가 active일 때만 `EVAL`; active profile이 해당 diagnostic projection을 포함할 때 optional `EVIDENCE-MANIFEST`. Dry-run, pre-commit state-conflict, rejected pre-commit request는 projection job을 대기열에 넣지 않습니다.

ValidatorResults emitted: `surface_capability_check`.

Core checks/preconditions: `state_envelope`, `same_session_verify_guard`, `baseline_freshness`, `artifact_integrity`, `evidence_sufficiency`, `approval_scope`.

Possible errors: `STATE_CONFLICT`, `NO_ACTIVE_TASK`, `VERIFY_NOT_DETACHED`, `EVIDENCE_INSUFFICIENT`, `BASELINE_STALE`, `ARTIFACT_MISSING`, `VALIDATOR_FAILED`, `CAPABILITY_INSUFFICIENT`, `MCP_UNAVAILABLE`.

Idempotency behavior: repeated request는 같은 Eval과 assurance decision을 반환합니다. 같은 key에서 changed verdict, independence payload, artifact input이 들어오면 `STATE_CONFLICT`입니다.

<a id="harnessrecord_manual_qa"></a>

### `harness.record_manual_qa`

Purpose: individual human QA outcome을 record하고 required QA가 satisfied, failed, waived될 때 `qa_gate`를 update합니다.

Stage/profile: Manual QA policy/profile이 enabled된 v0.3 Agency Assurance behavior입니다. v0.1에서는 active가 아니며, browser capture나 note는 human QA record 또는 valid waiver path를 대체하지 않습니다.

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

수동 QA가 Change Unit에 적용되는 경우 `change_unit_id`를 제공해야 합니다. 단일 Change Unit에 scoped되지 않는 Task-level QA에서는 `null`일 수 있습니다.

`RecordManualQaRequest.result`는 실제 수동 QA record의 record-level result이며 `passed`, `failed`, `waived`로 제한됩니다. Pending required QA는 `RecordManualQaRequest.result=pending`이 아니라 aggregate `qa_gate=pending`으로 표현합니다.

`result=waived`에서 product/user risk 또는 policy-required judgment가 있으면 `waiver_decision_packet_ref`가 reference하는 `qa_waiver` Decision Packet이 필요합니다. `waiver_reason`만으로 가능한 경우는 policy가 허용한 low-risk waiver에 한정됩니다.

수동 QA가 selected Feedback Loop인 경우 `feedback_loop_ref`는 `record_kind=feedback_loop`인 기준 `feedback_loops` row를 reference해야 합니다. Core는 수동 QA row를 record하고, resulting 수동 QA 참조와 registered artifact를 그 Feedback Loop에 추가하며, QA result에 따라 status를 `executed`, `blocked`, 또는 `waived`로 업데이트합니다. 이 link는 execution evidence만 업데이트하며 selected-loop definition을 생성하지 않습니다.

수동 QA artifact ref도 다른 evidence와 같은 이후 규칙을 따릅니다. `secret_omitted` QA artifact는 생략된 value를 증명하지 않고 보이는 workflow 또는 UI finding을 뒷받침할 수 있습니다. `blocked` QA capture artifact는 screenshot, log, trace, recording input을 사용할 수 없다는 표시입니다. Replacement capture, waiver, Decision Packet outcome, Residual Risk record의 accepted-risk 상태, 또는 documented fallback이 QA path를 해소하기 전까지 수동 QA record 또는 projection은 사용할 수 없는 입력을 보여야 하며, aggregate `qa_gate`는 valid waiver가 `qa_gate=waived`를 set하지 않는 한 pending 또는 failed로 남아야 합니다.

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

State transition summary: 수동 QA를 record합니다. `passed`는 `qa_gate=passed`를 set할 수 있습니다. `failed`는 `qa_gate=failed`를 set하고 rework/blocked로 route합니다. `waived`는 compatible `qa_waiver` Decision Packet 또는 policy-permitted low-risk waiver reason을 요구하고 `qa_gate=waived`를 set합니다. Required QA가 충족 기록을 아직 만들지 못했거나 latest relevant 기록이 policy를 충족하지 못하면 aggregate gate는 `qa_gate=pending`으로 남습니다.

implementation-local detail/audit를 위해 반환될 수 있는 non-stable EventRef values: `manual_qa_recorded`, `qa_passed`, `qa_failed`, `qa_waived`, `artifact_registered`, `feedback_loop_updated`.

Projection job 대기열 추가: source record를 변경하는 committed non-dry-run 수동 QA record에서는 task-summary projection support가 범위에 있으면 `TASK`; Manual QA profile이 active일 때 `MANUAL-QA`; active profile이 해당 diagnostic projection을 포함할 때 optional `EVIDENCE-MANIFEST`. Dry-run, pre-commit state-conflict, rejected pre-commit request는 projection job을 대기열에 넣지 않습니다. Waiver Decision Packet visibility는 여전히 status/next response, judgment-context resource, decision-packet resource, 최소 `TASK` 또는 card display를 통해 나타납니다.

위 committed non-dry-run path에서 standalone Decision Packet projection이 켜져 있고 waiver Decision Packet이 visibility에 영향을 줄 때만 optional `DEC` job을 대기열에 넣습니다.

ValidatorResults emitted: `manual_qa_required`, `decision_quality_check`, `residual_risk_visibility_check`.

Core checks/preconditions: `state_envelope`, `qa_waiver_reason`, `artifact_integrity`, `evidence_sufficiency`.

Possible errors: `STATE_CONFLICT`, `DECISION_REQUIRED`, `DECISION_UNRESOLVED`, `NO_ACTIVE_TASK`, `QA_REQUIRED`, `RESIDUAL_RISK_NOT_VISIBLE`, `ARTIFACT_MISSING`, `EVIDENCE_INSUFFICIENT`, `VALIDATOR_FAILED`, `MCP_UNAVAILABLE`.

Idempotency behavior: repeated request는 같은 수동 QA 기록과 gate update를 반환합니다. Waiver reason과 artifact input은 일치해야 합니다.
