# Authoring Guide

## Document Role

This document owns the rules that keep the harness documentation set small, implementable, and correctly layered.

It does not own runtime behavior, user procedure, conformance fixture content, MCP schemas, SQLite DDL, or projection templates.

## Ownership Boundaries

Use exactly one canonical owner for each concept. Other documents may include a one-sentence summary and a link.

| Layer | Canonical owner |
|---|---|
| one-sentence definition, reader paths, document list, target tree summary | `README.md` |
| shared reader mental model, three-space summary, core concepts introduction | `00-introduction.md` |
| project purpose, target users, values, scope, non-goals, automation philosophy | `01-project-charter.md` |
| why, failure model, MVP boundary, Strategic Invariants, Kernel Authority Invariants, Design Stewardship Defaults | `02-strategy.md` |
| entity meanings, lifecycle, gates, state transitions, close semantics, `prepare_write` and `close_task` logic | `03-kernel-spec.md` |
| three spaces, runtime authority flow, artifact architecture, projection/reconcile architecture, guarantee levels | `04-runtime-architecture.md` |
| MCP resources/tools, request/response schemas, error taxonomy, validator result schema, artifact ref shape | `05-mcp-api-and-schemas.md` |
| reference MVP implementation order, SQLite DDL, migrations, storage layout, validator runner skeleton | `06-reference-mvp.md` |
| Markdown projection principles, managed blocks, human-editable sections, template tiers, template summaries | `07-document-projection.md` |
| shared design, decision quality, autonomy boundary, domain language, vertical slice, feedback loop/TDD, module/interface, codebase stewardship, Manual QA, context hygiene policies | `08-design-quality-policy-pack.md` |
| agent surface capability profile, common connector contract, fallback semantics | `09-agent-integration.md` |
| user-facing conversation, status reading, resume procedure, approval/assurance/QA/acceptance explanation | `10-user-guide.md` |
| connect, doctor, serve MCP, projection refresh, reconcile, recover, export, artifact integrity, conformance | `11-operations-and-conformance.md` |
| full templates and expanded variants | `appendix/A-template-library.md` |
| surface-specific cookbooks | `appendix/B-surface-cookbook.md` |
| later automation and derived analytics | `appendix/C-later-roadmap.md` |
| old-to-new mapping and migration notes | `appendix/D-migration-notes.md` |
| official term definitions | `glossary.md` |

## Principle Group Language

The strategy owns three principle groups: Strategic Invariants, Kernel Authority Invariants, and Design Stewardship Defaults. Do not promote helpful practices into Kernel Authority Invariants unless the owner doc is updated.

Strategic Invariants wording should preserve the differentiated promise:

```text
Strategic agency stays with the user.
The work journey remains followable from current state.
```

Kernel Authority Invariants wording should sound mandatory and structural:

```text
Product write requires an active scoped Change Unit.
Blocking product judgment requires a recorded Decision Packet.
Projection cannot override canonical state.
```

Design Stewardship Defaults wording should name applicability, waiver, record, validator, and close impact:

```text
Vertical slice is the default for feature work when it applies.
A horizontal exception may be recorded with a reason and follow-up.
```

Current Design Stewardship Defaults are shared design, domain language consistency, vertical slice default, TDD trace for suitable work, module/interface review, codebase stewardship, Manual QA, feedback loops, and context hygiene.

## MVP, v1, And Later Labels

Use these labels consistently:

| Label | Meaning |
|---|---|
| MVP | required for the reference implementation to validate Kernel Authority Invariants and Agency Conformance |
| v1 | plausible next version after MVP, still requiring fixtures and ownership |
| later | useful future automation that must not read as an MVP requirement |

Rules:

- Main docs may mention later work only as non-MVP context and should point to `appendix/C-later-roadmap.md`.
- Do not put Appendix C later-automation items or team workflow expansion into MVP requirements.
- If a later item becomes v1, add conformance expectations and an owner before changing main docs.
- Derived metrics are analytics unless explicitly promoted as MVP-critical conformance signals.

## Source-Of-Truth Phrasing

Use this phrasing family:

```text
Operational state is canonical in state.sqlite current records plus state.sqlite.task_events.
Raw evidence is canonical in the artifact store.
Markdown reports are projections generated from state records and artifact refs.
Human-editable sections are input surfaces.
Accepted human edits become state only through reconcile or a Core state-changing action.
```

Avoid phrasing that implies a separate MVP event store:

```text
phrases that put state.sqlite beside a separate event log
```

If historical comparison needs that idea, immediately clarify that MVP event history is `state.sqlite.task_events`.

Do not write:

```text
TASK is canonical state.
Projection updates state.
User Notes are the source-of-truth.
Domain Language is canonical in the Markdown document.
Report projections are raw artifacts by default.
```

Preferred authority paths:

```text
User Notes: human-editable input -> reconcile_items -> accepted state event/record
Domain Language: domain_terms -> DOMAIN-LANGUAGE projection
Module Map: module_map_items -> MODULE-MAP projection
Interface Contract: interface_contracts -> INTERFACE-CONTRACT projection
```

## Schema And Template Ownership

MCP tool request/response schemas, common envelope, error taxonomy, validator result schema, and artifact ref shape belong only in `05-mcp-api-and-schemas.md`.

SQLite DDL, migration/versioning, lock policy, artifact directory layout, and reference implementation storage details belong only in `06-reference-mvp.md`.

Projection rules and template tiers belong in `07-document-projection.md`. Full template bodies and expanded report variants belong in `appendix/A-template-library.md`.

User-facing examples may show Journey Cards or short report snippets, but they must not become schema definitions.

## Current-State Writing

Write canonical docs as current truth, not as rewrite history.

Preferred:

```text
The harness uses lifecycle fields plus gates.
```

Avoid in main docs:

```text
Unlike the old version, the harness now uses lifecycle fields plus gates.
```

Version comparison, removed sections, and old file names belong in `appendix/D-migration-notes.md`.

## Cross-Reference Rules

Use links to point to owners instead of duplicating contracts.

Minimum references:

- Strategy references kernel and policy pack.
- Kernel references API and reference MVP.
- Runtime architecture references kernel, projection, and integration.
- API references kernel and reference MVP.
- Reference MVP references kernel, API, and operations.
- Projection references kernel and Appendix A.
- Policy pack references kernel and projection.
- Integration references API and Appendix B.
- Operations references API and reference MVP.

## TODO Rules

Use `TODO_DECISION` only when a real product or architecture decision is unresolved. Include the decision needed, affected docs, and likely owner.

Use `TODO_IMPLEMENT` only when the decision is already made but implementation detail, DDL, fixture coverage, or CLI behavior is not yet filled in.

Do not use `TODO_REWRITE` in finished v2 canonical sections. A remaining `TODO_REWRITE` means the section is still a migration stub.

## Authoring Checklist

```text
[ ] Does this concept have exactly one canonical owner?
[ ] Are schema and DDL kept in their owner docs?
[ ] Are Strategic Invariants, Kernel Authority Invariants, and Design Stewardship Defaults kept separate?
[ ] Are Design Stewardship Defaults written with applicability and waiver boundaries?
[ ] Are MVP, v1, and later labels clear?
[ ] Are long-term analytics kept out of MVP requirements?
[ ] Does source-of-truth phrasing preserve state/artifact/projection boundaries?
[ ] Does the user guide avoid DB/API/connector internals?
[ ] Does operations use fixture-based conformance?
[ ] Are legacy names confined to migration notes?
[ ] Are official terms aligned with glossary?
```
