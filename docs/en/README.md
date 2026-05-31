# Harness Documentation

This is the English routing page for the Harness documentation set.

This repository is currently a documentation-only redesign/review repository. Its intended future role is the Harness Server source repository. No Harness Server/runtime implementation exists here yet, and server/runtime implementation may start only after documentation acceptance and a separate implementation-planning readiness decision. These docs are source material for understanding and implementing Harness; they are not Harness runtime objects governed by the lifecycle they describe.

This page carries detailed repository status for English readers. Learn and Use pages should keep status brief and start with the user workflow: what to ask, what the agent clarifies, what Harness preserves, and what the user should see.

Harness is a local authority record and judgment-routing layer for AI-assisted product work. It keeps scope, user-owned judgments, evidence, verification, QA expectations, final acceptance, and residual-risk status outside fragile chat context.

Harness solves four recurring problems:

- Scope drifts or becomes implicit.
- User-owned judgment is silently replaced by agent judgment.
- Evidence, verification, QA, and completion claims get mixed.
- Chat or Markdown output is mistaken for operational truth.

## Current Status Model

The current status separates documentation review, implementation planning readiness, and runtime implementation:

| Status category | Current status |
|---|---|
| Documentation review status | Post-redesign review; documentation acceptance candidate only. Maintainers have not accepted the docs yet. |
| Implementation planning readiness | Not accepted. Maintainers must confirm the implementation-readiness criteria before first runtime-batch planning. Editorial cleanup is separate from schema/design decisions and stage-boundary decisions. |
| Runtime implementation status | Not started. No runtime artifacts or conformance results exist here yet. |

Documentation acceptance, when it happens, is a maintainer review milestone. It does not by itself start runtime/server implementation or prove runtime conformance.

## Stage Taxonomy

| Stage | Meaning |
|---|---|
| v0.1 Core Authority Slice | First internal Core authority loop. It proves the smallest authority path and is not the product MVP. |
| v0.2 User-Facing Harness MVP | First product MVP where users experience scope preservation, judgment routing, evidence, close readiness, final acceptance, and residual-risk value. |
| v0.3 Agency Assurance Pack | Hardens verification, QA, residual risk, final acceptance, and stewardship behavior. |
| v0.4 Operations & Handoff Pack | Adds recover/export, release handoff, artifact integrity, broader conformance coverage, and operator behavior. |
| v1+ Expansion | Dashboard, hosted UI, browser capture automation, team workflows, and other candidates remain outside staged delivery until promoted. |

## Primary Reader Path

Use this path when you do not know where to start:

1. [Overview](learn/overview.md) for the first mental model.
2. [User Guide](use/user-guide.md) for how to interact with Harness during work.
3. [Concepts](learn/concepts.md) for the vocabulary that appears in examples, status, and specs.
4. [Implementation Overview](build/implementation-overview.md) and [MVP Plan](build/mvp-plan.md) when you are reviewing or building the server plan.
5. [Reference](#reference) only when you need exact contracts, schemas, gates, storage, projection, security, or template details.

## Reader Paths By Role

| Reader | Start | Then use |
|---|---|---|
| User | [Overview](learn/overview.md) | [User Guide](use/user-guide.md), [Concepts](learn/concepts.md), then [Decision Packet Cookbook](use/decision-packet-cookbook.md) only when decisions get complex. |
| Agent integrator | [Overview](learn/overview.md) | [User Guide](use/user-guide.md), [Agent Session Flow](use/agent-session-flow.md), [Agent Integration Reference](reference/agent-integration.md), [Surface Cookbook](reference/surface-cookbook.md), and [MCP API And Schemas](reference/mcp-api-and-schemas.md). |
| Implementer | [Overview](learn/overview.md) | [Concepts](learn/concepts.md), [Implementation Overview handoff](build/implementation-overview.md#maintainer-handoff-summary), [MVP Plan decisions](build/mvp-plan.md#implementation-decisions-needed-before-server-coding), [First Runnable Slice](build/first-runnable-slice.md), [MVP Plan](build/mvp-plan.md), [Runtime Walkthrough](build/runtime-walkthrough.md), then the relevant Reference owner. |
| Reviewer / maintainer | [Overview](learn/overview.md) | [Authoring Guide](maintain/authoring-guide.md), [Translation Guide](maintain/translation-guide.md), [Roadmap](roadmap.md), and Reference owners when checking strict meaning. |

Operators and conformance authors usually begin in Reference: [Operations And Conformance Reference](reference/operations-and-conformance.md), [Conformance Fixtures Reference](reference/conformance-fixtures.md), [Runtime Architecture Reference](reference/runtime-architecture.md), [Security Threat Model Reference](reference/security-threat-model.md), [MCP API And Schemas](reference/mcp-api-and-schemas.md), [Storage And DDL](reference/storage-and-ddl.md), and [Kernel Reference](reference/kernel.md).

## Document Roles

The Learn and Use pages are kept separate, but each has a narrower job:

| Page | Role |
|---|---|
| [Overview](learn/overview.md) | Primary first read. Explains the product thesis, the three spaces, what Harness records, and what Harness is not. |
| [Purpose and Principles](learn/purpose-and-principles.md) | Values, non-goals, failure model, and MVP boundary. Use it when reviewing whether wording or scope still matches the thesis. |
| [Concepts](learn/concepts.md) | Vocabulary bridge from ordinary user language to implementation terms. It is not another overview or tutorial. |
| [Harness in 15 Minutes](learn/harness-in-15-minutes.md) | Scenario sampler. Six short examples show common Harness moments before strict specs. |
| [Harness in One Task](learn/harness-in-one-task.md) | Tutorial walkthrough. One small change and one tracked task show the full work journey. |
| [User Guide](use/user-guide.md) | Primary user-facing entry for starting, resuming, unblocking, and closing work. |
| [Decision Packet Cookbook](use/decision-packet-cookbook.md) | Advanced usage and reference-adjacent examples for writing focused user-decision prompts. |
| [Agent Session Flow](use/agent-session-flow.md) | Agent/integration guidance for presentation, context, blockers, writes, and close. It is not a required user read. |

## Where Am I?

Harness keeps three spaces separate:

| Space | What belongs there |
|---|---|
| Product Repository | The user's product workspace: product code, tests, product docs, and human-readable Harness projections. |
| Harness Server source repository | The future codebase for the local Harness Server / Installation: API surface, request validation, Core state transitions, validators, projection, reconcile, and operator tools. |
| Harness Runtime Home | Per-user/per-installation operational data: state database, artifact store, projection output, logs, and local registration/configuration. |

This repository's current role is documentation review/redesign. Its intended future role is the Harness Server source repository. It is not the Product Repository or the Harness Runtime Home. Documentation acceptance alone does not create implementation authority, runtime state, conformance, or server code; first implementation planning must be accepted separately before Harness Server / Installation code starts here.

## Documentation Redesign Scope

Documentation review status, implementation-planning readiness, and runtime implementation status are tracked in [Implementation Overview](build/implementation-overview.md#documentation-acceptance-status). The current revision is a documentation acceptance candidate in post-redesign review, not an accepted implementation start.

The redesign may change terminology, MVP staging, schema structure, projection structure, security wording, and document organization. Preserve the clarified product thesis and feasible implementation path over continuity with existing prose.

The [Authoring Guide](maintain/authoring-guide.md#current-redesign-scope) owns the full redesign scope, preserved principles, document-family guidance, and maintainer review checklist. Its tracker separates observed drift, candidates to verify, regression checks, and baseline status checks, and routes confirmed findings as documentation drift, schema/design decisions, stage boundary decisions, implementation-readiness criteria, or future roadmap items.

## Maintainer Handoff

Before starting Harness Server code, implementers should read:

1. [Maintainer handoff summary](build/implementation-overview.md#maintainer-handoff-summary) for the current phase, preserved principles, stage model, clarified boundaries, and open-question status.
2. [Documentation acceptance status](build/implementation-overview.md#documentation-acceptance-status) to confirm the three-part status model and whether maintainers have accepted first runtime-batch planning.
3. [Implementation-readiness criteria](build/implementation-overview.md#implementation-readiness-criteria) for the checks that must be true before planning readiness changes.
4. [Implementation decisions needed before server coding](build/mvp-plan.md#implementation-decisions-needed-before-server-coding) for any major decisions current review uncovers. At this baseline, the log is empty, but that is not a claim that no decisions remain.

This handoff says the documentation is available for maintainer acceptance review as a candidate. It does not claim the docs have been accepted, it does not make the docs implementation-ready, and it does not start server/runtime implementation.

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
| Code review | Provides human or team review of changes. | Harness can reference review outcomes, but it does not replace review or turn review into final acceptance, residual-risk acceptance, or close. |
| Specs | Describe intended behavior, design, or constraints. | Harness can use specs as input, but it records operational state for live work: scope, decisions, evidence, QA expectations, final acceptance, and remaining risk. |

## Ownership Rule

Reference docs own exact contracts: schemas, DDL, gates, state transitions, enum values, fixture semantics, template bodies, and official definitions. Learn, Use, and Build docs explain the idea for their reader and link to Reference instead of copying strict contract blocks.

Documentation-maintenance checks are editorial quality checks for drift, owner boundaries, links, and language parity. They are not runtime conformance or implementation readiness. Use the [Authoring Guide](maintain/authoring-guide.md#docs-maintenance-checks) for drift categories and owner-first resolution; use [Operations And Conformance](reference/operations-and-conformance.md#docs-maintenance-profile) only for the docs-maintenance profile reporting boundary.

## Learn

Use Learn when you want the mental model before exact contracts.

- [Overview](learn/overview.md)
- [Purpose and Principles](learn/purpose-and-principles.md)
- [Concepts](learn/concepts.md)
- [Harness in 15 Minutes](learn/harness-in-15-minutes.md)
- [Harness in One Task](learn/harness-in-one-task.md)

## Use

Use this path when you want to run an AI-assisted development session under Harness. The primary user-facing entry is [User Guide](use/user-guide.md). [Decision Packet Cookbook](use/decision-packet-cookbook.md) is for advanced decision examples. [Agent Session Flow](use/agent-session-flow.md) is agent/integration guidance, not a required user read.

- [User Guide](use/user-guide.md)
- [Decision Packet Cookbook](use/decision-packet-cookbook.md)
- [Agent Session Flow](use/agent-session-flow.md)

## Build

Use this path for implementation orientation and planning review. Start with the [Documentation Acceptance Status](build/implementation-overview.md#documentation-acceptance-status). Until maintainers deliberately accept implementation planning there, Build pages remain planning guidance and do not authorize runtime/server implementation.

- [Implementation Overview](build/implementation-overview.md)
- [Maintainer Handoff Summary](build/implementation-overview.md#maintainer-handoff-summary)
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
