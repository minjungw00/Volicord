# Harness Documentation

This is the English entrypoint for the reader-centered Harness documentation set.

Harness is an agency-preserving local authority kernel for AI-assisted product work. It keeps a local operating record of scope, user-owned judgment, write authority, evidence, verification, QA, acceptance, residual risk, and close.

For the full identity, authority model, integration-surface boundaries, and non-goals, read [Overview](learn/overview.md) and [Purpose and Principles](learn/purpose-and-principles.md).

## What Harness Is / Is Not

Harness is meant to make AI-assisted work followable from local state, durable evidence, and readable projections. This repository is still in documentation review; it does not contain the Harness server or runtime implementation.

Harness is not a chat script, prompt bundle, test harness, evaluation harness, dashboard, or replacement for the user's product repository, version control, tests, code review, or product and technical judgment.

## Quick Routes

| If you need to... | Start with | Then read |
|---|---|---|
| Understand Harness for the first time | [Overview](learn/overview.md) | [Harness in One Task](learn/harness-in-one-task.md) |
| Use Harness during assisted development | [User Guide](use/user-guide.md) | [Agent Session Flow](use/agent-session-flow.md) |
| Prepare implementation after doc acceptance | [Implementation Overview](build/implementation-overview.md) | [First Runnable Slice](build/first-runnable-slice.md), [MVP Plan](build/mvp-plan.md), then Reference |
| Look up exact behavior or stable names | [Reference](#reference) | The owner page for the contract you need |
| Maintain or review the docs | [Authoring Guide](maintain/authoring-guide.md) | [Translation Guide](maintain/translation-guide.md) |

## Ownership Rule

Reference docs own exact contracts: schemas, DDL, gates, state transitions, enum values, fixture semantics, template bodies, and official definitions. Learn, Use, and Build docs explain the idea for their reader and link to Reference instead of copying strict contract blocks.

## Learn

Start here to understand Harness before using or building it. Learn docs explain the mental model with concrete examples; strict contracts stay in Reference. The recommended path is [Overview](learn/overview.md) first, then [Harness in One Task](learn/harness-in-one-task.md).

- [Overview](learn/overview.md)
- [Harness in One Task](learn/harness-in-one-task.md)
- [Concepts](learn/concepts.md)
- [Purpose and Principles](learn/purpose-and-principles.md)

## Use

Use this path when you want to run an AI-assisted development session under Harness. Use docs prioritize user-facing flow, status interpretation, decisions, and recovery paths. Start with the user guide, then use the agent-session flow when you need to understand how the agent should proceed.

- [User Guide](use/user-guide.md)
- [Agent Session Flow](use/agent-session-flow.md)

## Build

Use this path for implementation orientation and later planning review. These pages do not authorize starting Harness server or runtime implementation, and first runtime-batch planning may begin only after maintainers deliberately update the Documentation Acceptance Status. Build docs explain order, module boundaries, runnable slices, and verification strategy without duplicating exact schemas or DDL.

Start with the [Documentation Acceptance Status](build/implementation-overview.md#documentation-acceptance-status) in Implementation Overview. It is the maintainer-updated place to tell whether work is still documentation maintenance, first runtime-batch planning may begin, runtime/server implementation has started, or open documentation follow-up issues are recorded.

- [Implementation Overview](build/implementation-overview.md)
- [First Runnable Slice](build/first-runnable-slice.md)
- [MVP Plan](build/mvp-plan.md)

## Reference

Use this path to look up detailed contracts, schemas, policies, and definitions. If another path summarizes a strict rule, the Reference owner is the source to update first.

- [Kernel Reference](reference/kernel.md)
- [Runtime Architecture Reference](reference/runtime-architecture.md)
- [MCP API And Schemas](reference/mcp-api-and-schemas.md)
- [Storage And DDL](reference/storage-and-ddl.md)
- [Document Projection Reference](reference/document-projection.md)
- [Design Quality Policies](reference/design-quality-policies.md)
- [Agent Integration Reference](reference/agent-integration.md)
- [Surface Cookbook](reference/surface-cookbook.md)
- [Operations And Conformance Reference](reference/operations-and-conformance.md)
- [Glossary Reference](reference/glossary.md)
- [Template Reference](reference/templates/README.md)

## Maintain

Use this path to keep the docs and future Harness system coherent over time. Maintain docs govern documentation maintenance, not runtime behavior.

- [Authoring Guide](maintain/authoring-guide.md)
- [Translation Guide](maintain/translation-guide.md)

## Roadmap

- [Roadmap](roadmap.md)

Post-MVP items live in the roadmap. The roadmap is not part of the MVP implementation contract unless a future owner explicitly promotes an item with scope, fixtures, and fallback behavior.

## Language Parity

The English and Korean documentation sets keep the same file map and semantic content. Korean headings and prose may be natural Korean rather than sentence-by-sentence mirrors of English.
