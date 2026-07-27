# External User-Judgment Authority Design

## Purpose

This design describes how the implementation keeps user-owned judgment
resolution outside the Agent Connection while allowing an agent to request
input and later continue from an agent-safe projection.

## Design

Core represents a user-owned action as one strict `UserActionRequest` and at
most one immutable `UserActionResolution`. The MCP adapter exposes only the
agent-facing request and resume path. The local CLI inbox obtains typed
adapter-neutral `PendingUserActionFacts`, uses shared presentation to render
the semantic `UserActionResolutionForm` as a typed CLI inbox model with a
command-model invocation, and submits an explicit local-user resolution through
Core.

The Core UserAction service validates typed action intent, constructs and
materializes the canonical request, interprets current authority, and projects
semantic pending, current, availability, and safe resolution facts. The direct
UserAction method module owns request/resume and resolution orchestration plus
those neutral fact reads. Core does not construct CLI commands, channel labels,
rendered instructions, capture metadata, or MCP envelopes.

Core policy modules evaluate current basis, action relevance, actor
provenance, and close or write compatibility. Adapters do not infer judgment
from chat text, summary text, or a model-authored recommendation.

## Invariants

- The Agent Connection may request or resume but cannot resolve a user-owned
  action.
- Local-user provenance is derived at the User Channel boundary.
- One request has at most one immutable resolution.
- Agent-safe projections exclude private form, note, path, command, and
  user-only result material.
- Policy-based close evaluation remains distinct from an inferred user answer.
- Prompt capture and diagnostic observations do not create judgment authority.

## Responsibility boundaries

The Core UserAction service owns shared request construction, materialization,
authority interpretation, lifecycle policy, and adapter-neutral facts. The
direct method module owns the typed public request/resolution transition and
request-specific composition. Core policy modules own pure policy evaluation.
`volicord-command-model` owns canonical CLI syntax.
`volicord-user-action-presentation` owns shared CLI-oriented inbox and recovery
presentation. `volicord-cli` owns terminal rendering and explicit choice
collection. `volicord-mcp` owns the agent-safe protocol projection, neutral
failure mapping, and attachment of the shared CLI fallback. Store owns strict
request and resolution records and coherent resolution snapshots.

## Execution flow

1. An agent-facing Core call creates or resumes a current request.
2. MCP projects only the agent-safe summary and continuation route.
3. The CLI inbox asks Core for current adapter-neutral pending facts.
4. Shared presentation projects the semantic resolution form and derives the
   canonical command-model invocation.
5. The local user selects one explicit action in the typed CLI presentation.
6. Core revalidates actor provenance, basis, expiry, and current work
   coordinates.
7. Store commits the immutable resolution and associated authority event.
8. A later agent call observes only MCP's safe current projection.

## Failure behavior

Stale, expired, superseded, corrupt, already-resolved, or provenance-mismatched
requests fail without a new resolution. Concurrent matching replay returns the
existing immutable result; conflicting input cannot fork the request. Adapter
fallback output never fabricates a form or answer.

## Scope exclusions

This design does not define user identity, authentication,
non-repudiation, judgment kinds, option meanings, or close policy. It does not
require one UI for every host and does not make ordinary chat a User Channel.

## Implementation routes

- [`crates/volicord-core/src/user_action/`](../../../../crates/volicord-core/src/user_action/):
  typed request construction and materialization, current authority
  interpretation, lifecycle policy, neutral reads, and semantic facts.
- [`crates/volicord-core/src/methods/user_action.rs`](../../../../crates/volicord-core/src/methods/user_action.rs):
  direct request and resolution method orchestration.
- [`crates/volicord-core/src/policy/user_action_relevance.rs`](../../../../crates/volicord-core/src/policy/user_action_relevance.rs)
  and [`policy/close_readiness.rs`](../../../../crates/volicord-core/src/policy/close_readiness.rs):
  current relevance and authority evaluation.
- [`crates/volicord-cli/src/user_command.rs`](../../../../crates/volicord-cli/src/user_command.rs):
  local User Channel orchestration and terminal rendering.
- [`crates/volicord-command-model/src/lib.rs`](../../../../crates/volicord-command-model/src/lib.rs)
  and [`crates/volicord-user-action-presentation/src/lib.rs`](../../../../crates/volicord-user-action-presentation/src/lib.rs):
  canonical CLI syntax and shared CLI-oriented presentation.
- [`crates/volicord-mcp/src/user_action_projection.rs`](../../../../crates/volicord-mcp/src/user_action_projection.rs):
  agent-safe compound protocol projection and neutral failure mapping.
- [`crates/volicord-store/src/core_pipeline/user_actions.rs`](../../../../crates/volicord-store/src/core_pipeline/user_actions.rs):
  strict request, snapshot, and resolution persistence.

## Reference owners

Exact authority and method behavior remains in
[Core Model](../../reference/core-model.md),
[Request User Action](../../reference/api/method-request-user-action.md),
[Resolve User Action](../../reference/api/method-resolve-user-action.md),
[User Action Schemas](../../reference/api/schema-user-action.md),
[Agent Connection](../../reference/agent-connection.md),
[Administrative CLI](../../reference/admin-cli.md), and
[Security](../../reference/security.md).
