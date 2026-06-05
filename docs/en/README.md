# Harness Documentation

This is the English routing page for the Harness documentation set.

Harness is a local work-authority server for AI-assisted product work. Its job is to keep fragile work criteria out of chat-only memory. It preserves the local basis for scope, user-owned judgment, evidence, verification expectations, final acceptance, close readiness, and residual risk. When an agent should not decide, Harness routes that decision back to the user.

| Harness is not | Harness does |
|---|---|
| A prompt pack or chat script. | Keeps work authority outside prompts and conversation. |
| MCP itself or an API wrapper. | May use MCP/API surfaces, but the product thesis is the local work-authority record. |
| A workflow engine, report generator, or dashboard. | Records the basis for work and can derive readable views from that record. |
| A hosted agent platform. | Is designed around a local Harness Server / Installation. |
| A sandbox or OS permission system. | Preserves authority boundaries without claiming OS-level isolation or arbitrary-tool permission control. |

This repository is documentation-only today and its intended future role is the Harness Server source repository. It is not the user's Product Repository and not a Harness Runtime Home. No Harness Server, runtime, generated projection system, conformance runner, runtime data, product implementation code, or generated operational artifact exists here yet. Documentation acceptance does not authorize implementation by itself; server/runtime implementation may start only after documentation acceptance and a separate implementation-planning readiness decision.

## Minimal First-Read Path

Use this path when you do not know where to start:

1. [Overview](learn/overview.md) for the first mental model.
2. [One Task](learn/one-task.md) for the feel of one user work loop.
3. [Concepts](learn/concepts.md) for the minimum vocabulary.
4. [User Guide](use/user-guide.md) for practical user and agent interaction.
5. [Implementation Overview](build/implementation-overview.md) only if you are reviewing or planning future Harness Server implementation.
6. [Reference Index](reference/README.md) only when you need exact contracts.

This path is intentionally small. First-time readers do not need to read large Reference docs before they understand what Harness is for.

## Reader Paths By Role

| Reader | Start here | Then use |
|---|---|---|
| General user | [Overview](learn/overview.md) | [One Task](learn/one-task.md) for the work-loop feel; [User Guide](use/user-guide.md) for practical session behavior; [Concepts](learn/concepts.md) only when terms need names. |
| Agent instruction writer | [Agent Guide](use/agent-guide.md) | [Agent Integration Reference](reference/agent-integration.md), [Surface Cookbook](reference/surface-cookbook.md), and the specific API owner only when exact fields are needed. |
| Server implementer | [Implementation Overview](build/implementation-overview.md) | [Engineering Checkpoint](build/engineering-checkpoint.md) -> [MVP-1 User Work Loop](build/mvp-user-work-loop.md) -> [MVP API](reference/api/mvp-api.md) -> [Storage](reference/storage.md) -> [Security Reference](reference/security.md). Use [Runtime Walkthrough](build/runtime-walkthrough.md) only for the intended request-to-close design path. |
| Documentation maintainer | [Authoring Guide](maintain/authoring-guide.md) | [Documentation Checks](maintain/documentation-checks.md), [Translation Guide](maintain/translation-guide.md), [Rewrite Plan](maintain/rewrite-plan.md), [Rewrite Acceptance Review](maintain/rewrite-acceptance-review.md), and Reference owners only when checking strict meaning. |
| Later/profile reader | [Assurance Profile](later/assurance-profile.md) | [Operations Profile](later/operations-profile.md), [Future Fixtures](later/future-fixtures.md), and [Roadmap](roadmap.md). These are outside the MVP path unless an owner promotes them. |

## Document Roles

Learn, Use, Build, Reference, Later, and Maintain do different jobs:

| Family | Role |
|---|---|
| Learn | Why Harness exists, where authority lives, and the concepts readers need before strict contracts. |
| Use | How users and agents should interact during Harness-assisted work. |
| Build | Future implementation sequence, stage boundaries, and maintainer handoff. |
| Reference | Exact owner contracts: schemas, DDL, gates, state transitions, projection rules, security meanings, conformance semantics, templates, and terminology. |
| Later | Later assurance, operations, future fixture, and roadmap material kept out of the MVP implementation path. |
| Maintain | Documentation rules, redesign scope, parity expectations, and drift handling. |

## Learn

Use Learn when you want the authority-boundary mental model before exact contracts.

| Page | Distinct role |
|---|---|
| [Overview](learn/overview.md) | Primary first read: what Harness is, why it exists, what it keeps separate, and what it is not. |
| [One Task](learn/one-task.md) | Primary Learn walkthrough: one ordinary request through clarification, scope, evidence, checks, residual risk, acceptance, and close. |
| [Concepts](learn/concepts.md) | Minimum vocabulary for first-time readers, with internal labels kept optional. |
| [Harness in 15 Minutes](learn/harness-in-15-minutes.md) | Shortened old-link route that points readers to the active Learn path. |
| [Purpose and Principles](learn/purpose-and-principles.md) | Optional thesis check for reviewers: values, failure model, non-goals, and MVP boundary. |

## Use

Use this path when you want to run or describe an AI-assisted development session under Harness.

- [User Guide](use/user-guide.md) is the primary user entry.
- [Agent Guide](use/agent-guide.md) is agent/integration behavior guidance.
- [User-owned judgment examples](use/decision-packet-cookbook.md) are advanced usage and reference-adjacent decision example material.

## Build

Use Build for implementation orientation and planning review. Until [Documentation Acceptance Status](build/implementation-overview.md#documentation-acceptance-status) explicitly accepts implementation planning readiness, Build pages remain planning guidance and do not authorize runtime/server implementation.

Server implementer fast path:

1. [Implementation Overview](build/implementation-overview.md) for current status, maintainer handoff, and the future repository role.
2. [Engineering Checkpoint](build/engineering-checkpoint.md) for the first internal authority-loop smoke, explicitly not product MVP.
3. [MVP-1 User Work Loop](build/mvp-user-work-loop.md) for the first user-value implementation plan and server-coding decision log.
4. [MVP API](reference/api/mvp-api.md), [API Schema Core](reference/api/schema-core.md), and [API Errors](reference/api/errors.md) for active MVP-1 tools, shared shapes, resources, errors, idempotency, and state conflicts.
5. [Storage](reference/storage.md) for runtime layout, staged storage profiles, locks, artifacts, and migrations.
6. [Security Reference](reference/security.md) for MVP-1 cooperative/limited-detective guarantee wording and local-access boundaries.

[Runtime Walkthrough](build/runtime-walkthrough.md) is a design walkthrough of intended behavior, not proof that runtime exists. [Core Model Reference](reference/core-model.md) owns exact request-to-close state behavior.

Keep future/diagnostic material outside the MVP implementation path unless a Build or Reference owner explicitly promotes it for the stage being planned.

## Reference

Use Reference to look up exact contracts. Do not read the whole Reference set by default; pick the owner for the question in front of you. The [Reference Index](reference/README.md) is the compact owner-contract map.

| Need | Owner |
|---|---|
| Core authority, entities, gates, state transitions, write authority, and close semantics | [Core Model Reference](reference/core-model.md) |
| MVP public tools, envelopes, schemas, errors, idempotency, state conflict behavior, shared refs, and validator result schema | [MVP API](reference/api/mvp-api.md), [API Schema Core](reference/api/schema-core.md), [API Errors](reference/api/errors.md) |
| Later/profile-gated API methods and future schema material | [API Schema Later](reference/api/schema-later.md) and [Assurance Profile](later/assurance-profile.md) |
| Runtime layout, DDL profiles, storage JSON, locks, artifacts, migrations, baselines, projection job storage, and validator storage | [Storage](reference/storage.md) |
| Readable views, projection freshness, managed blocks, and template bodies | [Projection And Templates Reference](reference/projection-and-templates.md) and [Template Reference](reference/templates/README.md) |
| Trust boundaries, assets, threat categories, controls, and guarantee-level wording | [Security Reference](reference/security.md) |
| Operator behavior, diagnostics, recover/reconcile/export/artifact checks, and conformance run entrypoint | [Operations And Conformance Reference](reference/operations-and-conformance.md) |
| Fixture model, fixture body, runner/assertion semantics, Kernel Smoke queue, and later scenario inventory | [Conformance Fixtures Reference](reference/conformance-fixtures.md) and [Future Fixtures](later/future-fixtures.md) |
| Connector profiles, context push/pull, fallback behavior, surface recipes, and user-facing integration patterns | [Agent Integration Reference](reference/agent-integration.md) and [Surface Cookbook](reference/surface-cookbook.md) |
| Design-quality policies, validator IDs, severity composition, waiver semantics, and policy close impact | [Design Quality Policies](reference/design-quality-policies.md) |
| Public/internal terminology and owner routing | [Glossary Reference](reference/glossary.md) |
| Runtime spaces, Core transaction placement, architecture flow, artifacts, projection/reconcile placement, and recovery overview | [Runtime Architecture Reference](reference/runtime-architecture.md) |

## Later

Use Later docs for material that must stay out of the MVP implementation path unless an owner promotes it.

- [Assurance Profile](later/assurance-profile.md)
- [Operations Profile](later/operations-profile.md)
- [Future Fixtures](later/future-fixtures.md)
- [Roadmap](roadmap.md)

## Maintain

Use Maintain to keep the docs and future Harness system coherent over time. Maintain docs govern documentation maintenance, not runtime behavior. Documentation Checks are read-only docs-maintenance checks; their `PASS`, `WARN`, and `FAIL` labels help review but do not create runtime conformance, manual acceptance, close readiness, or implementation readiness.

- [Authoring Guide](maintain/authoring-guide.md)
- [Documentation Checks](maintain/documentation-checks.md)
- [Translation Guide](maintain/translation-guide.md)
- [Rewrite Plan](maintain/rewrite-plan.md)
- [Rewrite Acceptance Review](maintain/rewrite-acceptance-review.md)

## Current Status Model

The current status separates documentation review, implementation planning readiness, and runtime implementation:

| Status category | Current status |
|---|---|
| Documentation review status | Post-redesign review; documentation acceptance candidate only. Maintainers have not accepted the docs yet. |
| Implementation planning readiness | Not accepted. Maintainers must confirm the implementation-readiness criteria before first runtime-batch planning. |
| Runtime implementation status | Not started. No runtime artifacts or conformance results exist here yet. |
| Implementation decision status | Open server-coding decision-ledger items are recorded in the [MVP-1 User Work Loop](build/mvp-user-work-loop.md#implementation-decisions-needed-before-server-coding). No server/runtime implementation decision has been formally accepted for coding. Affected implementation work must wait until the relevant decision is accepted or explicitly deferred with stage impact. |

Documentation acceptance, when it happens, is a maintainer review milestone. It does not by itself start runtime/server implementation or prove runtime conformance.

## Maintainer Handoff

Before starting Harness Server code, implementers should read:

1. [Maintainer handoff summary](build/implementation-overview.md#maintainer-handoff-summary).
2. [Documentation acceptance status](build/implementation-overview.md#documentation-acceptance-status).
3. [Rewrite Acceptance Review](maintain/rewrite-acceptance-review.md).
4. [Implementation-readiness criteria](build/implementation-overview.md#implementation-readiness-criteria).
5. [Implementation decisions needed before server coding](build/mvp-user-work-loop.md#implementation-decisions-needed-before-server-coding).

This handoff says the documentation is available for maintainer acceptance review as a candidate. It also separates documentation acceptance status in the Implementation Overview from the open server-coding decision ledger in the MVP-1 User Work Loop. No server/runtime implementation decisions have been formally accepted for coding yet, and affected implementation work must wait until the relevant decision is accepted or explicitly deferred with stage impact. The handoff does not claim the docs have been accepted, it does not make the docs implementation-ready, and it does not start server/runtime implementation.

## Where Am I?

Harness keeps three spaces separate:

| Space | What belongs there |
|---|---|
| Product Repository | The user's product workspace: product code, tests, product docs, and future human-readable Harness views when generated for that product. |
| Harness Server source repository | The future codebase for the local Harness Server / Installation. This repository is intended to become that source repository after documentation acceptance and implementation-planning readiness. |
| Harness Runtime Home | Per-user/per-installation operational data: state database, artifact store, projection output, logs, and local registration/configuration. |

This repository's current role is documentation review/redesign. Documentation acceptance alone does not create implementation authority, runtime state, conformance, or server code.

## Comparison

Harness is not the same kind of thing as agent instructions, MCP, reusable workflows, tests, review, reports, dashboards, hosted agent platforms, sandboxes, or specs.

| Nearby piece | Role it plays | Harness role |
|---|---|---|
| AGENTS.md / agent instruction files | Tell agents how to behave in a repository or session. | Harness may rely on those instructions, but it keeps the local record of scope, user-owned judgment, evidence, close readiness, and risk. |
| MCP / API surfaces | Define protocol boundaries for tools, resources, and calls. | Harness may expose MCP/API surfaces, but those surfaces are mechanisms. The product authority comes from Core-owned local state and artifact references. |
| Skills / reusable workflows | Package repeated instructions or procedures for an agent to follow. | Harness can be used by those workflows, but it records the current work state and routes judgments for this task. |
| Test runners | Execute checks and produce results. | Harness links relevant results as evidence and keeps verification strength separate from final acceptance. |
| Code review | Provides human or team review of changes. | Harness can reference review outcomes, but it does not replace review or turn review into final acceptance, residual-risk acceptance, or close. |
| Reports / dashboards | Present readable summaries, status, or analytics. | Harness can derive readable views, but view text is not the operating record. |
| Hosted agent platforms | Run or coordinate agents as a service. | Harness is designed around a local work-authority server, not a hosted agent platform. |
| Sandboxes / OS permission systems | Provide isolation or permission enforcement at the system boundary. | Harness does not claim OS-level isolation or arbitrary-tool permission control unless a proven mechanism exists for that operation. |
| Specs | Describe intended behavior, design, or constraints. | Harness can use specs as input, but it records operational state for live work: scope, decisions, evidence, QA expectations, final acceptance, and remaining risk. |

## Agent Context Loading

Reader paths are not prompt-loading bundles. Connected agents should keep always-on context to one screen or less: current Task summary, work shape, scope/non-goals, pending user judgments, active blockers, next safe actions, evidence gaps, close blockers, residual-risk summary, guarantee level, and source refs/freshness.

Use owner sections by phase and pull only what explains the next action. The detailed phase profile map lives in [Agent Integration Reference: Context Push/Pull Principles](reference/agent-integration.md#context-pushpull-principles), with user-facing behavior summarized in [Agent Guide](use/agent-guide.md).

## Roadmap

- [Roadmap](roadmap.md)

Future candidate items live in the roadmap. The roadmap is not part of Build-owned staged delivery unless a future owner explicitly promotes an item through the Roadmap criteria.

## Language Parity

The English and Korean documentation sets keep the same file map and semantic content. Korean headings and prose may be natural Korean rather than sentence-by-sentence mirrors of English.
