# 문서 Projection 참조

## 권한 규칙

- Projection은 Core가 소유한 상태 기록과 아티팩트 참조에서 파생됩니다.
- Projection은 Core 상태가 아닙니다.
- 사용자가 Projection을 편집해도 그 내용이 자동으로 받아들여진 상태가 되지는 않습니다.
- Chat과 Markdown은 Core 상태를 덮어쓸 수 없습니다.

## 이 문서로 할 수 있는 일

이 참조 문서는 하네스가 Core 소유 상태 기록과 아티팩트 참조를 바탕으로 읽기용 요약을 어떻게 생성하는지 확인할 때 사용합니다.

Projection의 권한 경계, managed block 동작, 사람이 편집할 수 있는 영역, 아티팩트 참조 표시 방식, 산출물 계층, 템플릿 구현 계층, projection 최신성 규칙을 정의합니다. 기준 kernel state, MCP request/response schema, SQLite DDL, 설계 품질 정책 요구사항, 전체 template 본문은 이 문서가 정의하지 않습니다. 전체 template 본문과 표시 카드 형태는 [Template 참조](templates/README.md)에 있으며, 현재 projection 또는 display repair에 특정 template이 필요할 때만 불러옵니다.

이 문서는 향후 Harness 동작을 위한 참조 문서입니다. 현재 저장소 단계와 구현 인계 상태는 [구현 개요](../build/implementation-overview.md#문서-수락-상태)에 있습니다.

## 이런 때 읽기

- 읽기용 Markdown(Projection) 동작을 구현하거나 리뷰할 때
- 보고서, status card, Journey Card가 기준 상태가 아님을 확인할 때
- projected Markdown에 남긴 사람이 쓴 내용이 상태로 반영되는 경로를 판단할 때
- 사용자 읽기용 요약, 에이전트 compact context, reference 또는 diagnostic projection을 구분해야 할 때
- 최신이 아니거나, failed이거나, drifted된 projection을 진단할 때

## 읽기 전에

기준 상태와 gate authority는 [커널 참조](kernel.md)를 사용합니다. `ProjectionKind`와 projection ref는 [API Schema Core](api/schema-core.md#projectionkind-support)를 사용하고, projection job storage는 [Storage와 DDL](storage-and-ddl.md)을 사용하며, 전체 rendered body와 display card는 그 owner section이 필요할 때만 [Template 참조](templates/README.md)를 사용합니다. 이 읽기 목록은 기본 context bundle이 아닙니다. Agent는 [Agent 통합: Context Push/Pull Principles](agent-integration.md#context-pushpull-principles)의 단계별 맥락 지도를 따라야 합니다.

## 핵심 생각

Projection은 읽기용 요약입니다. Core 소유 상태와 아티팩트 참조에서 생성되며 현재 상태, 참조, 최신성, 제안된 편집을 표시할 수 있습니다. 하지만 Core 소유 상태를 대체하지 않습니다. 사람이 projection을 편집해도 future Core/reconcile path가 state-changing action으로 받아들이기 전까지는 상태 변경이 아닙니다.

## Projection을 쉽게 말하면

하네스 projection은 이미 기준 상태나 artifact storage에 기록된 작업을 사람이 읽기 쉽게 보여주는 읽기용 요약입니다. 향후 Projector는 `state.sqlite` record, `state.sqlite.task_events`, 등록된 아티팩트 참조를 읽어 compact status card를 렌더링하고, 나중에 profile이 켜지면 `TASK`, `APR`, `RUN-SUMMARY`, `EVIDENCE-MANIFEST`, `EVAL`, `DIRECT-RESULT`, 그 밖의 report projection 같은 Markdown을 생성합니다.

Markdown은 사람이 작업을 이해하고, 맥락을 다시 잡고, 근거를 검토하거나 수정 제안을 할 수 있게 돕습니다. 하지만 Markdown이 작업을 소유하지는 않습니다. 보고서는 gate를 요약하거나, evidence link를 제공하거나, Write Authorization 참조를 표시하거나, user judgment 또는 선택적 full-format Decision Packet presentation을 보여줄 수 있지만, 보고서 문장 자체가 gate, evidence, authorization, judgment가 되지는 않습니다.

Projection은 secret/PII를 보호하는 표시 경계이기도 합니다. Projector는 아티팩트 참조, integrity metadata, redaction state, redaction/omission/blocking note를 렌더링합니다. `secret_omitted` 또는 `blocked` artifact를 Markdown 본문으로 펼치면 안 됩니다.

## 산출물 계층

하네스는 읽기용 파생 산출물을 단계별 전달 계획에 맞춰 나눕니다.

| 계층 | 단계 경계 | 목적과 규칙 |
|---|---|---|
| Core status output | 내부 엔지니어링 점검 | 현재 Core 상태에서 나온 최소 구조화 상태/막힘 출력입니다. Plain API text나 compact card일 수 있으며, persisted Markdown, projection job, 여러 `ProjectionKind` 값, full renderer를 요구하지 않습니다. |
| MVP-1 사용자 작업 루프 projection | MVP-1 사용자 작업 루프 | Core 상태와 ref에서 파생한 하나의 compact status card입니다. 현재 Task 요약, 작업 모양, 범위/하지 않을 일, 대기 중인 사용자 판단, 활성 blocker, 다음 안전한 행동, 근거 gap, close blocker, 보이는 잔여 위험, guarantee level, source/freshness ref를 보여줍니다. `TASK`, Journey, Run Summary, Evidence Manifest, Eval, 수동 QA, Export, polished report를 요구하지 않습니다. |
| Agent compact context/reference payload | MVP-1 지원 접점과 이후 단계 | 같은 Core 상태와 ref에서 파생한 에이전트용 간결한 payload입니다. 다음 행동에 필요한 id, ref, blocker label, freshness를 담을 수 있지만 사용자 대상 권한이 아니며 기본으로 full projection body를 넣지 않습니다. |
| Agency assurance reports | 보증 프로필 profile | 해당 profile이 켜졌을 때의 compact approval, 수동 QA, verification, waiver, assurance display입니다. Owner record와 ref에서 나온 report/card view이며 첫 조각이나 최소 MVP 요구사항이 아닙니다. |
| Operations/export reports | 운영 프로필 profile | Operations support가 켜졌을 때의 projection freshness, reconcile/readiness, export, release-handoff, artifact-integrity, operator report view입니다. Core 상태나 artifact 권한을 대체하지 않습니다. |
| Future/diagnostic projections | Owner가 승격한 later profile 또는 diagnostic | Detailed Journey Card 또는 Journey Spine view, Run Summary, TDD Trace, Module Map, Interface Contract, standalone full-format Decision Packet Markdown, detailed Evidence Manifest, detailed Eval, design/domain-language map, 기타 diagnostic view입니다. 필요할 때만 가져오거나 승격된 profile을 통해서만 켭니다. |

MCP read-only resource staging도 같은 계층을 따릅니다. 내부 엔지니어링 점검 resource는 첫 authority loop를 위한 current project/current task/status output을 노출합니다. MVP-1 resource는 compact status card, 사용자 판단 prompt context, agent compact context/reference payload를 노출할 수 있습니다. Evidence Manifest, reports, bundles, Journey/Spine, design/domain resource는 owner profile이 명시적으로 승격하기 전까지 later-profile 또는 diagnostic read로 남습니다. Projection을 읽는 resource도 여전히 read입니다. Projection job을 만들거나 projection을 authority로 만들면 안 됩니다.

에이전트용 간결한 현재 맥락은 별도 권한 계층이 아니며 context API가 구현되어 있음을 증명하지도 않습니다. Current Core status output 또는 MVP-1 compact card를 소비하는 쪽입니다. `source_state_version`과 최신성이 다음 행동에 맞을 때만 projection을 읽기용 요약으로 사용할 수 있습니다. 상태가 중요하고 projection이 stale, failed, unknown이거나 너무 넓다면 current Core state 또는 state-derived compact context를 가져와야 합니다. Markdown projection, Journey Card, status card, old report, generated summary, 읽기용 요약 전체 본문, full artifact contents, 관련 없는 template, future catalog material을 항상 주입되는 prompt payload나 authority로 만들면 안 됩니다. 읽기용 요약 전체 본문은 특정 단계가 그 내용을 요구할 때만 pull-on-demand로 읽고, 기본으로는 ref, 한 줄 summary, freshness만 push합니다. 이들은 살펴볼 current ref를 가리킬 수는 있지만 write를 허가하거나, gate를 충족하거나, 근거를 만들거나, 검증을 수행하거나, 수동 QA를 기록하거나, 결과를 수락하거나, 잔여 위험을 받아들이거나, Task를 close할 수 없습니다.

### MVP-1 compact status card

MVP-1 사용자 작업 루프 projection은 하나의 compact status card입니다. 이 card 자체가 제품 가치는 아니지만, Core 권한을 사용자가 읽을 수 있게 만드는 작은 표시 접점입니다. 반드시 다음을 보여줘야 합니다.

- 현재 Task 요약
- 작업 모양
- 현재 범위와 하지 않을 일
- 대기 중인 사용자 판단
- 활성 blocker
- 다음 안전한 행동
- 알려진 근거와 근거 gap
- close blocker
- 보이는 잔여 위험, 또는 명시적인 `none`/아직 보이지 않음 상태
- guarantee level
- source/freshness ref. 여기에는 source state version, 관련 owner ref, 필요할 때 artifact ref, 렌더링 시각, freshness state가 포함됩니다.

Card는 사용자가 읽기 쉬우면서도 에이전트가 부담 없이 다룰 만큼 작아야 합니다. Schema field, DDL, event log, full artifact, full artifact contents, full reference doc, full Evidence Manifest, full report body, 관련 없는 template, future catalog material을 쏟아내면 안 됩니다. 작업 수락 필요 여부/상태와 잔여 위험 표시는 관련 있을 때 distinct Core meaning으로 남지만, 별도 필수 projection kind가 아니라 compact status card 또는 별도 사용자 판단 prompt 안에 나타납니다.

Projection audience는 분리합니다.

| Audience | Shape | Rule |
|---|---|---|
| User-facing compact card | 평범한 말과 ref를 담은 짧은 status card | MVP-1 사용자 작업 루프 projection입니다. 표시 전용이며 Core 상태와 ref에서 파생됩니다. |
| Agent compact context/reference payload | 다음 행동에 필요한 ref, blocker, source clock, next-action hint를 담은 prompt 크기 또는 structured payload | 파생된 지원 payload입니다. 다음 에이전트 행동을 도울 수 있지만 authority가 아니며 기본으로 full Markdown body를 담지 않습니다. |
| Future/diagnostic reports | `TASK`, Journey Card/Spine, Run Summary, detailed Evidence Manifest, detailed Eval, full 수동 QA, TDD Trace, Domain Language, Module Map, Interface Contract, Design, Export, full Approval Card, 그 밖의 polished report | Later/profile output 또는 diagnostic입니다. Repository에 template이 있다는 사실은 현재 단계에서 필수라는 뜻이 아닙니다. |

엄격한 경계는 다음과 같습니다.

| Item | What it is | Authority |
|---|---|---|
| Raw artifact | diff, log, screenshot, checkpoint, bundle, manifest file 같은 durable evidence file | artifact store |
| 상태 기록 | Task, Change Unit, User Judgment, Journey Spine Entry, Residual Risk, Run, Approval, Write Authorization, Eval, 수동 QA record, Evidence Manifest, Artifact record, Reconcile Item 같은 기준 structured record | `state.sqlite` |
| Markdown 보고서 | record 및 아티팩트 참조에서 만든 사람이 읽을 수 있는 projection | projector output |

Markdown 보고서는 evidence link를 제공하고 상태를 요약할 수 있지만 raw artifact나 상태 기록은 아닙니다.

### Projection, 상태, artifact 권한 지도

이 source-of-truth 도식은 향후 projection 동작을 위한 설계 계약입니다. Core current record, event history, 등록된 artifact ref가 authority입니다. Projection, chat, connector output, tool output은 Core 경로가 state-changing action을 기록하기 전까지 읽기용 표면 또는 입력 표면입니다.

```mermaid
flowchart LR
  Chat["chat, connector, tool output"]
  Projection["Markdown projection"]
  Edit["사람 edit 또는 proposal"]
  Reconcile["reconcile candidate"]
  CorePath["Core state-changing path"]
  Records["Core current records"]
  Events["state.sqlite.task_events"]
  Artifacts["artifact store와 ArtifactRefs"]
  Projector["Projector"]

  Chat -. 입력일 뿐 .-> CorePath
  Projection -. 읽기용 보기 .-> Chat
  Projection -. edit input .-> Edit
  Edit -. candidate .-> Reconcile
  Reconcile --> CorePath
  CorePath --> Records
  CorePath --> Events
  CorePath --> Artifacts
  Records --> Projector
  Events --> Projector
  Artifacts --> Projector
  Projector --> Projection
```

엄격한 projection behavior는 이 reference가 담당하며, 특히 [Document authority matrix](#document-authority-matrix), [Managed block rules](#managed-block-rules), [Freshness and failure rules](#freshness-and-failure-rules)를 봅니다. Canonical state와 gates는 [커널 참조](kernel.md)가, artifact relation storage는 [Storage와 DDL](storage-and-ddl.md)이, public projection refs는 [API Schema Core](api/schema-core.md#projectionkind-support)가 담당합니다. 이 도식은 authority direction만 요약하며, 이 저장소에 projection system이 구현되어 있다는 뜻이 아닙니다.

생성된 보고서는 독자가 이 참조 문서를 몰라도 그 경계를 볼 수 있어야 합니다. 예시와 template에서 `source_state_version`은 렌더링에 사용한 state clock을 가리키고, `projection_version` 또는 projection status는 렌더링된 view를 가리키며, `updated_at`은 그 view가 만들어진 시각을 가리킵니다. Freshness line은 이 view가 source record와 아직 맞는지 표시할 뿐입니다. 이 field들이 Markdown을 Task state, gate, approval, 근거, 검증, 수동 QA, user judgment, 작업 수락, 잔여 위험 표시, 잔여 위험 수용의 owner로 만들지는 않습니다.

최신성 표시는 진단 정보이며 운영상 중요할 수 있지만 여전히 표시입니다. 오래되었거나 failed인 projection은 current readable context가 필요한 close/readiness view를 막거나, 담당 API path를 통해 `PROJECTION_STALE`을 보고하게 만들 수 있습니다. 하지만 committed Core 상태를 롤백하거나, gate value를 바꾸거나, Task를 failed로 표시하거나, 오래된 report를 authoritative하게 만들면 안 됩니다.

## 담당하는 참조 범위

이 문서는 다음을 담당합니다.

- projection principles
- document authority matrix
- managed block rules
- 사람이 편집할 수 있는 영역의 rules
- 아티팩트 참조 표시 rules
- 산출물 계층과 템플릿 구현 계층
- projection 기준 기록 rules
- projection 최신성 and failure rules
- projection rule 수준의 `source_state_version`과 `managed_hash` 해석

## 여기서 다루지 않는 것

이 문서는 다음을 담당하지 않습니다.

- 기준 kernel state와 transition rules. [Kernel Reference](kernel.md)를 봅니다.
- public MCP request/response schemas. [MVP API](api/mvp-api.md)와 [API Schema Core](api/schema-core.md)를 봅니다.
- SQLite DDL과 storage layout. [Storage And DDL](storage-and-ddl.md)를 봅니다.
- 설계 품질 정책 계약. [설계 품질 정책](design-quality-policies.md)을 봅니다.
- operator command 의미. [Operations And Conformance](operations-and-conformance.md)를 봅니다.
- conformance fixture assertion 의미. [Conformance Fixtures 참조](conformance-fixtures.md#fixture-assertion-semantics)를 봅니다.
- connector capability profile. [Agent 통합 참조](agent-integration.md)를 봅니다.
- surface recipe. [Surface Cookbook](surface-cookbook.md)을 봅니다.
- 전체 template 본문과 표시 카드 형태. [Template 참조](templates/README.md)를 봅니다.

## 작은 compact status card 예시

일부러 작게 보여주는 예시입니다. Full template renderer, generated report, `TASK` Markdown이 이미 있다는 뜻이 아닙니다.

```text
TASK-0001 Add Import Preview
표시 전용: Core 상태와 ref에서 파생된 보기이며 Core 상태나 쓰기 권한이 아닙니다.
하는 일: import preview slice의 근거 검토를 준비합니다.
현재 범위: import 전에 CSV row를 미리 보여줍니다. 데이터 write는 하지 않습니다.
하지 않을 일: bulk import 실행, account migration, production deploy.
대기 중인 사용자 판단: 없음.
근거: diff ref DIFF-0001. Smoke check는 아직 기록되지 않았습니다.
근거 gap: preview path에 대한 run/evidence record가 아직 없습니다.
닫기 막힘: 근거 공백. 작업 수락은 아직 요청하지 않았습니다.
보이는 잔여 위험: 이 slice에 기록된 잔여 위험 없음.
다음 안전한 행동: preview check를 실행하고 근거를 기록합니다.
출처/최신성: state=42; refs=TASK-0001, CU-01, DIFF-0001; rendered=2026-05-06T09:30:15+09:00; freshness=current.
```

## 사람이 편집할 수 있는 것

사람은 다음과 같이 명시적으로 편집 가능하다고 표시된 영역을 편집할 수 있습니다.

```md
## User Notes and Proposals
-
```

사람이 편집할 수 있는 text는 입력입니다. Note, question, correction, proposal을 담을 수 있습니다. 상태 변경 경로는 명시적입니다. proposal -> `reconcile_items` candidate -> explicit reconcile outcome -> 추가된 `state.sqlite.task_events` row가 있는 accepted Core state-changing action, 또는 reject, defer, note 전환입니다. 이 경로가 accepted Core outcome을 기록하기 전까지 proposal은 Task state, Domain Language, Module Map, Interface Contract, 수동 QA state, 작업 수락, 근거가 아닙니다.

사람이 편집한 proposal은 Task summary, acceptance criteria, Domain Language, Module Map, Interface Contract, 수동 QA note, 기타 상태 기반 기록을 대상으로 삼을 수 있지만 proposal 자체가 target 기록은 아닙니다.

## 사람이 상태에 직접 반영할 수 없는 것

사람은 다음 projection text를 기준 상태에 직접 반영할 수 없습니다.

- managed block content
- `source_state_version` 같은 front matter field
- current gate value, lifecycle phase, result, close reason, assurance level
- approval, 검증, 수동 QA, 작업 수락, 잔여 위험 status
- User Judgment, 선택적 full-format Decision Packet, Journey Card, Journey Spine, Autonomy Boundary, Write Authority Summary, Implementation Micro-Plan, Change Unit DAG, Residual Risk, Stewardship Impact, Review Stage, Write Authorization 표시 문구
- artifact 참조 identity, hash, redaction state, artifact availability
- status card, Journey Card, 기타 generated display 접점
- template body

Managed block을 직접 편집한 내용은 수용된 상태가 아니라 drift입니다. 권한처럼 보이는 문구를 직접 편집해도 write를 허가하거나, decision을 해결하거나, 필수 근거 조건을 충족하지 않습니다. 또한 verification 또는 수동 QA를 대체하거나, 잔여 위험을 받아들이거나, assurance를 높이거나, 작업을 닫거나, owner 기록을 변경하지 않습니다.

## Projection principles

1. Projection은 읽기용 파생 view이며 기준 기록이 아닙니다.
2. 운영 상태의 기준 기록은 `state.sqlite` current record 및 `state.sqlite.task_events`입니다.
3. Raw evidence의 기준 위치는 artifact store입니다.
4. Markdown 보고서는 Core 상태 기록 및 artifact 참조를 바탕으로 생성됩니다.
5. Markdown 보고서는 기본적으로 raw artifact가 아닙니다.
6. Front matter는 identity, projection version 또는 status, `source_state_version`, timestamp/freshness metadata만 가집니다.
7. Managed block은 projector가 생성하며 필요하면 다시 생성될 수 있습니다.
8. 사람이 편집할 수 있는 영역은 note와 proposal을 위한 입력 영역입니다.
9. 수용된 human edit만 reconcile과 `state.sqlite.task_events`를 추가하는 Core state-changing action을 통해 상태가 됩니다. Rejected, deferred, note outcome은 owner record를 변경하지 않습니다.
10. Large log, diff, trace, screenshot, bundle, checkpoint와 민감한 artifact는 embed하지 않고 artifact ref로 연결합니다.
11. Projection failure 또는 최신이 아님은 committed Core 상태를 롤백하거나 `state.sqlite.task_events`를 rewrite하거나 underlying task result를 절대 바꾸지 않습니다.
12. User-facing card는 friendly label을 사용할 수 있지만 기준 gate name은 kernel field로 남습니다.
13. User Judgment, 선택적 full-format Decision Packet, Journey Card, Journey Spine, Autonomy Boundary, Write Authority Summary, Implementation Micro-Plan, Change Unit DAG, Residual Risk, Stewardship Impact, Review Stage 표시는 owner 기록 및 artifact ref에서 만든 기준 기록이 아닌 projection입니다.

Projection과 report surface는 current record, ref, advisory next action을 표시할 수 있습니다. Write를 허가하거나, Write Authorization을 만들거나, gate를 충족하거나, 근거를 만들거나, 검증을 수행 또는 기록하거나, 수동 QA를 기록하거나, Approval을 부여하거나, QA 또는 검증을 면제하거나, 작업 수락을 기록하거나, 잔여 위험을 받아들이는 판단을 기록하거나, projection을 말만으로 refresh하거나, 구현 준비 상태를 선언하거나, Task를 닫거나, owner record를 변경하면 안 됩니다. 그런 효과는 아래 matrix에 이름 붙은 owner Core/MCP path에서 와야 합니다.

사용자 판단 display는 모든 항목을 "Judgment" 또는 "Approval"로 뭉치지 말고 판단 유형을 이름 붙여야 합니다. 판단이 pending이면 제품/UX 판단, 기술 판단, 민감 동작 승인, 작업 수락, 잔여 위험 수용을 사용자 대상 label로 렌더링합니다. 보안/개인정보, 범위/자율성, QA 면제, 검증 면제 세부사항은 context, affected gate, owner ref로 보여줄 수 있지만 별도 표시 범주는 아닙니다. 여러 판단이 대기 중이면 별도 줄 또는 card로 표시합니다. 잔여 위험 수용 display는 수용하는 위험을 이름 붙여야 합니다.

Close/readiness display는 관련 있을 때 근거, 검증, 수동 QA, 작업 수락, 잔여 위험 표시, 잔여 위험 수용을 별도 줄로 유지해야 합니다. Projection은 test pass, Eval, QA waiver, 작업 수락 user judgment, accepted Residual Risk ref를 요약할 수 있지만, 그중 하나를 다른 범주나 모든 것을 대신하는 "완료" flag로 렌더링하면 안 됩니다.

## Document authority matrix

| 사실 또는 접점 | 기준 출처 | Projection 또는 표시되는 보기 | Update path |
|---|---|---|---|
| Current Task state | `state.sqlite.tasks`, `task_gates`, `state.sqlite.task_events` | `TASK` Current Summary와 status card | Core transition, then projector |
| Task continuity | `state.sqlite` Task, Change Unit, Run, Evidence Manifest, Eval, 수동 QA, User Judgment, Approval, Residual Risk, `task_gates.acceptance_gate`, 작업 수락 user judgment state, close events, artifact ref, 필요할 때 `task_spine_entries` / public `journey_spine_entry` records, `state.sqlite.task_events` | `TASK` Journey Spine | Core transition 또는 reconcile, Journey reconstruction, then projector |
| User Judgment | `judgment_type`, `presentation`, `display_label`을 포함한 `state.sqlite.user_judgments`, 관련 `decision_gate` state, judgment event, 관련 approval 또는 reconcile record, artifact ref, 필요할 때 연결된 `state.sqlite.residual_risks` | Compact status card judgment line, status/next responses, user-judgment resources, 또는 dedicated prompt; later/standalone projection이 켜진 경우 optional full-format Decision Packet, `TASK`, Journey Card, `DEC` display | `request_user_judgment` / `record_user_judgment`, then projector |
| Journey Spine | `state.sqlite` Task, Change Unit, Run, User Judgment, Approval, Evidence Manifest, Eval, 수동 QA, Residual Risk, `task_gates.acceptance_gate`, 작업 수락 user judgment state, close events, artifact ref, 필요할 때 `task_spine_entries` / public `journey_spine_entry` records, `state.sqlite.task_events` | `TASK` Journey Spine section, resume view, Journey Spine-oriented card | Core transition 또는 reconcile, Journey reconstruction, then projector |
| Journey Card | 현재 `state.sqlite` Task state, gate, active Change Unit, Autonomy Boundary summary, active user judgment ref, residual-risk summary, latest evidence/eval/QA/보고서 ref, projection 최신성 | `JOURNEY-CARD`, status card, `harness.status` / `status.next_actions` card text, later/compatibility `harness.next` 현재 위치 text, significant resume output | 현재 상태에서 read 또는 projection 새로고침; card를 직접 편집하지 않음 |
| Autonomy Boundary | active `state.sqlite.change_units` Autonomy Boundary field와 관련 user judgment 해소/event | `TASK` Autonomy Boundary, Change Unit block, Journey Card autonomy line, standalone projection이 켜져 있을 때 optional related `DEC` | shaping update 또는 user judgment 해소, then projector |
| Write Authorization | `state.sqlite.write_authorizations`와 관련 Task, Change Unit, approval, user judgment, baseline, consumed Run ref | `TASK` Write Authority Summary, Journey Card Write Authority Summary line, `RUN-SUMMARY` relation | `prepare_write`가 생성함; idempotent replay는 already committed response를 반환함; `record_run`이 authorization을 consume한 뒤 projector |
| Implementation Micro-Plan | 현재 `state.sqlite` Task state와 gate, active Change Unit scope와 Autonomy Boundary, Change Unit dependency summary, selected feedback-loop records, TDD가 selected된 경우 TDD traces, expected evidence needs, user judgment blockers, latest 보고서 refs | `TASK` Implementation Micro-Plan managed section | Accepted reconcile outcome 또는 Core state-changing action이 owner 기록을 업데이트한 뒤 projector |
| Change Unit DAG | `state.sqlite.change_units`, `state.sqlite.change_unit_dependencies`, dependency 관련 event, active Task state | `TASK` Change Unit Dependencies / DAG summary | shaping update 또는 reconcile, then projector |
| Residual Risk | `state.sqlite.residual_risks`, accepted-risk metadata와 residual-risk refs, related user judgment, evidence/QA/eval ref, artifact ref | `TASK` Residual Risk, standalone projection이 켜져 있을 때 optional `DEC` accepted-risk context, Journey Card residual-risk line | judgment, evidence, QA, Eval, reconcile 또는 close flow에서 Core transition, then projector |
| Stewardship Impact Summary | `domain_terms`, `module_map_items`, `interface_contracts`, `feedback_loops`, TDD가 selected된 경우 TDD records, `state.sqlite.residual_risks`, `state.sqlite.user_judgments`, policy validator 결과, related refs | `TASK` Stewardship Impact와 status/resume stewardship display | Owner 기록 업데이트, validator 결과, reconcile, close flow, then projector |
| Review Stages | Task, Change Unit, gate state, evidence summaries, Run refs, ArtifactRefs, active일 때 Evidence Manifest, validator 결과, 수동 QA, Eval, active일 때 민감 동작 승인 user judgment 또는 Approval, Residual Risk, stewardship owner refs, structured blocker refs | Spec Compliance Review와 Code Quality / Stewardship Review라는 `TASK` 및 `RUN-SUMMARY` sections | Existing owner-record update, validator result, user judgment, evidence, 수동 QA, Eval, residual-risk, close-blocker, Change Unit, follow-up path, then projector |
| User Notes | human-editable input -> `reconcile_items` -> accepted Core state-changing action과 `state.sqlite.task_events`, 또는 rejected/deferred/note outcome | `TASK` User Notes and Proposals | human edit, reconcile decision, Core event |
| Shared Design | shared design record 및 event | `TASK` summary, `DESIGN`, standalone projection이 켜져 있을 때 optional `DEC` | Core transition 또는 reconcile, then projector |
| Domain Language | `domain_terms` table | `DOMAIN-LANGUAGE` projection | Core transition 또는 reconcile, then projector |
| Module Map | `module_map_items` table | `MODULE-MAP` projection | Core transition 또는 reconcile, then projector |
| Interface Contract | `interface_contracts` table | `INTERFACE-CONTRACT` projection | Core transition 또는 reconcile, then projector |
| Feedback Loop | `feedback_loops` table plus runs, artifacts, TDD traces, 수동 QA, evidence manifests refs | `TASK` Stewardship Impact와 Evidence Manifest 설계 품질 coverage; current reference catalog에는 standalone Feedback Loop projection이 없음 | `record_run` shaping 또는 evidence update의 `FeedbackLoopUpdate`, `record_manual_qa`의 `feedback_loop_ref`, 또는 reconcile, then projector |
| 민감 동작 승인 | Minimum MVP-1: `judgment_type=sensitive_action_approval`, `judgment_payload.approval_scope`, judgment event를 가진 `state.sqlite.user_judgments`. Later Approval profile: `approvals`, 연결된 민감 동작 승인 user judgment, 구현이 유지하는 경우 optional user-judgment request routing/replay record, event. `approval_request_candidate`만 있는 경우는 제외 | Minimum MVP-1: compact status card 또는 user judgment prompt/display. Later Approval profile: `APR` projection과 Approval Card | Minimum MVP-1: `request_user_judgment(judgment_type=sensitive_action_approval)`이 user judgment를 만들고 `record_user_judgment`가 해소한 뒤 projector/card가 필요하면 렌더링합니다. Later Approval profile은 Approval record를 추가로 만들거나 update한 뒤 projector를 실행할 수 있습니다 |
| Run summary | `runs` table plus artifact refs | `RUN-SUMMARY` projection | `record_run`, then projector |
| Direct result | direct run record plus artifact refs | `DIRECT-RESULT` projection | `record_run` / `close_task`, then projector |
| Evidence coverage | Minimum MVP-1: evidence summary records/refs, Run refs, ArtifactRefs, 보이는 gap summary. Full Evidence Manifest profile: `evidence_manifests` plus artifact refs | MVP-1에서는 compact status card evidence summary/gap display. `EVIDENCE-MANIFEST` projection은 full evidence profile이 active일 때만 사용 | MVP-1에서는 `record_run` evidence update. Full Evidence Manifest profile이 active일 때 evidence module update, then projector |
| Verification verdict | `evals` plus artifact refs | `EVAL` projection과 verification card | `record_eval`, then projector |
| TDD trace | `tdd_traces` plus artifact refs | `TDD-TRACE` projection | `record_run` 또는 reconcile, then projector |
| 수동 QA | Aggregate QA 요구사항 상태에는 `qa_gate`; 기록이 있을 때 `manual_qa_records` plus artifact refs | `MANUAL-QA` projection과 QA card | `record_manual_qa`, then projector |
| Raw evidence | artifact store plus `artifacts` records | 보고서 안의 artifact 참조 | artifact registry |
| Projection freshness | `projection_jobs.source_state_version`, `projection_jobs.projection_version`, job status, managed hashes, artifact records | front matter mirror, status card, operations output | projector and recovery tools |

Full-format user judgment projection과 card는 구체적인 judgment type을 보여줘야 합니다. Canonical user judgment field와 friendly label은 분리해서 렌더링합니다. `judgment_type`은 compact internal type이고, `presentation`은 prompt/detail level이며, `display_label`은 제품/UX 판단, 기술 판단, 민감 동작 승인, 작업 수락, 잔여 위험 수용 중 하나입니다. 민감 동작 승인이 작업 수락처럼 보이면 안 됩니다. 작업 수락도 잔여 위험 수용처럼 보이면 안 됩니다. 영향을 받는 gate나 막힌 행동은 display label이 아니라 `affected_gates`와 관련 owner record에서 옵니다. `judgment_category`, `judgment_route`, `display_depth` 같은 legacy field는 migration note 또는 compatibility drill-down에서만 나타날 수 있습니다.

필수 권한 설명:

- User Notes: human-editable input -> `reconcile_items` -> accepted Core state-changing action과 `state.sqlite.task_events`, 또는 rejected/deferred/note outcome
- Domain Language: `domain_terms` table -> `DOMAIN-LANGUAGE` projection; 기준 term row에 대한 public ref는 `StateRecordRef.record_kind=domain_term`을 사용합니다.
- Module Map: `module_map_items` table -> `MODULE-MAP` projection; 기준 module row에 대한 public ref는 `StateRecordRef.record_kind=module_map_item`을 사용합니다.
- Interface Contract: `interface_contracts` table -> `INTERFACE-CONTRACT` projection; 기준 contract row에 대한 public ref는 `StateRecordRef.record_kind=interface_contract`를 사용합니다.
- Feedback Loop: `feedback_loops` table -> `TASK`와 Evidence Manifest display; 기준 feedback-loop row에 대한 public ref는 `StateRecordRef.record_kind=feedback_loop`를 사용합니다. TDD Trace refs는 separate execution evidence refs로 남습니다.
- User Judgment: `judgment_type`, `presentation`, `display_label`을 포함한 `state.sqlite.user_judgments`와 관련 ref -> compact status card judgment line, status/next responses, judgment-context resources, user-judgment resources, 또는 dedicated prompt; later/standalone full-format projection이 켜진 경우 optional `TASK`, Journey Card, `DEC` projection
- Journey Spine: owner 기록, artifact ref, 필요할 때 `task_spine_entries` / public `journey_spine_entry` records, `state.sqlite.task_events`에서 재구성합니다. 자체 권한 기록은 아닙니다.
- Journey Card: 현재 상태와 ref에서 만든 파생 표시입니다. 절대 기준 상태가 아닙니다.
- Autonomy Boundary: active `state.sqlite.change_units` boundary field -> projection 접점. 판단 재량이지 범위 권한이 아닙니다.
- Write Authority Summary: active scope, approval, Write Authorization, baseline, guarantee ref에서 만든 파생 표시입니다. 절대 기준 상태가 아니며 work를 허가할 수 없습니다.
- Write Authorization: `state.sqlite.write_authorizations`는 specific allowed write attempt를 기록합니다. Scope, approval, evidence, verification, QA, 작업 수락, 잔여 위험 수용이 아닙니다.
- Implementation Micro-Plan: 현재 Task와 Change Unit의 owner 기록 및 관련 참조에서 생성되는 `TASK`의 managed execution-aid section. 기준 상태가 아니고, 새 `ProjectionKind`도 아니며, 범위 권한, Approval, Write Authorization이 아닙니다.
- 민감 동작 승인: minimum MVP-1은 민감 동작 승인 user judgment -> compact status card 또는 user judgment prompt/display로 보여줍니다. Later Approval profile은 `approvals`와 관련 user judgment -> Approval 기록 존재 또는 변경 뒤에만 `APR` projection을 만듭니다. `prepare_write`가 반환한 `approval_request_candidate`는 candidate 표시로 보여줄 수 있지만 `APR` source는 아닙니다.
- Change Unit DAG: `state.sqlite.change_unit_dependencies`와 Change Unit ref -> dependency projection. scheduler 또는 authorization 접점이 아닙니다.
- Residual Risk: accepted-risk metadata/refs를 포함한 `state.sqlite.residual_risks` -> 잔여 위험 표시
- Stewardship Impact Summary: owner 기록, validator 결과, 참조에서 파생됨 -> `StewardshipImpactSummary` display. 기준 record는 아닙니다.
- Review Stages: Task, Change Unit, gates, 근거, validator 결과, residual-risk refs, stewardship owner refs -> Spec Compliance Review와 Code Quality / Stewardship Review라는 managed `TASK` 또는 `RUN-SUMMARY` display sections. 기준 records가 아니며, 새 `ProjectionKind` values, Approval, 근거, 검증, 수동 QA, 작업 수락, 잔여 위험 수용, close, Write Authorization, 분리 검증도 아닙니다.

## Managed block rules

Managed block은 projector가 덮어쓸 수 있는 유일한 Markdown 영역입니다.

```md
<!-- HARNESS:BEGIN managed -->
...
<!-- HARNESS:END managed -->
```

규칙:

- Managed block content는 committed 상태 기록 및 artifact ref에서 생성됩니다.
- Projector는 `projection_jobs.source_state_version`, projection version, 렌더링 timestamp, managed hash를 기록합니다. Front matter는 operator를 위해 recorded source state version을 그대로 비춥니다.
- Managed hash는 `HARNESS:BEGIN`과 `HARNESS:END` marker lines를 제외한 projector-owned managed block body에서 계산하며, line endings를 LF로 normalize하고 projector rules가 요구하는 meaningful whitespace를 보존합니다.
- 렌더링 전에 managed block hash가 last projected hash와 다르면 projector는 reconcile item을 만들거나 업데이트합니다.
- Managed hash는 drift detection에만 사용하며 렌더링된 Markdown을 기준 상태로 만들지 않습니다.
- Projector는 managed block 내부의 direct edit를 accepted state로 조용히 취급하지 않습니다.
- Managed block을 다시 렌더링할 때 관련 없는 사람이 편집할 수 있는 영역은 보존해야 합니다.
- 렌더링 실패는 projection 최신성을 `failed` 또는 `stale`로 표시하며 상태를 롤백하지 않습니다.
- Rendered template은 report가 view이고, managed block은 projector-owned이며, 사람이 편집할 수 있는 section은 proposal input임을 독자가 알 수 있도록 top 또는 managed summary 근처에 짧은 projection boundary notice를 포함해야 합니다.

Front matter는 diagnostic 용도로 compact하게 유지합니다. 렌더링된 object를 식별하고, projection version 또는 status를 표시하며, `source_state_version`을 그대로 비추고, 렌더링 timestamp를 포함할 수 있습니다. Large state summary, evidence body, gate rollup, artifact inventory는 포함하면 안 됩니다.

`projection_version`은 projection/template/job version입니다. State clock이 아니며 source-state freshness basis로 사용하면 안 됩니다. `source_state_version`은 렌더링 source로 사용한 affected-scope state clock 값입니다. Projection이 task-scoped이면 Task State Version이고, 그렇지 않으면 Project State Version 또는 extension-defined owner state clock입니다.

기준 per-projection value는 성공한 렌더링 job의 `projection_jobs.source_state_version`입니다. Front matter `source_state_version`은 operator diagnosis를 위해 그 값을 그대로 비출 뿐입니다.

표시 문제를 하나의 status로 뭉개면 안 됩니다.

- Stale projection은 읽기용 Markdown 또는 card가 owner record나 artifact ref보다 뒤처졌을 수 있다는 뜻입니다.
- Stale state, stale baseline, stale evidence는 underlying state 또는 evidence input이 이동했거나 더 이상 충분하지 않다는 뜻입니다. Projection은 current여도 그 blocker를 표시할 수 있습니다.
- MCP unavailable은 surface 또는 caller가 필요한 Harness/Core capability에 닿지 못한다는 뜻입니다. Core에 닿을 수 없다면 표시 내용만으로 Core의 기준 상태 변경, projection repair, approval, close, gate update를 주장할 수 없습니다.

## Human-editable section rules

- 사람이 편집할 수 있는 text는 입력이지 기준 상태가 아닙니다.
- Reconcile은 edit를 읽고 상태 변경이 필요할 수 있으면 `reconcile_items` candidate를 만듭니다.
- Accepted proposal은 Core transition과 추가된 `state.sqlite.task_events` row를 통해서만 상태가 됩니다.
- Rejected proposal은 note 또는 rejected reconcile item으로 남습니다.
- Projector는 refresh 중 사람이 편집할 수 있는 content를 보존해야 합니다.
- 사람이 편집한 proposal은 Task summary, acceptance criteria, Domain Language, Module Map, Interface Contract, 수동 QA note, 기타 상태 기반 기록을 대상으로 삼을 수 있지만 proposal 자체가 target 기록은 아닙니다.

## Embed와 reference

사용자 읽기용 output은 짧은 요약과 안전한 label만 본문에 넣어야 합니다. 큰 log, diff, trace, screenshot, recording, checkpoint, bundle, export component, 민감한 artifact는 읽기용 Markdown에 붙여 넣지 말고 `ArtifactRef` 또는 `StateRecordRef`로 참조합니다.

Reference/diagnostic output은 더 자세한 artifact inventory, hash, retention state, availability note를 나열할 수 있지만, 여전히 크거나 민감한 bytes는 재구성하지 않고 참조합니다. Projection은 ref가 무엇을 뒷받침하는지, input이 redacted, omitted, blocked, stale, missing 중 무엇인지 말할 수 있습니다. 하지만 생략된 secret/PII value, blocked payload bytes, 제한 없는 raw evidence를 inline하면 안 됩니다.

## Artifact reference 렌더링

Markdown 보고서는 artifact reference를 compact하고 consistent하게 표시합니다. Payload shape는 MCP API document가 담당하며, projection은 presentation rule만 담당합니다.

권장 display:

```text
- Diff: DIFF-0001 (`artifact_id=ART-0001`, sha256:abc123..., redaction:none)
- Test log: LOG-0002 (`artifact_id=ART-0002`, sha256:def456..., redaction:redacted)
- Bundle: BUNDLE-0001 (`artifact_id=ART-0003`, sha256:789abc..., redaction:secret_omitted)
- Browser trace: TRACE-0004 (`artifact_id=ART-0004`, sha256:012def..., redaction:blocked)
```

규칙:

- 모든 artifact ref는 artifact record로 해석되어야 합니다.
- 모든 raw artifact ref는 integrity metadata와 redaction 상태를 가져야 합니다.
- Large 또는 sensitive evidence는 Markdown에 붙여 넣지 않고 link만 둡니다.
- 사용자에게 보이는 artifact line은 ref identity, 필요할 때 owner 또는 supporting record, hash 또는 short hash, 필요할 때 size, redaction state, retention class 또는 availability, evidence 영향 이해에 필요한 omission/block note를 표시해야 합니다.
- `secret_omitted` artifact는 ref와 omission note 또는 handle로 표시합니다. 보이는 nonsecret claim은 뒷받침할 수 있지만, projection text가 omitted value를 검토했거나 증명한 것처럼 암시하면 안 됩니다.
- `blocked` artifact는 ref와 block note로 표시합니다. 표시되는 hash, size, content type은 committed metadata-only notice bytes를 설명하며 금지된 원본 payload가 아닙니다. Projector는 생략되었거나 차단된 원본 value를 재구성하거나, inline하거나, 요약하거나, export하면 안 됩니다.
- `blocked` artifact 표시는 raw capture를 사용할 수 없다는 뜻입니다. Evidence, QA, verification, Release Handoff, export display는 replacement, waiver, User Judgment outcome, accepted risk, 또는 documented fallback이 해당 path를 해소하기 전까지 blocked, insufficient, unavailable input, unresolved 중 적절한 상태로 그 block을 드러내야 합니다.
- Missing 또는 hash-mismatched artifact는 related evidence, projection 최신성, export display, close-readiness view를 `stale` 또는 blocked로 표시합니다. 사용자에게 보이는 보고서는 affected ref와 `artifacts check` 또는 recovery path를 가리켜야 하며, report prose를 replacement evidence로 붙여 넣으면 안 됩니다.
- StateRecordRef는 record identity와 optional projection path를 사용합니다. `record_kind=projection`에서 identity는 `projection_jobs.projection_job_id`이고 path는 locator일 뿐입니다. Raw artifact ref로 표시하지 않습니다.
- `artifact_links.record_kind`는 existing same-Task state owner 또는 projection ref로 해석되어야 합니다. Current Task-scoped artifact links는 Task-scoped로 남습니다. `record_kind=projection`은 같은 `task_id`를 가진 completed `projection_jobs` row로 해석되며, link의 `record_id`는 `projection_jobs.projection_job_id`이고 path display는 `StateRecordRef.projection_path` 또는 `projection_jobs.output_path`를 사용합니다. Project-level owner rows와 project-level projection jobs는 future extension이 project-scoped artifact linking을 추가하지 않는 한 state 또는 projection job freshness/output metadata를 사용합니다. `EXPORT`는 `ProjectionKind`일 뿐입니다. Export snapshot과 component는 owner record 또는 `record_kind=projection`에 link되는 artifact로 남으며 `record_kind=export`에 link하지 않습니다.

## 템플릿 구현 계층

Template catalog는 초기 구현 set보다 의도적으로 넓습니다. 템플릿 구현 계층은 렌더링 shape의 단계를 나눌 뿐 projection authority를 바꾸지 않습니다.

| 계층 | Template 또는 산출물 shape | Rule |
|---|---|---|
| Core status output | 최소 [Compact Status Card](templates/compact-status-card.md) 또는 동등한 status/blocker response shape | 내부 엔지니어링 점검의 현재 Core state에서 오는 structured status/blocker output을 렌더링할 때 선택할 수 있습니다. Plain structured response만으로 충분하며 persisted Markdown projection job이나 full renderer는 필요하지 않습니다. |
| MVP-1 사용자 작업 루프 projection | [Compact Status Card](templates/compact-status-card.md) | MVP-1 projection shape는 하나의 compact card입니다. 무엇을 하는지, 범위/하지 않을 일, 대기 중인 사용자 판단, 알려진 근거 summary, 근거 gap, close blocker, 보이는 잔여 위험, 다음 안전한 행동, source/freshness ref를 보여줍니다. 작업 수락과 잔여 위험 사실은 관련 있을 때 distinct하게 남지만 필수 projection kind를 늘리지는 않습니다. |
| Agent compact context/reference payload | 간결한 structured refs 또는 prompt 크기의 state summary | 에이전트용 파생 지원 payload입니다. Id, ref, blocker label, freshness를 작게 유지하고 기본으로 full Markdown body를 push하지 않습니다. |
| User judgment prompt shape | standalone `DEC` `ProjectionKind`가 아닌 [Decision Packet](templates/decision-packet.md) 사용자 판단 요청 display/card shape | 사용자 소유 판단이 대기 중일 때 쓰는 prompt/display shape입니다. MVP-1 사용자 작업 루프 projection도 아니고 standalone `DEC` `ProjectionKind`도 아닙니다. Standalone persisted `DEC` Markdown은 standalone Decision Packet projection 기능이 켜진 경우에만 사용합니다. |
| Agency assurance reports | [APR](templates/approval.md), [Approval Card](templates/approval-card.md), [MANUAL-QA](templates/manual-qa.md), [Manual QA Card](templates/manual-qa-card.md), [Verification Result Card](templates/verification-result-card.md) | 해당 approval, 수동 QA, waiver, verification, assurance profile이 active일 때만 사용합니다. 내부 엔지니어링 점검이나 MVP-1 사용자 작업 루프 requirement가 아닙니다. |
| Operations/export reports | [EXPORT](templates/export.md)와 해당 operations profile이 켜졌을 때의 release-handoff/export report shape | Later operations output입니다. Export report는 ref, hash, redaction state, omission/block note, 포함된 projection snapshot을 나열하지만 Core state나 artifact authority를 대체하지 않습니다. |
| Future/diagnostic projections | [TASK](templates/task.md), [DIRECT-RESULT](templates/direct-result.md), [RUN-SUMMARY](templates/run-summary.md), [EVIDENCE-MANIFEST](templates/evidence-manifest.md), [EVAL](templates/eval.md), [TDD-TRACE](templates/tdd-trace.md), [DOMAIN-LANGUAGE](templates/domain-language.md), [MODULE-MAP](templates/module-map.md), [INTERFACE-CONTRACT](templates/interface-contract.md), [DESIGN](templates/design.md), [JOURNEY-CARD](templates/journey-card.md), full [Approval Card](templates/approval-card.md), enabled 상태의 standalone `DEC` Markdown | Detailed continuity, direct-result, reference, diagnostic, stewardship, map, trace, persisted Journey Card, Journey Spine-style, standalone Decision Packet, detailed Evaluation view입니다. 필요할 때만 가져오거나 owner가 승격한 profile에서 사용하되 초기 필수 범위로 만들지 않습니다. |

`Future/diagnostic projections`는 later-profile 또는 diagnostic 범위라는 뜻이며, 자동으로 로드맵 전용이라는 뜻은 아닙니다.

어떤 template class도 projection을 기준 상태로 만들거나 evidence, QA, verification, 작업 수락, 잔여 위험 수용, close authority, Write Authorization을 만들지 않습니다. Source record가 없으면 projector는 template completeness를 맞추기 위해 placeholder state를 만들어내면 안 됩니다.

`EXPORT` template은 선택적 projection output입니다. Artifact link를 위한 `export` 상태 기록을 도입하지 않습니다. Export projection은 artifact ref, hash, redaction state, redaction/omission/block note를 나열하며, 기본적으로 large 또는 sensitive artifact body를 embed하지 않습니다.

Persisted `JOURNEY-CARD` Markdown, Journey Spine output, `TASK` continuity report, Direct Result report, Run Summary, detailed Evidence Manifest, detailed Eval, full 수동 QA card, TDD Trace, Domain Language, Module Map, Interface Contract, Design, Export bundle, full Approval Card, 그 밖의 polished report는 초기 필수 범위가 아닙니다. Status, next, resume의 현재 위치 맥락은 Core status output, MVP-1 compact status card, 또는 agent compact context/reference payload로 충족합니다.

Active stage/profile이 요구하는 user judgment visibility는 status/next responses, judgment-context resources, user-judgment resources, enabled compatibility decision-packet resources, compact status card, 또는 별도 사용자 판단 prompt를 통해 제공됩니다. 이 요구사항은 standalone `DEC` `ProjectionKind`가 아니라 user judgment request display shape에 대한 것입니다. Standalone `DEC` Markdown은 standalone full-format Decision Packet projection 기능이 켜진 경우에만 사용합니다.

User Judgment record ID는 user judgment identifier family를 사용합니다. `projection_kind`의 `DEC`는 projection kind label일 뿐입니다. Standalone projection에 별도 identity가 필요하면 `DEC-PROJ-0001` 같은 별도 `projection_id`를 사용합니다.

표시 카드 형태도 [Template 참조](templates/README.md)에 있습니다: [Compact Status Card](templates/compact-status-card.md), [Approval Card](templates/approval-card.md), [Verification Result Card](templates/verification-result-card.md), [수동 QA Card](templates/manual-qa-card.md).

## Freshness and failure rules

Projection 최신성은 current owner 또는 affected-scope state clock, 기준 `projection_jobs.source_state_version`, projection job state, managed hash, artifact availability, 알려진 `stale` trigger에서 계산됩니다. Front matter `source_state_version`은 마지막 successful 렌더링의 기준 값을 그대로 비추어, operator가 Markdown을 기준 상태로 취급하지 않고도 최신이 아닌 projection을 진단할 수 있게 합니다.

Close/readiness display는 세 사실을 구분해야 합니다. 현재 Core state version, projection source version 또는 failed job status, 그리고 요청한 operation에 대해 readable view가 충분히 current한지입니다. 오래된 Markdown에서 readiness를 추론하면 안 되며, failed projection을 현재 Task, Run, evidence, Eval, QA, Approval, user judgment, residual-risk state의 rollback이나 mutation으로 취급하면 안 됩니다.

아래 표는 해당 stage 또는 owner profile이 켠 projection에만 적용되는 freshness rule입니다. 여기에 projection이 listed되어 있다는 사실만으로 MVP-1 사용자 작업 루프 범위에 포함되지는 않습니다.

| Projection | Generated when | Stale when |
|---|---|---|
| `TASK` | Task가 created, resumed, changed, refreshed될 때 | `TASK` projection에서 current `tasks.state_version > projection_jobs.source_state_version`인 경우, managed block drift, 해소되지 않은 reconcile required, stewardship owner ref 또는 설계 품질 validator 결과 changed |
| `APR` | later Approval profile이 `request_user_judgment(judgment_type=sensitive_action_approval)` 또는 `record_user_judgment`를 통해 committed Approval record를 만들거나 update할 때 | linked Approval record status, scope, baseline, expiry, 관련 민감 동작 승인 user judgment, judgment note가 changed |
| `RUN-SUMMARY` | `record_run`이 `runs.status=completed`, `interrupted`, `blocked`, `violation`을 포함한 Run을 commit할 때 | run relation changed, artifact ref missing, artifact integrity fails |
| `EVIDENCE-MANIFEST` | full Evidence Manifest profile이 active이고 criteria-to-evidence coverage가 changed될 때 | baseline drift, changed files modified, required 근거 missing/`stale`, linked 민감 동작 permission / Approval drift 또는 expiry |
| `EVAL` | verification result가 기록될 때 | Eval 후 baseline changes, evidence `stale` 상태, independence relation invalidated |
| `DIRECT-RESULT` | direct run이 closes 또는 escalates될 때 | changed file drift, escalation state changes, artifact ref missing |
| `EXPORT` | export/보고서 projection이 generated될 때. 켜진 경우 Release Handoff profile 포함 | 포함된 Task/gate/Change Unit/user judgment/Residual Risk/evidence/verification/수동 QA/artifact/projection/redaction/checklist source가 changed 또는 unavailable일 때 |
| `DEC` | standalone full-format Decision Packet projection이 켜져 있고 user judgment가 created, requested, `resolved`, `deferred`, `rejected`, `blocked`, `superseded`될 때 | user judgment status, affected scope, current-state context, related approval/reconcile state, residual-risk ref, evidence 참조가 바뀔 때 |
| `JOURNEY-CARD` | card가 렌더링되거나 projection으로 persisted될 때. `harness.status`와 later/compatibility `harness.next`가 projection job 없이 ephemeral하게 반환할 수도 있음 | 표시된 Task/gate/Change Unit/Autonomy Boundary/Write Authorization/approval/baseline/guarantee/User Judgment/Residual Risk/evidence/보고서/freshness source가 렌더링된 card보다 앞서 이동할 때 |
| `DOMAIN-LANGUAGE` | domain terms change | term conflict, accepted term record changes, related code representation moves |
| `MODULE-MAP` | module map records change | module path, public interface, dependency direction, internal complexity, 테스트 경계, owner 결정, watchpoint changes |
| `INTERFACE-CONTRACT` | interface contract records change | linked interface, caller, compatibility impact, boundary tests change |
| `TDD-TRACE` | trace recorded 또는 updated | red/green log missing, baseline drift, linked test file changes |
| `MANUAL-QA` | QA record created 또는 updated | linked UI/code changes, required capture missing, 해소되지 않은 finding |

Freshness state:

| State | Meaning |
|---|---|
| `current` | projected content가 기준 `projection_jobs.source_state_version`에 기록된 committed state version 및 managed hash와 match함 |
| `stale` | state 또는 referenced 근거가 projection보다 앞서 이동함 |
| `failed` | projector가 refresh를 attempted했고 failed함 |
| `unknown` | freshness를 compute할 수 없음. 보통 recovery 또는 migration 중 |

Projection `stale` 상태는 current report view에 의존하는 close/readiness check를 포함해 현재 readable context가 필요한 action을 block할 수 있지만, 그 자체로 lifecycle result, gate value, assurance를 바꾸지는 않습니다. Projection failure 또는 `stale` 상태는 underlying task result를 절대 바꾸지 않으며 committed Core 상태를 롤백하지도 않습니다.
