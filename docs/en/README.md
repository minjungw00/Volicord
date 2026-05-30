# Harness Documentation

This is the English routing page for the Harness documentation set.

This repository is currently a documentation-only redesign/review repository. After documentation acceptance, it is intended to become the Harness Server source repository. No Harness Server/runtime implementation exists here yet.

These docs are source material for understanding and implementing Harness. They are not Harness runtime objects governed by the lifecycle they describe.

## Product Thesis

One sentence: Harness is a local authority record and judgment-routing layer for AI-assisted product work, keeping scope, user-owned judgments, evidence, verification, QA expectations, final acceptance, and residual-risk status outside fragile chat context.

One paragraph: In practice, Harness gives the user and agent a local record of what work is in scope, which judgments belong to the user, what supports completion claims, what still needs verification or QA, whether final acceptance has been given, and what risk remains. Chat stays conversation. Markdown projections are readable views. Core-owned local state and artifact references are the source of operational truth. Harness may use agent instructions, MCP, reusable workflows, tests, reviews, and specs, but it is not identical to any of them.

Harness focuses on four recurring problems:

- Scope drifts or becomes implicit.
- User-owned judgment is silently replaced by agent judgment.
- Evidence, verification, QA, and completion claims get mixed.
- Chat or Markdown output is mistaken for operational truth.

## Where Am I?

Harness keeps three spaces separate:

| Space | What belongs there |
|---|---|
| Product Repository | The user's product workspace: product code, tests, product docs, and human-readable Harness projections. |
| Harness Server source repository | The future codebase for the local Harness Server / Installation: API surface, request validation, Core state transitions, validators, projection, reconcile, and operator tools. |
| Harness Runtime Home | Per-user/per-installation operational data: state database, artifact store, projection output, logs, and local registration/configuration. |

This repository's current role is documentation review/redesign. Its intended future role is the Harness Server source repository. It is not the Product Repository or the Harness Runtime Home. After documentation acceptance, the Harness Server / Installation implementation is expected to be built here.

## Documentation Redesign Scope

The current repository state is documentation review/redesign. Documentation acceptance and implementation-planning status are tracked in [Implementation Overview](build/implementation-overview.md#documentation-acceptance-status).

The redesign may change terminology, MVP staging, schema structure, projection structure, security wording, and document organization. Preserve the clarified product thesis and feasible implementation path over continuity with existing prose.

The [Authoring Guide](maintain/authoring-guide.md#current-redesign-scope) owns the full redesign scope, preserved principles, document-family guidance, and [known redesign issues tracker](maintain/authoring-guide.md#known-redesign-issues-tracker).

## What Harness Is Not

Harness is not the same kind of thing as agent instructions, MCP, reusable workflows, tests, review, or specs. Those pieces can be useful around Harness, but they do not become the local operational record or the owner of user judgment.

Harness is also not a prompt pack, chat script, evaluation harness, dashboard, or broad hosted agent platform.

## Comparison

| Nearby piece | Role it plays | Harness role |
|---|---|---|
| AGENTS.md / agent instruction files | Tell agents how to behave in a repository or session. | Harness may rely on those instructions, but it keeps the local record of scope, judgment, evidence, close readiness, and risk. |
| MCP | Defines a protocol boundary for tools and resources. | Harness may expose MCP tools or resources, but its authority comes from Core-owned local state and artifact references. |
| Skills / reusable workflows | Package repeated instructions or procedures for an agent to follow. | Harness can be used by those workflows, but it records the current work state and routes judgments for this task. |
| Test runners | Execute checks and produce results. | Harness links relevant results as evidence and keeps verification strength separate from final acceptance. |
| Code review | Provides human or team review of changes. | Harness can reference review outcomes, but it does not replace review or turn review into final acceptance or residual-risk acceptance. |
| Specs | Describe intended behavior, design, or constraints. | Harness can use specs as input, but it records operational state for live work: scope, decisions, evidence, QA expectations, final acceptance, and remaining risk. |

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

Documentation-maintenance checks are editorial quality checks for drift, owner boundaries, links, and language parity. They are not runtime conformance or implementation readiness. Use the [Authoring Guide](maintain/authoring-guide.md#docs-maintenance-checks) for drift categories and owner-first resolution; use [Operations And Conformance](reference/operations-and-conformance.md#docs-maintenance-profile) only for the docs-maintenance profile reporting boundary.

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

Start with the [Documentation Acceptance Status](build/implementation-overview.md#documentation-acceptance-status). Until maintainers deliberately accept implementation planning there, Build pages remain planning guidance and do not authorize runtime/server implementation.

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
