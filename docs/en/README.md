# Harness Documentation

This is the English routing page for the Harness documentation set.

Harness is a future local work-authority server for AI-assisted product work. Its authority is over Harness records and state transitions: scope, user-owned judgment, evidence, verification expectations, final acceptance, close readiness, and residual risk. It keeps fragile chat context from becoming the source of truth for those records.

That authority is not operating-system permission control, arbitrary-tool sandboxing, tamper-proof storage, default pre-tool blocking, or security isolation. MVP-1 wording should be read as cooperative plus limited detective behavior unless a specific future/profile mechanism is named and proven.

This repository is documentation-only today and is in post-redesign review. It is intended to become the Harness Server source repository only after documentation acceptance and a separate implementation-planning readiness decision. It is not the user's Product Repository, not a Harness Runtime Home, and not a running Harness instance.

No server/runtime implementation, runtime state, generated projection system, conformance runner, generated operational artifact, executable fixture, or product implementation code exists here. Documentation files are source material; they are not Harness runtime state, evidence, QA, acceptance, residual-risk, projection, or close records.

## Minimal First-Read Path

Use this path when you do not know where to start:

1. [Start](start.md) for the first mental model, one ordinary task, minimum concepts, and current guarantee boundary.
2. [User Guide](use/user-guide.md) for practical user and agent interaction.
3. [Implementation Overview](build/implementation-overview.md) only if you are reviewing future Harness Server implementation.
4. [Reference Index](reference/README.md) only when you need exact contracts.

This path intentionally stops before large Reference docs. First-time readers do not need schemas, DDL, transition tables, fixture bodies, or threat catalogs to understand what Harness is for.

## Reader Paths By Role

| Reader | Start here | Then use |
|---|---|---|
| General user | [Start](start.md) | [User Guide](use/user-guide.md) for practical session behavior. |
| Agent instruction writer | [Agent Guide](use/agent-guide.md) | [Agent Integration Reference](reference/agent-integration.md) and [Surface Cookbook](reference/surface-cookbook.md) only when exact connector or context behavior matters. |
| Future server implementer | [Implementation Overview](build/implementation-overview.md) | [Engineering Checkpoint](build/engineering-checkpoint.md) for the first internal proof, [MVP-1 User Work Loop](build/mvp-user-work-loop.md) for the first user-value slice, then [Reference Index](reference/README.md) for exact owners. |
| Exact contract reader | [Reference Index](reference/README.md) | Pick the owner for the specific contract instead of reading the whole Reference set. |
| Documentation maintainer | [Authoring Guide](maintain/authoring-guide.md) | [Translation Guide](maintain/translation-guide.md), [Documentation Checks](maintain/documentation-checks.md), [Rewrite Plan](maintain/rewrite-plan.md), and [Rewrite Acceptance Review](maintain/rewrite-acceptance-review.md). |
| Later/profile reader | [Assurance Profile](later/assurance-profile.md) | [Operations Profile](later/operations-profile.md), [Future Fixtures](later/future-fixtures.md), and [Roadmap](roadmap.md). These are outside the MVP path unless an owner promotes them. |

## Layer Responsibilities

| Family | Role | Boundary |
|---|---|---|
| Start | Explains why Harness exists, where authority lives, one ordinary task, the first concepts, and the current guarantee boundary. | Does not define schemas, gates, DDL, implementation sequence, or fixture mechanics. |
| Use | Explains user and agent usage through ordinary-language examples, agent behavior, judgment request handling, write checks, evidence summaries, and close flow. | Does not define canonical enums, DDL, or full transition tables. |
| Build | Explains future implementation sequence, active slice, first proof, active/later boundary, build reading path, and excluded areas. | Links to Reference for exact API shapes, schemas, DDL, storage tables, state transitions, fixture bodies, security guarantees, and threat catalogs. |
| Reference | Owns exact contracts: Core transition, API schema, Storage/DDL, Security, Agent Integration, Projection/Templates, Conformance, Glossary, runtime architecture, operations, and design-quality policy. | Does not serve as the first-read tutorial or staged implementation plan. |
| Later | Holds future/profile material outside the active MVP path. | Does not become active delivery unless an owner promotes it with scope and proof expectations. |
| Maintain | Governs documentation writing, translation, review, drift, owner-boundary, and link rules. | Does not decide runtime readiness, final acceptance, close readiness, or implementation readiness. |

## Build Route

Build pages are for future implementation orientation after documentation acceptance and a separate implementation-planning readiness decision. They describe sequence and stage boundaries; exact API, schema, storage, fixture, and security contracts stay in Reference. Build pages do not authorize server/runtime implementation.

Recommended Build order:

1. [Implementation Overview](build/implementation-overview.md): current state, handoff, readiness criteria, and reading path.
2. [Engineering Checkpoint](build/engineering-checkpoint.md): the first internal Core authority-loop proof, not the product MVP.
3. [MVP-1 User Work Loop](build/mvp-user-work-loop.md): the first user-value implementation plan and central server-coding decision log.
4. [Runtime Walkthrough](build/runtime-walkthrough.md): intended request-to-close design path, not evidence that runtime exists.
5. [Reference Index](reference/README.md): exact contract owners.

## Use Route

Use pages stay at the user and agent trust boundary.

- [User Guide](use/user-guide.md) is the primary user entry.
- [Agent Guide](use/agent-guide.md) is agent behavior guidance.
- [User-owned judgment examples](use/judgment-examples.md) gives practical judgment request examples without making full-format Decision Packet presentation a required user path.

Exact user judgment, write, run/evidence, close, projection, and error contracts are owned by Reference docs linked from the [Reference Index](reference/README.md).

## Reference Route

Use [Reference Index](reference/README.md) when you need exact contracts. It owns the compact map to Core state transitions, API schemas, Storage/DDL, Security, Agent Integration, Projection/Templates, Conformance, Glossary, runtime architecture, operations, and design-quality policy.

Do not copy Reference tables into Start, Use, Build, or Maintain pages. Non-owner pages should summarize the reader-visible consequence and link to the owner.

## Maintain Route

Use Maintain docs for documentation work only:

- [Authoring Guide](maintain/authoring-guide.md)
- [Translation Guide](maintain/translation-guide.md)
- [Documentation Checks](maintain/documentation-checks.md)
- [Rewrite Plan](maintain/rewrite-plan.md)
- [Rewrite Acceptance Review](maintain/rewrite-acceptance-review.md)

Docs-maintenance checks are read-only Markdown quality checks. Their `PASS`, `WARN`, and `FAIL` labels do not create runtime conformance, final acceptance, close readiness, or implementation readiness.

## Status Owners

Current handoff status lives in [Implementation Overview: Maintainer handoff summary](build/implementation-overview.md#maintainer-handoff-summary). Documentation acceptance status lives in [Implementation Overview: Documentation acceptance status](build/implementation-overview.md#documentation-acceptance-status). Server-coding decisions live in [MVP-1 User Work Loop: Implementation decisions needed before server coding](build/mvp-user-work-loop.md#implementation-decisions-needed-before-server-coding).

Documentation acceptance, when it happens, is a maintainer review milestone. It does not start runtime/server implementation and does not prove runtime conformance.

## Language Parity

English and Korean documentation keep the same active file map and semantic content. Korean prose may use natural Korean headings and paragraphing instead of sentence-by-sentence mirrors.
