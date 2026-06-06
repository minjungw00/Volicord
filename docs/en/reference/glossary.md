# Glossary Reference

## What this document helps you do

Use this glossary to confirm official Harness terms, capitalization, record names, and non-substitution boundaries while reading other docs.

This is reference documentation for future Harness behavior. Current repository phase and implementation handoff status are tracked in [MVP Plan](../build/mvp-plan.md#documentation-acceptance-status).

## Read this when

Read this when you need to check a Harness term, avoid mixing authority paths, or find the reference owner for exact behavior.

## Before you read

For a first explanation of Harness concepts, read [Start](../start.md). For exact behavior, follow the owner links below or the links inside individual definitions.

## Main idea

The glossary is a lookup aid and owner map. It keeps public terms, internal implementation terms, capitalization, and short non-substitution reminders consistent, but it is not a substitute for the owner reference documents.

## Reference scope

This glossary owns official term wording, capitalization reminders, record-name orientation, and owner routing. It does not own kernel behavior, public MCP schemas, storage DDL, projection rules, template bodies, connector capability profiles, or conformance fixture semantics.

## Public Terms

Use these six concepts first in user-facing docs, prompts, and status summaries. They are intentionally plain so users can work with Harness without learning record names.

| Public term | Plain meaning |
|---|---|
| work / task | The thing the user wants completed, answered, investigated, or decided. Use `Task` only when naming the internal record. |
| scope | What may change, what is out of scope, and where the agent should stop before continuing. For small pieces of scoped work, user-facing Korean may say `작업 조각`. |
| judgment / thing to decide | A user-owned choice. User-facing displays should use a focused label such as Product decision, Technical decision, Scope decision, Sensitive action approval, QA waiver, Verification risk acceptance, Final acceptance, Residual risk acceptance, or Cancellation. |
| evidence | Durable support for a claim about the work, such as changed paths, diffs, logs, test output, screenshots, inspection notes, or artifact refs. |
| check / verification | An ordinary confirmation such as a test, diff review, inspection, or source lookup; use `Verification` only for the formal recorded correctness-check path. Manual QA is a human check when the surface needs human judgment. |
| close | What still has to be true before the work can finish or close, including blockers, required final acceptance, next safe action, and remaining risk when they matter. |

User-facing docs should explain the plain concept first. More specific phrases such as requirement clarification, judgment request, judgment summary, evidence reference, evidence list, status view, status card, Manual QA, final acceptance, residual risk, close blocker, close readiness, and next safe action may appear when useful, but they should support one of the six concepts rather than becoming a required concept model. Add exact Harness labels in parentheses only when they help explain a boundary, blocker, source ref, or reference link.

Korean user-facing prose should usually use `요구사항 구체화` for Discovery, `범위` or `작업 조각` for Change Unit, `판단 요청` or `판단 요약` for Decision Packet, `쓰기 전 범위 확인` for Write Authorization, `상태 보기`, `요약`, or `상태 카드` for Projection, `증거 목록` for Evidence Manifest, `닫기 가능 여부` or `닫기 준비 상태` for Close Readiness, and `잔여 위험` or `남은 위험` for Residual Risk. Use the exact English label only when a record, schema, API, file path, heading, stable product identifier, or owner link needs that precision.

## User-Facing Term Rules

- Do not start user examples with internal terms. Start with work, scope, judgment or thing to decide, evidence, check or verification, or close.
- Do not require the user to say "Discovery," "Change Unit," "Decision Packet," "Write Authorization," "Evidence Manifest," "Projection," "Gate," or `task_events`.
- Use "judgment request" in English user-facing docs for the ordinary interaction. In Korean user-facing docs, use natural Korean such as `판단 요청`, `무엇을 결정해야 하나요?`, or another sentence that fits the page.
- Introduce internal labels only as optional or internal explanations, and only after the plain meaning is clear.
- Korean prose should put the Korean concept first, use exact English labels only when needed, avoid sentences made mostly of English nouns plus Korean particles, and sound natural to Korean technical readers.
- Broad phrases such as "approve," "yes, do it," "go ahead," "proceed," or "looks good" must not be stretched across unrelated product decision, technical decision, scope decision, sensitive approval, QA waiver, verification-risk acceptance, final acceptance, residual-risk acceptance, cancellation, or write authority.

## Internal / Reference Terms

These are implementation labels used by references, APIs, schemas, records, and status refs. Users do not need to use these terms in prompts; agents should translate ordinary requests into the right Harness procedure.

| Internal term | Plain-language explanation |
|---|---|
| Task | The durable internal unit for the work the user wants completed, answered, investigated, or decided. Use plain "work" for first-read user prose. |
| Discovery | The internal name for requirement clarification before implementation planning or write authority. Users can ask for this as "help me clarify the plan before implementation"; Korean user prose should say `요구사항 구체화`. |
| Change Unit | The internal scoped work unit for product writes. It says what may change but does not authorize a write by itself. User-facing docs should explain the scope or work piece before naming the record. |
| User Judgment | The canonical recorded path for a specific user-owned judgment that blocks progress, write, final acceptance, risk handling, or close. Public refs use `record_kind=user_judgment`. |
| Decision Packet | A full judgment presentation for a complex `user_judgment`, plus a legacy label in older refs. It is not the default user-facing mechanism and not a separate authority family. |
| Write Authorization | The internal cooperative Harness record produced only by non-dry-run `prepare_write.decision=allowed` for one stored `AuthorizedAttemptScope`. That scope is the operation/path/tool/command/class/product-write/network/secret/sensitive-category/baseline/Task/Change Unit/state/surface/judgment/guarantee boundary Core later compares during `record_run`. Its lifecycle status is `active`, `consumed`, `expired`, `stale`, or `revoked`; `allowed` and `blocked` are prepare-write decisions, not durable lifecycle statuses. It is not OS permission, sandboxing, tamper-proof enforcement, preventive blocking, or isolation. |
| Evidence Manifest | A detailed evidence-list record mapping completion conditions or acceptance criteria to supporting evidence refs. |
| Eval | A verification result record. It records the target, verdict, checks performed, evidence reviewed, independence qualifier, freshness, blockers, and artifact refs. |
| Projection | A derived view rendered from Harness state, such as a report, status view, summary, or Journey Card. It displays state but does not replace it. |
| Gate | A kernel readiness or compatibility condition. User-facing docs should usually show the blocker or check in plain language before naming a gate. |
| Autonomy Boundary | The choices the agent may make inside the active scope without asking the user again. |
| `task_events` | The internal event log table for task state changes. It is a reference/schema term, not user-facing vocabulary. |

## Schema/API Identifiers

Keep these exact in schemas, API docs, code-like examples, records, DDL/table contexts, method names, field names, enum values, file paths, literal markers, stable product identifiers, and diagnostic output. User-facing explanations should translate their meaning into ordinary language.

| Identifier | Meaning and display guidance |
|---|---|
| `user_judgment` | Canonical public record family for user-owned judgments, including MVP-1 Sensitive action approval judgments. |
| `UserJudgment` | Canonical schema shape for a user judgment. Keep exact in schema/API contexts. |
| `judgment_kind` | Canonical compact internal judgment kind. Values are `product_decision`, `technical_decision`, `scope_decision`, `sensitive_approval`, `qa_waiver`, `verification_risk_acceptance`, `final_acceptance`, `residual_risk_acceptance`, and `cancellation`. |
| `presentation` | Canonical prompt depth field. Use `short` for compact prompts and `full` for full-format Decision Packet presentation. |
| `display_label` | Compatibility or response-only user-facing judgment label when a surface exposes that name. It is not an active canonical schema/storage field; renderers derive labels from `judgment_kind` and locale. |
| `request_user_decision`, `record_user_decision` | Compatibility aliases for `request_user_judgment` and `record_user_judgment`. Preserve only in compatibility docs or migration notes. |
| `judgment_type`, `judgment_domain`, `decision_kind`, `decision_profile` | Compatibility aliases mapped to `judgment_kind`, route-specific payload validation, and `presentation`. Preserve only in compatibility docs or old payloads. |
| `judgment_category`, `judgment_route`, `display_depth` | Legacy or implementation routing terms from older Decision Packet drafts. Do not use as primary public concepts in new docs, examples, or fixtures. |
| `Task`, `UserJudgment`, `ArtifactRef`, `ProjectionKind`, `ValidatorResult` | Schema or API shape names. Keep exact when naming contracts. |
| `prepare_write`, `record_run`, `close_task`, `harness.request_user_judgment`, `harness.record_user_judgment` | Tool/API identifiers. Keep exact and explain their user-visible result in plain language. |

## Future / Later-Profile Terms

These labels may appear in roadmap, reference, template, or diagnostic material. They should not be first-read user vocabulary or required commands unless an owner profile promotes the feature.

| Later-profile term | Status and display guidance |
|---|---|
| Context Index | Later read-only retrieval support. It can suggest sources to inspect but does not authorize writes, satisfy gates, accept residual risk, or close work. |
| Journey Card / Journey Spine | Later continuity display. It helps orientation when enabled and fresh, but it is not Core-owned state. |
| Browser QA Capture | Roadmap candidate capture support for browser artifacts. It is not Manual QA, final acceptance, or detached verification by itself. |
| Standalone `DEC` projection | Optional full-format Decision Packet Markdown rendering when enabled. User judgment visibility does not depend on users reading standalone `DEC` files. |
| Operations Profile displays | Later or profile-gated operational/reporting surfaces. They display or export owner records; they do not replace Core authority. |

## Delivery Labels

Use the active delivery labels consistently.

| Label | Status |
|---|---|
| Engineering Checkpoint | Internal authority-loop smoke. It is not the product MVP and not the first user-value slice. |
| MVP-1 User Work Loop | First narrow user-value milestone. |
| Assurance Profile | Later hardening for assurance behavior. |
| Operations Profile | Later hardening for operations and handoff behavior. |
| Roadmap | Future scope unless owner docs promote and prove an item. |

`Kernel Smoke` is not a stage. Keep it only as the narrow future smoke-check authoring label under Engineering Checkpoint. Historical legacy labels such as `v0.1 Core Authority Smoke`, `v0.2 First User-Value Slice`, `v0.3 Agency Assurance Pack`, `v0.4 Operations & Handoff Pack`, and `v1+ Expansion` are not current stage names and may appear only as legacy aliases.

## Owner map

| Term family | Reference owner |
|---|---|
| Task, Change Unit, gates, close, sensitive-action approval, final acceptance, verification, QA, residual risk, write authority | [Core Model Reference](core-model.md) |
| MCP resources, MCP tools, public schemas, errors, `ValidatorResult`, `ProjectionKind` | [MVP API](api/mvp-api.md), [API Schema Core](api/schema-core.md), [API Errors](api/errors.md), and [API Schema Later](../later/index.md#later-schema-candidates) |
| SQLite records, artifact layout, enum hardening, `tree_hash`, `request_hash` storage use | [Storage](storage.md) |
| Derived views / Projections, managed blocks, projection freshness, Markdown reports, template bodies | [Projection And Templates Reference](projection-and-templates.md); [Template Reference](templates/README.md) |
| Discovery and Shared Design, design quality, stewardship, Feedback Loop finding routing, context hygiene, severity composition, policy contracts | [Design Quality Policies](design-quality-policies.md) |
| Surface capability, guarantee display, connector behavior | [Agent Integration Reference](agent-integration.md) |
| Security assets, trust boundaries, threat categories, high-risk control expectations, guarantee-level meanings | [Security Reference](security.md) |
| Operator procedures, conformance run overview, docs-maintenance reporting | [Operations And Conformance Reference](operations-and-conformance.md) |
| Core conformance model, fixture body shape, runner behavior, assertion semantics, fixture profiles, and reduced Kernel Smoke queue | [Conformance Fixtures Reference](conformance-fixtures.md) |
| Compact future scenario-family inventory, promotion criteria, suite-family labels, and catalog-only future candidates | [Future Fixtures](../later/index.md#future-fixture-families) |

## Official Terms

### Agency Conformance

The degree to which harness behavior, projections, validators, and close decisions preserve the user's Strategic Agency. Agency conformance checks whether the work journey is followable, user-owned judgment is explicit, autonomy boundaries are respected, blocking user judgments are visible, and residual risk is visible before final acceptance.

### Acceptance

The user's final judgment that the result of the work is acceptable after evidence, verification, Manual QA status, scope, sensitive-action permission, and close-relevant residual risk are shown or confirmed absent. Required Acceptance is recorded through the kernel acceptance path, including a `user_judgment` with `judgment_kind=final_acceptance`, `task_gates.acceptance_gate`, and `state.sqlite.task_events`. Acceptance is separate from sensitive approval, assurance, verification, Manual QA, evidence sufficiency, QA waiver, verification-risk acceptance, and residual-risk acceptance. It does not authorize more writes, approve sensitive action, accept known risk by itself, erase residual risk, or retroactively satisfy a missing check.

### Acceptance Gate

The kernel gate for required final acceptance. Its value set and compatibility meaning are owned by [Acceptance Gate](core-model.md#acceptance-gate). Acceptance cannot substitute for QA or verification.

Required Acceptance in the current reference model is recorded through a Final acceptance user judgment, `task_gates.acceptance_gate`, and `state.sqlite.task_events`; there is no separate acceptance record or table.

### Approval

A limited prior user authorization allowing a specific sensitive action or bounded sensitive operation to proceed within a defined scope. Approval is bound to paths, tools, commands or command classes, network targets, secret scope, baseline, sensitive categories, and expiry conditions. In minimum MVP-1, Core captures the user judgment through a Sensitive action approval user judgment with `judgment_kind=sensitive_approval` and `judgment_payload.approval_scope`; later Approval profiles may also create a linked committed Approval record. Granted sensitive-action permission still requires a later compatible `prepare_write` result before any Write Authorization exists. Approval is sensitive-action permission only: it is not generic agreement, product decision, technical decision, scope decision, final acceptance, residual-risk acceptance, QA waiver, verification-risk acceptance, correctness proof, or cancellation.

### Approval Gate

The kernel gate for sensitive-action permission. It is required only when sensitive categories are present. Granted sensitive-action permission does not prove correctness, imply final acceptance, accept residual risk, waive QA or verification, resolve user-owned judgment, or create Write Authorization.

### Assumption Register

A Discovery or later/profile Shared Design support/projection list of assumptions the agent is using before implementation planning. It should name source, confidence, owner, and what would change if the assumption fails. These are recommended display/support contents, not a standalone schema or canonical record field list. In active MVP-1, assumption context supports Task shaping, Change Unit shaping, and user-judgment candidates; it is not generic user consent, sensitive-action Approval, final acceptance, residual-risk acceptance, evidence, close readiness, scope authority, or Write Authorization.

### Artifact

A registered output used for evidence, recovery, or audit after Core accepts it from an allowed source and records integrity, redaction, owner, and retention metadata. See Raw Artifact for the evidence-file boundary.

### Artifact Reference

A structured pointer to a registered artifact file or safe metadata notice in the artifact store. It includes artifact identity, owner scope, kind, `uri`, `sha256`, `size_bytes`, `content_type`, `redaction_state`, `produced_by`, relation owner, `retention_class`, and availability metadata. `ArtifactRef` is the exact schema name for this pointer shape; it is not an arbitrary caller-supplied path. In [Storage](storage.md), artifact refs and `artifact_links` are Task-scoped. Artifact kinds such as `bundle`, `manifest`, or `export_component` describe files; owner links still point to existing state or Task-scoped projection records.

### Autonomy Boundary

The Change Unit semantics that record the user-owned judgment boundary inside which an agent may proceed without asking for additional user judgment. In plain terms, it says what the agent may decide alone inside the active Change Unit. Routine implementation details may be inside the boundary; public API or module contract changes, security or privacy trade-offs, UX or product behavior trade-offs, material dependency or migration direction, scope expansion, and residual-risk acceptance require explicit user judgment and must not be inferred from broad autonomy.

It is not a scope grant or write authority and does not authorize paths, tools, commands, network targets, secret access, or sensitive categories outside the active Change Unit. A user judgment may authorize updating the Autonomy Boundary or proposing a Change Unit update, but the resulting write still requires compatible Change Unit scope and sensitive-action approval when sensitive categories apply. Exact kernel behavior is owned by [Autonomy Boundary](core-model.md#autonomy-boundary), with policy placement in [Design Quality Policies](design-quality-policies.md#autonomy-boundary-autonomy_boundary).

### Assurance

The technical confidence level supported by recorded checks and verification independence.

```text
none | self_checked | detached_verified
```

An Eval verdict alone does not upgrade assurance. `detached_verified` requires passed verification with valid independence and no same-session self-review violation.

### Baseline

A captured repository state used to judge scope, approval drift, evidence freshness, and verification validity.

### Blocker

A specific condition that prevents progress, write, close, or another requested step until it is resolved or validly deferred. A useful blocker names what is blocked, who owns the next move, the smallest unblocker, and any relevant owner refs. A blocker is not a generic note, evidence by itself, final acceptance, residual-risk acceptance, or sensitive-action Approval.

### `tree_hash`

The deterministic hash of a baseline file snapshot, computed from sorted NFC-normalized relative POSIX paths after ignored paths are excluded, with file bytes, size, executable bit, and symlink target handling defined by [Storage](storage.md).

### Capability Profile

A declared and verified description of what a connected agent surface can actually do. It records target profile, support tier, guarantee level, supported features, risks, fallbacks, and last verification time. The harness does not infer capability from product name alone.

### Capability Tier

A coarse integration level for a connected surface.

```text
T0 Context | T1 Skill | T2 MCP | T3 Capture |
T4 Guard | T5 Isolation | T6 QA Capture
```

Capability tiers describe available integration support; they are not kernel gates.

### Change Unit

The scoped implementation unit that bounds product writes. A product write requires an active Change Unit whose scope covers the intended paths, tools, commands, network targets, and sensitive categories, but the Change Unit does not itself authorize the write. Core allows the write through `prepare_write` and applicable gates.

User-facing docs should normally describe the relevant scope or work piece first. Use `Change Unit` when naming the internal scoped work unit, record, or reference owner.

### Close Reason

The canonical reason a Task reached a terminal close state.

```text
none | completed_verified | completed_self_checked |
completed_with_risk_accepted | cancelled | superseded
```

### Close Readiness

The user-facing summary of whether work can close now and what remains before it can close honestly. It may show close blockers, missing evidence, verification or Manual QA status, required final acceptance, visible residual risk, residual-risk acceptance needs, and the next safe action. Close readiness is derived from owner records and gates; it is not itself final acceptance, residual-risk acceptance, evidence, verification, QA, Write Authorization, or a close event.

### Codebase Stewardship

The responsibility to preserve the product codebase as a durable asset. It includes attention to domain language, module boundaries, interface contracts, dependency direction, testability, maintainability, and future-change risk.

### Common Tool Envelope

The shared fields carried by public MCP tool calls: `request_id`, `idempotency_key`, `expected_state_version`, `project_id`, optional `task_id`, `surface_id`, optional `run_id`, `actor_kind`, and `dry_run`.

### Core-owned State

Operational state owned by Harness Core through committed owner records and `state.sqlite.task_events`. Core-owned state is the authority for gates, decisions, write authorization, evidence state, QA, verification, final acceptance, residual risk, and close. Chat, generated Markdown projections, connector files, and product repository docs can inform Core through owner paths, but they do not replace Core-owned state.

### Cooperative Guarantee

A guarantee level where the agent surface is expected to follow harness instructions and MCP decisions. The harness can guide behavior, but the surface may not provide hard pre-execution enforcement.

### Connector Manifest

A generated manifest that records connector-generated and connector-managed paths, MCP config snippets, managed block hashes, capability/profile freshness, capture/guard/isolation notes or mechanisms, manual fallback notes, and drift or stale status. It prevents generated or managed surface files from being silently overwritten. The full manifest contract is owned by [Agent Integration Reference](agent-integration.md#generated-manifest-expectations).

### Context Hygiene

The policy of keeping always-on context short and current: keep the compact rule set to one screen or less, read current status or current-position context first, use Journey Card only when that projection/profile is enabled and fresh, and keep larger records pull-on-demand. The always-on envelope carries only current Task summary, work shape, scope/non-goals, pending user judgments, active blockers, next safe actions, evidence gaps, close blockers, residual-risk summary, guarantee level, and source refs/freshness. Older PRDs, designs, logs, module maps, old projections, closed issues, Reference contracts, full artifact contents, and future catalog material are pulled only when planning/clarification, write preparation, execution/run recording, evidence review, close readiness, user judgment request, recovery/error, or a verification bundle needs them. Indexed, retrieved, remembered, or summarized context belongs here as refs or source-linked excerpts. It helps decide what to inspect, not what Harness has authorized, verified, accepted, waived, risk-accepted, or closed.

Stale chat memory is pull-only context. It cannot authorize writes, satisfy gates, close tasks, record final acceptance, waive QA or verification, accept residual risk, replace current state, or repair stale projections unless the relevant owner path records the change.

### Context Index

A later read-only context provider that may surface relevant projections, artifact refs, repo files, docs, or notes. Until promoted through owner docs, it is a Roadmap candidate and non-authoritative retrieval only; even after promotion, it cannot replace existing authority paths unless those owner docs explicitly change. Retrieved context may point to sources to inspect, but it must not authorize writes, resolve decisions, grant Approval, create evidence, perform verification, accept residual risk, satisfy gates, or close Tasks. Context Index remains a roadmap candidate; see [Roadmap: Candidate Inventory](../later/index.md#roadmap-candidates), with connector handling in [Agent Integration](agent-integration.md#context-pushpull-principles).

### Decision Gate

The Task-level aggregate gate for blocking user-owned judgment before progress, write, or close can continue. The canonical field is `decision_gate`; its value set and recompute rule are owned by [Decision Gate](core-model.md#decision-gate). It is recomputed from relevant blocking user judgments and detected blockers, and it does not substitute for sensitive-action approval, verification, Manual QA, final acceptance, or residual-risk acceptance.

### User Judgment

The canonical record family for user-owned judgment. A `UserJudgment` names the exact question, `judgment_kind`, `presentation`, pending options or chosen outcome, affected Task/Change Unit/write/close scope, affected object refs, supporting refs, recommendation, rationale, uncertainty, no-decision consequence, why the agent cannot decide, owner, status, and next action. Public refs use `StateRecordRef.record_kind=user_judgment`. User judgment visibility required by the active stage/profile is provided through Task/status/next/judgment-context and user-judgment resources; standalone `DEC` Markdown renderings are optional full-format projections unless enabled.

The supported `judgment_kind` values are `product_decision`, `technical_decision`, `scope_decision`, `sensitive_approval`, `qa_waiver`, `verification_risk_acceptance`, `final_acceptance`, `residual_risk_acceptance`, and `cancellation`.

The agent may recommend, but the user decides user-owned judgments. Broad approval text does not satisfy a user judgment unless it answers the specific pending `judgment_kind`, option, affected object, scope, consequences, and "does not settle" boundary. "Yes, do it," "proceed," "go ahead," and "looks good" must not automatically become sensitive approval, final acceptance, QA waiver, verification-risk acceptance, residual-risk acceptance, cancellation, or scope change.

### Judgment Kind Display

A user-facing label for the specific kind of pending user-owned judgment. New docs should use only these labels:

- Product decision
- Technical decision
- Scope decision
- Sensitive action approval
- QA waiver
- Verification risk acceptance
- Final acceptance
- Residual risk acceptance
- Cancellation

Sensitive action approval permits only the named sensitive step. Final acceptance records the user's result judgment and does not accept known residual risk by itself. Residual risk acceptance must name the risk being accepted and does not make verification or QA pass. QA waiver does not create QA evidence or a passed QA result. Verification-risk acceptance does not create detached verification. Scope expansion uses `scope_decision`; broad approval does not expand scope.

### Presentation

The schema field that controls prompt length and detail. Use `presentation=short` for compact one-screen prompts and `presentation=full` for full-format Decision Packet-style presentation. Presentation is not an authority path and does not change which judgment is being recorded.

### Decision Packet

A full judgment presentation for a complex `UserJudgment`, plus a legacy label in older references. A Decision Packet can render recommendation, uncertainty, detailed trade-offs, evidence, residual risk, approval scope, waiver context, acceptance context, or reconcile target when the active profile requires more context. It is not the default user-facing mechanism for every judgment, not a separate authority record family, and not a replacement for `user_judgment`.

Legacy `decision_packet` refs, `DecisionPacket` shapes, or `DEC-*` projection ids may remain in compatibility or migration notes. New docs, examples, fixtures, and payloads should use `user_judgment` and `UserJudgment` unless they are explicitly describing full-format Decision Packet presentation.

### Judgment Route

Legacy or implementation routing terminology from older Decision Packet drafts. New public docs should not use `judgment_route` as a primary concept. If old payloads appear, map the route into `judgment_kind` plus route-specific payload validation and explain the result in ordinary language.

### Display Depth

Legacy prompt-depth terminology from older Decision Packet drafts. New public docs should use `presentation=short` or `presentation=full`.

### Judgment Category

Legacy grouping terminology from older Decision Packet drafts. New public docs should use `judgment_kind` and locale-derived rendered labels. The old field may appear only in compatibility docs or old payloads.

### User Judgment Request

Optional routing, interaction, idempotency replay, or compatibility handoff metadata that may point to a canonical `UserJudgment`. A minimal Engineering Checkpoint implementation may omit it. A User Judgment Request is not judgment authority, never satisfies `decision_gate`, sensitive-action approval, final acceptance, waiver, residual-risk acceptance, or close by itself, and is only relevant to gate aggregation through a linked compatible `user_judgment_id`.

### Decision Request

Legacy name for User Judgment Request. Use only in migration notes or old payload compatibility.

### Design Gate

The kernel gate surface where enabled design-quality policy findings are routed. In active MVP, write or close blocks by default only for the small Core-backed set in [Design Quality Policies: Active MVP blocking set](design-quality-policies.md#active-mvp-blocking-set); broader domain-language, TDD, module/interface, stewardship, feedback-loop, Manual QA, and detached-verification catalog findings are candidate or advisory/later unless an active owner path promotes them.

### Design-Quality Policy Pack

The owner document for design-quality policy contracts, impact classes, routed actions, and severity composition. It covers shared design, decision quality, autonomy boundary, domain language, vertical slice, feedback loop, TDD trace, module/interface review, codebase stewardship, Manual QA, and context hygiene. Findings influence gates, validators, evidence, user judgment requests, residual-risk markers, advisory next actions, write blockers, or close blockers only through the allowed route and active owner path; the document does not redefine the kernel state machine.

### Detached Verification

Verification performed across a meaningful independence boundary, such as a fresh session, fresh worktree, sandbox, or manual evaluator bundle. This supports verification independence and stale-context control; it is not automatically OS-level security isolation. Same-session self-review is not detached verification, and subagent context is not detached by default.

### Discovery

The internal name for requirement clarification before implementation planning and before write authority. It separates goal, user value, non-goals, acceptance criteria, facts the agent can inspect from repo/docs/Harness state, assumptions, judgments only the user can make, Product decision candidates, Technical decision candidates, Scope decision candidates, Sensitive action approval needs, QA and verification expectations, remaining uncertainty, and safe next-work candidates or work split proposals. It asks the user only for decisions the codebase and current Harness context cannot answer, may ask multiple targeted questions grouped by judgment area, and can pause or proceed when inspectable facts and user-owned judgments are separated, goals/non-goals/acceptance criteria and major judgment candidates are clear enough, safe next work or a work split can be proposed without hiding unresolved judgment, and remaining uncertainty is explicit. In active MVP-1, requirement clarification outputs route to Task shaping fields, user judgment candidates/records, and proposed or active Change Unit shaping; later/profile owners may render or record Shared Design when enabled. Phrases such as safe next-work candidate and work split proposal are proposal/support phrases, not standalone schema fields, canonical record types, gate values, projection kinds, or authority paths. Discovery is not approval, sensitive-action approval, Write Authorization, evidence, verification, QA, final acceptance, residual-risk acceptance, close, scope authority, or a new authority path.

### Discovery Brief

A compact Discovery or later/profile Shared Design support/derived-view summary of the clarified goal, user value, non-goals, acceptance criteria, inspectable facts, question queue, assumption register, separated user-owned judgments, Product decision, Technical decision, Scope decision, Sensitive action approval needs, QA and verification expectations, remaining uncertainty, and safe next-work candidate or work split. It may include a First Safe Change Unit Candidate when product writes are near. These are recommended display/support contents, not a standalone schema or canonical record field list. In active MVP-1, a Discovery Brief can inform Task shaping fields, user judgment candidates/records, and Change Unit shaping; later/profile owners may render or record Shared Design when enabled. It does not by itself create canonical scope, resolve decisions, authorize writes, prove evidence, record residual-risk acceptance, record final acceptance, or close a task.

### Detective Guarantee

A guarantee level where the harness can detect violations and mark state blocked, stale, partial, or failed after observation.

### Direct

A work mode for small, low-risk changes with obvious scope and result. Direct product writes still require an active scoped Change Unit. Direct includes the tiny direct profile for trivial typo, single-sentence docs, or obvious rename work; Tiny is not a top-level mode and does not bypass user-owned judgment, sensitive-action Approval, security boundaries, evidence, scope, Write Authorization, residual-risk visibility, or close rules.

### Docs-Maintenance Conformance

A read-only documentation maintenance check profile that detects drift in bilingual parity, links, owner boundaries, stable catalogs, glossary terms, source-of-truth phrasing, TODO usage, and non-owner duplicate contracts. Its rule bodies are owned by the [Authoring Guide](../maintain/authoring-guide.md#docs-maintenance-checks), and operator reporting and entrypoint expectations are owned by [Operations And Conformance Reference](operations-and-conformance.md#docs-maintenance-profile). It is a docs-only profile, not runtime conformance or task state authority.

### Domain Language

The product's canonical vocabulary and meanings for the later design/stewardship profile. The canonical source is `domain_terms`; Markdown domain-language documents are projections and proposal surfaces. A term conflict can affect `design_gate` through policy validation when that profile is active, and it routes to a user judgment when choosing the meaning is user-owned Product decision or material Technical decision.

### Domain Term

A later/profile canonical structured record in `domain_terms` that stores a product term, meaning, code representation, related terms, source, status, and boundaries such as "not this." Public state refs use `record_kind=domain_term` only when the owning design/stewardship profile is active.

### Evidence

Recorded support for claims about the work, such as diffs, logs, tests, run summaries, screenshots, Eval records, Manual QA records, evidence summaries, and registered artifact refs. Minimum MVP-1 evidence display uses `evidence_ref`, Run refs, ArtifactRefs, and visible gaps; evidence summaries are derived from those refs. The full Evidence Manifest profile adds criteria-to-evidence mapping through Evidence Manifest records. Evidence is not the agent merely saying the work is done, and it is not made sufficient by Markdown report prose alone.

### Evidence Gate

The kernel gate for required evidence coverage. Its value set and close meaning are owned by [Evidence Gate](core-model.md#evidence-gate).

### Evidence Manifest

A detailed evidence-list state record mapping acceptance criteria or completion conditions to supporting evidence references. Sufficiency depends on the coverage of those criteria and conditions by current owner records and `ArtifactRef` refs, not on artifact count or report prose. Minimum MVP-1 can show evidence summaries, Run refs, ArtifactRefs, and visible gaps without requiring this full record.

### Evidence Profile

A named evidence sufficiency profile, such as `advisor`, `direct docs-only`, `direct code`, `work feature`, `UI/UX/copy work`, `sensitive work`, or `verification-required work`, that tells validators what evidence is enough for the task shape. Tiny direct docs-only work is handled under Direct evidence expectations with the smallest changed-path, patch-summary or diff-ref, and self-check support; it is not a separate authorization path.

### Evidence Sufficiency

The close-relevant judgment that required acceptance criteria or completion conditions have compatible current support. Minimum MVP-1 displays known evidence through evidence summaries, Run refs, ArtifactRefs, and visible gaps. Full criteria-to-evidence sufficiency uses Evidence Manifest records only when the full Evidence Manifest profile is active. Sufficiency is not judged from chat text or Markdown report prose alone, and evidence can become stale through baseline drift, changed files, sensitive-action permission or Approval drift, missing artifacts, or relevant design record changes.

### Eval

A verification result record with verdict, checks performed, evidence reviewed, independence qualifier, blockers, and artifact references.

### Feedback Loop

A later/profile canonical support record and recorded path from checks and findings back into state, scope, design, evidence, follow-up work, or close status. Inputs can include tests, typecheck, lint, build, browser smoke, TDD red/green/refactor traces, Manual QA, Eval findings, user judgments, operational findings, and residual-risk decisions. Public refs use `StateRecordRef.record_kind=feedback_loop`, and public mutation uses `FeedbackLoopUpdate` on `record_run` or a Manual QA execution link, only when the owning profile is active. Feedback loops keep findings from vanishing into chat by routing them to existing owner paths such as Evidence Manifest coverage, user judgments, Change Unit updates, Residual Risk records, Manual QA or Eval records, close blockers, or follow-up Task/Change Unit records where applicable.

### Finding

An observed issue, gap, risk, blocker, or noteworthy result from a Run, Eval, Manual QA record, validator, review display, operator diagnostic, or conformance check. A finding is not a standalone authority path and does not affect gates or close by staying in chat or report prose. It becomes state-relevant only when routed through existing owner records or structured results, such as Evidence Manifest gaps, user judgment candidates or records, Change Unit updates, Feedback Loop or TDD Trace updates, Manual QA or Eval records, Residual Risk records, reconcile items, close blockers, or follow-up Task/Change Unit records. The routing contract is owned by [Design Quality Policies](design-quality-policies.md#finding-routing) and [Core Model Reference](core-model.md#finding-routing).

### First Safe Change Unit Candidate

The internal Change Unit-shaped expression of a safe next-work candidate when product writes are near. It should name included behavior, out-of-bounds behavior, completion conditions, known sensitive areas, and stop conditions without hiding unresolved user-owned judgment. Discovery may propose it after inspectable facts and user-owned judgments are separated and the safe next work is clear enough; later/profile Shared Design may include it when enabled. Discovery does not exist only to find this candidate. These are recommended display/support contents, not a standalone schema or canonical record field list. It is a candidate only: an active Change Unit, compatible scope gate state, and later `prepare_write` are still required before product writes.

### Fixture Assertion Semantics

The conformance comparison rules that say how `expected_state`, `expected_events`, `expected_artifacts`, `expected_projection`, and `expected_error` are matched against captured Core results. They are owned by [Conformance Fixtures Reference](conformance-fixtures.md#fixture-assertion-semantics), live outside the fixture body, and do not allow prose-only matching to pass a fixture.

### Fresh Session

A verification independence profile where the evaluator starts from a task/evidence bundle rather than continuing the lead chat context, reviews the Evidence Manifest and changed files, and records an Eval.

### Fresh Worktree

A verification independence profile where the evaluator checks baseline, changed paths, artifacts, and Evidence Manifest in a separate worktree or equivalent independent repository state. A fresh worktree can support scope, freshness, and drift detection, but it is not automatically an OS sandbox, permission boundary, or tamper-proof security boundary.

### Freeze

A user-facing safety control that requests a hold or narrower posture around current work. Freeze can hold product writes, make the next action stricter, or cause `prepare_write` to block or hold when existing scope is incompatible. It does not directly mutate Change Unit scope, allowed paths, Autonomy Boundary, AFK stop conditions, or related owner records; persistent owner-record changes still use the existing Core state-changing path, user judgment route, or owner-record update path. Freeze does not create Write Authorization, approval, evidence, verification, QA, final acceptance, residual-risk acceptance, close, or a new authority tier.

### Gate

A canonical kernel field that controls whether a Task may write, proceed, or close. Gates are state, not display text.

### Generated File

A repository file or managed block produced by a connector, projector, or operator tool. Generated files must be tracked by a manifest or projection job when they can drift from canonical state.

### Guarantee Display

The user-facing and connector-facing display of the actual guarantee level for a status or write decision, including limitation notes when enforcement is cooperative or detective.

### Guarantee Level

The honest enforcement strength available for a connected surface or runtime path.

```text
cooperative | detective | preventive | isolated
```

Capability affects validator results, blocked reasons, and display; it is not Approval, Write Authorization, verification, QA, final acceptance, residual-risk acceptance, close readiness, or a kernel gate. Exact level meanings are owned by [Security Reference](security.md#honest-guarantee-display).

### Guard

A user-facing safety control that applies the connected profile's actual enforcement or detection layer. Guard may be cooperative, detective, preventive, or isolated; the name does not imply pre-execution blocking unless a proven `T4` path covers the operation.

### Hardened Local Reference Target

The aggregate local reference behavior reached after MVP-1 User Work Loop by completing the owner-defined Assurance Profile and Operations Profile profiles. It is an umbrella target, not a separate delivery stage, not the first implementation batch, and not a fixture profile or suite name.

The hardened local reference target does not replace the boundaries for Engineering Checkpoint, MVP-1 User Work Loop, or Roadmap. Conformance is still proven through the named fixture profiles: Engineering Checkpoint fixtures, MVP-1 User Work Loop fixtures, Assurance Profile fixtures, and Operations Profile or promoted Roadmap fixtures.

### Harness Core

The runtime component that owns state transitions, gate updates, validator interpretation, artifact registration, projection job enqueueing, and close decisions.

### Harness Server

The future local Harness program and tool surface that receives agent requests, validates or records state changes through Core, runs validators, and produces readable projections. This documentation repository's intended future role is the Harness Server source repository; it is not a Product Repository or Harness Runtime Home. No Harness Server/runtime implementation exists here yet, and server/runtime implementation may start only after documentation acceptance and a separate implementation-planning readiness decision.

### Harness Runtime Home

The local runtime storage area that contains `registry.sqlite`, per-project `project.yaml`, per-project `state.sqlite`, and artifact directories.

### Human-editable Area

A Markdown area where a human can write notes, proposals, questions, or corrections. It is an input surface, not canonical state. Its authority path is `human-editable input -> reconcile_items -> accepted state event/record`.

### Implementation Micro-Plan

A managed `TASK` projection section that shows small execution steps or slices, their purpose, active Change Unit scope alignment or likely paths, selected feedback loop or TDD status when relevant, expected evidence, and stop conditions. It is an execution aid, not canonical state, not a `ProjectionKind`, not scope authority, not approval, and not Write Authorization. Editing its text does not mutate state except through an accepted reconcile outcome or Core state-changing action.

### Isolated Guarantee

A guarantee level where work or verification runs behind a documented separation boundary. A worktree or fresh evaluator bundle can provide scope, freshness, or blast-radius separation, but it is not automatically an OS sandbox, permission boundary, or tamper-proof security boundary unless the profile proves that exact isolation mechanism. Isolation alone does not approve, verify, record final acceptance, accept residual risk, close, or upgrade assurance.

### Journey Card

A compact human-readable projection of the current Task position: state, next action, scope, active scoped Change Unit, Autonomy Boundary, blockers, active user judgment, Write Authority Summary, acceptance criteria, approval status, evidence, verification, QA, final acceptance, residual risk, and projection freshness. A Journey Card is display, not canonical state, and it is rendered from current owner records rather than stale chat memory.

### Judgment Category (Legacy)

Legacy grouping field from older Decision Packet drafts. New public docs should use `judgment_kind`, `presentation`, and locale-derived rendered labels instead. Preserve `judgment_category` only in compatibility docs or old payload migration notes.

### Journey Spine

The state-derived continuity model for a Task's ordered work journey. It is reconstructed from Task, Change Unit, Run, User Judgment, Approval, Evidence Manifest, Eval, Manual QA, Residual Risk, `task_gates.acceptance_gate`, final-acceptance user judgment state, close events, artifact references, and `state.sqlite.task_events`, not from chat memory. Journey Card and Journey Spine Markdown views are projections.

### Journey Spine Entry

A canonical support record for durable continuity annotations that cannot be fully reconstructed from existing state events or owner records. Journey Spine Entry records supplement the Journey Spine; they do not replace Task, Change Unit, Run, User Judgment, Residual Risk, evidence, verification, QA, final-acceptance gate/judgment state, close state/events, artifact, or event authority.

### Interface Contract

The later/profile canonical record of a module or external boundary's public interface, inputs, outputs, errors, compatibility impact, callers, and boundary tests. The canonical source is `interface_contracts`. Public state refs use `record_kind=interface_contract` only when the owning design/stewardship profile is active. The record documents the interface understanding; it is not Approval, final acceptance, residual-risk acceptance, or Write Authorization. Public interface or compatibility choices route through the existing design-quality and user judgment paths when user-owned judgment is required.

### JSON `TEXT` Field

A SQLite `TEXT` column whose stored value is JSON. The `TEXT` type is reference storage flexibility only; Core must validate the value before commit against the API-owned or storage-owned shape, and malformed or schema-incompatible JSON is invalid state.

### Local Derived Metrics

Later diagnostic-only metrics derived from local records such as `state.sqlite.task_events`, runs, validator results, projection jobs, and reconcile items. Until promoted through owner docs, metric readouts may report rates, counts, durations, or guard-trigger summaries only as read-only diagnostics. Local Derived Metrics remain a roadmap candidate; see [Roadmap: Candidate Inventory](../later/index.md#roadmap-candidates).

### Manual QA

Human inspection of experiential product quality such as UX, workflow, copy, visual output, accessibility, and product fit. Manual QA is recorded through the Manual QA record or a valid QA waiver path when required; browser smoke, screenshots, Browser QA artifacts, tests, or verifier notes may support context but are not Manual QA judgment by themselves. Exact gate behavior is owned by [QA Gate](core-model.md#qa-gate), with policy requirements in [Design Quality Policies](design-quality-policies.md#manual-qa-manual_qa).

### Manual Bundle

A verification handoff package for a human or separate evaluator. It includes task summary, acceptance criteria, Change Unit scope, approval scope, diff/log/test artifacts, Evidence Manifest, known risks, and enough context to record an Eval verdict.

### Manual QA Record

A record-level Manual QA result, including performer, profile, result, artifacts, findings, waiver reason when applicable, and next action. Its result value set is owned by [QA Gate](core-model.md#qa-gate) and later/profile-gated [`harness.record_manual_qa`](../later/index.md#later-schema-candidates). Pending required QA is represented by `qa_gate=pending`; it is not a Manual QA record result.

### `managed_hash`

The drift-detection hash of the projector-owned managed block body, excluding `HARNESS:BEGIN` and `HARNESS:END` marker lines. It is not canonical state and does not make a Markdown projection authoritative.

### Managed Block

A Markdown block delimited by harness markers and regenerated by the projector from state records and artifact refs. Direct edits to a managed block create drift or reconcile candidates; they do not become state by themselves.

### MCP Resource

A read-only MCP surface for current project, task, design, policy, status, or bundle information. Resources do not mutate state.

### MCP Server Unavailable

`MCP_SERVER_UNAVAILABLE` is the diagnostic condition where a tool call cannot reach Core. No authoritative Core response is possible, and the caller must diagnose or reconnect before claiming state changes. The stable public error code remains `MCP_UNAVAILABLE`.

### Surface MCP Unavailable

`SURFACE_MCP_UNAVAILABLE` is the diagnostic condition where Core or an operator can observe that the connected surface lacks usable MCP, has stale MCP configuration, or cannot call required MCP tools. Product writes are held by instruction on cooperative surfaces or blocked by stronger guards when available. Core responses may use `MCP_UNAVAILABLE` or `CAPABILITY_INSUFFICIENT` with `details.mcp_unavailable_kind`; the diagnostic label is not a public `ErrorCode` value.

### MCP Tool

A public MCP operation that asks Core to validate, record, transition, or close state. State changes must go through tools or reconcile actions, not resource reads.

### Markdown Report

A human-readable document generated from state records and artifact references. A Markdown report is a projection by default and does not become canonical state or canonical evidence.

### Natural-Language Consent

A user utterance such as "yes, do it," "go ahead," "proceed," or "looks good" that may answer a pending question only when the active prompt makes the exact `judgment_kind`, option, affected object, scope, affected gates, consequences, and remaining unapproved, unaccepted, unwaived, or uncanceled items unambiguous. Natural-language consent is not its own authority path. Ambiguous consent must be clarified rather than broadened into sensitive approval, final acceptance, residual-risk acceptance, QA waiver, verification-risk acceptance, cancellation, scope change, or Write Authorization.

### Module Map

The product's later/profile map of modules, responsibilities, public interfaces, dependency direction, internal complexity, test boundaries, owner decisions, and watchpoints. The canonical source is `module_map_items`. A module boundary update records the shared technical understanding; it does not approve writes or accept residual risk. Boundary changes that shift product commitments, caller obligations, or architecture direction route through design-quality policy and user judgment paths when user-owned judgment is required.

### Module Map Item

A later/profile canonical structured record in `module_map_items` that stores a module's role, public interface, dependencies, internal complexity, test boundary, owner decision, and watchpoints. Public state refs use `record_kind=module_map_item` only when the owning design/stewardship profile is active.

### Policy Contract

The standard form used by design-quality policies: `name`, `applies_when`, `default_requirement`, `allowed_waiver`, `required_record`, `validator`, `evidence`, and `close_impact`.

### Preventive Guarantee

A guarantee level where Harness or a connector can block a covered violating action before it executes, with an owner-defined mechanism and fixture proof for that exact path. The label must name what is covered; it does not imply arbitrary-tool prevention, OS sandboxing, permission isolation, tamper-proof storage, or broader authority.

### Product Repository

The user's real product workspace: source code, tests, product documentation, and generated readable Harness reports when projection output is written there. The Product Repository remains the source for product content. It is not Harness Runtime Home, and product files become Harness operational facts only when an existing Core, artifact-registration, reconcile, or owner-record path records the relevant Harness fact.

### Projection

A derived view generated from Core state records and artifact references, such as a status view, summary, status card, or Markdown report. Projection is useful for reading and decision-making, but it cannot override or replace canonical state.

### ProjectionKind

The API enum for projection job and template kinds. Support classes, active values, and extension rules are owned by [API Schema Core](api/schema-core.md#projectionkind-support); later/profile-gated values stay in [API Schema Later](../later/index.md#later-schema-candidates). Support class labels are not Engineering Checkpoint run obligations; Engineering Checkpoint has no projection-rendering exit requirement beyond preserving any owner-produced freshness/read facts. No ProjectionKind makes a projection canonical state.

### Projection Freshness

The relationship between a projection and its source records, managed hash, artifact refs, and projection job state. Its value set is owned by [API Schema Core](api/schema-core.md#projectionkind-support) and [Projection And Templates Reference](projection-and-templates.md).

### Projection Job

A later/profile-promoted durable outbox record that asks the projector to render a Markdown projection from committed state records and artifact refs. MVP-1 compact view output does not require a `projection_jobs` table. When the projection job profile is active, `record_kind=projection` identity is `projection_jobs.projection_job_id`; project-level projection jobs do not by themselves create project-scoped artifact links in the current Task-scoped artifact DDL.

### Question Queue

A Discovery or later/profile Shared Design support/projection list of open questions classified as blocking, useful-but-not-blocking, or codebase-answerable. These are recommended display/support contents, not a standalone schema or canonical record field list. Blocking questions may route to a user judgment candidate when user-owned judgment is required. Useful-but-not-blocking questions can be parked, deferred, or turned into follow-up work. Codebase-answerable questions should be answered from current repo, docs, Harness state, or source refs rather than asked of the user. The queue is not a user judgment, gate, approval, evidence, final acceptance, close, or Write Authorization.

### QA Gate

The canonical kernel gate for required Manual QA. `manual_qa_record.result` is record-level; `qa_gate` is the close-relevant aggregate state. `qa_gate=pending` means required QA has not yet produced a satisfying Manual QA record, or the latest relevant Manual QA record does not satisfy policy.

### Raw Artifact

A durable evidence file in the artifact store, such as a diff, log, bundle, screenshot, checkpoint, or manifest file, after registration from Harness staging, an approved capture adapter, or an existing committed artifact ref. Registered artifact files are distinct from state records and Markdown reports; they need `ArtifactRef`, owner relation, integrity, redaction, and retention metadata before close-relevant evidence can rely on them.

### Reconcile

The process that turns human-editable input or projection drift into an accepted state change, rejected proposal, note, decision, or deferred item.

### Reconcile Item

The canonical candidate record created from human-editable input or projection drift before a reconcile decision accepts, rejects, converts, or defers it.

### Reference Surface

The single agent surface targeted by Engineering Checkpoint. It demonstrates the kernel and connector contract without implying broad connector-surface support.

### Recommended Playbook

Non-authoritative status/next display guidance computed from current state and policy/playbook context. It suggests a procedure for the current stage, such as review, TDD, QA, guard check, release handoff, or browser-QA candidacy. Its `playbook_id` is a stable display/routing string identifier, not a Core-owned closed enum or DDL-backed value set. It is not a canonical kernel record, has no DDL table, task event, or projection job of its own, does not authorize writes, satisfy gates, record final acceptance, accept residual risk, or close tasks, and routes user-owned judgment to user judgment paths or other existing Core/MCP mutation paths.

### Release Handoff

An optional report/export profile that summarizes release readiness for external PR, review, deployment, rollback, and monitoring processes. It includes close readiness, blockers, evidence refs, verification refs, Manual QA refs, residual-risk refs, changed files, projection freshness, redaction notes, and suggested checklist items. The exact report/export authority boundary is owned by [Operations And Conformance](operations-and-conformance.md#release-handoff-export-profile).

### Role Lens

A non-authoritative skill or playbook surface that lets a user ask for a product, engineering, design, security, QA, or release-handoff review posture. Role Lens output reuses existing routes such as `RecommendedPlaybook`, `UserJudgmentCandidate`, validator/check routes, evidence, Eval or verification, Manual QA, Approval, residual-risk, Change Unit update, and release handoff routes. It is read-only guidance until an existing Core/MCP path records the underlying action, so it does not mutate state, authorize writes, satisfy gates, record final acceptance, accept residual risk, close tasks, or upgrade assurance by itself. The exact non-authority boundary is owned by [Agent Integration](agent-integration.md#role-lens-behavior).

### Report Projection

A Markdown report generated from state records and artifact references, such as a Task report, approval report, run summary, evidence manifest report, Eval report, or direct-result report.

The named report projection kinds are projections generated from state records and artifact refs; state authority stays with Core records and evidence-file authority stays with registered artifact files. Exact projection rules are owned by [Projection And Templates Reference](projection-and-templates.md), and full rendered bodies are owned by [Template Reference](templates/README.md).

### Review Stages

A managed display/procedure split that separates Spec Compliance Review from Code Quality / Stewardship Review. Spec Compliance Review asks whether the requested work is complete under current Harness authority. Code Quality / Stewardship Review asks whether the implementation is maintainable inside the codebase. Review Stages can route findings to validator results, evidence gaps, user judgment candidates, Eval or verification needs, Manual QA needs, sensitive-action permission needs, later Approval needs when that profile is active, residual-risk candidates, Change Unit update recommendations, or close blockers. They are not canonical records, `ProjectionKind` values, sensitive-action permission / Approval, evidence, verification, QA, final acceptance, residual-risk acceptance, close, or Write Authorization. Their exact display-only boundary is owned by [Design Quality Policies](design-quality-policies.md#two-stage-review-display); same-session Review Stages do not create `assurance_level=detached_verified`.

### `request_hash`

The idempotency hash of a tool request, computed from canonical UTF-8 JSON covering `tool_name`, the schema-normalized request body, and the envelope fields other than `request_id` and `idempotency_key`.

### Residual Risk

A canonical close-relevant support record for known remaining risk, uncertainty, trade-off, limitation, or unchecked condition after evidence, verification, QA, and final-acceptance review. It records source refs, affected scope, related user judgment when applicable, visibility status, accepted risk when applicable, follow-up requirement, and close impact. Known close-relevant Residual Risk must be visible before any successful final acceptance or close, or `ResidualRiskSummary.status=none` must confirm no known close-relevant risk. Residual-risk acceptance means the user explicitly accepts a named known remaining risk; it does not mean the result has otherwise completed verification, final acceptance, sensitive-action approval, or waiver. Accepted risk is metadata/state on the Residual Risk record in the current reference model, not a separate `accepted_risk` state record.

### Risk Accepted Close

A successful close where the user accepts visible close-relevant residual risk, including verification risk when verification was waived. It uses `close_reason=completed_with_risk_accepted`, requires a compatible residual-risk acceptance `user_judgment` and visible blocker/evidence refs in MVP-1, and must not display `assurance_level=detached_verified`. Rich Residual Risk refs are later/profile-promoted. User-facing summaries must keep this close reason distinct from normal `completed_verified` or `completed_self_checked` close.

### Run

An execution attempt by an agent, evaluator, operator, or other actor against a Task and optionally a Change Unit. Runs record baseline, surface, observed changes, commands, artifacts, and summary. A rejected pre-commit `record_run` request is not a Run and must not receive a fabricated Run ID; an audit or violation attempt becomes a Run only when Core deliberately commits it.

### Scope Gate

The kernel gate requiring product writes to be covered by an active scoped Change Unit. Scope is required for write-capable direct and work modes even when approval is not required. Scope Gate does not grant sensitive-action Approval, resolve user-owned judgment, or create Write Authorization; exact values and compatibility are owned by [Scope Gate](core-model.md#scope-gate).

### Severity Composition

The policy-owned rule for merging multiple applicable task-shape defaults, policy contracts, and validator findings. The same concern is the same policy-relevant target, not the whole Task or merely the same validator ID. The rule keeps all findings visible, preserves impacts across different affected gates or blocker targets, and uses the strongest applicable impact only for competing impacts on the same concern. It affects validators, gates, write blockers, close blockers, waivers, and user judgment needs, while public primary `ToolError` selection remains API-owned. Exact policy behavior is owned by [Severity composition rule](design-quality-policies.md#severity-composition-rule).

### Shared Design

A later/profile design-support record or projection label for recorded shared understanding before implementation hardens into a plan. In active MVP-1, requirements shaping does not create a committed Shared Design record; it persists through Task, Change Unit, and User Judgment owner paths. Discovery Briefs, Question Queues, Assumption Registers, safe next-work candidates or work splits, and First Safe Change Unit Candidates are support/display names unless an owner profile explicitly enables a separate record. Shared Design can support shaping and `design_gate` readiness when enabled, but it is not sensitive-action Approval, final acceptance, residual-risk acceptance, QA judgment, evidence, close readiness, or Write Authorization. Markdown renderings of Shared Design are projections and proposal surfaces. Exact policy requirements are owned by [Design Quality Policies](design-quality-policies.md#shared-design-shared_design).

### Source-of-truth

The authoritative source for a fact. In Harness, operational state is canonical in `state.sqlite` current records; `state.sqlite.task_events` is audit and ordering history, not the normal current-state source. Raw evidence files are canonical in the artifact store, and Markdown documents are projections. Product repository files remain the source for product content; they do not become Harness operational state unless an existing Core, reconcile, artifact-registration, or owner-record path records the relevant Harness fact.

### `state.sqlite.task_events`

The append-only event history table inside `state.sqlite`. Reference event storage does not use a separate event store. Deterministic order is `task_events.event_seq`, not timestamps or event IDs.

### Stable Event Catalog

The kernel-owned compact list of `task_events.event_type` names that staged/reference conformance fixtures may assert in `expected_events`. It classifies stable event names separately from prose examples, fixture shorthand, non-stable implementation-local detail or audit events, validator IDs, Core check names, projection status shorthands, and future extension events.

### State Record

A canonical structured record in kernel state. Active MVP-1 state records are limited to the active schema/storage owner set, such as Task, Change Unit, User Judgment, Run, Write Authorization, Artifact record, Evidence Summary, and Blocker. Later/profile records such as Journey Spine Entry, Residual Risk, Approval, Evidence Manifest, Eval, Manual QA record, Shared Design record, Domain Term, Module Map Item, Interface Contract, Feedback Loop, TDD Trace, or Reconcile Item are state records only when their owner profile enables them.

### State Version

An optimistic-concurrency clock for a Core-resolved state scope. Core resolves the primary Task from tool-specific `task_id`, then `ToolEnvelope.task_id`, then active Task resolution when one applies. `expected_state_version`, `ToolResponseBase.state_version`, `EventRef.state_version`, and `task_events.state_version` are interpreted by that affected scope, not as one global event-store sequence.

### Project State Version

The project-scoped state clock stored in `project_state.state_version`. Project-scoped mutations with no Core-resolved primary Task compare `expected_state_version` against this value and return the resulting value as the primary response `state_version`.

### Task State Version

The task-scoped state clock stored in `tasks.state_version`. Task-scoped mutations compare `expected_state_version` against the Core-resolved primary Task's value and return the resulting value as the primary response `state_version`.

### Strategic Agency

The user's durable authority to understand the work journey and make or withhold judgment over goals, scope, design, trade-offs, codebase stewardship, QA, final acceptance, and residual risk. The harness preserves Strategic Agency by making state, decisions, evidence, blockers, and remaining risk explicit outside chat.

### Secret Handle

A display-safe reference to sensitive material such as credentials, tokens, certificates, keys, or other secret values. A secret handle may support evidence or approval scope without storing the raw secret in artifacts, connector manifests, projections, exports, screenshots, logs, summaries, or prompt context. Exact storage behavior stays with [Storage](storage.md), and exact API behavior stays with [MVP API](api/mvp-api.md) and [API Schema Core](api/schema-core.md).

<a id="security-threat-model"></a>

### Security Reference

The reference owner for Harness security assets, trust boundaries, threat categories, control expectations, guarantee levels, and honest security wording. It explains risks such as prompt injection in repo docs, projection tampering, stale approval replay, out-of-scope writes, MCP-unavailable state claims, secret leakage through evidence artifacts, artifact `hash_mismatch`, malicious generated connector files, capability overclaiming, and stale context poisoning. It does not own exact DDL, public API schemas, or Core Model transitions.

### Surface Capability Check

A validator that reports whether a connected agent surface can satisfy required harness behavior. It affects blocked reasons and guarantee display, but it is not a kernel gate.

### Surface Cookbook

The reference document that contains surface-specific connector notes, generated file details, and profile examples. Common integration rules belong in the agent integration document, not the cookbook.

### Subagent Context

A verification independence profile where a subagent or helper reviews work with some inherited implementation context. It is not detached by default and can qualify only when stricter profile metadata proves a real independence boundary.

### Task

The user value unit tracked by the kernel. It carries mode, lifecycle phase, gates, result, close reason, assurance, current summary, decisions, evidence, and projection status.

### Task Level

A display and routing label for task shape: Tiny, Direct, Work, or High-risk Work. Tiny is a profile under `direct`; Direct is small low-risk code or docs work; Work covers features, UX workflow, auth-facing behavior, schema, public API/interface, and multi-file or multi-step delivery; High-risk Work covers auth, security, privacy, secrets, infra, and similarly sensitive categories. Task Level is not a new kernel `mode` enum, gate, schema field, approval, or Write Authorization source.

### TDD Trace

A record of red, green, and refactor evidence for a Change Unit or behavior slice, or a recorded non-TDD justification where policy allows it. A RED target or plan describes the intended failing check; RED evidence means an actual failing test artifact/log/result or another explicit policy-recognized failing-check evidence. When required, the normal path records RED evidence before non-test implementation writes, GREEN evidence after implementation, and refactor/check evidence when relevant, then links the trace to Evidence Manifest coverage. TDD Trace can be execution evidence for a Feedback Loop, but it is not the canonical selected-loop record; a waiver must point back to the alternate Feedback Loop that will prove behavior.

### Tiny Direct Profile

A Direct subprofile for a typo, single docs sentence, or obvious rename where scope, result, and no-user-judgment boundary are immediately clear. It keeps interaction minimal, but it must escalate to ordinary Direct when scope broadens while remaining low-risk and narrow, or when Evidence Manifest coverage, artifact refs, link/render proof, or other evidence beyond the tiny result note is needed. It must route to Work when product decision, material technical decision, architecture choice, public interface/API impact, UX workflow, schema, sensitive category, or multi-step delivery appears.

### Trust Boundary

A separation between Harness surfaces, files, callers, or runtime spaces where input from one side must not be treated as authority on the other side without an owner path. For example, chat text, Product Repository documents, projections, generated connector files, artifact bytes, and MCP caller claims can inform Harness, but they do not become canonical operational state unless Core or another documented owner path accepts their meaning. The trust-boundary map is owned by [Security Reference](security.md).

### Verification

The process of checking whether the result satisfies the relevant criteria. Verification may support assurance when recorded through a valid Eval path and independence profile, but same-session self-check is not detached verification. Verification is separate from approval, Manual QA, final acceptance, and residual-risk acceptance. Exact gate and independence behavior is owned by [Verification Gate](core-model.md#verification-gate) and later/profile-gated [`harness.record_eval`](../later/index.md#later-schema-candidates).

### Verification Gate

The kernel gate for required verification. A user waiver sets `verification_gate=waived_by_user`; it does not create `detached_verified` assurance.

### Verification Independence Profile

A named minimum qualification for an Eval independence context, such as `same_session`, `subagent_context`, `fresh_session`, `fresh_worktree`, `sandbox`, or `manual_bundle`. A passed Eval must satisfy a valid profile before it can support `detached_verified`; the profile must separately name and prove any security-isolation claim.

### Validator Result

A structured result from a validator, including status, guarantee level, target, findings, blocked reasons, and suggested next action.

### Vertical Slice

A Change Unit shape that connects a thin path from trigger/input through domain logic, persistence or state, caller/API boundary, observable output, tests, and optional Manual QA.

### Waiver

An explicit recorded exception to a gate or policy requirement where policy allows it. A waiver names the policy or gate, Task and Change Unit, skipped check or surface, reason, actor, expiry or follow-up when needed, affected gate or close impact, and any close-relevant residual risk that must be visible or accepted through the residual-risk path when required. Verification-risk acceptance, design waiver, and QA waiver are allowed under defined rules only when explicit and scoped. Product-write scope, sensitive-action Approval, required evidence coverage, and required final acceptance are not waived for successful completion. Verification-risk acceptance and QA waiver do not upgrade assurance, imply final acceptance, accept unrelated residual risk, or make skipped checks appear passed.

### Write Authorization

An internal cooperative durable state record created only by non-dry-run `prepare_write.decision=allowed` for one stored `AuthorizedAttemptScope`. The stored scope preserves the intended operation, paths, tools, commands and command classes, product-file-write intent, network targets, secret handles or scope, sensitive categories, baseline, Task, Change Unit, `basis_state_version`, `surface_id`, related user judgment refs, and `guarantee_level` that Core later compares during `record_run`. Distinct compatible non-dry-run `prepare_write` requests create distinct active authorizations; dry-run allowed responses are candidates only, and idempotent replay returns the original committed response. It is single-use for a committed implementation or direct Run, and it does not replace Change Unit scope, sensitive-action Approval, user judgment compatibility, evidence, verification, Manual QA, final acceptance, or residual-risk visibility. It is not OS permission, sandboxing, tamper-proof enforcement, preventive blocking, or isolation.

### Write Authorization Lifecycle Events

The stable event-name set for Write Authorization creation, return, consumption, expiry, staling, revocation, and violation detection. The exact vocabulary and its relationship to `scope_violation_detected` are owned by the [Core Model Stable Event Catalog](core-model.md#stable-event-catalog).

### Write Authority Summary

A user-facing display summary of current write authority for an intended operation, derived from active Change Unit scope, `prepare_write`, approval, baseline, guarantee, user judgment refs, and any Write Authorization ref. It is display, not a separate authority record, and it does not authorize work by itself.
