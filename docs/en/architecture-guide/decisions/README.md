# Architecture decisions

This directory contains a small set of durable architecture decisions for the
current Rust implementation. These pages explain stable intended structure,
consequences, non-goals, relevant source, tests, and Reference owners.

They do not define public API behavior, schemas, storage effects, security
guarantees, runtime behavior, Core authority semantics, product acceptance,
close readiness, or conformance results.

## Decision set

| Decision | Use it for |
|---|---|
| [Agent Connection and host routing](agent-connection-routing.md) | Why coding-agent MCP setup is bound to an Agent Connection and explicit Connection Project membership rather than one fixed Product Repository. |
| [Core and adapter dependency boundary](core-adapter-boundary.md) | Why Core does not depend on MCP or CLI adapters, and what adapter code may do before calling Core. |
| [Task control levels and integration profiles are separate axes](task-control-levels-vs-integration-profiles.md) | Why Record and Detective remain host-integration choices while Core records a separate project-policy-constrained control level for each Task and handles normalized write-authority changes independently. |
| [Write Ticket validity is bound to relevant work state](state-bound-write-ticket-validity.md) | Why ticket compatibility follows the Task, Change Unit, scope, baseline, workspace, approval basis, and normalized project write authority instead of unrelated project-wide mutations or a fixed default lifetime. |
| [Session Stop and Task completion are separate outcomes](stop-completion-separation.md) | Why a host session may end while Close Status still suppresses a completion claim. |
| [Confirmed effects and heuristic observations have different authority](observation-confidence-boundary.md) | Why deterministic path facts can support pre-action enforcement while uncertain shell effects remain warnings until corroborated after action. |
| [User judgment authority remains outside the Agent Connection](external-user-judgment-authority.md) | Why an agent may request and consume a judgment but only a separate User Channel can record the user's answer. |
| [MCP uses a static compact tool list by default](static-compact-mcp-tool-list.md) | Why runtime schemas omit documentation examples and use returned next-action routing before considering state-dependent dynamic tool lists. |
| [Final-output authority disclosure](final-output-authority-disclosure.md) | Why fresh authority disclosure uses one shared status/receipt validator and a profile-independent host UI path separate from Detective Stop enforcement. |
| [Host-capability verification for credential delivery](host-capability-verification.md) | Why credential-bearing local-web delivery requires exact, expiring live-host evidence in addition to listener readiness and a cooperative client declaration. |
| [Managed-host session/thread binding and per-call turn validation](managed-host-session-turn-binding.md) | Why managed launch provenance stays pending until exact per-call Codex session and thread metadata binds one stdio process. |
| [External host release evidence gate](host-release-evidence-gate.md) | Why one external exact-final candidate uses a fixed twelve-cell canonical gate and a separate-process recalculating audit. |
| [Host feature support-state evaluation](host-feature-support-state-evaluation.md) | Why implementation, configuration, exact live evidence, and current runtime readiness use one typed evaluator rather than ambiguous support booleans. |
| [Durable operation-result retrieval](operation-result-retrieval.md) | Why exact historical mutation responses reuse immutable replay rows and bounded, access-checked paging. |
| [Evidence-capture intent and producer finalization](evidence-capture-producer-finalization.md) | Why source-owned receipts are bound by an expiring intent and become producer authority only inside `record_run`. |
| [Unified user-action request and resolution](unified-user-action-request-resolution.md) | Why judgments and user evidence observations share one pending request, immutable resolution, and channel-adapter lifecycle. |
| [Planning before atomic mutation commit](plan-and-atomic-commit.md) | Why methods plan effects before Store commit and why Store owns the atomic transaction boundary. |
| [Canonical Core UTC clock](canonical-core-utc-clock.md) | Why project time has one non-decreasing persisted floor, one prepared-operation sample, and one canonical Core commit timestamp distinct from `state_version`. |
| [Runtime Home and Product Repository separation](runtime-home-and-product-repository.md) | Why runtime state and product files stay in separate locations and how implementation code reflects that split. |

Use [Implementation Architecture](../architecture.md) for the workspace
architecture overview, dependency-boundary overview, durable implementation
boundaries, and detail routes. Use [Source Map](../source-map.md) for exact
source path responsibilities and module placement,
[Design Patterns](../design-patterns.md) for recurring implementation
structures, [Storage and Transactions](../storage-and-transactions.md) for the
Store commit and artifact boundaries, and `Cargo.toml` manifests for exact
Cargo dependency edges.
