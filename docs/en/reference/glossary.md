# Glossary Reference

Use this glossary to check Harness terms, capitalization, exact identifiers, and owner routing. It is source documentation for planned Harness behavior only; this repository is documentation-only unless [MVP Plan](../build/mvp-plan.md) says otherwise.

The glossary defines names and routing, not full contracts. Core behavior, API schemas, storage DDL, security guarantees, projection templates, connector behavior, conformance fixtures, and later candidate contracts stay in their owner documents.

## Public terms

Use these terms first in user-facing docs, prompts, and status summaries. Add exact Harness identifiers only when they clarify a blocker, boundary, source reference, or owner route.

| Public term | Meaning | Owner route |
|---|---|---|
| work / task | The thing the user wants completed, answered, investigated, or decided. Use `Task` only for the internal record. | [Core Model](core-model.md) |
| scope | What may change, what is out of scope, and where the agent should stop before continuing. | [Core Model](core-model.md) |
| out of scope | A file, behavior, decision, claim, or action excluded from the current scope. | [Core Model](core-model.md) |
| requirement clarification | Plain-language shaping before implementation planning or write-capable work. Internal references may call this `Discovery`. | [Core Model](core-model.md) |
| work piece | A small scoped portion of work. Internal references may call write-capable scoped work a `Change Unit`. | [Core Model](core-model.md) |
| user-owned judgment | A choice Harness must preserve for the user instead of inferring from agent judgment, evidence, display text, or broad consent. | [Core Model](core-model.md) |
| judgment request | A focused prompt asking the user to make one user-owned judgment. API references use `UserJudgment`. | [MVP API](api/mvp-api.md), [API Schema Core](api/schema-core.md) |
| product judgment | A user-owned choice about product behavior, copy, flow, UX, or user value. | [Core Model](core-model.md) |
| technical judgment | A user-owned choice about architecture, dependency, migration, interface, security/privacy, or material technical direction. | [Core Model](core-model.md) |
| scope judgment | A user-owned choice about scope expansion, non-goal removal, Change Unit boundary, or Autonomy Boundary change. | [Core Model](core-model.md) |
| sensitive-action approval | User permission for one named sensitive step inside a bounded scope. It is not final acceptance or broad approval. | [Core Model](core-model.md) |
| evidence | Durable support for a claim about the work, such as changed paths, diffs, logs, screenshots, inspection notes, or artifact refs. | [API Schema Core](api/schema-core.md), [Storage](storage.md) |
| verification | Recorded correctness checking when an owner path requires it. It does not replace evidence, QA, final acceptance, or residual-risk acceptance. | [Core Model](core-model.md) |
| Manual QA | Human quality review when the surface requires judgment that automated checks or evidence cannot provide. | [Core Model](core-model.md), [Later](../later/index.md) |
| QA waiver | A later/reserved user-owned judgment candidate to waive or limit a QA expectation if a future owner path allows it. It does not create evidence or final acceptance. | [Later](../later/index.md), [Core Model](core-model.md) |
| final acceptance | The user's result judgment when the work path requires acceptance. It does not approve sensitive actions or accept residual risk by itself. | [Core Model](core-model.md) |
| residual risk | Known remaining uncertainty, unchecked condition, limitation, or trade-off that matters to close. | [Core Model](core-model.md) |
| residual-risk acceptance | A user-owned judgment accepting known residual risk. It is distinct from final acceptance and later/reserved QA waiver or verification-risk acceptance. | [Core Model](core-model.md), [API Schema Core](api/schema-core.md) |
| close readiness | Whether work can honestly close now and what remains before it can close. | [Core Model](core-model.md) |
| close blocker | A concrete reason progress, write, or close cannot proceed honestly until fixed or validly deferred. | [Core Model](core-model.md) |
| next safe action | The next action that can proceed without hiding unresolved scope, judgment, evidence, QA, verification, acceptance, or risk. | [API Schema Core](api/schema-core.md) |
| authority boundary | The line between what creates Harness authority and what only informs it. Chat, projections, and reports are not authority. | [Runtime Boundaries](runtime-boundaries.md) |
| derived display | User-visible output rendered from owner records, such as a status card or projection. It does not replace Core-owned state. | [Projection And Templates](projection-and-templates.md) |
| current MVP | The active planned MVP reference scope, not proof that runtime/server implementation exists. | [MVP Plan](../build/mvp-plan.md) |
| later candidate | Future material outside active MVP scope until an owner promotes it with scope, fallback behavior, and proof expectations. | [Later Candidate Index](../later/index.md) |

## Core terms

These terms orient readers to Core authority. Exact lifecycle, gate, close, waiver, and non-substitution semantics live in [Core Model Reference](core-model.md).

| Core term | Short orientation | Owner route |
|---|---|---|
| Core-owned state | Committed owner records and `state.sqlite.task_events` that serve as Harness operational authority. | [Core Model](core-model.md), [Storage](storage.md) |
| `Task` | Internal unit for the user's work, state, blockers, evidence status, close readiness, and result. | [Core Model](core-model.md) |
| `Task.lifecycle_phase` | Persisted Task lifecycle field. Values: `shaping`, `ready`, `executing`, `waiting_user`, `blocked`, `completed`, `cancelled`, `superseded`. `intake` is not a value, and `superseded` is terminal. | [Core Model](core-model.md), [API Schema Core](api/schema-core.md), [Storage](storage.md) |
| `Task.close_reason` | Persisted close-reason detail. Values: `none`, `completed_self_checked`, `completed_with_risk_accepted`, `cancelled`, `superseded`. It is separate from lifecycle and coarse result. | [Core Model](core-model.md), [API Schema Core](api/schema-core.md), [Storage](storage.md) |
| `Task.result` | Coarse Task outcome. Values: `none`, `advice_only`, `completed`, `cancelled`, `superseded`. `passed` and `failed` are not terminal Task results. | [Core Model](core-model.md), [API Schema Core](api/schema-core.md), [Storage](storage.md) |
| `Change Unit` | Active scoped work boundary for write-capable work. It does not authorize a write by itself. | [Core Model](core-model.md) |
| `Autonomy Boundary` | Choices the agent may make inside an active `Change Unit` without asking again. It is not scope grant, approval, or write authority. | [Core Model](core-model.md) |
| `user_judgment` | Canonical record family for user-owned choices. | [Core Model](core-model.md), [API Schema Core](api/schema-core.md) |
| `Gate` | Core compatibility dimension for progress, write, run recording, or close. Requiredness depends on the active owner path. | [Core Model](core-model.md) |
| `Blocker` | Structured reason progress, write, close, or another requested step cannot proceed honestly. | [Core Model](core-model.md) |
| `Write Authorization` | Single-use cooperative Core record created only by compatible non-dry-run `prepare_write`. It is not OS permission or isolation. | [Core Model](core-model.md) |
| `Run` | Committed execution or observation record. Product-write Runs must consume compatible active `Write Authorization`. | [Core Model](core-model.md) |
| `update_scope` | Core path for updating active Task scope and the active Change Unit after intake. Public API method: `harness.update_scope`. | [Core Model](core-model.md), [MVP API](api/mvp-api.md) |
| `prepare_write` | Core pre-write compatibility decision point for product-file writes. Public API method: `harness.prepare_write`. | [Core Model](core-model.md), [MVP API](api/mvp-api.md) |
| `record_run` | Core path for recording execution or observation and consuming compatible `Write Authorization` when needed. Public API method: `harness.record_run`. | [Core Model](core-model.md), [MVP API](api/mvp-api.md) |
| `close_task` | Core completion decision point. Public API method: `harness.close_task`. | [Core Model](core-model.md), [MVP API](api/mvp-api.md) |

## API/schema identifiers

Keep these identifiers exact in schemas, API docs, records, examples, file paths, diagnostic output, and code-like prose. Meanings and value sets are owned by [MVP API](api/mvp-api.md), [API Schema Core](api/schema-core.md), and [API Errors](api/errors.md).

| Identifier | Short orientation | Owner route |
|---|---|---|
| Active MCP methods | `harness.intake`, `harness.status`, `harness.update_scope`, `harness.prepare_write`, `harness.record_run`, `harness.request_user_judgment`, `harness.record_user_judgment`, `harness.close_task`. | [MVP API](api/mvp-api.md) |
| `ToolEnvelope` / `ToolResponseBase` / `ToolError` / `EventRef` | Shared call identity, response, error, and event reference shapes. | [API Schema Core](api/schema-core.md) |
| `StateSummary` / `StateRecordRef` / `NextActionSummary` / `GuaranteeDisplay` | Current-state, owner-ref, next-action, and guarantee-display shapes. | [API Schema Core](api/schema-core.md) |
| `ArtifactRef` / `ArtifactInput` | Public artifact pointer and accepted `record_run` artifact input shapes. | [API Schema Core](api/schema-core.md), [Storage](storage.md) |
| `EvidenceSummary` / `EvidenceCoverageItem` | Compact evidence status and per-claim coverage shapes. | [API Schema Core](api/schema-core.md) |
| `AuthorizedAttemptScope` | Stored scope of one allowed write attempt. | [API Schema Core](api/schema-core.md), [Core Model](core-model.md) |
| `WriteAuthorizationSummary` / `WriteAuthoritySummary` | Public summaries for one `Write Authorization` and the current write-authority position. | [API Schema Core](api/schema-core.md) |
| `RunSummary` / `ObservedChanges` | Public run result and observed-change summary shapes. | [API Schema Core](api/schema-core.md) |
| `UserJudgment` / `UserJudgmentCandidate` / `UserJudgmentResolution` / `RecordUserJudgmentPayload` / `AcceptedRiskInput` | Judgment request, candidate, stored resolution, answer detail, and residual-risk acceptance input shapes. | [API Schema Core](api/schema-core.md) |
| `judgment_kind` | Canonical user judgment kind field. Keep values exact; do not replace them with localized labels. | [API Schema Core](api/schema-core.md) |
| `presentation` | Active compact prompt/detail field. `short` is active; `full` belongs to later full-format presentation. | [API Schema Core](api/schema-core.md), [Later](../later/index.md) |
| `CloseTaskResponse.close_state` | Response-level close status from `harness.close_task`. Values: `ready`, `blocked`, `closed`, `cancelled`, `superseded`. It is separate from persisted `Task.lifecycle_phase`. | [MVP API](api/mvp-api.md) |
| `CloseBlocker` | Structured close/progress blocker result. Prose-only report text is not a blocker result. | [API Schema Core](api/schema-core.md), [API Errors](api/errors.md) |
| `ValidatorResult` | Structured validator output. Active stable validator ID: `surface_capability_check`. | [API Schema Core](api/schema-core.md) |
| sensitive categories | Exact values such as `auth_change`, `destructive_write`, `privacy_or_pii_change`, `data_export`, and `policy_override`. | [API Schema Core](api/schema-core.md) |
| public error codes | Stable public errors such as `MCP_UNAVAILABLE`, `LOCAL_ACCESS_MISMATCH`, `CAPABILITY_INSUFFICIENT`, and `PROJECTION_STALE`. | [API Errors](api/errors.md) |

## Storage terms

Storage terms identify where future Harness records live. Exact table roles, JSON `TEXT` rules, state clocks, locks, migrations, and artifact handling are owned by [Storage](storage.md).

| Storage term | Short orientation | Owner route |
|---|---|---|
| Product Repository | The user's product workspace. Product files are not Harness operational authority merely because they are nearby. | [Runtime Boundaries](runtime-boundaries.md) |
| Harness Server / Installation | Future local Harness control-plane program. It is not a general OS sandbox or permission system. | [Runtime Boundaries](runtime-boundaries.md) |
| Harness Runtime Home | Per-user/per-installation operational data home for registry, project state, and artifacts. | [Runtime Boundaries](runtime-boundaries.md), [Storage](storage.md) |
| runtime identity files | `registry.sqlite`, `project.yaml`, and `state.sqlite` identify the runtime home, static project configuration, and project-local Core state. | [Storage](storage.md) |
| active storage records | Active table names include `project_state`, `surfaces`, `tasks`, `change_units`, `user_judgments`, `write_authorizations`, `runs`, `artifacts`, `artifact_links`, `evidence_summaries`, `blockers`, `task_events`, and `tool_invocations`. | [Storage](storage.md) |
| JSON `TEXT` columns | SQLite `TEXT` columns that store owner-shaped JSON after Core/API/storage validation. They are not arbitrary JSON containers. | [Storage](storage.md) |
| artifact storage links | `artifacts` and `artifact_links` register evidence bytes or safe metadata and connect them to owner records. Links do not satisfy gates by themselves. | [Storage](storage.md) |
| event and replay storage | `task_events` is the committed mutation audit trail; `tool_invocations` stores committed idempotency replay rows. | [Storage](storage.md) |
| state clocks and hashes | `state_version`, `project_state.state_version`, `tasks.state_version`, `tree_hash`, and `request_hash` support stale-state, baseline, and idempotency checks. | [Storage](storage.md), [API Errors](api/errors.md) |

## Security guarantee terms

Security wording must match the control the owner docs define and prove. Exact guarantee meanings and non-claims are owned by [Security Reference](security.md).

| Security term | Meaning | Owner route |
|---|---|---|
| `cooperative` | Harness can guide, record, compare, or refuse Harness state-changing paths when the connected surface follows the procedure. It is not hard blocking. | [Security](security.md), [Agent Integration](agent-integration.md) |
| `detective` | Harness can detect, record, or report supported facts after an action or when they become observable. It is not prevention. | [Security](security.md), [Agent Integration](agent-integration.md) |
| `preventive` | A claim that a named mechanism can block a covered operation before execution. The current MVP has no default preventive claim. | [Security](security.md) |
| `isolated` | A claim that a named and proven separation boundary isolates one thing from another for a covered operation. The current MVP has no default isolation claim. | [Security](security.md), [Runtime Boundaries](runtime-boundaries.md) |
| honest guarantee display | User-visible guarantee text must match `capability_profile` facts and owner-proof level. Unsupported stronger claims must be lowered or blocked. | [Security](security.md), [API Errors](api/errors.md) |
| explicit non-claims / trust boundary | The current MVP does not provide OS-level permission control, arbitrary-tool sandboxing, tamper-proof storage, default pre-tool blocking, or security isolation. | [Security](security.md), [Runtime Boundaries](runtime-boundaries.md) |

## Agent/context terms

Agent and connector terms explain how a surface should use owner records with low context cost. Exact connector behavior is owned by [Agent Integration Reference](agent-integration.md).

| Agent/context term | Short orientation | Owner route |
|---|---|---|
| agent surface / `surface_id` | Connected environment and API caller identifier. Surface name or `surface_id` alone does not grant capability or authority. | [Agent Integration](agent-integration.md) |
| `capability_profile` | Declared and refreshed facts about what the surface can actually do, including MCP posture, observation, capture, guard, and isolation support. | [Agent Integration](agent-integration.md), [Security](security.md) |
| connector manifest | Connector-managed path, snippet, managed block hash, profile freshness, drift, and fallback summary. | [Agent Integration](agent-integration.md) |
| always-on context | One-screen current context: task summary, scope, pending judgments, blockers, next safe actions, evidence gaps, close blockers, residual risk, guarantee level, and fresh refs. | [Agent Integration](agent-integration.md) |
| phase-relevant context / push-pull | Push compact current context; pull only the owner sections needed for the next action. | [Agent Integration](agent-integration.md), [Reference Index](README.md) |
| Role Lens | Read-only posture guidance. A lens recommendation has no authority until an owner path records the action. | [Agent Integration](agent-integration.md) |
| reference local MCP surface | Active reference integration profile `reference-local-mcp`, with cooperative behavior and limited detective behavior only where supported. | [Agent Integration](agent-integration.md) |
| fallback behavior | Connector response when Core, MCP, projections, local access, or capability is unavailable or insufficient. | [Agent Integration](agent-integration.md), [API Errors](api/errors.md) |

## Later terms

Later terms are candidates or delivery labels. They are not active API/schema/storage contracts, fixture bodies, runtime behavior, generated artifacts, or current MVP requirements unless an owner promotes them.

| Later term | Current status | Owner route |
|---|---|---|
| Context Index | Later read-only retrieval support. It cannot authorize writes, satisfy gates, accept risk, or close work. | [Later](../later/index.md) |
| Journey Card / Journey Spine | Later continuity display. It can help orientation when enabled and fresh, but it is not Core-owned state. | [Later](../later/index.md) |
| Browser QA Capture | Later capture support candidate. It is not Manual QA, final acceptance, or detached verification by itself. | [Later](../later/index.md) |

## Retired / compatibility terms

Keep these only where they prevent confusion with compatibility labels. Do not use them as primary concepts in new active docs.

| Term | Compatibility note | Current route |
|---|---|---|
| Decision Packet | Full-format later presentation for user judgment. It is not a required active user-path format. | [API Schema Core](api/schema-core.md), [Later](../later/index.md) |
| `MVP-1` | Older label for the current active MVP scope. Use it only where compatibility explanation is needed; prefer current MVP in active docs. | [MVP Plan](../build/mvp-plan.md) |
