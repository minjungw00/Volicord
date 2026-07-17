# Unified UserAction Request And Resolution

## Context

An agent may discover that progress requires a local user's judgment. The
request must be durable and resumable without allowing the agent-facing channel
to impersonate that user.

## Decision

Core owns one strict `UserActionRequest` and at most one immutable
`UserActionResolution`. `volicord.request_user_action` creates a pending
request or uses its explicit read-only resume branch. The MCP adapter returns an
agent-safe summary and never receives the complete resolving form.

The local CLI inbox reads and displays the strict form. Only the CLI resolution
path supplies local-user provenance and calls
`volicord.resolve_user_action`. Resolution revalidates the request, expiry,
current work coordinates, canonical answer, and replay identity in one atomic
mutation.

Guard prompt capture is an observation. It cannot act as a delivery channel,
submit an answer, or create user authority.

## Consequences

- The original request result and later resolution remain separate records.
- One request has at most one resolution; matching replay returns the original
  result and conflicts cannot fork it.
- Expired, stale, corrupt, or irrelevant requests fail through their owned
  branches without an answer mutation.
- MCP can create or resume a request but cannot resolve it.
- The schema has one current delivery path: `channel_kind=cli`.

See [User Action Schemas](../../reference/api/schema-user-action.md),
[Request User Action](../../reference/api/method-request-user-action.md), and
[Resolve User Action](../../reference/api/method-resolve-user-action.md).
