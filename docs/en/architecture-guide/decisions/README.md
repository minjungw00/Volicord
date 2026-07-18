# Architecture Decisions

These pages explain durable implementation structure for the current Rust
workspace. Focused Reference owners remain authoritative for public behavior,
schemas, effects, security, and value sets.

| Decision | Structural purpose |
|---|---|
| [Agent Connection routing](agent-connection-routing.md) | Bind one Codex Record connection and its explicit project membership to one managed stdio process. |
| [Core and adapter boundary](core-adapter-boundary.md) | Keep Core independent of CLI and MCP details. |
| [State-bound Write Ticket validity](state-bound-write-ticket-validity.md) | Reuse tickets only while relevant work and authority coordinates remain valid. |
| [Observation confidence boundary](observation-confidence-boundary.md) | Separate deterministic path facts from uncertain observations. |
| [External user judgment authority](external-user-judgment-authority.md) | Keep user answers outside the Agent Connection. |
| [Static compact MCP tool list](static-compact-mcp-tool-list.md) | Keep the public tool registry closed and compact. |
| [Durable operation-result retrieval](operation-result-retrieval.md) | Recover eligible immutable mutation results through bounded lookup. |
| [Evidence-capture producer finalization](evidence-capture-producer-finalization.md) | Bind source receipts to an intent and finalize producer authority atomically. |
| [Unified UserAction request and resolution](unified-user-action-request-resolution.md) | Separate agent request/resume from CLI-only immutable resolution. |
| [Plan and atomic commit](plan-and-atomic-commit.md) | Plan effects before the Store transaction. |
| [Canonical Core UTC clock](canonical-core-utc-clock.md) | Use one non-decreasing prepared-operation time model. |
| [Runtime Home and Product Repository](runtime-home-and-product-repository.md) | Separate runtime state from product files. |

See [Architecture](../architecture.md), [Source Map](../source-map.md), and the
[Reference Index](../../reference/README.md) for owner routing.
