# Security Threat Model Reference

## What this document helps you do

Use this reference to identify Harness security assets, trust boundaries, threat categories, and control expectations before runtime implementation planning.

It is a lookup document for implementers, operators, connector authors, and conformance authors who need to keep local authority boundaries explicit. It does not replace the architecture, API, storage, kernel, connector, or operations owner documents.

This is reference documentation for future Harness behavior. Current repository phase and implementation handoff status are tracked in [Implementation Overview](../build/implementation-overview.md#documentation-acceptance-status).

## Read this when

- You are deciding which files, calls, artifacts, or generated connector outputs are security-sensitive.
- You need to explain why a repo document, projection, generated file, chat transcript, or caller claim is not operational authority.
- You are reviewing MCP exposure, artifact handling, connector generation, stale context, approval replay, or capability claims.
- You need to decide whether cooperative, detective, preventive, or isolated wording is honest for a security-sensitive path.
- You are writing operator diagnostics or conformance coverage that names security or threat-model findings.

## Before you read

Use [Runtime Architecture Reference](runtime-architecture.md) for the runtime spaces, Core process model, transaction ordering, and architecture placement. Use [Agent Integration Reference](agent-integration.md) for connector capability profiles, generated manifests, context push/pull, and fallback display. Use [Operations And Conformance Reference](operations-and-conformance.md) for stage-specific `doctor`, `serve mcp`, artifact checks, recover, and reconcile behavior. Use [Conformance Fixtures Reference](conformance-fixtures.md) for fixture semantics.

Use [API Schema Core](api/schema-core.md) for public tool envelopes and shared shapes, and [API Errors](api/errors.md) for public errors and replay behavior. Use [Storage And DDL](storage-and-ddl.md) for exact storage layout, artifact rows, and DDL. Use [Kernel Reference](kernel.md) for state transitions, gates, Approval, `prepare_write`, Write Authorization, acceptance, residual risk, and close.

This document links to those exact contracts instead of duplicating them.

## Main idea

Harness is a local authority layer, not a general operating-system security boundary. A local file, local process, generated connector output, external command, or agent surface can try to influence Harness, but it does not become authority just because it is nearby.

Canonical operational meaning flows through Core-owned state-changing paths. Product repository documents, chat text, generated connector files, projections, artifacts, external command output, MCP caller claims, and remembered context are inputs until the relevant owner path accepts them.

Security display must match the real control. Cooperative and detective surfaces can hold by instruction or detect after action. Preventive wording requires fixture-proven pre-tool blocking for the covered operation, and isolated wording requires a documented and proven separation boundary. High-risk work must not rely on cooperative-only claims when the work requires preventive or isolated controls.

Early local Harness stages do not automatically provide operating-system permissions, sandbox arbitrary tools, make local files tamper-proof, or convert cooperative agent behavior into preventive security. Engineering Checkpoint and MVP-1 may refuse Core state-changing actions that lack authority, record state, validate the minimal artifact/evidence refs required by the active Core path, report stale or mismatched facts, and display honest guarantee limits. A structured blocker means Core or a connected surface reports that the Harness authority path cannot proceed; it is not a claim that Harness physically stopped a process before execution. User-facing wording should distinguish "not allowed by Harness authority state" or "held by instruction" from "physically prevented by runtime." Preventive controls are future/profile-specific until owner docs and conformance prove the exact covered operation; isolated controls are future/profile-specific until they prove the exact separation boundary.

Operator entrypoints inherit the same guarantee level as the stage and connector profile that introduced them. A later recover, export, reconcile, artifact check, conformance run, or release handoff surface must not be described as preventing or enforcing more than its proven cooperative, detective, preventive, or isolated capability allows.

Isolation claims must name what kind of separation is being claimed. A fresh evaluator bundle, fresh session, or separate worktree can support verification independence, stale-context control, or blast-radius reduction. A sandbox, permission layer, locked-down runner, process boundary, or container boundary can support stronger security isolation only when the connector profile names and proves that exact mechanism.

## Guarantee levels by stage

These are the default staged guarantees for the local reference path. A concrete connector, operator path, or later profile may claim a stronger level only when it names the exact covered operation or separation boundary and points to owner documentation plus conformance proof.

| Stage | Default guarantee posture | Honest claim boundary |
|---|---|---|
| Engineering Checkpoint | Cooperative plus limited detective behavior. | Core can refuse unauthorized state-changing calls, produce structured status/blocker output, consume one compatible Write Authorization, record one Run, and validate the minimal artifact/evidence ref required by the active path. It does not stop a local process or agent from editing files outside Harness unless a separate preventive profile is proven. |
| MVP-1 User Work Loop | Cooperative plus user-visible blockers/status and limited detective behavior. | Users can see missing scope, missing decisions, missing evidence, close blockers, MCP availability, and honest guarantee status. Product/runtime/code writes hold by instruction when authority cannot be checked. This is still not default pre-tool blocking or isolation. |
| Assurance Profile | Cooperative/detective assurance with stronger separation of verification, QA, residual risk, work acceptance, and sensitive-action Approval. | Harness can record and report assurance gaps, stale evidence, missing independence, QA blockers, waiver/risk/acceptance boundaries, and context-hygiene findings. It does not become preventive or isolated unless a specific profile proves that capability. |
| Operations Profile | Detective operational behavior around recover, export, readiness, artifact integrity, projection freshness, and handoff reporting. | Operator surfaces can diagnose, report, repair through owner paths, export safe bundles, and check artifact integrity. They do not make Runtime Home tamper-proof, make projections authoritative, or isolate arbitrary tools by default. |
| Roadmap | Preventive or isolated candidates only when promoted by owner docs and proven for the covered operation or boundary. | Stronger claims require exact contracts, covered operations, fixture proof, fallback behavior, and, for isolation, a real named separation boundary such as a proven sandbox, permission boundary, locked-down runner, process boundary, or container boundary. |

The stage map does not lower Core authority. Core may always refuse an invalid state transition, deny Write Authorization, mark a gate or derived view stale/blocked, or report a structured blocker according to the active owner contract. The map only limits security wording about whether Harness can physically stop an action before it happens or isolate the action behind a security boundary.

## Engineering Checkpoint / MVP-1 feasible control baseline

The Engineering Checkpoint and MVP-1 reference path can use these controls without claiming a preventive or isolated runtime boundary:

- local-only posture display for the registered project surface
- clear Product Repository / Harness Server / Harness Runtime Home separation
- raw secret and token response prohibition, with display-safe handles, redaction, omission, or blocked-payload notices
- artifact path validation, owner relation checks, and basic fingerprint/hash checks where the active owner path requires them
- `expected_state_version` freshness checks and idempotency keys for state-changing calls
- single-use Write Authorization returned by `prepare_write` and consumed by a compatible `record_run`
- stale context blockers or warnings for stale projections, stale sensitive-action permissions or later Approval records, stale baselines, stale connector profiles, stale evaluator bundles, and stale retrieved context
- fail-closed authority claims when MCP/Core is unavailable
- cooperative/detective blocker display that says what Core cannot authorize or what the surface can detect, without implying physical pre-tool enforcement

These controls can refuse Core state changes, keep authority claims from being invented, or make inconsistencies visible. By default they do not physically prevent arbitrary local processes or tools from writing files.

## Future or profile-promoted controls

The following controls are future or profile-specific until an owner document implements the mechanism, names the covered operation or separation boundary, and conformance proves it:

- operating-system sandboxing
- arbitrary-tool isolation
- tamper-proof Harness Runtime Home storage
- preventive pre-tool blocking for product/runtime/code writes
- hardened multi-user permissions
- broad connector security model across local, remote, shared, cloud, CI, and cross-user postures
- full secret manager or data-loss-prevention system

Until promoted that way, references to guards, freeze modes, careful modes, sidecars, hooks, wrappers, worktrees, bundles, or local files are cooperative or detective control descriptions unless the exact preventive or isolated boundary is proven.

## Scenario posture by stage

| Scenario | Engineering Checkpoint | MVP-1 User Work Loop | Assurance Profile | Operations Profile | Roadmap |
|---|---|---|---|---|---|
| MCP unavailable | Authority-dependent calls fail or hold; no Core state, Write Authorization, evidence, acceptance, residual-risk acceptance, or close claim is invented from chat or cached text. | The user sees an availability blocker/status and the next reconnect or diagnosis action. Product/runtime/code writes hold by instruction unless a proven stronger profile covers the operation. | Assurance paths report that verification, QA, waiver, risk, or acceptance state cannot be trusted through the unavailable path. | `serve mcp`, `doctor`, and `recover` distinguish `MCP_SERVER_UNAVAILABLE` from `SURFACE_MCP_UNAVAILABLE` and preserve the public `MCP_UNAVAILABLE`/capability error boundary. | A promoted guard may stop a covered write before execution, or a promoted isolation profile may route work through a real boundary, only for the proven path. |
| Out-of-scope write | `prepare_write` can refuse Write Authorization and return a structured blocker; external edits are only detected if the active path observes them. | The user sees what is outside scope and can narrow or deliberately expand scope through the proper decision path. | Autonomy, approval, evidence, and changed-path checks can mark the run, evidence, verification, or close readiness stale/blocked/insufficient. | Doctor, recover, and reconcile can report changed-path or generated-file drift and route repair through owner paths. | A preventive profile may block covered paths/commands/network/secrets before execution only when fixture proof covers that operation. |
| Sensitive-action approval | Full Approval semantics are outside the minimal slice unless an owner profile promotes a narrow case; sensitive actions outside the active scope are held or treated as unsupported. | The user sees the named sensitive step, whether permission is needed or granted, and that permission is not work acceptance or residual-risk acceptance. | Approval is separated from User Judgments, Write Authorization, QA/verification waivers, work acceptance, and residual-risk acceptance. | Operator diagnostics and export/handoff reports can show Approval status without creating external approval or deployment authority. | Policy wrappers or permission systems may become preventive only for exact covered actions with proof. |
| Stale projection | Persisted projections are not required; stale readable text is not Core state. | Readable summaries/cards may warn about freshness and should not be used as authority when stale. | Assurance and context-hygiene checks can require fresh state, fresh evaluator bundles, or reconcile before verification/QA/close depends on the view. | Projection refresh, reconcile, doctor, export, and recover can report or repair freshness through owner paths while keeping committed state intact. | Richer projection/UI systems remain read-only unless owner docs define and prove a mutation path. |
| Artifact tampering | Registered artifact refs and minimal integrity facts are checked where active; a direct file edit is not evidence authority. | Evidence and close summaries show missing, stale, or mismatched artifact support. | Evidence, Eval, Manual QA, waiver, risk, and close paths can become stale, insufficient, blocked, or unresolved until replacement or an owner decision resolves the gap. | Artifact checks, recover, and export validate hashes, retention, redaction, omitted-secret, and blocked-payload metadata without trusting staged files or Markdown. | Storage hardening or locked artifact handling may be stronger only when a real boundary and conformance proof exist. |
| Prompt injection | Repo docs, generated files, old projections, and chat are inputs; they cannot create authority or bypass Core. | User-facing status and judgment prompts should show current scope and judgments instead of treating broad approval-like prose as authority. | Context-hygiene, stewardship, evaluator freshness, and User Judgment routes make stale or malicious context visible before assurance claims rely on it. | Doctor and reconcile can report generated-file drift, stale context, projection tampering, and managed-block edits. | Content filters, isolated evaluators, or stronger prompt-containment mechanisms are expansion candidates unless proven for the exact boundary. |
| Secret leakage | Raw secrets should not become artifacts, manifests, projections, or prompt context; minimal evidence paths use redaction, omission, or safe handles when required. | Users see evidence gaps, omitted-secret notes, or safe secret handles without raw values. | Evidence, QA, Eval, waiver, and residual-risk paths account for redaction, omission, and blocked payloads before assurance or close claims rely on them. | Artifact checks and export/handoff preserve omission/block metadata and avoid copying raw staged, omitted, blocked, secret, or PII values. | Secret scanners, permission wrappers, or data-loss-prevention controls are preventive only if they block covered leakage before storage or transmission and prove that path. |

## Reference scope

This document owns:

- threat-model concepts and vocabulary
- the security asset map
- the trust-boundary map
- the required threat and control categories
- guarantee-level meanings and honest-display rules
- the rule that high-risk work cannot depend on cooperative-only claims when preventive or isolated controls are required
- the non-substitution boundary between threat-model concepts and exact DDL, API schemas, and kernel transitions

## Not covered here

This document does not own:

- public MCP request/response schemas; see [MVP API](api/mvp-api.md) and [API Schema Core](api/schema-core.md)
- public error shapes or idempotency/replay contracts; see [API Errors](api/errors.md)
- SQLite DDL, storage layout, canonical enum hardening, artifact row shape, or exact file layout; see [Storage And DDL](storage-and-ddl.md)
- kernel state transitions, gates, Approval lifecycle, `prepare_write`, Write Authorization, work acceptance, residual-risk acceptance, or close; see [Kernel Reference](kernel.md)
- stage-specific operator command semantics, diagnostic severity baselines, or recover/reconcile/export behavior; see [Operations And Conformance Reference](operations-and-conformance.md)
- fixture assertion semantics; see [Conformance Fixtures Reference](conformance-fixtures.md)
- connector capability-profile field details, generated-manifest contracts, or surface recipes; see [Agent Integration Reference](agent-integration.md) and [Surface Cookbook](surface-cookbook.md)
- projection template bodies or managed-block rendering rules; see [Document Projection Reference](document-projection.md)
- runtime implementation, generated operational files, executable fixtures, runtime data, or production deployment

## Baseline assumptions

The Engineering Checkpoint and staged-delivery default are local-first. The expected baseline is a user-controlled Product Repository, a local Harness Server / Installation, a Harness Runtime Home, an MCP server exposed only through the registered local connector posture, and one or more connected agent surfaces.

Local-first does not mean every local process is trusted. Another process, stale connector configuration, broad file permissions, a forwarded port, a hand-edited generated file, or stale chat context may still affect what an agent sees or does. Harness therefore treats nearby surfaces as separate trust zones and accepts operational meaning only through owner paths.

Remote or shared MCP exposure remains outside the Engineering Checkpoint baseline and staged delivery unless owner documentation and conformance promote and prove a specific connector posture. A promoted posture must still show the access-control contract, secret/PII handling, redaction or omission behavior, honest guarantee display, and Core validation that remain in force.

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
| User conversation surface | Chat may contain intent, approval-like words, stale memory, or malicious pasted content. | Treat conversation as input. User-owned judgment must be recorded through the relevant Decision Packet, sensitive-action Approval, work acceptance, or residual-risk acceptance path when it affects authority. |
| Agent surface | The surface may skip MCP, overclaim capability, continue from stale context, or perform actions outside scope. | Capability must be declared for the actual host/profile and displayed honestly. Product/runtime/code writes hold when required authority cannot be checked. |
| Harness Server / Installation | The local control-plane process, connector adapter, projector, reconciler, or operator entrypoint may be stale, misconfigured, or asked to trust inputs that bypass Core. | Treat the installation as the control plane, not as a general OS sandbox. State-changing effects go through Core owner paths; adapters and tools report capability, diagnostics, or proposals rather than creating authority. |
| Local process | A shell, editor, test runner, package manager, sidecar, or other local process may mutate files, read secrets, or call local endpoints outside the intended profile. | Local execution is not trust by itself. Bound process behavior through scope, Approval, connector capability, least-privilege tool choice, and stronger controls when cooperative/detective posture is insufficient. |
| Local socket or API surface | Local endpoints can be reached by the wrong caller, stale configuration, forwarded ports, weak socket/config permissions, or off-profile access material. | Use local process, local socket, localhost-loopback, in-process/stdio, or a promoted connector posture with documented access control. Validate public envelopes through Core, and do not treat reachability as authorization. |
| Core | Core is the authority boundary for canonical mutation. | Core alone commits operational state changes and owner-record effects. No report, projection, generated file, or caller claim bypasses Core. |
| Harness Runtime Home | Local files may be read or written by unrelated users, shared containers, or off-profile automation. | Treat broad read/write access as tampering or confidentiality risk. Direct edits are invalid until Core, recovery, or artifact-integrity paths validate the effect. Do not claim these files are tamper-proof merely because they are local. |
| Product Repository | Human-editable docs, generated Markdown, product files, and repo rules can influence agent behavior. | Repo files are inputs, product work, or projections. They do not become canonical operational state by being present in the repo. |
| Artifact store | Staged or committed evidence may contain secrets, be swapped, or fail integrity checks. | Validate paths, task/run ownership, hashes, sizes, content types, redaction/omission/block state, and retention before relying on the bytes. |
| Generated projections | Managed Markdown, compact status cards, reports, and generated connector views may be stale, edited, prompt-injected, or confused with authority. | Treat projections as readable views or proposal surfaces. Freshness, managed-block hashes, and reconcile route changes through owner paths before they affect state. |
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

The Engineering Checkpoint baseline and staged-delivery default MCP posture is local-only for a registered project surface. Local-only means local process, local socket, localhost-loopback, in-process/stdio, process-scoped configuration material, a per-project token or handle, or an equivalent local IPC/control path for the expected local user/profile.

Where a transport has an origin, caller identity, authentication token, socket path, filesystem permission, or bind address, the connector profile and operations display must make the access-control class visible without printing raw secrets. Non-loopback binding, forwarded or tunneled endpoints, shared sockets, cloud/CI relays, cross-user paths, remote callers, and stale access material are off-profile unless a connector owner has promoted and proven that posture.

MCP reachability is not authorization. Public tool calls still rely on Core envelope validation, `project_id`, `task_id`, `surface_id`, `run_id`, and `actor_kind` compatibility, idempotency, expected state version, and API-owned error handling.

If Core cannot be reached, no authoritative Core response exists and the API-visible path is `MCP_UNAVAILABLE` or an operations diagnostic such as `MCP_SERVER_UNAVAILABLE`. If Core or an operator can classify a reachable local caller or access path as outside the registered local profile, the API-visible path is `LOCAL_ACCESS_MISMATCH` with display-safe details. If the caller is on a recognized profile but the profile lacks a required capability, use `CAPABILITY_INSUFFICIENT`.

### Least privilege and high-risk allowlists

High-risk work should use the smallest tool, command, path, network target, and secret scope that can satisfy the active Change Unit. Sensitive categories such as destructive writes, network writes, external service writes, data export, infrastructure or deployment changes, production configuration changes, CI/CD changes, billing or cost changes, telemetry or logging changes, auth changes, permission model changes, secret access, privacy/PII changes, license/compliance changes, model or prompt policy changes, and policy overrides are not made safe by local execution.

Command/path/network allowlists are a control concept here, not a new schema in this document. The exact authority comes from existing owner paths: Change Unit scope, sensitive-action Approval, `prepare_write`, Write Authorization, connector capability profiles, and operator diagnostics. When the risk requires preventive blocking or isolation, a cooperative-only instruction is insufficient; the work must narrow, wait, use a fixture-proven preventive path, or use the documented and proven separation boundary claimed by the connector profile.

### Redaction before storage

Evidence capture must account for secrets and PII before bytes become durable artifacts, projections, exports, or long-lived summaries. Redaction, omission, and blocked-payload notices are evidence-handling controls, not cosmetic formatting.

Raw secrets should not be stored as artifacts, connector manifest fields, projections, exported bundle text, or prompt context. When secret-related evidence is required, use a display-safe secret handle, redacted artifact, omission note, or operator note allowed by the relevant owner path.

### Artifact path and integrity validation

Artifact inputs are untrusted until registration validates the path boundary, task/run ownership, artifact kind, size, hash, content type, redaction or omission state, and retention/availability facts. Path validation must keep a staged path, traversal, symlink surprise, or off-profile location from becoming trusted evidence by accident.

An artifact hash mismatch is a security and evidence-integrity finding. It does not repair by editing Markdown or copying bytes directly into place. Recovery or replacement must go through the documented artifact registration and recovery paths.

### Freshness, replay, and stale context

Baseline and state-version checks help catch replay and stale context before authority depends on them. Old sensitive-action permissions or later Approval records, old status text, old projections, old evaluator bundles, and chat memory cannot authorize current writes or close current work. If authority depends on them, they must be refreshed, reconciled, superseded, or replaced through an owner path.

Expected state version, idempotency, baseline compatibility, approval expiry, projection freshness, and connector profile freshness are separate controls. This document names the threat-model reason for them; their exact fields and behavior stay with the API, kernel, storage, projection, connector, and operations owners.

### Fail closed when authority is unavailable

If the authority path needed for a state-changing, write-capable, sensitive, verification, QA, work acceptance, residual-risk acceptance, or close-relevant action is unavailable, the action must fail, hold, or report capability insufficiency rather than continuing from chat, stale projection text, generated files, cached context, or operator prose.

For MCP unavailability, operations and connectors use the existing diagnostic distinction between `MCP_SERVER_UNAVAILABLE` and `SURFACE_MCP_UNAVAILABLE`, while API-visible failures use the API-owned `MCP_UNAVAILABLE` or `CAPABILITY_INSUFFICIENT` paths where applicable.

### Honest guarantee display

Security wording must match the proven control:

| Guarantee | Honest security meaning |
|---|---|
| `cooperative` | The surface is instructed to hold or follow Core decisions. This is instruction-following behavior, not a hard security boundary or a pre-execution block. |
| `detective` | Harness can detect, record, or report violations after the action or when the violation becomes observable. This is detection and reporting, not prevention. |
| `preventive` | A concrete hook, wrapper, permission layer, policy engine, sidecar, or equivalent blocks the covered operation before it happens, with fixture proof for that exact path. |
| `isolated` | Work or verification runs behind a real documented separation boundary for the claim being made. A worktree or fresh evaluator bundle can provide scope, freshness, or blast-radius separation, but it is not automatically an OS sandbox, permission boundary, or tamper-proof security boundary unless the profile proves that exact isolation mechanism. Isolation alone does not approve, verify, accept, accept risk, close, or upgrade assurance. |

Guard, freeze, careful-mode, recipe names, product names, surface names, and friendly mode labels do not upgrade a guarantee. High-risk work must show the control it actually uses, and it must not depend on cooperative-only claims when preventive or isolated controls are required.

## Owner map for exact contracts

| Threat-model concept | Exact contract owner |
|---|---|
| MCP tool envelope and `ToolError` shape | [API Schema Core](api/schema-core.md#common-response) |
| Public errors, idempotency, replay, expected state version | [API Errors](api/errors.md) |
| Kernel state transitions, gates, Approval, `prepare_write`, Write Authorization, acceptance, residual risk, close | [Kernel Reference](kernel.md) |
| `state.sqlite`, `task_events`, artifact storage rows, DDL, enum hardening, hashes, storage layout | [Storage And DDL](storage-and-ddl.md) |
| Guarantee-level meanings and honest display rules | This document: [Honest guarantee display](#honest-guarantee-display) |
| Runtime spaces, Core transaction ordering, and artifact/projection architecture placement | [Runtime Architecture Reference](runtime-architecture.md) |
| Connector capability profiles, generated manifests, context push/pull, fallback display | [Agent Integration Reference](agent-integration.md) |
| Stage-specific operator diagnostics, severity baselines, `doctor`, `serve mcp`, artifact check, recover, reconcile | [Operations And Conformance Reference](operations-and-conformance.md) |
| Core fixture mechanics: fixture body shape, runner behavior, assertion semantics, fixture profiles, suite metadata boundaries, reduced Kernel Smoke queue | [Conformance Fixtures Reference](conformance-fixtures.md) |
| Detailed future scenario candidates, future fixture examples, staged fixture coverage maps, fixture suite family summaries, catalog-only future candidates | [Future Fixture Catalog](future-fixture-catalog.md) |
| Projection freshness, managed blocks, reconcile behavior, template ownership | [Document Projection Reference](document-projection.md) and [Template Reference](templates/README.md) |
