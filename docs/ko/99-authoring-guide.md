# 작성 가이드

## 문서 역할

이 문서는 Harness 문서 세트를 작고, 구현 가능하고, 올바르게 계층화된 상태로 유지하는 규칙을 담당합니다.

Runtime behavior, user procedure, conformance fixture content, MCP schemas, SQLite DDL, projection templates는 담당하지 않습니다.

## 담당 경계와 owner boundary

각 concept에는 정확히 하나의 canonical owner를 둡니다. 다른 문서는 한 문장 요약과 link만 포함할 수 있습니다. 이 owner boundary를 흐리면 schema, DDL, policy, projection rules가 여러 문서에 중복되어 source-of-truth 경계가 약해집니다.

| 계층 | Canonical owner |
|---|---|
| one-sentence definition, reader paths, document list, target tree summary | `README.md` |
| shared reader mental model, three-space summary, core concepts introduction | `00-introduction.md` |
| project purpose, target users, values, scope, non-goals, automation philosophy | `01-project-charter.md` |
| why, failure model, MVP boundary, Strategic Invariants, Kernel Authority Invariants, Design Stewardship Defaults | `02-strategy.md` |
| entity meanings, lifecycle, gates, state transitions, close semantics, `prepare_write` and `close_task` logic | `03-kernel-spec.md` |
| three spaces, runtime authority flow, artifact architecture, projection/reconcile architecture, guarantee levels | `04-runtime-architecture.md` |
| MCP resources/tools, request/response schemas, error taxonomy, validator result schema, artifact ref shape | `05-mcp-api-and-schemas.md` |
| reference MVP implementation order, SQLite DDL, migrations, storage layout, validator runner skeleton | `06-reference-mvp.md` |
| Markdown projection principles, managed blocks, human-editable sections, template tiers, template summaries | `07-document-projection.md` |
| shared design, decision quality, autonomy boundary, domain language, vertical slice, feedback loop/TDD, module/interface, codebase stewardship, Manual QA, context hygiene policies | `08-design-quality-policy-pack.md` |
| agent surface capability profile, common connector contract, fallback semantics | `09-agent-integration.md` |
| user-facing conversation, status reading, resume procedure, approval/assurance/QA/acceptance explanation | `10-user-guide.md` |
| connect, doctor, serve MCP, projection refresh, reconcile, recover, export, artifact integrity, conformance | `11-operations-and-conformance.md` |
| full templates and expanded variants | `appendix/A-template-library.md` |
| surface-specific cookbooks | `appendix/B-surface-cookbook.md` |
| later automation and derived analytics | `appendix/C-later-roadmap.md` |
| old-to-new mapping and migration notes | `appendix/D-migration-notes.md` |
| official term definitions | `glossary.md` |

## Principle Group Language

Strategy는 세 원칙 그룹을 담당합니다. Strategic Invariants, Kernel Authority Invariants, Design Stewardship Defaults입니다. Owner doc이 업데이트되지 않았다면 helpful practices를 Kernel Authority Invariants로 승격하지 않습니다.

Strategic Invariants 문구는 구분된 약속을 보존해야 합니다.

```text
Strategic agency는 사용자에게 남아 있습니다.
작업 여정은 current state에서 따라갈 수 있어야 합니다.
```

Kernel Authority Invariants 문구는 mandatory하고 structural하게 써야 합니다.

```text
Product write에는 active scoped Change Unit이 필요합니다.
Blocking product judgment에는 recorded Decision Packet이 필요합니다.
Projection은 canonical state를 override할 수 없습니다.
```

Design Stewardship Defaults 문구는 applicability, waiver, record, validator, close impact를 함께 드러내야 합니다.

```text
Vertical slice는 적용 가능한 feature work의 default입니다.
Horizontal exception은 reason과 follow-up을 남겨 기록할 수 있습니다.
```

현재 Design Stewardship Defaults는 shared design, domain language consistency, vertical slice default, suitable work에 대한 TDD trace, module/interface review, Codebase Stewardship, Manual QA, feedback loops, context hygiene입니다.

## MVP, v1, Later Labels

이 labels를 일관되게 사용합니다.

| Label | 의미 |
|---|---|
| MVP | reference implementation이 Kernel Authority Invariants와 Agency Conformance를 검증하는 데 required |
| v1 | MVP 이후의 plausible next version이며, 여전히 fixtures와 담당 문서 필요 |
| later | MVP requirement처럼 읽히면 안 되는 유용한 future automation |

규칙:

- Main docs는 later work를 non-MVP context로만 언급할 수 있으며 `appendix/C-later-roadmap.md`를 가리켜야 합니다.
- Appendix C의 later-automation items나 team workflow expansion을 MVP requirements에 넣지 않습니다.
- later item이 v1이 되면 main docs를 바꾸기 전에 conformance expectations와 owner를 추가합니다.
- Derived metrics는 MVP-critical conformance signals로 명시적으로 승격되지 않는 한 analytics입니다.

## Source-Of-Truth 표현

다음 표현 계열을 사용합니다.

```text
Operational state는 state.sqlite current records와 state.sqlite.task_events에서 canonical합니다.
Raw evidence는 artifact store에서 canonical합니다.
Markdown reports는 state records와 artifact refs에서 생성되는 projections입니다.
Human-editable sections는 input surfaces입니다.
수용된 human edits는 reconcile 또는 Core state-changing action을 통해서만 state가 됩니다.
```

별도의 MVP event store가 있음을 암시하는 표현은 피합니다.

```text
state.sqlite와 별도의 event log를 나란히 두는 표현
```

Historical comparison에 이 개념이 필요하면 MVP event history가 `state.sqlite.task_events`임을 즉시 명확히 합니다.

다음처럼 쓰지 않습니다.

```text
TASK는 canonical state다.
Projection은 state를 update한다.
User Notes가 source-of-truth다.
Domain Language는 Markdown document에서 canonical하다.
Report projections는 기본적으로 raw artifacts다.
```

선호하는 authority paths:

```text
User Notes: human-editable input -> reconcile_items -> accepted state event/record
Domain Language: domain_terms -> DOMAIN-LANGUAGE projection
Module Map: module_map_items -> MODULE-MAP projection
Interface Contract: interface_contracts -> INTERFACE-CONTRACT projection
```

## Schema와 Template 담당 경계

MCP tool request/response schemas, common envelope, error taxonomy, validator result schema, artifact ref shape는 `05-mcp-api-and-schemas.md`에만 둡니다.

SQLite DDL, migration/versioning, lock policy, artifact directory layout, reference implementation storage details는 `06-reference-mvp.md`에만 둡니다.

Projection rules와 template tiers는 `07-document-projection.md`에 둡니다. Full template bodies와 expanded report variants는 `appendix/A-template-library.md`에 둡니다.

User-facing examples는 Journey Cards나 짧은 report snippets를 보여줄 수 있지만, schema definitions가 되면 안 됩니다.

## Current-State Writing

Canonical docs는 rewrite history가 아니라 current truth로 씁니다.

선호:

```text
Harness는 lifecycle fields와 gates를 사용합니다.
```

Main docs에서 피할 표현:

```text
이전 버전과 달리 Harness는 이제 lifecycle fields와 gates를 사용합니다.
```

Version comparison, removed sections, old file names는 `appendix/D-migration-notes.md`에 둡니다.

## Cross-Reference Rules

Contracts를 중복하지 말고 links로 owner를 가리킵니다.

최소 references:

- Strategy는 kernel과 policy pack을 참조합니다.
- Kernel은 API와 reference MVP를 참조합니다.
- Runtime architecture는 kernel, projection, integration을 참조합니다.
- API는 kernel과 reference MVP를 참조합니다.
- Reference MVP는 kernel, API, operations를 참조합니다.
- Projection은 kernel과 Appendix A를 참조합니다.
- Policy pack은 kernel과 projection을 참조합니다.
- Integration은 API와 Appendix B를 참조합니다.
- Operations는 API와 reference MVP를 참조합니다.

## TODO Rules

실제 product 또는 architecture decision이 미해결일 때만 `TODO_DECISION`을 사용합니다. 필요한 decision, affected docs, likely owner를 포함합니다.

Decision은 이미 내려졌지만 implementation detail, DDL, fixture coverage, CLI behavior가 아직 채워지지 않았을 때만 `TODO_IMPLEMENT`를 사용합니다.

완성된 v2 canonical sections에는 `TODO_REWRITE`를 사용하지 않습니다. 남아 있는 `TODO_REWRITE`는 해당 section이 아직 migration stub이라는 뜻입니다.

## Authoring Checklist

```text
[ ] 이 concept에는 정확히 하나의 canonical owner가 있는가?
[ ] Schema와 DDL은 owner docs에만 있는가?
[ ] Strategic Invariants, Kernel Authority Invariants, Design Stewardship Defaults가 분리되어 있는가?
[ ] Design Stewardship Defaults가 applicability와 waiver boundaries를 함께 드러내는가?
[ ] MVP, v1, later labels가 명확한가?
[ ] long-term analytics가 MVP requirements 밖에 있는가?
[ ] source-of-truth 표현이 state/artifact/projection boundaries를 보존하는가?
[ ] user guide가 DB/API/connector internals를 피하는가?
[ ] operations가 fixture-based conformance를 사용하는가?
[ ] legacy names가 migration notes에만 있는가?
[ ] official terms가 glossary와 aligned되어 있는가?
```
