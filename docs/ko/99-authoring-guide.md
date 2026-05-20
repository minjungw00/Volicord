# 작성 가이드

## 문서 역할

이 문서는 Harness 문서 세트를 작고, 구현 가능하고, 올바르게 계층화된 상태로 유지하는 규칙을 담당합니다.

또한 documentation drift를 찾기 위한 docs-maintenance conformance checklist를 담당합니다.

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
| connect, doctor, serve MCP, projection refresh, reconcile, recover, export, artifact integrity, runtime conformance, docs-maintenance smoke reporting | `11-operations-and-conformance.md` |
| full templates and expanded variants | `appendix/A-template-library.md` |
| surface-specific cookbooks | `appendix/B-surface-cookbook.md` |
| later automation and derived analytics | `appendix/C-later-roadmap.md` |
| old-to-new mapping and migration notes | `appendix/D-migration-notes.md` |
| document ownership, authoring rules, docs-maintenance conformance checklist | `99-authoring-guide.md` |
| official term definitions | `glossary.md` |

## Bilingual Sync

English and Korean documentation sets는 같은 file structure와 heading structure를 mirror합니다.

`docs/en`에 semantic change가 있으면 같은 batch에서 `docs/ko`에 반영합니다. Translation은 idiomatic할 수 있지만 authority boundaries, stable terms, schema names, DDL names, error codes, validator IDs는 일치해야 합니다.

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
- Browser QA Capture, preventive `T4` guard expansion, Context Index, analytics 또는 derived metrics, deployment/canary/rollback automation, team workflow, parallel orchestration은 owner document가 fixture coverage와 implementation ownership으로 명시적으로 promote하지 않는 한 v1-or-later로 취급합니다.
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

다음처럼 취급하는 표현을 쓰지 않습니다.

```text
TASK, Journey, Markdown, report text를 state authority로 취급.
Rendering output이 state를 mutate하는 것처럼 취급.
User Notes를 human-editable input 이상으로 취급.
DOMAIN-LANGUAGE Markdown을 vocabulary owner로 취급.
Report projections를 기본 raw evidence file로 취급.
```

선호하는 authority paths:

```text
User Notes: human-editable input -> reconcile_items -> accepted state event/record
Domain Language: domain_terms -> DOMAIN-LANGUAGE projection
Module Map: module_map_items -> MODULE-MAP projection
Interface Contract: interface_contracts -> INTERFACE-CONTRACT projection
```

## Judgment Surface, Not Lecture

User-facing docs는 사용자가 판단하는 데 필요한 context, choices, trade-offs, evidence, risk, recommendation, uncertainty, next action을 드러냅니다.

모든 internal gate를 설명하려 들지 않습니다. Gate는 progress, write, close, QA, acceptance, risk acceptance가 왜 blocked인지 설명할 때만 이름 붙입니다.

작업 판단의 owner는 사용자입니다. Agent와 Harness는 current state와 options를 드러내며, 사용자의 decision을 대신하지 않습니다.

## Schema와 Template 담당 경계

MCP tool request/response schemas, common envelope, error taxonomy, validator result schema, artifact ref shape는 `05-mcp-api-and-schemas.md`에만 둡니다.

SQLite DDL, migration/versioning, lock policy, artifact directory layout, reference implementation storage details는 `06-reference-mvp.md`에만 둡니다.

JSON `TEXT` fields를 문서화할 때는 split을 명시적으로 유지합니다. API payload validation shapes는 `05-mcp-api-and-schemas.md`에, SQLite column과 storage details는 `06-reference-mvp.md`에, doctor/recover/conformance expectations는 `11-operations-and-conformance.md`에 둡니다. Core가 commit 전에 storage JSON을 validate한다는 boundary note는 반복할 수 있지만, schema bodies나 DDL을 duplicate하면 안 됩니다.

Projection rules와 template tiers는 `07-document-projection.md`에 둡니다. Full template bodies와 expanded report variants는 `appendix/A-template-library.md`에 둡니다.

Conformance fixture body, suite catalog assertion-mode metadata, fixture assertion semantics는 `11-operations-and-conformance.md`에 둡니다. 다른 문서는 그 owner를 가리킬 수 있지만 comparison mini-language를 다시 정의하면 안 됩니다.

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

## Docs-Maintenance Conformance

Docs-maintenance conformance는 이 documentation corpus를 대상으로 하는 read-only review/check suite입니다. Core fixture conformance, runtime validator, canonical state transition, projection refresh, generated operational report, QA result, acceptance record, evidence artifact, residual-risk acceptance가 아닙니다.

Rule body는 이 guide에 둡니다. [Operations And Conformance](11-operations-and-conformance.md#docs-maintenance-smoke-profile)는 operator-maintenance profile이 이 checks를 어떻게 report하는지 설명할 수 있지만, full rule body를 duplicate하지 말고 여기로 link해야 합니다.

나중의 automated checker는 check category, file path, 가능한 경우 heading 또는 anchor, canonical owner document, observed drift, suggested fix를 report해야 합니다. Drift는 먼저 canonical owner를 update해서 해결하고, 그다음 non-owner duplicate를 summary plus link로 바꿉니다. 올바른 product 또는 architecture rule을 알 수 없으면 `TODO_DECISION`을 사용합니다. Rule은 알려져 있지만 checker wiring, fixture coverage, CLI behavior, implementation detail이 빠졌으면 `TODO_IMPLEMENT`를 사용합니다.

Report severity guidance:

| Severity | Meaning |
|---|---|
| `FAIL` | Broken owner links, schema/DDL/enum/stable event/`ValidatorResult`/`ProjectionKind` mismatch, English/Korean paired active file 누락, paired heading structure의 material difference, owner contract를 다시 정의하는 non-owner text처럼 active docs를 contradictory하거나 non-actionable하게 만들 수 있는 drift입니다. |
| `WARN` | Minor glossary phrasing drift, normative하지 않은 duplicate explanatory prose, stale하지만 non-blocking인 cross-reference wording, incomplete하지만 still understandable한 TODO metadata처럼 정리해야 하지만 아직 owner contract와 모순되지는 않는 drift입니다. |
| `PASS` | 해당 category에서 relevant drift가 발견되지 않았습니다. |

필수 check categories:

| Category | Required check |
|---|---|
| English/Korean file structure parity | 명시적인 예외가 이 guide에 문서화되지 않는 한 `docs/en`과 `docs/ko`는 같은 active document paths와 appendix paths를 유지합니다. |
| English/Korean heading parity | Paired files는 같은 heading order와 depth를 유지합니다. Heading text는 idiomatic할 수 있지만 stable names, IDs, enum values, schema names, DDL names, owner section links는 semantic하게 aligned되어야 합니다. |
| Broken cross-reference detection | Markdown links, heading anchors, appendix links, paired-language entry links가 active docs로 resolve됩니다. Owner section link는 subject가 migration context일 때가 아니면 migration notes를 가리키면 안 됩니다. |
| Owner-boundary drift | Public schemas는 `05-mcp-api-and-schemas.md`에, SQLite DDL과 reference storage details는 `06-reference-mvp.md`에, kernel transitions와 stable events는 `03-kernel-spec.md`에, projection rules와 template tiers는 `07-document-projection.md`에, full template bodies는 `appendix/A-template-library.md`에, fixture body shape와 fixture assertion semantics와 fixture suite behavior는 `11-operations-and-conformance.md`에, official definitions는 `glossary.md`에 둡니다. |
| Fixture/action schema and code drift | Operations fixture examples의 `action`과 executable `input`은 `05-mcp-api-and-schemas.md`의 public MCP request schemas와 `11-operations-and-conformance.md`의 `ToolEnvelope` expansion convention에 aligned되어야 합니다. Required fixture events는 Kernel Stable Event Catalog names로 유지하고, `expected_error.code`와 `CloseTaskResponse.blockers[].code`는 API `ErrorCode` values여야 합니다. Finding code는 validator findings 또는 equivalent expected validator output에 남깁니다. 이 check는 fixture semantics를 여기서 restate하지 않고 Operations, API, Kernel owner로 link합니다. |
| Enum drift across owners | State, gate, result, close, assurance, error, projection, validator, storage enum values는 이를 정의하는 owner doc과 일치해야 합니다. Non-owner docs는 필요할 때만 값을 summarize하고 owner로 link해야 합니다. |
| Stable Event Catalog drift | Operations fixtures, API tool descriptions, Reference MVP conformance text가 fixture-stable로 요구하는 event name은 Kernel Stable Event Catalog에 있어야 합니다. Non-catalog names는 illustrative, implementation-local detail/audit, future extension으로 표시하거나 kernel owner를 통해 promote해야 합니다. |
| Stable ValidatorResult ID drift | Stable `ValidatorResult` IDs는 API-owned list와 Reference MVP validator runner text와 일치해야 합니다. Core checks와 preconditions는 API 또는 Reference MVP owner가 promote하지 않는 한 validator IDs로 drift하면 안 됩니다. |
| ProjectionKind tier drift | `ProjectionKind` values와 tiers는 API, Document Projection, Reference MVP, Appendix A, Operations, Glossary에서 일치해야 합니다. Extension / appendix values는 owner docs 밖에서 반복되면서 MVP-required가 되면 안 됩니다. |
| Glossary term drift | Official terms, capitalization, record ID prefixes, source-of-truth meanings는 `glossary.md`와 일치해야 합니다. 반복해서 쓰이는 새 term에는 glossary entry를 추가하거나 local로 유지한다는 explicit decision이 필요합니다. |
| Source-of-truth phrasing drift | State, raw evidence, Markdown projections, human-editable sections, reconcile, accepted human edits는 이 guide의 phrasing family를 사용하고 separate state authority를 암시하지 않아야 합니다. |
| `TODO_DECISION` and `TODO_IMPLEMENT` compliance | TODO는 allowed labels를 사용하고, 필요한 decision 또는 알려진 implementation gap을 포함하며, 유용하면 affected docs를 이름 붙이고, finished canonical sections에 실제 `TODO_REWRITE` markers를 남기지 않습니다. |
| Non-owner duplicate full contracts | Owner doc 밖에서 full schemas, DDL, transition tables, fixture mini-languages, template bodies, glossary definitions를 restate하는 paragraphs는 one-sentence summary plus owner link로 바꾸어야 합니다. |

## TODO Rules

실제 product 또는 architecture decision이 미해결일 때만 `TODO_DECISION`을 사용합니다. 필요한 decision, affected docs, likely owner를 포함합니다.

Decision은 이미 내려졌지만 implementation detail, DDL, fixture coverage, CLI behavior가 아직 채워지지 않았을 때만 `TODO_IMPLEMENT`를 사용합니다.

완성된 v2 canonical sections에는 `TODO_REWRITE`를 사용하지 않습니다. 남아 있는 `TODO_REWRITE` marker는 해당 section이 아직 migration stub이라는 뜻입니다. 이 Authoring Guide rule text에서 `TODO_REWRITE`를 설명적으로 언급하는 것은 허용되며 leftover migration stub으로 세면 안 됩니다.

## Authoring Checklist

```text
[ ] 이 concept에는 정확히 하나의 canonical owner가 있는가?
[ ] Schema와 DDL은 owner docs에만 있는가?
[ ] Strategic Invariants, Kernel Authority Invariants, Design Stewardship Defaults가 분리되어 있는가?
[ ] Design Stewardship Defaults가 applicability와 waiver boundaries를 함께 드러내는가?
[ ] MVP, v1, later labels가 명확한가?
[ ] long-term analytics가 MVP requirements 밖에 있는가?
[ ] source-of-truth 표현이 state/artifact/projection boundaries를 보존하는가?
[ ] semantic changes가 `docs/en`과 `docs/ko`에 같은 batch로 mirrored되었는가?
[ ] user-facing docs가 불필요한 internal gates를 가르치지 않고 judgment context를 드러내는가?
[ ] user guide가 DB/API/connector internals를 피하는가?
[ ] operations가 prose-only matching 대신 executable assertion이 있는 fixture-based conformance를 사용하는가?
[ ] docs-maintenance가 Operations, API, Kernel owner links를 통해 fixture/action schema drift와 event/error-code drift를 확인하고 fixture semantics를 duplicate하지 않았는가?
[ ] docs-maintenance conformance 관점에서 bilingual parity, links, owner boundaries, stable catalogs, glossary terms, source-of-truth phrasing, TODO rules를 검토했는가?
[ ] docs-maintenance conformance references가 runtime validators나 canonical state transitions가 아니라 read-only documentation maintenance로 쓰였는가?
[ ] non-owner full-contract paragraphs가 summaries plus owner links로 줄었는가?
[ ] legacy names가 migration notes에만 있는가?
[ ] official terms가 glossary와 aligned되어 있는가?
```
