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
| [Durable operation-result retrieval](operation-result-retrieval.md) | Why exact historical mutation responses reuse immutable replay rows and bounded, access-checked paging. |
| [Planning before atomic mutation commit](plan-and-atomic-commit.md) | Why methods plan effects before Store commit and why Store owns the atomic transaction boundary. |
| [Runtime Home and Product Repository separation](runtime-home-and-product-repository.md) | Why runtime state and product files stay in separate locations and how implementation code reflects that split. |

Use [Implementation Architecture](../architecture.md) for the workspace
architecture overview, dependency-boundary overview, durable implementation
boundaries, and detail routes. Use [Source Map](../source-map.md) for exact
source path responsibilities and module placement,
[Design Patterns](../design-patterns.md) for recurring implementation
structures, [Storage and Transactions](../storage-and-transactions.md) for the
Store commit and artifact boundaries, and `Cargo.toml` manifests for exact
Cargo dependency edges.
