# State-Bound Write Ticket Validity

## Context

A Write Ticket authorizes one prepared write against specific current work and
project authority. A fixed lifetime alone cannot detect relevant workflow,
scope, baseline, approval, workspace, or path-authority changes.

## Decision

Bind ticket issuance and reuse to the owner-defined Task, current Change Unit,
normalized scope, baseline, workspace, approval basis, connection/project
context, and normalized write-authority fingerprint. Core validates structural
preconditions before ticket lookup and revalidates every compatibility
coordinate before a write.

Irrelevant global changes do not invalidate a ticket merely because a project
counter changed. Any relevant mismatch makes the ticket unusable. Successful
consumption occurs in the same atomic commit as the protected mutation.

## Consequences

- Tickets are neither transferable capabilities nor user identity.
- A missing current Change Unit is a structural rejection before ticket effects.
- Replay cannot consume the same ticket twice.
- Path normalization and fingerprint construction must be deterministic.
- Guard observations outside the owner-defined authority coordinates do not
  silently widen ticket scope.

See [Prepare Write](../../reference/api/method-prepare-write.md),
[Storage Effects](../../reference/storage-effects.md), and
[Security](../../reference/security.md).
