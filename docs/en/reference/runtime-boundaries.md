# Runtime Boundaries Reference

This reference defines the active runtime boundary model for future Harness Server planning. It explains which space owns product files, which space runs Harness authority checks, which space persists Core-owned state, and what remains derived display or artifact support.

Runtime boundaries are authority and storage boundaries, not OS isolation boundaries. They separate who may create Harness authority, where Core-owned records and artifacts are persisted, and what remains derived display. They do not imply process isolation, sandboxing, permission enforcement, arbitrary-tool control, tamper-proof storage, or security isolation.

This is source documentation only. No Harness Server/runtime implementation, Harness Runtime Home, generated projection system, conformance runner, or runtime data exists in this repository today. Current repository phase and handoff status remain owned by [MVP Plan](../build/mvp-plan.md#documentation-acceptance-status).

Use [Core Model Reference](core-model.md), [Storage](storage.md), [MVP API](api/mvp-api.md), [API Schema Core](api/schema-core.md), [API Errors](api/errors.md), [Projection And Templates Reference](projection-and-templates.md), [Security Reference](security.md), and [Agent Integration Reference](agent-integration.md) for exact contracts. This page owns only the small boundary model.

## 1. Product Repository

The Product Repository is the user's product workspace. Product source files, tests, repository-level agent rules, and product documentation live there. Product work happens there through the user's normal tools and agent actions.

Product Repository files are not canonical Harness state. A product file can be an input, a changed work target, or product-owned truth about the product, but it does not become Harness operational authority merely because it is near Harness.

The active MVP does not require generated projection files in the Product Repository. A surface may show compact readable outputs such as `status-card`, `judgment-request`, `run-evidence-summary`, `close-result`, or `agent-context-packet`, but those outputs are read-time derived display, not Core-owned state. Editing a rendered projection, Markdown status card, or generated document in the Product Repository does not mutate Harness state; a desired state change must enter through the normal natural-language or active API flow. Editable projection areas, managed Markdown blocks, reconcile queues, persistent projection jobs, and drift repair remain later candidates until promoted by their owners.

This documentation repository is also not the user's Product Repository. It is a documentation-only planning repository intended to become the future Harness Server source repository after documentation acceptance and a separate implementation-planning readiness decision.

## 2. Harness Server / Installation

The Harness Server / Installation is the future local Harness program boundary. It receives local tool/resource calls, runs Core-owned authority checks, records state-changing actions through Core, invokes validators where the active profile requires them, registers artifacts, and renders derived display when projection support is in scope.

The MVP boundary does not require a service fleet or detailed process split. It is compatible with one local process as long as the authority and storage boundaries stay clear: callers ask, Core evaluates and records compatible state changes, storage persists, and display derives from recorded state.

The Harness Server / Installation is not the Product Repository and not the Harness Runtime Home. It may read product files, write product files only through the user's chosen work surface and the documented cooperative Harness checks, and persist Harness records only through Runtime Home storage paths owned by [Storage](storage.md).

Local tool/resource reachability is an entry path, not authorization. `ToolEnvelope.surface_id` is required, but it is only a selector for a same-project `LocalSurfaceRegistration`. The Harness Server must derive a matching `VerifiedSurfaceContext` from the local transport/session/binding before mutating API access or artifact body reads can rely on that surface. The registration must be `status=active`; when applicable, `project_id`, `surface_id`, `surface_instance_id`, `task_id`, and `expected_state_version` must be compatible, and the active surface must have the required access-class capability. Exact access classes and public errors are owned by [MVP API](api/mvp-api.md#shared-request-rules) and [API Errors](api/errors.md).

## 3. Harness Runtime Home

Harness Runtime Home is the per-user or per-installation operational data space. The reference location and exact layout are owned by [Storage](storage.md). Typical future contents include project registration data, project configuration, `state.sqlite`, and artifact storage.

Canonical Harness state lives in Core-owned current records persisted in Runtime Home storage. `state.sqlite.task_events` records audit and ordering history inside the state store; it is not a separate display log and not a replacement for current records.

Runtime Home must be enough to recover Harness operational meaning when chat history is gone or Product Repository projections are stale. Product Repository display can be regenerated from state records and artifact refs where projection support exists. Display cannot replace those records.

Runtime Home files should be treated as private local control data, but Harness does not claim to enforce operating-system permissions, make those files tamper-proof, or isolate them from arbitrary local tools by itself.

## 4. Core mutation authority

Canonical Harness state changes occur only through Core state-changing paths. Core owns the Harness-record authority to create or update records for scope, user-owned judgment, evidence and artifact refs, verification and QA expectations, final acceptance, residual-risk status, and close readiness.

Agents, MCP callers, CLI text, operator output, product files, projection Markdown, templates, status cards, artifact bytes, and chat transcripts do not mutate canonical Harness state by themselves. They also cannot create, modify, or refresh local surface registration. They can provide input or evidence candidates only when the relevant owner path accepts them.

`harness.update_scope`, `prepare_write`, Write Authorization, `record_run`, and `close_task` remain Core/API-owned contracts. Write Authorization is a cooperative Harness record and check. It is not OS permission, sandbox enforcement, tamper-proof protection, physical pre-execution blocking, or a security-isolation mechanism.

`harness.stage_artifact` is a local artifact utility, not a Core mutation authority. It can create a temporary `StagedArtifactHandle`, but it does not create canonical evidence, satisfy gates, register a persistent `ArtifactRef`, or affect close readiness until a compatible Core/API path consumes it.

Exact state transitions, gate effects, row boundaries, idempotency behavior, and response shapes stay with [Core Model Reference](core-model.md), [Storage](storage.md), [MVP API](api/mvp-api.md), [API Schema Core](api/schema-core.md), and [API Errors](api/errors.md).

## 5. Projection derivation boundary

Projections, templates, status cards, generated Markdown, and read-only status resources are read-time derived display. They are rendered from current Core-owned state records and registered artifact refs. They may include freshness, failure, blocker, and next-action information, but those facts remain display of owner records, not a second authority source.

A projection can be stale, missing, failed, or manually edited. None of those conditions changes canonical Harness state by itself. A stale or failed projection may create a visible blocker or freshness warning; it does not roll back Core state, satisfy evidence, pass verification or QA, record final acceptance, accept residual risk, or close a task.

Editing a rendered projection, Markdown status card, generated document, managed block, or front matter field is not a Core mutation path. If the user wants a state change, the request must route through the owning natural-language/API path. The active MVP has no reconcile queue, no persistent projection jobs, and no managed block drift repair.

## 6. Artifact storage boundary

The artifact storage boundary separates durable evidence support from canonical state. The artifact store may hold registered evidence bytes or safe metadata notices. The authoritative Harness meaning comes from the registered `ArtifactRef`, owner relation, integrity metadata, redaction/availability state, and related Core records.

Raw paths, caller claims, chat text, Markdown prose, unregistered files, and artifact bytes without an owner relation are not sufficient evidence by themselves. If required artifact metadata is missing, stale, redacted, unavailable, blocked, or fails integrity checks, Core-owned evidence and close-readiness records must reflect that condition.

Artifact reads use the same authority boundary. A raw artifact-store path is not granted to local callers by default. Reading artifact metadata or bytes requires a registered `ArtifactRef`, compatible project/task context, a matching owner relation, and redaction/availability checks owned by [Storage](storage.md) and [API Schema Core](api/schema-core.md#artifactref). Artifact body reads additionally require a server-verified local surface context for `access_class=artifact_read`; a copied `surface_id`, generated Markdown claim, or Product Repository file cannot authorize access to artifact bytes.

Artifact registration uses only owner-documented `StagedArtifactHandle` values from the active `harness.stage_artifact` utility or compatible existing `ArtifactRef` values. New artifact bytes enter the active MVP only through staging; existing bytes are reused only through `existing_artifact` references. A raw local path, raw log, arbitrary local path string, copied file, `captured_artifact` handle, raw capture-adapter output, or native capture claim is not registration authority in the active MVP.

Artifacts can support evidence, verification, QA, final-acceptance visibility, residual-risk visibility, and close-readiness display. They do not prove success, approve work, accept risk, or close work without the separate owner records and user-owned judgments required by Core.

## 7. Recovery boundary

Recovery is bounded by the same authority model. Recovery may use Runtime Home state records, `state.sqlite.task_events`, artifact refs, integrity metadata, and projection freshness facts to classify what is stale, interrupted, missing, or inconsistent.

Recovery may regenerate derived display, rescan or re-register artifacts through an owner path, mark dependent evidence or views stale or blocked, interrupt stale work records where the owner contract allows it, or route a needed user judgment or Core action. It must not create a second state model.

Recovery cannot infer successful implementation from chat, generated Markdown, stale projections, export text, operator console output, staging paths, or recovery artifacts. It does not satisfy evidence, pass verification or QA, record final acceptance, accept residual risk, or close a task by itself.

## 8. What the current MVP does not isolate

The current MVP boundary is cooperative by default, with detective behavior only for supported observable facts after the relevant capability check has passed, unless a future owner promotes and proves a stronger mechanism for a named operation. It does not claim OS-level permissions, arbitrary-tool sandboxing, permission enforcement, tamper-proof storage, universal pre-tool blocking, or security isolation. `preventive` and `isolated` are not current MVP defaults; they remain profile-gated display values owned by the relevant Reference owners.

Local-only MCP reachability is not authorization. A reachable caller still needs valid Core/API state, project/task/surface compatibility, an active `LocalSurfaceRegistration`, a matching `VerifiedSurfaceContext`, state-version compatibility, and the active surface capability. `MCP_UNAVAILABLE` means reachability itself is unavailable; `LOCAL_ACCESS_MISMATCH` means the registered local transport/session/binding expectation does not match or local access was revoked; `CAPABILITY_INSUFFICIENT` means the recognized surface lacks a required capability. `allowed` means compatible with Harness state and active surface capability. `blocked` means the Harness owner path or capability check says the action should not proceed. Neither word means physical prevention unless a proven preventive mechanism names that exact covered operation.

No surface name, connector recipe, friendly mode label, projection, template, status card, artifact, or documentation check upgrades the guarantee level. Stronger preventive or isolated claims require a documented mechanism, covered operation, owner, and proof path in the relevant Reference owners.
