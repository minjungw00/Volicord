# Harness Documentation

This is the English routing page for the Harness documentation set.

This repository is in documentation redesign / feedback incorporation and documentation review. This page does not authorize Harness server/runtime implementation, generated operational files, executable fixtures, or runtime data. First runtime-batch planning may not begin until maintainers deliberately accept the docs in the implementation handoff. The first product MVP target is v0.1 Kernel MVP, exercised by the Kernel Smoke conformance profile; v0.2 through v0.4 are staged packs toward the Agency-Hardened MVP reference conformance target, and v1+ Expansion stays in the roadmap unless owner docs promote and prove it.

Harness is a local work ledger and judgment router for AI-assisted product work. It records what may change, who must decide, what evidence exists, what risk remains, and whether the work can close.

Harness still follows the agency-preserving local authority kernel principle: durable work facts stay in local state, artifact refs, and readable projections, while user-owned product and material technical judgment stays with the user.

## What Harness Is Not

Harness is not:

- a prompt pack
- a replacement for source control, tests, code review, or user judgment
- MCP itself
- a broad hosted agent platform

Harness is also not a chat script, test harness, evaluation harness, or dashboard.

## Comparison

| Nearby piece | How Harness differs |
|---|---|
| AGENTS.md / agent rules | Agent rules tell agents how to behave in a repository or session. Harness keeps the local work ledger that records scope, evidence, judgment needs, risk, and close readiness. |
| MCP | MCP is a protocol boundary for tools and resources. Harness may expose MCP tools, but Harness is not MCP itself; its authority comes from local Core-owned records. |
| Spec-driven workflows | Specs describe intended behavior or design. Harness records the live work state around a task: allowed change boundary, user decisions, evidence, risk, and whether the task can close. |
| Hooks / sidecars | Hooks and sidecars can observe, block, or report depending on their actual guarantee level. Harness records those limits and routes any effect through the relevant owner paths. |
| Test runners / code review | Tests and review check product work. Harness links their results as evidence while keeping acceptance, residual risk, and user-owned judgment separate. |

## Reader Routes

| Reader role | Start here | Then use |
|---|---|---|
| New reader | [Harness in 15 Minutes](learn/harness-in-15-minutes.md) | [Overview](learn/overview.md), [Harness in One Task](learn/harness-in-one-task.md), then [Concepts](learn/concepts.md) |
| User | [User Guide](use/user-guide.md) | [Decision Packet Cookbook](use/decision-packet-cookbook.md), then [Agent Session Flow](use/agent-session-flow.md) when you need the agent-facing flow |
| Implementer | [Implementation Overview](build/implementation-overview.md) | [First Runnable Slice](build/first-runnable-slice.md), [Runtime Walkthrough](build/runtime-walkthrough.md), [MVP Plan](build/mvp-plan.md), then the relevant Reference owner |
| Operator | [Operations And Conformance Reference](reference/operations-and-conformance.md#contract-map) | [Runtime Architecture](reference/runtime-architecture.md), [Security Threat Model](reference/security-threat-model.md), [MCP API And Schemas](reference/mcp-api-and-schemas.md), [Storage And DDL](reference/storage-and-ddl.md) |
| Conformance author | [Conformance Fixtures Reference](reference/conformance-fixtures.md#conformance-navigation-map) | [Operations And Conformance Reference](reference/operations-and-conformance.md#conformance-run), [MCP API And Schemas](reference/mcp-api-and-schemas.md), [Storage And DDL](reference/storage-and-ddl.md), [Kernel Reference](reference/kernel.md) |
| Documentation maintainer | [Authoring Guide](maintain/authoring-guide.md) | [Translation Guide](maintain/translation-guide.md) |

## Ownership Rule

Reference docs own exact contracts: schemas, DDL, gates, state transitions, enum values, fixture semantics, template bodies, and official definitions. Learn, Use, and Build docs explain the idea for their reader and link to Reference instead of copying strict contract blocks.

Documentation-maintenance checks are read-only review guidance, not runtime conformance or implementation readiness. Use the [Authoring Guide](maintain/authoring-guide.md#docs-maintenance-checks) for drift categories and owner-first resolution; use [Operations And Conformance](reference/operations-and-conformance.md#docs-maintenance-profile) only for the docs-maintenance profile reporting boundary.

Operators use [Operations And Conformance Reference](reference/operations-and-conformance.md) for procedures and the conformance run overview. Fixture authors use [Conformance Fixtures Reference](reference/conformance-fixtures.md) for fixture body shape, assertion semantics, suite catalogs, examples, and catalog-only future candidates.

## Learn

Use Learn when you want the mental model before exact contracts.

- [Overview](learn/overview.md)
- [Harness in 15 Minutes](learn/harness-in-15-minutes.md)
- [Harness in One Task](learn/harness-in-one-task.md)
- [Concepts](learn/concepts.md)
- [Purpose and Principles](learn/purpose-and-principles.md)

## Use

Use this path when you want to run an AI-assisted development session under Harness. These pages prioritize user-facing flow, status interpretation, decisions, and recovery paths.

- [User Guide](use/user-guide.md)
- [Decision Packet Cookbook](use/decision-packet-cookbook.md)
- [Agent Session Flow](use/agent-session-flow.md)

## Build

Use this path for implementation orientation and planning review. These pages keep the first path narrow: v0.1 Kernel MVP first, Kernel Smoke as its narrow conformance profile, v0.2 through v0.4 as staged packs toward Agency-Hardened MVP, and v1+ Expansion outside staged delivery unless owner docs promote and prove it.

Start with the [Documentation Acceptance Status](build/implementation-overview.md#documentation-acceptance-status). Until maintainers deliberately accept first runtime-batch planning there, Build pages remain planning guidance and do not authorize runtime/server implementation.

- [Implementation Overview](build/implementation-overview.md)
- [First Runnable Slice](build/first-runnable-slice.md)
- [Runtime Walkthrough](build/runtime-walkthrough.md)
- [MVP Plan](build/mvp-plan.md)

## Reference

Use this path to look up strict contracts. If another path summarizes a strict rule, update the Reference owner first.

- [Kernel Reference](reference/kernel.md)
- [Runtime Architecture Reference](reference/runtime-architecture.md)
- [Security Threat Model Reference](reference/security-threat-model.md)
- [MCP API And Schemas](reference/mcp-api-and-schemas.md)
- [Storage And DDL](reference/storage-and-ddl.md)
- [Document Projection Reference](reference/document-projection.md)
- [Design Quality Policies](reference/design-quality-policies.md)
- [Agent Integration Reference](reference/agent-integration.md)
- [Surface Cookbook](reference/surface-cookbook.md)
- [Operations And Conformance Reference](reference/operations-and-conformance.md)
- [Conformance Fixtures Reference](reference/conformance-fixtures.md)
- [Glossary Reference](reference/glossary.md)
- [Template Reference](reference/templates/README.md)

## Maintain

Use this path to keep the docs and future Harness system coherent over time. Maintain docs govern documentation maintenance, not runtime behavior.

- [Authoring Guide](maintain/authoring-guide.md)
- [Translation Guide](maintain/translation-guide.md)

## Roadmap

- [Roadmap](roadmap.md)

Post-MVP items live in the roadmap. The roadmap is not part of Build-owned staged delivery unless a future owner explicitly promotes an item with scope, fixtures, and fallback behavior.

## Language Parity

The English and Korean documentation sets keep the same file map and semantic content. Korean headings and prose may be natural Korean rather than sentence-by-sentence mirrors of English.
