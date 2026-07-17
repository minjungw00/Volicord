# Observation Confidence Boundary

## Context

Guard inputs include exact structured paths and less precise command or
repository observations. Treating every observation as equivalent would either
overstate authority or hide unrecorded work.

## Decision

Classify deterministic structured path facts separately from suspected effects.
Exact pre-action facts may participate in owner-defined Write Ticket checks.
Uncertain observations remain non-authoritative until a post-action repository
comparison confirms paths.

Confirmed unrecorded changes enter reconciliation. Suppression removes only
exact owner-defined expected-write matches. When suppression is unavailable,
the complete observed path set remains visible; partial best-effort suppression
is not reported as complete.

Prompt capture records what the Guard observed. It neither records a user's
answer nor resolves a UserAction.

## Consequences

- Observation does not prove actor identity or provide an OS sandbox.
- A suspected effect cannot silently become confirmed authority.
- Close-readiness projection uses stored authoritative state and explicit
  unresolved observations.
- Missing observation coverage is reported as a limitation, not filled by
  inference.

See [Guard Suppression](../../reference/guard-suppression.md),
[Security](../../reference/security.md), and
[Reconcile Changes](../../reference/api/method-reconcile-changes.md).
