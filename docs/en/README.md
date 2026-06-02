# Harness Documentation

This is the English routing page for the Harness documentation set.

Harness is a local authority record and judgment-routing layer for AI-assisted product work. It keeps scope, user-owned judgments, evidence, verification, QA expectations, final acceptance, and residual-risk status outside fragile chat context.

This repository is currently documentation-only. Its intended future role is the Harness Server source repository. It is not a Product Repository or a Harness Runtime Home, and no Harness Server/runtime implementation exists here yet. Server/runtime implementation may start only after documentation acceptance and a separate implementation-planning readiness decision.

## Minimal First-Read Path

Use this path when you do not know where to start:

1. [Overview](learn/overview.md) for the first mental model.
2. [User Guide](use/user-guide.md) for how users and agents interact during work.
3. [Concepts](learn/concepts.md) only when terminology starts appearing in examples or reference docs.
4. [Implementation Overview](build/implementation-overview.md) only if you are reviewing or planning future Harness Server implementation.
5. [Reference Index](reference/README.md) only when you need exact contracts.

## Reader Paths By Role

| Reader | Start here | Then use |
|---|---|---|
| New user trying to understand Harness | [Overview](learn/overview.md) | [User Guide](use/user-guide.md); [Concepts](learn/concepts.md) only when terms need names; [Harness in 15 Minutes](learn/harness-in-15-minutes.md) for quick scenarios. |
| User or product lead working with an agent | [User Guide](use/user-guide.md) | [Harness in One Task](learn/harness-in-one-task.md) for a full work story; [Decision Packet Cookbook](use/decision-packet-cookbook.md) for complex user-owned choices. |
| Agent behavior/integration author | [Agent Session Flow](use/agent-session-flow.md) | [Agent Integration Reference](reference/agent-integration.md), [Surface Cookbook](reference/surface-cookbook.md), and the specific API owner when exact fields are needed. |
| Harness Server implementer | [Implementation Overview](build/implementation-overview.md) | [MVP Plan](build/mvp-plan.md) for the stage plan, [First Runnable Slice](build/first-runnable-slice.md) for v0.1 sequence, [Runtime Walkthrough](build/runtime-walkthrough.md) for the request-to-close path, then the relevant Reference owner. |
| Schema/reference reader | [Reference Index](reference/README.md) | Open only the owner doc for the contract you need: kernel, API/schema, storage, projection/template, security, operations/conformance, agent integration, design policy, glossary, or runtime architecture. |
| Documentation maintainer | [Authoring Guide](maintain/authoring-guide.md) | [Translation Guide](maintain/translation-guide.md), [Roadmap](roadmap.md), and Reference owners only when checking strict meaning. |

## Document Roles

Learn, Use, Build, Reference, and Maintain do different jobs:

| Family | Role |
|---|---|
| Learn | Why Harness exists and the concepts readers need before strict contracts. |
| Use | How users and agents should interact during Harness-assisted work. |
| Build | Future implementation sequence, stage boundaries, and maintainer handoff. |
| Reference | Exact owner contracts: schemas, DDL, gates, state transitions, projection rules, security meanings, conformance semantics, templates, and terminology. |
| Maintain | Documentation rules, redesign scope, parity expectations, and drift handling. |

## Learn

Use Learn when you want the mental model before exact contracts.

| Page | Distinct role |
|---|---|
| [Overview](learn/overview.md) | Primary first read: product thesis, three spaces, what Harness records, and what Harness is not. |
| [Purpose and Principles](learn/purpose-and-principles.md) | Values, non-goals, failure model, and MVP boundary. Use it to check whether wording or scope still matches the thesis. |
| [Concepts](learn/concepts.md) | Vocabulary bridge from ordinary user language to implementation terms. Not another overview or tutorial. |
| [Harness in 15 Minutes](learn/harness-in-15-minutes.md) | Short scenario sampler for common Harness moments. |
| [Harness in One Task](learn/harness-in-one-task.md) | Fuller tutorial walkthrough through one small change and one tracked task. |

## Use

Use this path when you want to run or describe an AI-assisted development session under Harness.

- [User Guide](use/user-guide.md) is the primary user entry.
- [Agent Session Flow](use/agent-session-flow.md) is agent/integration behavior guidance.
- [Decision Packet Cookbook](use/decision-packet-cookbook.md) is advanced usage and reference-adjacent decision example material.

## Build

Use Build for implementation orientation and planning review. Until [Documentation Acceptance Status](build/implementation-overview.md#documentation-acceptance-status) explicitly accepts implementation planning readiness, Build pages remain planning guidance and do not authorize runtime/server implementation.

Read Build in this order:

1. [Implementation Overview](build/implementation-overview.md) for current status, maintainer handoff, and the future system shape.
2. [MVP Plan](build/mvp-plan.md) for v0.1 through v0.4 stage plan and server-coding decision log.
3. [First Runnable Slice](build/first-runnable-slice.md) for the v0.1 implementation sequence.
4. [Runtime Walkthrough](build/runtime-walkthrough.md) for the request-to-close runtime path.

## Reference

Use Reference to look up exact contracts. Do not read the whole Reference set by default; pick the owner for the question in front of you. The [Reference Index](reference/README.md) is the compact owner-contract map.

| Need | Owner |
|---|---|
| Core authority, entities, gates, state transitions, write authority, and close semantics | [Kernel Reference](reference/kernel.md) |
| Public MCP tools, envelopes, schemas, errors, idempotency, state conflict behavior, shared refs, and validator result schema | [MCP API And Schemas](reference/mcp-api-and-schemas.md) |
| Runtime layout, DDL profiles, storage JSON, locks, artifacts, migrations, baselines, projection job storage, and validator storage | [Storage And DDL](reference/storage-and-ddl.md) |
| Readable views, projection freshness, managed blocks, and template bodies | [Document Projection Reference](reference/document-projection.md) and [Template Reference](reference/templates/README.md) |
| Trust boundaries, assets, threat categories, controls, and guarantee-level wording | [Security Threat Model Reference](reference/security-threat-model.md) |
| Operator behavior, diagnostics, recover/reconcile/export/artifact checks, and conformance run entrypoint | [Operations And Conformance Reference](reference/operations-and-conformance.md) |
| Fixture model, fixture body, runner/assertion semantics, Kernel Smoke queue, and later scenario inventory | [Conformance Fixtures Reference](reference/conformance-fixtures.md) and [Future Fixture Catalog](reference/future-fixture-catalog.md) |
| Connector profiles, context push/pull, fallback behavior, surface recipes, and user-facing integration patterns | [Agent Integration Reference](reference/agent-integration.md) and [Surface Cookbook](reference/surface-cookbook.md) |
| Design-quality policies, validator IDs, severity composition, waiver semantics, and policy close impact | [Design Quality Policies](reference/design-quality-policies.md) |
| Public/internal terminology and owner routing | [Glossary Reference](reference/glossary.md) |
| Runtime spaces, Core transaction placement, architecture flow, artifacts, projection/reconcile placement, and recovery overview | [Runtime Architecture Reference](reference/runtime-architecture.md) |

## Maintain

Use Maintain to keep the docs and future Harness system coherent over time. Maintain docs govern documentation maintenance, not runtime behavior.

- [Authoring Guide](maintain/authoring-guide.md)
- [Translation Guide](maintain/translation-guide.md)

## Current Status Model

The current status separates documentation review, implementation planning readiness, and runtime implementation:

| Status category | Current status |
|---|---|
| Documentation review status | Post-redesign review; documentation acceptance candidate only. Maintainers have not accepted the docs yet. |
| Implementation planning readiness | Not accepted. Maintainers must confirm the implementation-readiness criteria before first runtime-batch planning. |
| Runtime implementation status | Not started. No runtime artifacts or conformance results exist here yet. |

Documentation acceptance, when it happens, is a maintainer review milestone. It does not by itself start runtime/server implementation or prove runtime conformance.

## Maintainer Handoff

Before starting Harness Server code, implementers should read:

1. [Maintainer handoff summary](build/implementation-overview.md#maintainer-handoff-summary).
2. [Documentation acceptance status](build/implementation-overview.md#documentation-acceptance-status).
3. [Implementation-readiness criteria](build/implementation-overview.md#implementation-readiness-criteria).
4. [Implementation decisions needed before server coding](build/mvp-plan.md#implementation-decisions-needed-before-server-coding).

This handoff says the documentation is available for maintainer acceptance review as a candidate. It also centralizes remaining decision and drift status. It does not claim the docs have been accepted, it does not make the docs implementation-ready, and it does not start server/runtime implementation.

## Where Am I?

Harness keeps three spaces separate:

| Space | What belongs there |
|---|---|
| Product Repository | The user's product workspace: product code, tests, product docs, and human-readable Harness projections. |
| Harness Server source repository | The future codebase for the local Harness Server / Installation. This repository is intended to become that source repository after documentation acceptance and implementation-planning readiness. |
| Harness Runtime Home | Per-user/per-installation operational data: state database, artifact store, projection output, logs, and local registration/configuration. |

This repository's current role is documentation review/redesign. Documentation acceptance alone does not create implementation authority, runtime state, conformance, or server code.

## Comparison

Harness is not the same kind of thing as agent instructions, MCP, reusable workflows, tests, review, or specs.

| Nearby piece | Role it plays | Harness role |
|---|---|---|
| AGENTS.md / agent instruction files | Tell agents how to behave in a repository or session. | Harness may rely on those instructions, but it keeps the local record of scope, user-owned judgment, evidence, close readiness, and risk. |
| MCP | Defines a protocol boundary for tools and resources. | Harness may expose MCP tools or resources, but its authority comes from Core-owned local state and artifact references. |
| Skills / reusable workflows | Package repeated instructions or procedures for an agent to follow. | Harness can be used by those workflows, but it records the current work state and routes judgments for this task. |
| Test runners | Execute checks and produce results. | Harness links relevant results as evidence and keeps verification strength separate from final acceptance. |
| Code review | Provides human or team review of changes. | Harness can reference review outcomes, but it does not replace review or turn review into final acceptance, residual-risk acceptance, or close. |
| Specs | Describe intended behavior, design, or constraints. | Harness can use specs as input, but it records operational state for live work: scope, decisions, evidence, QA expectations, final acceptance, and remaining risk. |

## Agent Context Loading

Reader paths are not prompt-loading bundles. Connected agents should keep always-on context to one screen or less: role or surface posture, current phase/context profile, current Task summary, active blockers, pending user-owned judgments, and next allowed action.

Use owner sections by phase and pull only what explains the next action. The detailed phase profile map lives in [Agent Integration Reference: Context Push/Pull Principles](reference/agent-integration.md#context-pushpull-principles), with user-facing behavior summarized in [Agent Session Flow](use/agent-session-flow.md).

## Roadmap

- [Roadmap](roadmap.md)

Future candidate items live in the roadmap. The roadmap is not part of Build-owned staged delivery unless a future owner explicitly promotes an item through the Roadmap criteria.

## Language Parity

The English and Korean documentation sets keep the same file map and semantic content. Korean headings and prose may be natural Korean rather than sentence-by-sentence mirrors of English.
