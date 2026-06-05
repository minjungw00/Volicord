# Translation Guide

## What this document helps you do

Use this guide when editing English and Korean Harness documentation together.

This is maintenance documentation for bilingual documentation. It does not authorize runtime/server implementation, generated operational files, executable fixtures, runtime data, or product state changes before documentation acceptance and a separate implementation-planning readiness decision, and it does not define conformance pass/fail, evidence, QA, final acceptance, close readiness, or implementation readiness. The first future implementation target is Engineering Checkpoint, an internal authority-loop smoke that is not a product MVP. Kernel Smoke is only a narrow future smoke-check authoring label under that checkpoint. The first user-value target is MVP-1 User Work Loop. Assurance Profile and Operations Profile harden assurance and operations later. Completing those profiles after MVP-1 reaches the glossary-defined hardened local reference target; the target itself is an umbrella target only, not an additional stage, fixture profile, or suite name. Roadmap remains future scope unless owner docs promote and prove an item.

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

In user-facing Korean, prefer the natural public concept first: `작업`, `범위` or `작업 조각`, `판단` or `결정할 것`, `근거`, `확인` or `검증`, and `마무리` or `닫기`. More specific phrases such as `요구사항 구체화`, `쓰기 전 범위 확인`, `판단 요청`, `판단 요약`, `근거 목록`, `상태 보기`, `요약`, `상태 카드`, `수동 QA`, `최종 수락`, `잔여 위험` or `남은 위험`, `닫기 가능 여부`, `닫기 준비 상태`, `닫기 막힘`, and `다음 안전한 행동` may appear when they support those concepts. Add labels such as `Discovery`, `Change Unit`, `Decision Packet`, `Write Authorization`, `Evidence Manifest`, `Projection`, `Residual Risk`, `Manual QA`, `detached verification`, or `Acceptance` in parentheses only when both the reader-friendly phrase and the Harness label matter. Reference Korean may preserve exact schema identifiers, enum values, field names, API terms, and stable product labels whenever precision matters.

## User-Facing Vocabulary Rule

Korean user-facing docs should primarily use natural public concepts: `작업`, `범위` or `작업 조각`, `판단` or `결정할 것`, `근거`, `확인` or `검증`, `마무리` or `닫기`. Supporting phrases such as `요구사항 구체화`, `범위 밖`, `쓰기 전 범위 확인`, `판단 요청`, `판단 요약`, `근거 목록`, `상태 보기`, `요약`, `상태 카드`, `수동 QA`, `최종 수락`, `잔여 위험`, `남은 위험`, `닫기 가능 여부`, `닫기 준비 상태`, `닫기 막힘`, and `다음 안전한 행동` are useful when they explain those concepts, but they should not become a larger required concept model for new users. Stable English identifiers should be preserved mainly in reference docs, schema/API contexts, exact record names, code-like strings, anchors, and tables that intentionally teach implementation terms.

When a user-facing page needs an internal implementation term, explain the Korean concept first and add the exact English label in parentheses only when it clarifies a real boundary, blocker, source ref, or reference link. Avoid Korean sentences that are mostly English nouns joined by Korean particles. User examples should start with ordinary user language, not record labels or procedure names.

- Do not start user examples with internal terms such as `Discovery`, `Change Unit`, `Decision Packet`, `Write Authorization`, `Evidence Manifest`, `Projection`, `Gate`, or `task_events`.
- Do not require users to say `Discovery`, `Change Unit`, or `Decision Packet` to get the behavior. Show ordinary examples such as `구현 전에 계획을 구체화해줘`, `내가 결정해야 할 것과 네가 확인할 수 있는 것을 나눠서 보여줘`, and `작업 범위가 커지면 먼저 알려줘`.
- Use `판단 요청` or a natural Korean question for user-facing decision prompts. Introduce `Decision Packet` only as an optional/internal label after the choice is clear.
- Use `근거 목록` for the user-facing idea of a detailed evidence list. Introduce `Evidence Manifest` only when naming the record, template, API shape, or reference owner.
- Use `상태 보기`, `요약`, or `상태 카드` for user-facing derived views. Introduce `Projection` only when naming the exact projection system, API fields, template kinds, or owner reference.
- On first mention, add an English identifier in parentheses only when it helps the reader connect to a record, schema, API, or reference section. After that, use clear Korean where possible.
- Avoid awkward mixed phrases in user-facing docs. Prefer a full Korean sentence over English nouns joined by Korean particles, and rewrite examples until they sound natural to Korean technical readers.

## Keep exact

Keep these unchanged across English and Korean docs:

- API names
- method names
- schema names
- enum values
- DDL and table names
- code identifiers
- field names
- file names and path names
- literal markers
- stable identifiers
- stable product identifiers
- error codes and validator IDs

Do not translate code, method names, API method names, enum values, field names, DDL/table names, file paths, literal markers, stable identifiers, stable product identifiers, or other exact strings inside code blocks.

Keep these exact when they refer to literal identifiers, schema/API values, file/template names, heading anchors, or code-like references. In ordinary Korean prose, prefer the stable Korean terms in the [Bilingual Terminology Table](#bilingual-terminology-table).

- Task
- Discovery
- Change Unit
- Decision Packet
- Write Authorization
- Evidence Manifest
- Projection
- Close Readiness
- Residual Risk
- Eval
- Gate
- ProjectionKind
- MCP
- Core
- state.sqlite
- task_events
- user_judgment
- UserJudgment
- judgment_kind
- product_decision
- technical_decision
- scope_decision
- sensitive_approval
- qa_waiver
- verification_risk_acceptance
- final_acceptance
- residual_risk_acceptance
- cancellation
- presentation
- display_label
- request_user_judgment
- record_user_judgment
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

Use the active delivery labels consistently: `Engineering Checkpoint`, `MVP-1 User Work Loop`, `Assurance Profile`, `Operations Profile`, and `Roadmap`. `Kernel Smoke` is not a stage; keep it only as the narrow future smoke-check authoring label under Engineering Checkpoint.

In Korean prose, prefer the natural Korean labels `내부 엔지니어링 점검`, `MVP-1 사용자 작업 루프`, `보증 프로필`, `운영 프로필`, and `로드맵`. Add the active English label in parentheses only when a reference heading, table, or implementation lookup needs it. Historical legacy English labels such as `v0.1 Core Authority Smoke`, `v0.2 First User-Value Slice`, `v0.3 Agency Assurance Pack`, `v0.4 Operations & Handoff Pack`, and `v1+ Expansion` are not current stage names and may appear only as legacy aliases, after the Korean explanation or in an explicit alias table.

`hardened local reference target` is not a stage label; use it only as the glossary-defined umbrella target reached after MVP-1 by completing the owner-defined Assurance Profile and Operations Profile work, and never as an additional stage, fixture profile, or suite name. For the three-space model in Korean prose, use `제품 저장소`, `하네스 서버 소스 저장소` when referring to this repository's future source role, and `하네스 런타임 홈`; add the English labels in parentheses only when they help disambiguate the architecture term.

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
| scope | 범위 | Use in user-facing docs for what may change and what is out of bounds. Use `작업 조각` when a small scoped piece of work is clearer. Add `Change Unit` only when naming the internal scoped-work record. |
| out of scope | 범위 밖 | Use for excluded behavior, files, decisions, or claims. Avoid `out-of-scope` as a mixed Korean adjective unless quoting an identifier. |
| Discovery | 요구사항 구체화 | Explain as requirement clarification before implementation planning, not as a command name alone. Keep `Discovery` exact in reference/schema contexts. |
| Change Unit | 범위 / 작업 조각 | In user-facing prose, explain the scoped work boundary as `범위` or a small `작업 조각` first. Keep `Change Unit` exact only when naming the internal scoped-work record or reference term. |
| judgment | 판단 | Use for user-owned choices. Add `Decision Packet` only when naming optional full-format presentation or legacy/compatibility material. |
| judgment request | 판단 요청 | Use for the ordinary user-facing prompt. Prefer `무엇을 결정해야 하나요?` or another natural question when that reads better. |
| user-owned judgment | 사용자 소유 판단 | Use for the broad agency-preserving principle. Do not replace it globally with `사용자 결정`. |
| Product decision | 제품 판단 | Use for product behavior, copy, flow, and UX choices owned by the user. |
| Technical decision | 기술 판단 | Use for architecture, dependency, migration, interface, security/privacy, QA/verification expectation, or other material technical direction choices owned by the user. |
| Scope decision | 범위 판단 | Use for scope expansion, non-goal removal, Change Unit boundary, or Autonomy Boundary choices owned by the user. |
| Sensitive action approval | 민감 동작 승인 | Use for scoped permission for a named sensitive step. Keep distinct from product decision, technical decision, scope decision, final acceptance, and residual-risk acceptance. |
| QA waiver | QA 면제 판단 | Use only for a scoped QA waiver where policy allows it. It is not QA evidence or a passed QA result. |
| Verification risk acceptance | 검증 위험 수락 | Use when the user accepts the risk of missing or waived verification. It does not create detached verification. |
| Final acceptance | 최종 수락 | Use for the user's result judgment when the work path requires acceptance. |
| Residual risk acceptance | 잔여 위험 수락 | Use for explicit acceptance of a named visible remaining risk. |
| Cancellation | 취소 판단 | Use when the user decides to stop the task without a successful result. |
| `user_judgment` | 사용자 판단 기록 | Canonical record family. Preserve the identifier in schema/API/reference contexts. |
| `UserJudgment` | UserJudgment | Canonical schema shape. Preserve exact. |
| `judgment_kind` | 판단 종류 | Canonical compact kind field. Preserve field name and enum values in schema/API/reference contexts. Values are `product_decision`, `technical_decision`, `scope_decision`, `sensitive_approval`, `qa_waiver`, `verification_risk_acceptance`, `final_acceptance`, `residual_risk_acceptance`, and `cancellation`. |
| `presentation` | 표시 형식 | Canonical prompt/detail field. Use `short` for compact prompts and `full` for full-format Decision Packet presentation. Preserve exact in schema/API contexts. |
| `display_label` | 표시 라벨 | User-facing label field. Allowed labels include `제품 판단`, `기술 판단`, `범위 판단`, `민감 동작 승인`, `QA 면제 판단`, `검증 위험 수락`, `최종 수락`, `잔여 위험 수락`, and `취소 판단`. |
| `judgment_category`, `judgment_route`, `display_depth` | legacy 판단 field | Legacy or implementation routing terms from older Decision Packet drafts. Preserve exact only in old schema/API/reference compatibility contexts. New examples should prefer `judgment_kind`, `presentation`, and `display_label`. |
| `judgment_domain`, `decision_kind`, `decision_profile` | legacy 판단 alias | Compatibility aliases for older request shapes. Preserve exact only in old payloads or migration notes. |
| Decision Packet | 판단 요청 / 판단 요약 | Treat `Decision Packet` as the full judgment presentation label. Keep it when naming optional full-format presentation, legacy refs, template files, anchors, or migration notes. In user-facing prose, use `판단 요청` or `판단 요약` first, or omit the label when it does not help. |
| pre-write scope check | 쓰기 전 범위 확인 | Preferred user-facing phrase for the check before a product write. Use this before internal labels such as `Write Authorization`. |
| Write Authorization | 쓰기 전 범위 확인 / 쓰기 허가 기록 | In user-facing prose, prefer `쓰기 전 범위 확인`. Use `쓰기 허가 기록(Write Authorization)` only when naming the internal cooperative Harness record or result of `prepare_write`. Keep exact API/tool names and fields. Explain that it is not OS permission, sandboxing, tamper-proof enforcement, preventive blocking, or isolation. |
| evidence | 근거 | Use in user-facing prose for support behind a claim. Keep `Evidence`, `Evidence Manifest`, and schema fields exact when naming records or APIs. |
| Evidence Manifest | 근거 목록 | Use for a detailed evidence list in user-facing prose. Keep `Evidence Manifest` exact in record/template/schema/API contexts. |
| check | 확인 | Use for ordinary tests, diff review, inspection, or source lookup. Use `검증` only when the formal Verification path is intended. |
| Verification | 검증 | Use for recorded correctness checking. Use `확인` for ordinary checking only when the formal Verification concept is not meant. |
| Manual QA | 수동 QA | Keep `Manual QA` in exact template/schema/API contexts. |
| final acceptance / Acceptance | 최종 수락 | Use for the user's result-acceptance judgment when the task path requires it. Do not use it for sensitive-action permission. In schema/API contexts preserve `final_acceptance`; in Korean prose use `최종 수락`. |
| Approval | 민감 동작 승인 | Use for the canonical Approval concept in public Korean. `허가` may explain permission in prose, but it is not a second canonical term. Do not use generic `승인` for final acceptance, product decision, QA waiver, residual-risk acceptance, or Write Authorization. Keep `Approval` in reference/schema contexts. |
| Residual Risk | 잔여 위험 / 남은 위험 | Use `잔여 위험` consistently when naming the product concept. `남은 위험` or explanatory wording such as `남은 불확실성` is acceptable when plain prose reads better. |
| residual-risk acceptance | 잔여 위험 수락 | Use for the user's explicit acceptance of a named remaining risk. Keep it distinct from `최종 수락(Acceptance)`. |
| close / Close | 마무리 / 닫기 | Use as the plain concept for whether work can honestly finish. `마무리` often reads natural in user requests; `닫기` is useful when matching Harness close status or close blockers. Keep exact identifiers such as `close_task`. |
| close readiness | 닫기 가능 여부 / 닫기 준비 상태 | Use for the public summary of whether close can proceed and what remains. Keep `Close Readiness` only when mirroring the English display-group label or exact docs heading. |
| close blocker | 닫기 막힘 | Use for a concrete reason close cannot proceed. API/reference contexts may keep `close blocker` or exact schema names. |
| next safe action | 다음 안전한 행동 | Use for the next action that can proceed without hiding unresolved judgment, scope, evidence, QA, verification, final acceptance, or risk. |
| blocker | 막힘 | User-facing prose may use `막힘` for the thing preventing progress or close. API/reference contexts should keep `blocker`, or explain it as `차단 조건(blocker)` when clarity helps. Do not translate exact field names, template keys, enum-like values, or schema names such as `blockers` or `CloseBlockerCategory`. |
| ArtifactRef | `ArtifactRef` / 아티팩트 참조 | Keep the schema name exact. In prose, use `아티팩트 참조`; in evidence contexts, `근거 아티팩트 참조` is also acceptable. |
| artifact ref | 아티팩트 참조 | In evidence contexts, `근거 아티팩트 참조` is also acceptable. Keep the `ArtifactRef` schema name exact. |
| projection / Projection | 상태 보기 / 요약 / 상태 카드 | User-facing explanation should use `상태 보기`, `요약`, or `상태 카드` first and omit `Projection` unless exact API/schema precision or a reference link needs it. Translate Markdown projection as `Markdown 상태 보기`, `Markdown 요약`, or `Markdown으로 렌더링된 상태 카드` when the exact English phrase is not itself the subject. Projection is a derived view and not operational authority. Keep `Projection`, `ProjectionKind`, `projection freshness`, API fields, template kinds, or `projection view` in reference/schema contexts. |
| kernel | 커널 | Use `커널` outside exact headings and owner links. |
| gate | 관문 | Prefer `관문` in Learn/Use docs. Reference docs may retain `gate` when referring to kernel fields or values. |
| detached verification | 분리 검증 | May retain `detached verification` in assurance explanations. |
| cooperative | 협력형 / 협력형 확인 | Retain the English label in guarantee-level tables. In explanatory prose, `협력형 확인` is preferred. |
| detective | 탐지형 / 사후 확인 | Retain the English label in guarantee-level tables. In explanatory prose, `사후 확인` is preferred. |
| preventive | 사전 차단 | Retain the English label in guarantee-level tables. MVP-1 wording should say `사전 차단 아님` when clarifying non-claims. |
| isolated | 격리형 / 격리 경계 | Retain the English label in guarantee-level tables. MVP-1 wording should say `권한 격리 아님` when clarifying non-claims. |

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
| evidence manifest / detailed evidence list | Use `근거 목록` in user-facing prose. Use `Evidence Manifest` only for the internal record, template, schema/API context, or owner reference. |
| acceptance / final acceptance | When this means the user's judgment that the result is acceptable, use `final acceptance` in English and `최종 수락` in Korean. Preserve `final_acceptance` in schema/API contexts. |
| acceptance criteria | Use `수용 기준` for formal acceptance criteria. Use `완료 기준` when the sentence is about task completion rather than formal criteria. Do not use `수락 기준`. |
| residual-risk acceptance / accepted risk | Use `잔여 위험 수락` for the canonical route. In explanatory prose, use `잔여 위험을 수락하는 판단` or `잔여 위험을 수락하다`. Keep exact enum/field names in schema/reference contexts. Do not translate this concept with generic `승인` phrasing. Keep it distinct from `최종 수락(Acceptance)`. |
| Acceptance Gate / acceptance_gate | Keep exact identifiers such as `Acceptance Gate` or `acceptance_gate` where needed. Explain the meaning in Korean prose instead of inventing a new unstable term. |
| residual risk | Use `잔여 위험` as the canonical term. Plain explanatory wording may describe the uncertainty, but keep terminology consistent. |
| approval / Approval | Use `민감 동작 승인` in user-facing prose for the sensitive-action permission concept. Use `Approval` when naming the canonical Harness status, gate, record, schema, or exact reference term. Generic `승인` must not mean final acceptance, product decision, QA waiver, residual-risk acceptance, or Write Authorization. |
| write authority | In user-facing prose, prefer `쓰기 전 범위 확인`. Use `쓰기 허가 기록(Write Authorization)` only when naming the Harness record produced by `prepare_write`. Do not imply OS-level permission, sandboxing, or tamper-proof enforcement. |
| projection / derived view | In user-facing prose, choose the visible shape: `상태 보기`, `요약`, or `상태 카드`. Keep `Projection`, `ProjectionKind`, and projection-related field names exact in reference/schema contexts. |
| sandbox | Use `샌드박스` or `격리 환경` only when an exact mechanism is being named. MVP-1 should say it is not a sandbox or permission-isolated boundary. |
| preventive control | Use `사전 차단 통제` or `사전 차단 장치`; for MVP-1 non-claims, prefer `사전 차단 아님`. |
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
- `잔여 위험을 수락한다`
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
| `acceptance가 필요하다.` | `최종 수락이 필요하다.` |
| `risk를 accept한다.` | `잔여 위험을 수락한다.` |
| `acceptance criteria를 수락 기준으로 쓴다.` | `수용 기준` 또는 문맥에 따라 `완료 기준`을 쓴다. |
| `residual-risk acceptance를 최종 수락처럼 쓴다.` | `잔여 위험 수락`처럼 최종 수락과 구분한다. |
| `acceptance_gate를 수락 게이트로 새로 번역한다.` | `acceptance_gate`를 유지하고 한국어 문장으로 의미를 설명한다. |
| `surface capability를 확인한다.` | `접점이 실제로 할 수 있는 일을 확인한다.` |
| `Harness 상태는 local state와 artifact ref에 있다.` | `하네스 상태는 지속 로컬 상태와 아티팩트 참조에 있다.` |
| `detached verification을 독립 검증으로 표시한다.` | `분리 검증(detached verification)으로 표시한다.` |

## Korean heading policy

Korean headings should be natural Korean headings, not mechanical English-heading copies.

Keep official identifiers exact inside headings when the heading is about that identifier. Otherwise, choose the heading a Korean technical reader would expect.

Heading order and document meaning should remain aligned with the English document. Heading text does not need to match word for word.

Translation drift checks are documentation-quality checks only. They may reveal enum, API, owner-boundary, source-of-truth, or terminology drift in the docs, but they do not prove Core/API/storage/surface behavior, runtime conformance, close readiness, manual acceptance, or implementation readiness.

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
[ ] Does the review avoid treating translation drift as runtime state, evidence, QA, final acceptance, close readiness, manual acceptance, runtime conformance, or implementation readiness?
```
