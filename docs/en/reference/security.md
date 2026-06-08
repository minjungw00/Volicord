# Security Reference

This reference owns the security boundary language for the active Harness MVP plan. The repository is still documentation-only: no Harness Server/runtime implementation, Harness Runtime Home, executable conformance runner, or runtime security proof exists here today. This document describes the boundary future implementation must preserve; it is not evidence that controls are already implemented.

Use this page when security wording, local-access posture, threat/control summaries, or guarantee labels need to stay honest. Use the exact owner documents for exact behavior: [Core Model Reference](core-model.md), [Runtime Boundaries Reference](runtime-boundaries.md), [Storage](storage.md), [Agent Integration Reference](agent-integration.md), [MVP API](api/mvp-api.md), [API Schema Core](api/schema-core.md), [API Errors](api/errors.md), and [Conformance Reference](conformance.md). Future operations candidates stay in [Later Candidate Index: Operations Candidates](../later/index.md#operations-candidates); they do not become active MVP security guarantees by being mentioned here.

## 1. Owns / Does Not Own

This document owns:

- security asset categories and trust-boundary categories
- the meaning of `cooperative`, `detective`, and the current-MVP non-claim boundary for later/profile-gated `preventive` / `isolated` labels
- the rule that security display must match the proven control
- the current MVP security non-claims
- the threat/control summary that keeps Core authority, user-owned judgment, evidence, storage, connectors, and projections distinct
- cross-owner review checks for security claims

This document does not own:

- Core state transitions, gates, `prepare_write`, Write Authorization, `record_run`, `close_task`, user judgments, final acceptance, or residual-risk acceptance; see [Core Model Reference](core-model.md)
- MCP method contracts, shared schemas, public errors, idempotency, replay, or `allowed` / `blocked` response shapes; see [MVP API](api/mvp-api.md), [API Schema Core](api/schema-core.md), and [API Errors](api/errors.md)
- SQLite DDL, Runtime Home layout, storage locks, artifact rows, hashes, migration rules, or storage-owned JSON; see [Storage](storage.md)
- Product Repository / Harness Server / Harness Runtime Home separation, projection authority, artifact boundary, or recovery boundary; see [Runtime Boundaries Reference](runtime-boundaries.md)
- connector `capability_profile` fields, generated manifests, fallback behavior, or surface recipes; see [Agent Integration Reference](agent-integration.md)
- operator command semantics or diagnostic output as active Reference scope; future candidates stay in [Later Candidate Index: Operations Candidates](../later/index.md#operations-candidates)
- executable proof, fixture assertions, runner behavior, or conformance pass/fail; see [Conformance Reference](conformance.md)

## 2. Current MVP Guarantee Level

<a id="honest-guarantee-display"></a>

The current MVP guarantee level is cooperative by default, with limited detective behavior only where the active reference surface can honestly observe the relevant fact and the relevant capability check has actually passed. The active reference surface is represented by a registered `capability_profile`; that profile constrains guarantee display and capability blockers, but it does not create write compatibility or a Write Authorization.

For the current MVP value set, `cooperative` and `detective` are the only `GuaranteeDisplay.level` values. `preventive` and `isolated` are later/profile-gated display names in [Later Candidate Index](../later/index.md), not current MVP schema values or active guarantees.

`allowed` means compatible with current Harness state, owner records, and the active surface capability. It does not mean the operating system permits the action. `blocked` means the Harness protocol, state, owner record, or capability check says the path must not proceed. It does not mean a process was physically stopped before execution.

The reference `capability_profile` has no preventive or isolated posture. Agents and connectors must not infer stronger guarantee labels from user intent, guard/freeze/careful-mode wording, or future profile ideas. Future support fields, covered operations, fallback behavior, errors, and proof paths belong to [Later Candidate Index](../later/index.md) until promoted by an owner document.

Write Authorization is a single-use cooperative Harness record created only by the compatible non-dry-run `prepare_write` path and consumed by compatible `record_run`. It is a Harness record/check, not OS permission, sandboxing, tamper-proof enforcement, physical pre-tool blocking, or isolation.

For the baseline `reference-local-mcp` profile, Write Authorization and product-write Run compatibility are path-level. The profile is cooperative by default and has limited detective support only for observed changed paths after the relevant capability check has passed. It has no command observation, network observation, secret-access observation, native artifact capture, pre-tool blocking, or isolation. Staged artifact registration may exist through the active `stage_artifact` path, but that is not connector artifact capture and does not verify how the artifact was produced.

Local access posture is also a Harness compatibility fact. `registered_local` means the API owner can treat the caller/transport as matching the registered local surface for the requested access class. It does not mean an OS account, editor, shell, package manager, or arbitrary local process is constrained. API access still requires a same-project registered `surface_id`, `surfaces.status=active`, compatible `project_id`/`surface_id`/`task_id`/`expected_state_version` when applicable, and active surface capability. `unavailable`, `mismatch`, and `revoked` posture states route to public API errors and safe diagnostics; they are not proof of a stronger security boundary.

Documentation checks, fixture drafts, examples, and conformance plans do not prove runtime security behavior. They can check wording and future contract intent, but preventive or isolated security claims require an implemented mechanism and proof for the covered operation or boundary.

## 3. Explicit Non-Claims

The current MVP does not provide:

- OS-level permission control
- arbitrary-tool sandboxing
- tamper-proof storage
- default pre-tool blocking
- native artifact capture in the baseline reference profile
- security isolation

These are explicit non-claims even when Harness returns a blocker, records a Write Authorization, validates an artifact hash, detects stale context, reports a capability mismatch, or marks a projection stale. Those outcomes may be cooperative, or detective only after the relevant capability check has passed. They are not preventive or isolated unless another owner documents and proves that exact mechanism for that exact operation.

The MVP also does not claim that local files are trustworthy because they are local, that runtime boundaries are OS isolation boundaries, that MCP reachability is authorization, that chat or generated Markdown can create authority, or that conformance fixture language proves runtime security behavior before implementation exists.

## 4. Assets

Security-sensitive assets include:

| Asset | Why it matters | Owner boundary |
|---|---|---|
| Core-owned state | Defines Harness authority over task scope, user-owned judgment, evidence references, write compatibility, close readiness, and residual-risk status. | [Core Model Reference](core-model.md) owns meaning; [Storage](storage.md) owns persistence. |
| `state.sqlite` and Runtime Home metadata | Persist project registration, current state, event history, surfaces, Write Authorizations, and artifact metadata. | [Storage](storage.md) owns layout and defensive checks; storage is not tamper-proof. |
| Write Authorization and `AuthorizedAttemptScope` | Records one compatible intended write attempt for one compatible consumption. | [Core Model Reference](core-model.md#write-authorization), [MVP API](api/mvp-api.md), and [Storage](storage.md) own exact behavior. |
| `user_judgment` records | Preserve active user-owned product, technical, scope, sensitive-action, final-acceptance, residual-risk, or cancellation judgments, and keep later/reserved QA/verification-risk routes distinct if promoted. | Core/API owners decide exact routes; chat text is input until recorded through the owner path. |
| Artifact refs and evidence metadata | Support evidence and close-readiness claims without trusting raw paths or unregistered bytes. | [API Schema Core](api/schema-core.md), [Storage](storage.md), and [Runtime Boundaries Reference](runtime-boundaries.md) own exact handling. |
| Connector `capability_profile` | Constrains guarantee display, capability blockers, and fallback behavior for the active surface. | [Agent Integration Reference](agent-integration.md) owns fields and refresh rules. |
| Product Repository files and generated projections | Can influence agents and users, but are input or derived display from a Harness perspective. | [Runtime Boundaries Reference](runtime-boundaries.md) and [Projection And Templates Reference](projection-and-templates.md) own display boundaries. |
| Secrets, tokens, PII, and display-safe handles | May leak through artifacts, logs, prompts, projections, manifests, or exports. | Owner paths must prefer redaction, omission, blocked-payload metadata, or display-safe handles. |

## 5. Trust Boundaries

| Boundary | Security posture |
|---|---|
| User conversation and agent surface | Treat chat, memory, pasted text, and approval-like wording as input. User-owned judgments become authority only through the documented `user_judgment` / owner path. |
| Product Repository | Product files, repository rules, generated Markdown, and projections are product work, inputs, or derived display. They are not Harness operational authority by presence or proximity. |
| Harness Server / Installation | The future local control-plane program runs Harness authority checks. It is not a general OS sandbox or arbitrary-tool permission system. |
| Harness Runtime Home | Runtime Home stores Core-owned records and artifacts for future operation. Treat broad local read/write access as tampering and confidentiality risk; do not claim tamper-proof storage. |
| MCP / local API surface | Reachability is not authorization. Core/API validation, project/task/surface compatibility, idempotency, expected state version, local access posture, surface status, and active capability still apply. |
| Connector-generated files | Generated manifests, snippets, prompts, or adapter files may drift or be edited. They do not create authority without the owner path and current `capability_profile`. |
| Artifact store | Artifact bytes are untrusted until registered, related to the owner record, and validated for required integrity/redaction metadata. |
| External tools, commands, and network calls | Local execution can mutate files, leak data, or affect external systems. Cooperative Harness checks do not physically restrain those tools by default. |

## 6. Threat/Control Summary

This summary names the active threat categories without turning the MVP document into a full future threat catalog.

| Threat category | Common path | MVP control posture |
|---|---|---|
| Authority spoofing | Chat, generated Markdown, caller claims, or stale projections pretend to approve, verify, accept, or close work. | Route authority through Core-owned records; fail or hold when MCP/Core authority is unavailable. |
| Out-of-scope write | A product-file path or sensitive category exceeds the active Change Unit, user judgment, sensitive-action permission, or stored `AuthorizedAttemptScope`. Command, network, and secret effects are separate capability and sensitive-action concerns unless a future profile promotes observation for them. | Use cooperative `prepare_write`, single-use Write Authorization, compatible `record_run`, and changed-path detection only where the surface can observe. Reject or block requests that require unobservable command, network, or secret guarantees. |
| Stale context or replay | Stale status text, approvals, projections, baselines, evaluator bundles, or cached state steer current work. | Check current state version, idempotency, freshness, and owner-record compatibility before relying on the input. |
| Artifact or evidence tampering | Bytes, paths, hashes, or metadata are swapped, stale, missing, redacted, blocked, or unrelated to the owner record. | Treat evidence as insufficient or blocked until registration, integrity, redaction, and owner relation checks pass. |
| Secret or PII exposure | Logs, screenshots, traces, prompts, artifacts, projections, manifests, or exports contain sensitive values. | Prefer redaction, omission, blocked-payload notices, display-safe handles, and owner-approved evidence summaries. |
| Capability overclaim | A surface claims blocking, capture, isolation, or MCP reachability beyond its actual `capability_profile`. | Lower the displayed guarantee, mark the claim unverified, return a capability blocker/error, or hold by instruction. |

## 7. Cooperative Behavior

Cooperative behavior means Harness can guide, record, compare, or refuse Harness state-changing paths when the connected agent or surface follows the documented procedure. It is not a hard security boundary.

Examples of cooperative behavior in the current MVP plan:

- a surface calls `prepare_write` before a product write
- Core declines to create a Write Authorization when scope, judgment, sensitive-action permission, state version, or capability is incompatible
- a compatible non-dry-run `prepare_write` creates one consumable Write Authorization
- `record_run` consumes that Write Authorization only when the observed changed paths are compatible to the extent the surface can honestly observe them
- the agent holds product/runtime/code writes by instruction when MCP/Core authority or required capability is unavailable
- generated status text tells the user what Harness can and cannot confirm

Cooperative behavior may keep honest agents aligned with Harness, but it does not stop an arbitrary local process, editor, shell, package manager, or network-capable tool by default.

## 8. Detective Behavior

Detective behavior means Harness can detect, record, or report a supported mismatch after the action or when the relevant fact becomes observable, but in the active MVP only after the relevant capability check has passed. It is after-the-fact checking, not prevention.

Examples of detective behavior in the current MVP plan:

- changed-path comparison after a run, when the surface supports it and the relevant capability check has passed
- artifact `sha256`, `size_bytes`, `content_type`, ownership, availability, redaction, omission, or blocked-payload checks for owner-registered artifact refs where the owner path requires them; these checks are not native artifact capture
- stale state, stale projection, stale connector profile, stale baseline, or stale retrieved-context reporting
- capability mismatch or unsupported-surface reporting
- generated-file or managed-block drift reporting where the owner path supports it

Detective behavior must say what was observed and what remains unverified. For baseline product-write compatibility, the `detective` label is justified only by changed-path observation after the relevant capability check has passed. Unsupported command, network, secret, artifact-capture, blocking, isolation, or external-system effects must not be reported as passed merely because nearby Harness checks succeeded.

## 9. Later Preventive Boundary

Preventive profiles are later/profile-gated material. The current MVP has no default pre-tool blocking profile and no active `preventive` guarantee. Do not describe `prepare_write`, Write Authorization, `allowed`, `blocked`, file locks, hashes, status cards, projections, documentation checks, fixture drafts, guard wording, freeze wording, or careful-mode wording as pre-execution blocking. Future preventive profile fields, covered operations, fallback behavior, errors, and proof expectations stay in [Later Candidate Index](../later/index.md) until promoted.

## 10. Later Isolation Boundary

Isolated profiles are later/profile-gated material. The current MVP has no default `isolated` guarantee and no active security-isolation boundary.

A separate worktree, fresh session, fresh evaluator bundle, or separate process may support freshness, verification independence, or blast-radius reduction. It is not automatically OS sandboxing, permission isolation, tamper-proof storage, or security isolation.

Do not use `isolated` merely because files are local, a bundle is fresh, a connector has a friendly mode name, a tool runs in another directory, or a document says to be careful. Future isolation profile fields, boundaries, covered operations, fallback behavior, errors, and proof expectations stay in [Later Candidate Index](../later/index.md) until promoted.

## 11. Cross-Owner Checks

Before adding or accepting a security claim, check the relevant owner:

| Question | Owner to check |
|---|---|
| Is this a Harness state transition, gate, judgment, write, run, close, waiver, or residual-risk rule? | [Core Model Reference](core-model.md) |
| Is this a public method, response field, error code, idempotency, replay, state-version, `allowed`, or `blocked` behavior? | [MVP API](api/mvp-api.md), [API Schema Core](api/schema-core.md), [API Errors](api/errors.md) |
| Is this about Runtime Home layout, `state.sqlite`, artifact rows, locks, hashes, migration, or storage validation? | [Storage](storage.md) |
| Is this about Product Repository / Harness Server / Harness Runtime Home separation, projection authority, artifact boundary, or recovery boundary? | [Runtime Boundaries Reference](runtime-boundaries.md) |
| Is this about a surface `capability_profile`, MCP availability from the surface, generated manifests, fallback, context push/pull, or guarantee display? | [Agent Integration Reference](agent-integration.md) |
| Is this an operator diagnostic, recovery, export, artifact check, or conformance entrypoint candidate? | [Later Candidate Index: Operations Candidates](../later/index.md#operations-candidates). Runtime conformance proof stays with [Conformance Reference](conformance.md). |
| Is this runtime proof, fixture assertion behavior, or pass/fail language? | [Conformance Reference](conformance.md) |

If the owner document does not define and prove a stronger control, use cooperative or detective wording, mark the claim unsupported, or state the non-claim explicitly. Do not turn future controls, operations-profile ideas, documentation checks, or conformance planning language into active MVP security guarantees.
