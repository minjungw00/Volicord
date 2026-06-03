# Translation Guide

## What this document helps you do

Use this guide when editing English and Korean Harness documentation together.

This is maintenance documentation for bilingual documentation. It does not authorize runtime/server implementation, generated operational files, executable fixtures, runtime data, or product state changes before documentation acceptance and a separate implementation-planning readiness decision, and it does not define conformance pass/fail, evidence, QA, work acceptance, close readiness, or implementation readiness. The first runnable target is v0.1 Core Authority Smoke, with Kernel Smoke as a narrow future smoke-check authoring label. The first user-value target is v0.2 First User-Value Slice. v0.3 Agency Assurance Pack and v0.4 Operations & Handoff Pack are real named stages/packs in the stage taxonomy. Completing the owner-defined profiles for those stages after v0.2 reaches the glossary-defined hardened local reference target; the target itself is an umbrella target only, not an additional stage, fixture profile, or suite name. v1+ Expansion remains roadmap scope unless owner docs promote and prove an item.

## Read this when

- You are changing meaning in an English or Korean doc.
- You are reviewing English/Korean semantic parity.
- You need to decide whether Korean wording should preserve an English identifier or use natural Korean prose.

## Before you read

Read [Authoring Guide](authoring-guide.md) for owner boundaries, docs-maintenance checks, and the rule that strict contracts stay in Reference docs.

For rewrite triage during redesign, use [Rewrite Plan](rewrite-plan.md).

## Main idea

English docs define the reference meaning for the bilingual documentation set. Korean docs preserve that meaning, but they should not sound like line-by-line English copies.

The goal is semantic parity, not sentence-by-sentence translation. Korean should read like natural technical Korean while preserving official identifiers, exact contracts, code-like names, and stable product terms.

In user-facing Korean, prefer the natural public phrase first and add the exact Harness label only when the label helps precision. For example, use `범위`, `범위 밖`, `판단 요청`, `근거`, `확인`, `검증`, `수동 QA`, `작업 수락`, `잔여 위험`, `닫기 막힘`, and `다음 안전한 행동` in ordinary explanation. Add labels such as `Change Unit`, `Decision Packet`, `Write Authorization`, `Evidence Manifest`, `Residual Risk`, `Manual QA`, `detached verification`, or `Acceptance` in parentheses only when both the reader-friendly phrase and the Harness label matter. Reference Korean may preserve exact schema identifiers, enum values, field names, and API terms whenever precision matters.

## User-Facing Vocabulary Rule

Korean user-facing docs should prefer natural public terms: `작업`, `범위`, `범위 밖`, `판단`, `판단 요청`, `근거`, `확인`, `검증`, `수동 QA`, `작업 수락`, `잔여 위험`, `닫기 막힘`, `닫기 준비 상태`, and `다음 안전한 행동`. Stable English identifiers should be preserved mainly in reference docs, schema/API contexts, exact record names, code-like strings, anchors, and tables that intentionally teach implementation terms.

When a user-facing page needs an internal implementation term, explain the easy concept first and add the exact term in parentheses only when it clarifies a real boundary, blocker, source ref, or reference link. Avoid Korean sentences that are mostly English nouns joined by Korean particles.

- Do not start user examples with internal terms such as `Discovery`, `Change Unit`, `Decision Packet`, `Write Authorization`, `Evidence Manifest`, `Projection`, `Gate`, or `task_events`.
- Do not require users to say `Discovery`, `Change Unit`, or `Decision Packet` to get the behavior. Show ordinary examples such as `구현 전에 계획을 구체화해줘`, `범위가 커지면 먼저 알려줘`, and `무엇을 결정해야 하는지 보여줘`.
- Use `판단 요청` or a natural Korean question for user-facing decision prompts. Introduce `Decision Packet` only as an optional/internal label after the choice is clear.
- On first mention, add an English identifier in parentheses only when it helps the reader connect to a record, schema, API, or reference section. After that, use clear Korean where possible.
- Avoid awkward mixed phrases in user-facing docs. Prefer a full Korean sentence over English nouns joined by Korean particles.

## Keep exact

Keep these unchanged across English and Korean docs:

- API names
- schema names
- enum values
- DDL names
- code identifiers
- field names
- file names and path names
- stable identifiers
- error codes and validator IDs

Do not translate code, API method names, enum values, field names, file paths, stable identifiers, or other exact strings inside code blocks.

Keep these exact when they refer to literal identifiers, schema/API values, file/template names, heading anchors, or code-like references. In ordinary Korean prose, prefer the stable Korean terms in the [Bilingual Terminology Table](#bilingual-terminology-table).

- Task
- Change Unit
- Decision Packet
- Write Authorization
- Evidence Manifest
- Eval
- Gate
- ProjectionKind
- MCP
- Core
- state.sqlite
- task_events
- judgment_category
- judgment_route
- display_depth
- judgment_domain
- decision_kind
- decision_profile
- prepare_write
- record_run
- close_task

Do not translate markers such as `HARNESS:BEGIN`, schema names such as `ArtifactRef`, `ProjectionKind`, `decision_kind=approval`, `approval_gate`, `ResidualRiskSummary.status=none`, validator IDs, error codes, file paths, API/tool/schema names, or other exact strings.

Keep exact stage labels exact: `v0.1 Core Authority Smoke`, `Kernel Smoke`, `v0.2 First User-Value Slice`, `v0.3 Agency Assurance Pack`, `v0.4 Operations & Handoff Pack`, and `v1+ Expansion`. In Korean prose, prefer the natural Korean label first with the exact English label in parentheses, such as `코어 권한 스모크(v0.1 Core Authority Smoke)`, `첫 사용자 가치 조각(v0.2 First User-Value Slice)`, `에이전시 보증 팩(v0.3 Agency Assurance Pack)`, and `운영과 인계 팩(v0.4 Operations & Handoff Pack)`. `hardened local reference target` is not a stage label; use it only as the glossary-defined umbrella target reached after v0.2 by completing the owner-defined profiles for the named v0.3 Agency Assurance Pack and v0.4 Operations & Handoff Pack stages/packs, and never as an additional stage, fixture profile, or suite name. For the three-space model in Korean prose, use `제품 저장소`, `하네스 서버 소스 저장소` when referring to this repository's future source role, and `하네스 런타임 홈`; add the English labels in parentheses only when they help disambiguate the architecture term.

Reference headings that serve as lookup anchors should remain stable unless a dedicated link/anchor migration updates all links. User-facing prose should prefer natural Korean. A Korean alias line may provide the natural term under a stable reference heading.

## Bilingual Terminology Table

Use these as the preferred terms in Korean prose. Keep exact English strings where the sentence refers to an identifier or contract value, and add the English label in parentheses when it helps the reader.

| English term | Korean canonical term | Usage note |
|---|---|---|
| Harness | 하네스 | Use for the product name in ordinary Korean prose. Keep literal strings such as `HARNESS:BEGIN`. |
| Harness Server | 하네스 서버 | Use for the local Harness program/installation in the three-space model. Do not use this for the user's product repository or runtime data home. |
| Harness Server source repository | 하네스 서버 소스 저장소 | Use for this repository's intended future source-code role. Starting server/runtime implementation still requires documentation acceptance and a separate implementation-planning readiness decision. |
| Product Repository | 제품 저장소 | Use for the user's product workspace. Add the English label only when disambiguating the three-space model. |
| Harness Runtime Home | 하네스 런타임 홈 | Use for the per-user/per-installation operational data home. Add the English label only when helpful. |
| Core-owned state | Core가 소유한 상태 | Use when stressing that Core records are operational authority. In user-facing prose, `운영 기준 상태` may be clearer when the Core boundary is already known. |
| durable local state | 지속 로컬 상태 | First use may include `지속 로컬 상태(durable local state)`. |
| work | 작업 | Use in user-facing docs for the thing the user wants done, answered, investigated, or decided. Keep `work` exact for mode values or code-like references. |
| scope | 범위 | Use in user-facing docs for what may change and what is out of bounds. Add `Change Unit` only when naming the internal scoped-write record. |
| out of scope | 범위 밖 | Use for excluded behavior, files, decisions, or claims. Avoid `out-of-scope` as a mixed Korean adjective unless quoting an identifier. |
| Discovery | 요구사항 구체화 | Explain as the agent's requirements-clarification posture before implementation planning, not as a command name alone. Keep `Discovery` exact in reference/schema contexts. |
| Change Unit | 범위 / Change Unit | In user-facing prose, explain the scoped work boundary as `범위` first. Keep `Change Unit` exact when naming the record or reference term. |
| judgment | 판단 | Use for user-owned choices. Add `Decision Packet` only when naming the recorded implementation route. |
| judgment request | 판단 요청 | Use for the ordinary user-facing prompt. Prefer `무엇을 결정해야 하나요?` or another natural question when that reads better. |
| user-owned judgment | 사용자 소유 판단 | Use for the broad agency-preserving principle. Do not replace it globally with `사용자 결정`. |
| User Decisions display group | 사용자 결정 | Use only for the user-facing display group that lists pending decision items. Do not treat it as a schema field, gate, route, or authority path. |
| decision type | 결정 유형 | Use when labeling a concrete pending decision item. |
| Product/UX judgment | 제품/UX 판단 | Use for product behavior, copy, flow, and UX choices owned by the user. |
| technical architecture judgment | 기술 구조 판단 | Use for architecture, dependency, migration, interface, or material technical direction choices owned by the user. |
| security/privacy judgment | 보안/개인정보 판단 | Use for security, privacy, retention, redaction, audit, or exposure choices owned by the user. |
| scope/autonomy judgment | 범위/자율성 판단 | Use for scope expansion, work split, or Autonomy Boundary choices owned by the user. |
| QA waiver | QA 면제 판단 | Use for a user decision to waive a required QA path. Do not translate it with a generic `승인` term. |
| verification waiver | 검증 면제 판단 | Use for a user decision to waive a required verification path. Do not translate it with a generic `승인` term. |
| `judgment_category` | 판단 묶음 / 판단 유형 | Preserve the field name and enum values in schema/API/reference contexts. User-facing display should show labels such as `제품/UX 판단`, `기술 구조 판단`, `보안/개인정보 판단`, `QA/검증`, `작업 수락`, or `잔여 위험`. |
| `judgment_route` | 판단 경로 | Preserve the field name and enum values in schema/API/reference contexts. In user-facing display, use the actual verb: choose, defer, waive, accept the result, accept risk, approve a sensitive step, or reconcile. |
| `display_depth` | 표시 깊이 | Preserve the field name in schema/API/reference contexts. Do not make ordinary users learn it; render the practical result as a short question, a trade-off question, a high-risk question, or a close-affecting question. |
| `judgment_domain` | 판단 영역 | Compatibility alias for `judgment_category`. Preserve the field name in old schema/API/reference contexts. New examples should prefer `judgment_category` when exact fields are needed. |
| `decision_kind` | 결정 경로 | Compatibility alias for `judgment_route`. Preserve the field name and enum values in old schema/API/reference contexts. |
| `decision_profile` | 결정 표시 깊이 | Compatibility alias for `display_depth`. Preserve the field name in old schema/API/reference contexts. |
| Decision Packet | Decision Packet / 판단 요청 기록 | Keep `Decision Packet` in literal record/API/schema/anchor contexts when needed. In user-facing prose, use `판단 요청` before the exact label, or omit the label when it does not help. |
| Write Authorization | 쓰기 허가 기록 | Use in prose for the record or result of `prepare_write`. Keep exact API/tool names and fields. |
| evidence | 근거 | Use in user-facing prose for support behind a claim. Keep `Evidence`, `Evidence Manifest`, and schema fields exact when naming records or APIs. |
| Evidence Manifest | 근거 매니페스트 | Use the Korean phrase only when helpful in prose. Keep `Evidence Manifest` exact in record/template/schema contexts. |
| check | 확인 | Use for ordinary tests, diff review, inspection, or source lookup. Use `검증` only when the formal Verification path is intended. |
| Verification | 검증 | Use for recorded correctness checking. Use `확인` for ordinary checking only when the formal Verification concept is not meant. |
| Manual QA | 수동 QA | Keep `Manual QA` in exact template/schema/API contexts. |
| work acceptance / Acceptance | 작업 수락 | Use for the user's result-acceptance judgment when the task path requires it. Do not use it for sensitive-action permission. If source or legacy English says `final acceptance`, prefer `work acceptance` in English and `작업 수락` in Korean; explain finality in surrounding prose instead of using generic `최종 수락`. |
| Approval | 민감 동작 승인 | Use for the canonical Approval concept in public Korean. `허가` may explain permission in prose, but it is not a second canonical term. Do not use generic `승인` for work acceptance, product judgment, QA waiver, residual-risk acceptance, or Write Authorization. Keep `Approval` in reference/schema contexts. |
| Residual Risk | 잔여 위험 | Use this consistently in user-facing prose. Plain explanatory wording such as `남은 불확실성` is acceptable when not naming the product concept. |
| residual-risk acceptance | 잔여 위험 수용 | Use for the user's explicit acceptance of a named remaining risk. Keep it distinct from `작업 수락(Acceptance)`. |
| close readiness | 닫기 준비 상태 | Use consistently for the public summary of what remains before finish or close. Keep `Close Readiness` only when mirroring the English display-group label or exact docs heading. |
| close blocker | 닫기 막힘 | Use for a concrete reason close cannot proceed. API/reference contexts may keep `close blocker` or exact schema names. |
| next safe action | 다음 안전한 행동 | Use for the next action that can proceed without hiding unresolved judgment, scope, evidence, QA, verification, acceptance, or risk. |
| blocker | 막힘 | User-facing prose may use `막힘` for the thing preventing progress or close. API/reference contexts should keep `blocker`, or explain it as `차단 조건(blocker)` when clarity helps. Do not translate exact field names, template keys, enum-like values, or schema names such as `blockers` or `CloseBlockerCategory`. |
| ArtifactRef | `ArtifactRef` / 아티팩트 참조 | Keep the schema name exact. In prose, use `아티팩트 참조`; in evidence contexts, `근거 아티팩트 참조` is also acceptable. |
| artifact ref | 아티팩트 참조 | In evidence contexts, `근거 아티팩트 참조` is also acceptable. Keep the `ArtifactRef` schema name exact. |
| projection / Projection | 읽기용 요약 | Use `읽기용 요약(Projection)` or `읽기용 요약` for the first user-facing explanation. After that, prefer `읽기용 요약` unless exact API/schema precision needs `Projection`. Translate Markdown projection as `Markdown 읽기용 요약` or `Markdown으로 렌더링된 읽기용 요약`. Projection is not operational authority. Keep `Projection`, `ProjectionKind`, `projection freshness`, API fields, template kinds, or `projection view` in reference/schema contexts. |
| kernel | 커널 | Use `커널` outside exact headings and owner links. |
| gate | 관문 | Prefer `관문` in Learn/Use docs. Reference docs may retain `gate` when referring to kernel fields or values. |
| detached verification | 분리 검증 | May retain `detached verification` in assurance explanations. |
| cooperative | 협력형 | Retain the English label in guarantee-level tables. |
| detective | 탐지형 | Retain the English label in guarantee-level tables. |
| preventive | 예방형 | Retain the English label in guarantee-level tables. |
| isolated | 격리형 | Retain the English label in guarantee-level tables. |

## Translate naturally

Use the term that fits the sentence and reader context.

| English term | Korean guidance |
|---|---|
| context | Use `context` in identifier-like or AI-session phrasing. Use `맥락` in ordinary prose. |
| boundary | Keep `boundary` in code or identifier context. Use `경계` in prose. |
| authority | Use `권한` for operational authority. Use `기준 권한` when the sentence needs to stress the source of authority. |
| canonical | Keep `canonical` in identifier context. Use `기준` or `기준 기록` in Korean prose. |
| mutate | Prefer `change` or `modify` in English prose. Use `변경하다` in Korean. |
| surface | Choose the concrete meaning: `interface`, `view`, `entrypoint`, `display area`, or the Korean equivalent by context. User-facing Korean often wants `접점`, `화면`, or `표시 영역`. |
| evidence | Use `evidence` only when it is a product term. Use `근거` or `증거` in Korean prose. |
| acceptance / final acceptance | When this means the user's judgment that the result is acceptable, use `work acceptance` in English and prefer `작업 수락` in Korean. If source English emphasizes finality, explain that in surrounding prose instead of switching to generic `최종 수락`. |
| acceptance criteria | Use `수용 기준` for formal acceptance criteria. Use `완료 기준` when the sentence is about task completion rather than formal criteria. Do not use `수락 기준`. |
| residual-risk acceptance / accepted risk | Use `잔여 위험 수용` for the canonical route. In explanatory prose, `잔여 위험을 받아들이는 판단` or `잔여 위험을 받아들이다` is also acceptable. Keep exact enum/field names in schema/reference contexts. Do not translate this concept with generic `수락` phrasing. Keep it distinct from `작업 수락(Acceptance)`. |
| Acceptance Gate / acceptance_gate | Keep exact identifiers such as `Acceptance Gate` or `acceptance_gate` where needed. Explain the meaning in Korean prose instead of inventing a new unstable term. |
| residual risk | Use `잔여 위험` as the canonical term. Plain explanatory wording may describe the uncertainty, but keep terminology consistent. |
| approval / Approval | Use `민감 동작 승인` in user-facing prose for the sensitive-action permission concept. Use `Approval` when naming the canonical Harness status, gate, record, schema, or exact reference term. Generic `승인` must not mean work acceptance, product judgment, QA waiver, residual-risk acceptance, or Write Authorization. |
| write authority | Use `쓰기 권한` in ordinary prose. Use `쓰기 허가 기록(Write Authorization)` when naming the Harness record produced by `prepare_write`. |
| gate | In user-facing flow, prefer `관문`, `확인`, `닫기 확인`, or `막힘` by context. Reference docs may retain `gate` for kernel fields or strict contracts. |

Capitalization rule: `Approval` is the canonical Harness permission concept for sensitive actions. Lowercase `approval` may remain only in stable identifiers, enum values, schema names, intentionally fixed phrases, or quoted legacy/user wording, such as `approval_gate`, `decision_kind=approval`, `approval_request_candidate`, `approval_scope`, `approval-shaped`, and approval drift.

## Avoid examples

Avoid mixed-language phrases that preserve English technical words without helping the reader:

- `state를 mutate한다`
- `authority boundary를 preserve한다`
- `surface에 expose한다`
- `projection freshness를 report한다`
- `acceptance를 complete한다`
- `risk를 accept한다`
- `acceptance criteria를 수락 기준으로 쓴다`
- `Harness 상태를 local state와 artifact ref에 둔다`
- `detached verification을 독립 검증이라고만 쓴다`

## Prefer examples

Prefer natural phrases that preserve the technical meaning:

- `상태를 변경한다`
- `권한 경계를 유지한다`
- `화면에 보여준다`
- `projection이 최신인지 표시한다`
- `결과를 수락한다`
- `잔여 위험을 받아들인다`
- `수용 기준을 확인한다`
- `하네스 상태를 지속 로컬 상태와 아티팩트 참조에 둔다`
- `분리 검증(detached verification)을 기록한다`

## Before / After examples

Use patterns like these when polishing Korean prose:

| Before | After |
|---|---|
| `Core가 state 변경을 수행한다.` | `Core가 상태를 변경한다.` |
| `Agent는 authority boundary 유지가 필요하다.` | `Agent는 권한 경계를 유지해야 한다.` |
| `이 surface는 blocker 표시를 담당한다.` | `이 화면은 blocker를 보여준다.` |
| `Operations는 projection freshness를 report한다.` | `Operations는 projection이 최신인지 표시한다.` |
| `canonical source를 update한다.` | `기준 기록을 업데이트한다.` |
| `context를 잃지 않도록 한다.` | `맥락을 잃지 않도록 한다.` |
| `acceptance가 필요하다.` | `작업 수락이 필요하다.` |
| `risk를 accept한다.` | `잔여 위험을 받아들인다.` |
| `acceptance criteria를 수락 기준으로 쓴다.` | `수용 기준` 또는 문맥에 따라 `완료 기준`을 쓴다. |
| `residual-risk acceptance를 작업 수락처럼 쓴다.` | `잔여 위험 수용`처럼 작업 수락과 구분한다. |
| `acceptance_gate를 수락 게이트로 새로 번역한다.` | `acceptance_gate`를 유지하고 한국어 문장으로 의미를 설명한다. |
| `surface capability를 확인한다.` | `접점이 실제로 할 수 있는 일을 확인한다.` |
| `Harness 상태는 local state와 artifact ref에 있다.` | `하네스 상태는 지속 로컬 상태와 아티팩트 참조에 있다.` |
| `detached verification을 독립 검증으로 표시한다.` | `분리 검증(detached verification)으로 표시한다.` |

## Korean heading policy

Korean headings should be natural Korean headings, not mechanical English-heading copies.

Keep official identifiers exact inside headings when the heading is about that identifier. Otherwise, choose the heading a Korean technical reader would expect.

Heading order and document meaning should remain aligned with the English document. Heading text does not need to match word for word.

## Bilingual review checklist

```text
[ ] Does the Korean page preserve the same meaning as the English page?
[ ] Does the paired file preserve the same active file path, reader purpose, semantic section coverage, owner links, and contractual detail?
[ ] Does the Korean prose read naturally to a Korean technical reader?
[ ] Are API names, schema names, enum values, DDL names, identifiers, paths, error codes, and validator IDs exact?
[ ] Are Korean canonical terms consistent in ordinary prose, while exact identifiers remain exact?
[ ] Are source-of-truth phrases and owner links aligned with the owner Reference docs?
[ ] Are non-owner duplicate contracts summarized with owner links instead of translated as full contract copies?
[ ] Are mixed-language phrases replaced with natural Korean where possible?
[ ] In user-facing docs, do natural Korean phrases appear before Harness labels when both are needed?
[ ] Are headings idiomatic while preserving the same document structure and scope?
[ ] Were English and Korean link changes made in the same batch?
[ ] Does the review avoid treating translation drift as runtime state, evidence, QA, work acceptance, close readiness, or implementation readiness?
```
