# Glossary Reference

Use this glossary to check Harness terms, capitalization, exact identifiers, and owner routing. It is source documentation for planned Harness behavior only; this repository is still documentation-only, and current repository status stays in [MVP Plan](../build/mvp-plan.md#documentation-acceptance-status).

This page is a compact lookup aid. It does not define Core behavior, API schemas, storage DDL, security guarantees, projection templates, connector contracts, conformance fixtures, or later-profile contracts. Follow the owner links for exact behavior.

## Public terms

Use these terms first in user-facing docs, prompts, and status summaries. Add exact Harness identifiers only when they explain a boundary, blocker, source ref, or owner link.

| Public term | Meaning | Owner route |
|---|---|---|
| work / task | The thing the user wants completed, answered, investigated, or decided. Use `Task` only for the internal record. | [Core Model](core-model.md#entity-model) |
| scope | What may change, what is out of scope, and where the agent should stop before continuing. | [Core Model](core-model.md#entity-model) |
| out of scope | A file, behavior, decision, claim, or action excluded from the current scope. | [Core Model](core-model.md#prepare_write) |
| requirement clarification | Plain-language shaping before implementation planning or write-capable work. Internal references may call this `Discovery`. | [Core Model](core-model.md#entity-model) |
| work piece | A small scoped portion of work. Internal references may call write-capable scoped work a `Change Unit`. | [Core Model](core-model.md#entity-model) |
| user-owned judgment | A choice Harness must preserve for the user instead of inferring from agent judgment, evidence, projection text, or broad consent. | [Core Model](core-model.md#judgment-route-boundaries) |
| judgment request | A focused prompt asking the user to make one user-owned judgment. API references use `UserJudgment`. | [MVP API](api/mvp-api.md#harnessrequest_user_judgment) |
| product judgment | A user-owned choice about product behavior, copy, flow, UX, or user value. | [Core Model](core-model.md#judgment-route-boundaries) |
| technical judgment | A user-owned choice about architecture, dependency, migration, interface, security/privacy, or material technical direction. | [Core Model](core-model.md#judgment-route-boundaries) |
| scope judgment | A user-owned choice about scope expansion, non-goal removal, Change Unit boundary, or Autonomy Boundary change. | [Core Model](core-model.md#judgment-route-boundaries) |
| sensitive-action approval | User permission for one named sensitive step inside a bounded scope. It is not final acceptance or broad approval. | [Core Model](core-model.md#judgment-route-boundaries) |
| evidence | Durable support for a claim about the work, such as changed paths, diffs, logs, screenshots, inspection notes, or artifact refs. | [API Schema Core](api/schema-core.md#evidence-and-pre-write-scope-schemas) and [Storage](storage.md#6-artifact-references) |
| check | An ordinary confirmation such as a test, diff review, inspection, or source lookup. Use `Verification` only for the formal recorded path. | [Core Model](core-model.md#evidence-verification-qa-final-acceptance-and-risk) |
| verification | Recorded correctness checking when an owner path requires it. It does not replace final acceptance, QA, evidence, or residual-risk acceptance. | [Core Model](core-model.md#evidence-verification-qa-final-acceptance-and-risk) |
| Manual QA | Human quality checking when the surface requires judgment that automated checks or evidence cannot provide. | [Core Model](core-model.md#evidence-verification-qa-final-acceptance-and-risk) and [Later](../later/index.md#assurance-candidates) |
| final acceptance | The user's result judgment when the work path requires acceptance. It does not approve sensitive actions or accept residual risk by itself. | [Core Model](core-model.md#judgment-route-boundaries) |
| close readiness | Whether work can honestly close now and what remains before it can close. | [Core Model](core-model.md#close_task) |
| close blocker | A concrete reason progress, write, or close cannot proceed honestly until resolved or validly deferred. | [Core Model](core-model.md#invalid-state-combinations) |
| residual risk | Known remaining uncertainty, unchecked condition, limitation, or trade-off that matters to close. | [Core Model](core-model.md#13-residual-risk) |
| next safe action | The next action that can proceed without hiding unresolved scope, judgment, evidence, QA, verification, acceptance, or risk. | [API Schema Core](api/schema-core.md#nextactionsummary) |
| authority boundary | The line between what creates Harness authority and what only informs it. Chat, projections, and reports are not authority. | [Runtime Boundaries](runtime-boundaries.md#4-core-mutation-authority) |
| derived display | User-visible output rendered from owner records, such as a status card or projection. It does not replace Core-owned state. | [Projection And Templates](projection-and-templates.md#authority-boundary) |
| current MVP | The active planned MVP reference scope, not proof that runtime/server implementation exists. | [MVP Plan](../build/mvp-plan.md#active-current-mvp-slice) |
| later candidate | Future or profile material outside active MVP scope until an owner promotes it with scope, fallback behavior, and proof expectations. | [Later Candidate Index](../later/index.md#boundary) |

## Core terms

These terms orient readers to Core authority. Exact lifecycle, gate, and close semantics live in [Core Model Reference](core-model.md).

| Core term | Short orientation | Owner route |
|---|---|---|
| Core-owned state | The committed owner records and `state.sqlite.task_events` that serve as Harness operational authority. | [Core Model](core-model.md#kernel-invariants), [Storage](storage.md) |
| Task | The internal unit for the user's work, state, blockers, evidence status, close readiness, and result. | [Core Model](core-model.md#entity-model) |
| Change Unit | The active scoped work boundary for write-capable work. It does not authorize a write by itself. | [Core Model](core-model.md#entity-model) |
| Autonomy Boundary | The choices the agent may make inside an active Change Unit without asking again. It is not scope grant, approval, or write authority. | [Core Model](core-model.md#autonomy-boundary) |
| `user_judgment` | The canonical record family for user-owned choices. | [Core Model](core-model.md#judgment-route-boundaries), [API Schema Core](api/schema-core.md#userjudgment) |
| Gate | A Core compatibility dimension for progress, write, run recording, or close. Requiredness depends on the active owner path. | [Core Model](core-model.md#gates) |
| Blocker | A structured reason progress, write, close, or another requested step cannot proceed honestly. | [Core Model](core-model.md#invalid-state-combinations) |
| Write Authorization | A single-use cooperative Core record created only by compatible non-dry-run `prepare_write`. It is not OS permission or isolation. | [Core Model](core-model.md#write-authorization) |
| Run | A committed execution or observation record. Product-write Runs must consume compatible active Write Authorization. | [Core Model](core-model.md#record_run) |
| `prepare_write` | Core's pre-write compatibility decision point for product-file writes. Public API method: `harness.prepare_write`. | [Core Model](core-model.md#prepare_write), [MVP API](api/mvp-api.md#harnessprepare_write) |
| `record_run` | Core's path for recording execution or observation and consuming compatible Write Authorization when needed. Public API method: `harness.record_run`. | [Core Model](core-model.md#record_run), [MVP API](api/mvp-api.md#harnessrecord_run) |
| `close_task` | Core's completion decision point. Public API method: `harness.close_task`. | [Core Model](core-model.md#close_task), [MVP API](api/mvp-api.md#harnessclose_task) |

## API/schema identifiers

Keep these exact in schemas, API docs, records, examples, file paths, diagnostic output, and code-like prose. Meanings and value sets are owned by [MVP API](api/mvp-api.md), [API Schema Core](api/schema-core.md), and [API Errors](api/errors.md).

| Identifier | Short orientation |
|---|---|
| Active MCP methods | `harness.intake`, `harness.status`, `harness.prepare_write`, `harness.record_run`, `harness.request_user_judgment`, `harness.record_user_judgment`, `harness.close_task`. |
| Request/response base shapes | `ToolEnvelope`, `ToolResponseBase`, `ToolError`, and `EventRef` carry shared call identity, state version, errors, validator results, and event refs. |
| State/display summary shapes | `StateSummary`, `StateRecordRef`, `NextActionSummary`, and `GuaranteeDisplay` orient callers to current state, refs, next actions, and guarantee display. |
| `ArtifactRef` / `ArtifactInput` | Public artifact pointer and accepted record-run artifact input shapes. |
| `EvidenceSummary` / `EvidenceCoverageItem` | Compact evidence status and per-claim coverage shapes. |
| `AuthorizedAttemptScope` | The exact stored scope of one allowed write attempt. |
| `WriteAuthorizationSummary` / `WriteAuthoritySummary` | Public summaries for one Write Authorization and the current write-authority position. |
| `RunSummary` / `ObservedChanges` | Public run result and observed change summary shapes. |
| Judgment shapes | `UserJudgment`, `UserJudgmentCandidate`, `RecordUserJudgmentPayload`, and `AcceptedRiskInput` represent a judgment request, candidate, resolution, or residual-risk acceptance input. |
| `judgment_kind` | Canonical user judgment kind field. Active values include `product_decision`, `technical_decision`, `scope_decision`, `sensitive_approval`, `qa_waiver`, `verification_risk_acceptance`, `final_acceptance`, `residual_risk_acceptance`, and `cancellation`. |
| `presentation` | Active MVP prompt/detail field. `short` is active; `full` Decision Packet presentation is later/profile material. |
| `CloseBlocker` | Structured close/progress blocker result. Prose-only report text is not a blocker result. |
| `ValidatorResult` | Structured validator output. Active stable validator ID: `surface_capability_check`. |
| Sensitive categories | Exact sensitive-category values such as `auth_change`, `destructive_write`, `secret_access`, `privacy_or_pii_change`, and `policy_override` are owned by API Schema Core. |
| Public error codes | Stable public errors such as `MCP_UNAVAILABLE`, `LOCAL_ACCESS_MISMATCH`, `CAPABILITY_INSUFFICIENT`, and `PROJECTION_STALE` are owned by API Errors. |

## Storage terms

Storage terms identify where future Harness records live. Exact table roles, JSON `TEXT` rules, state clocks, locks, migrations, and artifact handling are owned by [Storage](storage.md).

| Storage term | Short orientation | Owner route |
|---|---|---|
| Product Repository | The user's product workspace. Product files are not Harness operational authority merely because they are nearby. | [Runtime Boundaries](runtime-boundaries.md#1-product-repository) |
| Harness Server / Installation | The future local Harness control-plane program. It is not a general OS sandbox or permission system. | [Runtime Boundaries](runtime-boundaries.md#2-harness-server--installation) |
| Harness Runtime Home | The per-user/per-installation operational data home for registry, project state, and artifacts. | [Runtime Boundaries](runtime-boundaries.md#3-harness-runtime-home), [Storage](storage.md#2-runtime-home-identity) |
| Runtime identity files | `registry.sqlite` stores Runtime Home identity and minimal project registration; `project.yaml` stores static project configuration; `state.sqlite` stores project-local Core state. | [Storage](storage.md#2-runtime-home-identity) |
| Active storage records | `project_state`, `surfaces`, `tasks`, `change_units`, `user_judgments`, `write_authorizations`, `runs`, `artifacts`, `artifact_links`, `evidence_summaries`, `blockers`, `task_events`, and `tool_invocations`. | [Storage](storage.md#4-tables) |
| JSON `TEXT` columns | SQLite `TEXT` columns that store owner-shaped JSON after Core/API/storage validation. They are not arbitrary JSON containers. | [Storage](storage.md#5-json-text-columns) |
| Artifact storage links | `artifacts` and `artifact_links` register evidence bytes or safe metadata and connect them to owner records. Links do not satisfy gates by themselves. | [Storage](storage.md#6-artifact-references) |
| Event and replay storage | `task_events` is the committed mutation audit trail; `tool_invocations` stores committed idempotency replay rows. | [Storage](storage.md#7-idempotency-and-event-meaning) |
| State clocks and hashes | `state_version`, `project_state.state_version`, `tasks.state_version`, `tree_hash`, and `request_hash` support stale-state, baseline, and idempotency checks. | [Storage](storage.md), [API Errors](api/errors.md) |

## Security guarantee terms

Security wording must match the control that owner docs define and prove. Exact guarantee meanings and non-claims are owned by [Security Reference](security.md).

| Security term | Meaning |
|---|---|
| `cooperative` | Harness can guide, record, compare, or refuse Harness state-changing paths when the connected surface follows the procedure. It is not hard blocking. |
| `detective` | Harness can detect, record, or report supported facts after an action or when they become observable. It is not prevention. |
| `preventive` | A claim that a named mechanism can block a covered operation before execution. The current MVP has no default preventive claim. |
| `isolated` | A claim that a named and proven separation boundary isolates one thing from another for a covered operation. The current MVP has no default isolation claim. |
| honest guarantee display / capability overclaim | User-visible guarantee text must match `capability_profile` facts and owner-proof level. Unsupported stronger claims must lower display, return a blocker/error, or hold by instruction. |
| explicit non-claims / trust boundary | The current MVP does not provide OS-level permission control, arbitrary-tool sandboxing, tamper-proof storage, default pre-tool blocking, or security isolation. |

## Agent/context terms

Agent and connector terms explain how a surface should use owner records with low context cost. Exact connector behavior is owned by [Agent Integration Reference](agent-integration.md).

| Agent/context term | Short orientation |
|---|---|
| agent surface / `surface_id` | The connected environment and API caller identifier. Surface name or `surface_id` alone does not grant capability or authority. |
| `capability_profile` | Declared and refreshed facts about what the surface can actually do, including MCP posture, observation, capture, guard, and isolation support. |
| connector manifest | Generated manifest for connector-managed paths, snippets, managed block hashes, profile freshness, drift, and fallback behavior. |
| always-on context | One-screen current context: task summary, scope, pending judgments, blockers, next safe actions, evidence gaps, close blockers, residual risk, guarantee level, and fresh refs. |
| phase-relevant context / push-pull | Push compact current context; pull only the owner sections needed for planning, write preparation, evidence review, close readiness, judgment request, or recovery. |
| Role Lens | Read-only posture guidance. A lens recommendation has no authority until an owner path records the action. |
| reference local MCP surface | The active reference integration profile, `reference-local-mcp`, with cooperative behavior and limited detective behavior only where supported. |
| fallback behavior | The connector response when Core, MCP, projections, local access, or capability is unavailable or insufficient. |

## Later terms

Later terms are candidates or delivery labels. They are not active API/schema/storage contracts, fixture bodies, runtime behavior, generated artifacts, or MVP-1 requirements unless an owner promotes them.

| Later term | Current status | Owner route |
|---|---|---|
| Engineering Checkpoint | First future internal authority-loop smoke. It is not the product MVP. | [MVP Plan](../build/mvp-plan.md#first-internal-smoke-target) |
| `Kernel Smoke` | Narrow future smoke-check authoring label under Engineering Checkpoint; not a stage name. | [MVP Plan](../build/mvp-plan.md#first-internal-smoke-target) |
| MVP-1 User Work Loop | First narrow user-value milestone after the internal smoke target. | [MVP Plan](../build/mvp-plan.md#user-work-loop) |
| Assurance Profile | Later hardening for assurance behavior. | [Later](../later/index.md#assurance-candidates) |
| Operations Profile | Later hardening for operations and handoff behavior. | [Later](../later/index.md#operations-candidates) |
| Roadmap | Future scope unless owner docs promote and prove an item. | [Later](../later/index.md#roadmap-candidates) |
| hardened local reference target | Umbrella target reached after MVP-1 by completing owner-defined Assurance Profile and Operations Profile work; not an extra stage or suite. | [Translation Guide](../maintain/translation-guide.md) |
| Context Index | Later read-only retrieval support. It cannot authorize writes, satisfy gates, accept risk, or close work. | [Later](../later/index.md#roadmap-candidates) |
| Journey Card / Journey Spine | Later continuity display. It helps orientation when enabled and fresh, but it is not Core-owned state. | [Later](../later/index.md#later-template-candidates) |
| Browser QA Capture | Roadmap capture support candidate. It is not Manual QA, final acceptance, or detached verification by itself. | [Later](../later/index.md#roadmap-candidates) |

## Retired / compatibility terms

Keep these only where they prevent confusion with compatibility payloads or labels. Do not use them as primary concepts in new active docs.

| Term | Compatibility note | Current route |
|---|---|---|
| Decision Packet | Full-format user-judgment presentation and compatibility label. The active MVP uses compact `presentation=short`; `presentation=full` is later/profile material. | [API Schema Core](api/schema-core.md#userjudgment), [Later](../later/index.md#assurance-candidates) |
| `request_user_decision` / `record_user_decision` | Compatibility aliases for `request_user_judgment` / `record_user_judgment`. | [API Schema Core](api/schema-core.md#stage-specific-active-value-sets) |
| `judgment_type`, `judgment_domain`, `decision_kind`, `decision_profile` | Compatibility aliases. Prefer `judgment_kind`, route-specific payload validation, and `presentation`. | [API Schema Core](api/schema-core.md#userjudgment) |
| `display_label` | Compatibility or response-only display label when a surface exposes that name. It is not an active canonical schema/storage field; render labels from `judgment_kind` and locale. | [API Schema Core](api/schema-core.md#userjudgment), [Storage](storage.md#4-tables) |
| `MCP_SERVER_UNAVAILABLE` / `SURFACE_MCP_UNAVAILABLE` | Diagnostic conditions. The stable public availability code is `MCP_UNAVAILABLE`. | [Agent Integration](agent-integration.md#8-fallback-behavior), [API Errors](api/errors.md) |
