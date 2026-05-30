# Security Threat Model Reference

## What this document helps you do

Use this reference to identify Harness security assets, trust boundaries, threat categories, and control expectations before runtime implementation planning.

It is a lookup document for implementers, operators, connector authors, and conformance authors who need to keep local authority boundaries explicit. It does not replace the architecture, API, storage, kernel, connector, or operations owner documents.

This is reference documentation. It does not authorize runtime/server implementation, generated operational files, executable fixtures, or runtime data before the documentation set is accepted for implementation planning. The first runnable target is v0.1 Core Authority Slice, with Kernel Smoke as its narrow conformance authoring profile. The first product MVP target is v0.2 User-Facing Harness MVP. v0.3 and v0.4 harden assurance, stewardship, operations, and handoff behavior, and v1+ Expansion remains roadmap scope unless owner docs promote and prove it.

## Read this when

- You are deciding which files, calls, artifacts, or generated connector outputs are security-sensitive.
- You need to explain why a repo document, projection, generated file, chat transcript, or caller claim is not operational authority.
- You are reviewing MCP exposure, artifact handling, connector generation, stale context, approval replay, or capability claims.
- You need to decide whether cooperative, detective, preventive, or isolated wording is honest for a security-sensitive path.
- You are writing operator diagnostics or conformance coverage that names security or threat-model findings.

## Before you read

Use [Runtime Architecture Reference](runtime-architecture.md) for the runtime spaces, Core process model, transaction ordering, and guarantee-level definitions. Use [Agent Integration Reference](agent-integration.md) for connector capability profiles, generated manifests, context push/pull, and fallback display. Use [Operations And Conformance Reference](operations-and-conformance.md) for `doctor`, `serve mcp`, artifact checks, recover, and reconcile. Use [Conformance Fixtures Reference](conformance-fixtures.md) for fixture semantics.

Use [MCP API And Schemas](mcp-api-and-schemas.md) for public tool envelopes, errors, and replay behavior. Use [Storage And DDL](storage-and-ddl.md) for exact storage layout, artifact rows, and DDL. Use [Kernel Reference](kernel.md) for state transitions, gates, Approval, `prepare_write`, Write Authorization, acceptance, residual risk, and close.

This document links to those exact contracts instead of duplicating them.

## Main idea

Harness is a local authority layer, not a general operating-system security boundary. A local file, local process, generated connector output, external command, or agent surface can try to influence Harness, but it does not become authority just because it is nearby.

Canonical operational meaning flows through Core-owned state-changing paths. Product repository documents, chat text, generated connector files, projections, artifacts, external command output, MCP caller claims, and remembered context are inputs until the relevant owner path accepts them.

Security display must match the real control. Cooperative and detective surfaces can hold by instruction or detect after action. Preventive wording requires fixture-proven pre-tool blocking for the covered operation, and isolated wording requires an actual separation boundary. High-risk work must not rely on cooperative-only claims when the work requires preventive or isolated controls.

## Reference scope

This document owns:

- threat-model concepts and vocabulary
- the security asset map
- the trust-boundary map
- the required threat and control categories
- the rule that high-risk work cannot depend on cooperative-only claims when preventive or isolated controls are required
- the non-substitution boundary between threat-model concepts and exact DDL, API schemas, and kernel transitions

## Not covered here

This document does not own:

- public MCP request/response schemas, public error shapes, or idempotency/replay contracts; see [MCP API And Schemas](mcp-api-and-schemas.md)
- SQLite DDL, storage layout, canonical enum hardening, artifact row shape, or exact file layout; see [Storage And DDL](storage-and-ddl.md)
- kernel state transitions, gates, Approval lifecycle, `prepare_write`, Write Authorization, acceptance, residual-risk acceptance, or close; see [Kernel Reference](kernel.md)
- operator command semantics, diagnostic severity baselines, or recover/reconcile/export behavior; see [Operations And Conformance Reference](operations-and-conformance.md)
- fixture assertion semantics; see [Conformance Fixtures Reference](conformance-fixtures.md)
- connector capability-profile field details, generated-manifest contracts, or surface recipes; see [Agent Integration Reference](agent-integration.md) and [Surface Cookbook](surface-cookbook.md)
- projection template bodies or managed-block rendering rules; see [Document Projection Reference](document-projection.md)
- runtime implementation, generated operational files, executable fixtures, runtime data, or production deployment

## Baseline assumptions

The v0.1 Core Authority Slice and staged-delivery default are local-first. The expected baseline is a user-controlled Product Repository, a local Harness Server / Installation, a Harness Runtime Home, an MCP server exposed only through the registered local connector posture, and one or more connected agent surfaces.

Local-first does not mean every local process is trusted. Another process, stale connector configuration, broad file permissions, a forwarded port, a hand-edited generated file, or stale chat context may still affect what an agent sees or does. Harness therefore treats nearby surfaces as separate trust zones and accepts operational meaning only through owner paths.

Remote or shared MCP exposure remains outside the v0.1 baseline and staged delivery unless owner documentation and conformance promote and prove a specific connector posture. A promoted posture must still show the access-control contract, secret/PII handling, redaction or omission behavior, honest guarantee display, and Core validation that remain in force.

## Security assets

| Asset | Security concern | Boundary |
|---|---|---|
| `state.sqlite` | Canonical current operational records can be spoofed, replayed, or corrupted if edited outside Core. | Exact storage layout belongs to [Storage And DDL](storage-and-ddl.md). State-changing meaning must flow through [Kernel Reference](kernel.md) and Core transaction paths. |
| `state.sqlite.task_events` | Event history can be forged or rewritten if direct file edits are accepted as history. | Events are state-store history, not chat logs or report prose. Recovery adds compensating records rather than treating external edits as authority. |
| Artifact store | Evidence bytes can leak secrets, be poisoned, be oversized, or mismatch registered metadata. | Artifact refs, hashes, size, content type, redaction state, retention, and ownership are validated through storage and operations owner paths. |
| Projections | Markdown reports can be stale, tampered with, prompt-injected, or mistaken for state. | Projections are readable views or proposal surfaces. Freshness, managed blocks, and reconcile behavior are owned by [Document Projection Reference](document-projection.md). |
| MCP server | A caller can be unexpected, stale, remote, forwarded, or unable to reach Core while still claiming state changes. | Public tools enter through Core and the API-owned envelope, state-version, idempotency, and error contracts. |
| Connector-generated files | Generated instructions, manifests, MCP snippets, prompts, or adapter files can drift, be hand-edited, or become malicious context. | Generated or managed files are tracked by connector manifests and drift reporting. They do not create Task state or authority by themselves. |
| Local repo | Product code, tests, repo docs, AGENTS-style rules, and human-editable areas may contain prompt injection or stale facts. | The Product Repository is a work and input space, not the operational state store. Product writes still require the existing scope, Approval, and write-authority paths. |
| External commands | Shell commands, tools, tests, package managers, deploy tools, and network calls can mutate files, leak data, or create side effects. | High-risk command, path, network, and secret use must be bounded by the relevant Change Unit, Approval, connector capability, and operator controls. |
| Secret handles | A handle can point to sensitive material without exposing the raw value, but misuse can still leak or broaden access. | Raw secrets should not become artifacts or projections. Store display-safe handles or omission notes where owner docs allow them; never store raw token or secret values in connector manifests. |

## Trust boundaries

| Boundary | Trust risk | Required posture |
|---|---|---|
| User conversation surface | Chat may contain intent, approval-like words, stale memory, or malicious pasted content. | Treat conversation as input. User-owned judgment must be recorded through the relevant Decision Packet, Approval, acceptance, or residual-risk path when it affects authority. |
| Agent surface | The surface may skip MCP, overclaim capability, continue from stale context, or perform actions outside scope. | Capability must be declared for the actual host/profile and displayed honestly. Product/runtime/code writes hold when required authority cannot be checked. |
| MCP server | Local endpoints can be reached by the wrong caller, stale configuration, forwarded ports, or weak socket/config permissions. | Use local process, local socket, localhost-loopback, or a promoted connector posture with documented access control. Validate public envelopes through Core. |
| Core | Core is the authority boundary for canonical mutation. | Core alone commits operational state changes and owner-record effects. No report, projection, generated file, or caller claim bypasses Core. |
| Runtime Home | Local files may be read or written by unrelated users, shared containers, or off-profile automation. | Treat broad read/write access as tampering or confidentiality risk. Direct edits are invalid until Core, recovery, or artifact-integrity paths validate the effect. |
| Product Repository | Human-editable docs, generated Markdown, product files, and repo rules can influence agent behavior. | Repo files are inputs, product work, or projections. They do not become canonical operational state by being present in the repo. |
| Artifact store | Staged or committed evidence may contain secrets, be swapped, or fail integrity checks. | Validate paths, task/run ownership, hashes, sizes, content types, redaction/omission/block state, and retention before relying on the bytes. |
| External tools/network | Commands and network calls can affect systems outside Harness and may have irreversible side effects. | Use least-privilege tools and explicit command/path/network/secret bounds for high-risk work. Require stronger controls when cooperative holds are not enough. |

## Threat and control map

| Threat | Typical path | Required controls |
|---|---|---|
| Prompt injection in repo docs | A repo document, old projection, or generated instruction tells the agent to ignore Harness or spoof authority. | Keep context refs-first, treat repo docs as input, route authority through Core, and use current status/Journey/projection freshness instead of old prose. |
| Projection tampering | A managed Markdown report is edited to make a Task look approved, verified, or closed. | Use managed-block hashes, `source_state_version`, projection freshness, and reconcile. Do not accept Markdown edits as state without an owner path. |
| Stale approval replay | Old approval text or a stale Approval record is reused after scope, baseline, sensitive category, expiry, or actor context changes. | Check scope, baseline/state version, expiry, sensitive category, and actor compatibility through Kernel and MCP owner paths before write authority exists. |
| Out-of-scope write | An agent writes a path, runs a command, reaches a network target, or accesses a secret outside the active Change Unit or Approval. | Use active scope, `prepare_write`, Write Authorization, changed-path validation, and command/path/network/secret allowlists for high-risk work. |
| MCP unavailable but agent claims state update | Core is unreachable, or the surface cannot call required MCP tools, but the agent says state changed. | Fail closed for authority. Distinguish `MCP_SERVER_UNAVAILABLE` from `SURFACE_MCP_UNAVAILABLE`; hold product/runtime/code writes until MCP is reconnected or diagnosed. |
| Secret leakage through evidence artifacts | Logs, screenshots, traces, exports, or run summaries contain tokens, credentials, PII, or private customer data. | Redact or omit before durable storage, use secret handles or safe notes, and record redaction/omission/block metadata without storing forbidden bytes. |
| Artifact hash mismatch | Registered artifact metadata and stored bytes disagree, or a staged file is substituted. | Treat the artifact and dependent evidence, projection, export, or close-readiness view as stale or blocked until recovery or replacement validates a new artifact ref. |
| Malicious generated connector file | A generated instruction, MCP config snippet, manifest, or adapter file is edited to weaken controls or exfiltrate data. | Track generated and managed paths in connector manifests, detect drift, avoid silent overwrite, and route replacement through reconnect or reconcile. |
| Capability overclaiming | A surface says it can block, capture, isolate, or reach MCP when the actual profile cannot prove it. | Require current capability profiles, `surface_capability_check` or equivalent blocked reasons, and honest cooperative/detective/preventive/isolated display. |
| Stale context poisoning | Old chat, cached status, stale projections, stale PRDs, or old evaluator bundles steer the agent into unsafe or outdated action. | Treat stale context as pull-only input, display freshness, check baseline/state versions, refresh or reconcile before authority depends on it, and use fresh evaluator bundles for detached verification. |

## Control families

### MCP local access and caller boundaries

The v0.1 baseline and staged-delivery default MCP posture is local-only for a registered project surface. Local-only means local process, local socket, localhost-loopback, in-process/stdio, process-scoped configuration material, a per-project token or handle, or an equivalent local IPC/control path for the expected local user/profile.

Where a transport has an origin, caller identity, authentication token, socket path, filesystem permission, or bind address, the connector profile and operations display must make the access-control class visible without printing raw secrets. Non-loopback binding, forwarded or tunneled endpoints, shared sockets, cloud/CI relays, cross-user paths, remote callers, and stale access material are off-profile unless a connector owner has promoted and proven that posture.

MCP reachability is not authorization. Public tool calls still rely on Core envelope validation, `project_id`, `task_id`, `surface_id`, `run_id`, and `actor_kind` compatibility, idempotency, expected state version, and API-owned error handling.

### Least privilege and high-risk allowlists

High-risk work should use the smallest tool, command, path, network target, and secret scope that can satisfy the active Change Unit. Sensitive categories such as destructive writes, network writes, external service writes, data export, infrastructure or deployment changes, production configuration changes, CI/CD changes, billing or cost changes, telemetry or logging changes, auth changes, permission model changes, secret access, privacy/PII changes, license/compliance changes, model or prompt policy changes, and policy overrides are not made safe by local execution.

Command/path/network allowlists are a control concept here, not a new schema in this document. The exact authority comes from existing owner paths: Change Unit scope, sensitive-action Approval, `prepare_write`, Write Authorization, connector capability profiles, and operator diagnostics. When the risk requires preventive blocking or isolation, a cooperative-only instruction is insufficient; the work must narrow, wait, use a fixture-proven preventive path, or use an actual isolation boundary.

### Redaction before storage

Evidence capture must account for secrets and PII before bytes become durable artifacts, projections, exports, or long-lived summaries. Redaction, omission, and blocked-payload notices are evidence-handling controls, not cosmetic formatting.

Raw secrets should not be stored as artifacts, connector manifest fields, projections, exported bundle text, or prompt context. When secret-related evidence is required, use a display-safe secret handle, redacted artifact, omission note, or operator note allowed by the relevant owner path.

### Artifact path and integrity validation

Artifact inputs are untrusted until registration validates the path boundary, task/run ownership, artifact kind, size, hash, content type, redaction or omission state, and retention/availability facts. Path validation must prevent a staged path, traversal, symlink surprise, or off-profile location from becoming trusted evidence by accident.

An artifact hash mismatch is a security and evidence-integrity finding. It does not repair by editing Markdown or copying bytes directly into place. Recovery or replacement must go through the documented artifact registration and recovery paths.

### Freshness, replay, and stale context

Baseline and state-version checks protect against replay and stale context. Old approvals, old status text, old projections, old evaluator bundles, and chat memory cannot authorize current writes or close current work. If authority depends on them, they must be refreshed, reconciled, superseded, or replaced through an owner path.

Expected state version, idempotency, baseline compatibility, approval expiry, projection freshness, and connector profile freshness are separate controls. This document names the threat-model reason for them; their exact fields and behavior stay with the API, kernel, storage, projection, connector, and operations owners.

### Fail closed when authority is unavailable

If the authority path needed for a state-changing, write-capable, sensitive, verification, QA, acceptance, residual-risk, or close-relevant action is unavailable, the action must fail, hold, or report capability insufficiency rather than continuing from chat, stale projection text, generated files, cached context, or operator prose.

For MCP unavailability, operations and connectors use the existing diagnostic distinction between `MCP_SERVER_UNAVAILABLE` and `SURFACE_MCP_UNAVAILABLE`, while API-visible failures use the API-owned `MCP_UNAVAILABLE` or `CAPABILITY_INSUFFICIENT` paths where applicable.

### Honest guarantee display

Security wording must match the proven control:

| Guarantee | Honest security meaning |
|---|---|
| `cooperative` | The surface is instructed to hold or follow Core decisions. It is not a pre-execution block. |
| `detective` | Harness can observe and report violations after action. It is detection, not prevention. |
| `preventive` | A concrete hook, wrapper, permission layer, policy engine, sidecar, or equivalent has fixture-proven pre-tool blocking for the covered operation. |
| `isolated` | Work or verification runs across a separate worktree, sandbox, process, evaluator bundle, or equivalent boundary. Isolation limits blast radius but does not itself approve, verify, accept, or close work. |

Guard, freeze, careful-mode, recipe names, product names, surface names, and friendly mode labels do not upgrade a guarantee. High-risk work must show the control it actually uses, and it must not depend on cooperative-only claims when preventive or isolated controls are required.

## Owner map for exact contracts

| Threat-model concept | Exact contract owner |
|---|---|
| MCP tool envelope, `ToolError`, public errors, idempotency, replay, expected state version | [MCP API And Schemas](mcp-api-and-schemas.md) |
| Kernel state transitions, gates, Approval, `prepare_write`, Write Authorization, acceptance, residual risk, close | [Kernel Reference](kernel.md) |
| `state.sqlite`, `task_events`, artifact storage rows, DDL, enum hardening, hashes, storage layout | [Storage And DDL](storage-and-ddl.md) |
| Runtime spaces, Core transaction ordering, artifact architecture, guarantee level definitions | [Runtime Architecture Reference](runtime-architecture.md) |
| Connector capability profiles, generated manifests, context push/pull, fallback display | [Agent Integration Reference](agent-integration.md) |
| Operator diagnostics, severity baselines, `doctor`, `serve mcp`, artifact check, recover, reconcile | [Operations And Conformance Reference](operations-and-conformance.md) |
| Conformance fixture body shape, assertion semantics, suite catalogs, examples | [Conformance Fixtures Reference](conformance-fixtures.md) |
| Projection freshness, managed blocks, reconcile behavior, template ownership | [Document Projection Reference](document-projection.md) and [Template Reference](templates/README.md) |
