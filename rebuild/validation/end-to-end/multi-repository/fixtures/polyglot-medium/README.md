# Relay Desk

Relay Desk is a bounded polyglot fixture for repository-wide validation. A Java
gateway accepts a request, a Python formatter normalizes its message, and a
TypeScript client renders the response. `docs/flow.md` describes the
cross-process boundary; it is documentation evidence, not a direct call edge.

The fixture is intentionally self-contained and does not require downloaded
dependencies.
