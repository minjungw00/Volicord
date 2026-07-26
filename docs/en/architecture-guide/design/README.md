# Architecture Design

This family describes the current implementation design of the Volicord Rust
workspace. Each page is organized around present responsibilities, execution
structure, invariants, failure behavior, and source routes. Focused Reference
owners remain authoritative for public behavior, schemas, storage effects,
security guarantees, and value meanings.

## Design reference map

| Design reference | Current implementation concern |
|---|---|
| [Agent Connection routing](agent-connection-routing.md) | Bind a managed stdio process to one current Connection and explicitly admitted Product Repository. |
| [Core and adapter boundary](core-adapter-boundary.md) | Keep syntax, adapters, Core policy, Store, diagnostics, and repository tooling in distinct dependency layers. |
| [State-bound Write Ticket validity](state-bound-write-ticket-validity.md) | Evaluate ticket reuse from current authority coordinates and consume the ticket with the protected mutation. |
| [Observation confidence boundary](observation-confidence-boundary.md) | Keep structured path facts, uncertain observations, reconciliation, and typed diagnostics distinct. |
| [External user-judgment authority](external-user-judgment-authority.md) | Keep user-owned resolution on the local User Channel outside the Agent Connection. |
| [Static compact MCP tool list](static-compact-mcp-tool-list.md) | Project one closed, capability-aware tool catalog with compact runtime schemas. |
| [Operation-result retrieval](operation-result-retrieval.md) | Read bounded exact pages from immutable replay responses without re-executing effects. |
| [Evidence-capture producer finalization](evidence-capture-producer-finalization.md) | Bind source receipts to current capture intent and finalize producer records with the Run commit. |
| [Unified UserAction request and resolution](unified-user-action-request-resolution.md) | Separate agent-safe request and resume projection from local-user resolution. |
| [Plan and atomic commit](plan-and-atomic-commit.md) | Compose typed method results from planned fields and apply grouped Store mutations at one commit boundary. |
| [Canonical Core UTC clock](canonical-core-utc-clock.md) | Coordinate prepared-operation time and the project-scoped non-decreasing time floor. |
| [Runtime Home and Product Repository](runtime-home-and-product-repository.md) | Keep runtime records, product files, installation files, and repository tooling in their owned locations. |

## Reading routes

Start with [Implementation Architecture](../architecture.md) for the workspace
shape, [Source Map](../source-map.md) for exact module ownership,
[Request Lifecycle](../request-lifecycle.md) for a representative Core path,
and [Storage and Transactions](../storage-and-transactions.md) for persistence
coordination. Use the [Reference Index](../../reference/README.md) whenever
exact product behavior matters.
