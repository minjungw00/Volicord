# User judgment authority remains outside the Agent Connection

## Context

An agent can identify that product direction, scope, sensitive action,
acceptance, or residual risk needs a user decision. It can also present options
and a recommendation. It cannot turn its own prose, inferred intent, or tool
call into the user's authoritative answer.

Low-friction workflows make this boundary especially important. Reducing
intermediate calls must not let the same agent both request and resolve a
judgment or reinterpret a broad chat response as every pending approval.

## Decision

Keep user-owned judgment resolution on a User Channel that is external to the
Agent Connection. The agent-facing path may create a focused request and later
consume the resulting state, but it cannot submit the user's choice, final
acceptance, or residual-risk acceptance for them.

Project-owned policy may determine that a particular low-control workflow does
not require final acceptance when every owner-defined condition is satisfied.
That is a policy and Core close decision, not an inferred user judgment and not
an agent-generated waiver.

The exact judgment kinds, request and resolution schemas, compatible
provenance, close effects, and User Channel mechanisms remain owned by
[Core Model](../../reference/core-model.md), the user-action method and schema
owners, Agent Connection, Administrative CLI, and Security.

## Consequences

- Agent prompts and generated guidance can be simplified without weakening the
  source of user authority.
- Pending judgment survives session boundaries and can be resolved by a
  supported local user path.
- A model-authored summary or broad approval phrase cannot silently satisfy
  multiple distinct decisions.
- Automated close, where policy permits it, is explainable as policy evaluation
  rather than fabricated user acceptance.
- Tests keep agent and local-user provenance paths separate.

## Non-goals

- This decision does not define user identity, authentication, or non-repudiation.
- It does not require one particular UI for every host.
- It does not prevent the agent from offering a bounded recommendation or
  continuing independent safe work while a decision is pending.
- It does not make every chat message a User Channel resolution.

## Rejected alternatives

- Allowing the Agent Connection to resolve its own request was rejected because
  request provenance is not user authority.
- Inferring acceptance from conversational phrases was rejected because one
  phrase can be ambiguous across product direction, scope, final acceptance,
  and residual risk.
- Treating policy-based low-control close as implicit user acceptance was
  rejected because policy and judgment are distinct authority sources.

## Relevant implementation

- [`crates/volicord-core/src/methods/user_action.rs`](../../../../crates/volicord-core/src/methods/user_action.rs):
  request and resolution planning with distinct invocation categories.
- [`crates/volicord-cli/src/user_command.rs`](../../../../crates/volicord-cli/src/user_command.rs):
  local User Channel command orchestration.
- [`crates/volicord-mcp/src/adapter.rs`](../../../../crates/volicord-mcp/src/adapter.rs):
  agent-facing request dispatch without user-only resolution authority.
- Store user-action records and constraints under
  [`crates/volicord-store/src/`](../../../../crates/volicord-store/src/).

## Related tests and Reference owners

- Core user-action and close tests, CLI User Channel tests, MCP rejection tests,
  and [`tests/conformance/baseline.rs`](../../../../tests/conformance/baseline.rs).
- [Unified user-action request and resolution](unified-user-action-request-resolution.md).
- [Core Model](../../reference/core-model.md),
  [Request User Action](../../reference/api/method-request-user-action.md),
  [Resolve User Action](../../reference/api/method-resolve-user-action.md),
  [User Action Schemas](../../reference/api/schema-user-action.md),
  [Judgment Schemas](../../reference/api/schema-judgment.md),
  [Agent Connection](../../reference/agent-connection.md),
  [Administrative CLI](../../reference/admin-cli.md), and
  [Security](../../reference/security.md).
