# Harness Documentation Set

Harness is an agency-preserving local operating kernel for AI-assisted development. It keeps the work journey followable while preserving the user's strategic judgment over goals, scope, design, trade-offs, codebase stewardship, QA, acceptance, and residual risk.

This file is `docs/en/README.md`, the entry point for the English harness documentation set. The repository root `README.md` is the repository landing page.

## Principle Groups

Strategic Invariants, Kernel Authority Invariants, and Design Stewardship Defaults are owned by [02-strategy.md](02-strategy.md#principle-groups). Kernel Authority Invariants are distinct from Design Stewardship Defaults.

## Reader Paths

General users:

```text
00-introduction.md
-> 10-user-guide.md
```

Implementers:

```text
00-introduction.md
-> 02-strategy.md
-> 03-kernel-spec.md
-> 04-runtime-architecture.md
-> 05-mcp-api-and-schemas.md
-> 06-reference-mvp.md
-> 11-operations-and-conformance.md
```

Connector authors:

```text
09-agent-integration.md
-> appendix/B-surface-cookbook.md
-> 11-operations-and-conformance.md
```

Projection maintainers:

```text
07-document-projection.md
-> appendix/A-template-library.md
-> 11-operations-and-conformance.md
```

Design-quality owners:

```text
02-strategy.md
-> 08-design-quality-policy-pack.md
-> 11-operations-and-conformance.md
```

Documentation authors:

```text
99-authoring-guide.md
-> glossary.md
```

## MVP / v1 / Later

MVP is a small local operating kernel that validates Kernel Authority Invariants and Agency Conformance, not a platform that supports many agent surfaces at once.

MVP focuses on one reference surface, local state, artifacts, public MCP tools, write gating, evidence, verification, Manual QA, acceptance, projections, reconcile, recovery, export, and fixture-based conformance.

Later automation is cataloged in [appendix/C-later-roadmap.md](appendix/C-later-roadmap.md) and must not read as part of MVP scope.

## Target Tree

This English documentation set lives under `docs/en/`. The Korean documentation set under `docs/ko/` mirrors the same structure.

```text
docs/en/
  README.md
  00-introduction.md
  01-project-charter.md
  02-strategy.md
  03-kernel-spec.md
  04-runtime-architecture.md
  05-mcp-api-and-schemas.md
  06-reference-mvp.md
  07-document-projection.md
  08-design-quality-policy-pack.md
  09-agent-integration.md
  10-user-guide.md
  11-operations-and-conformance.md
  99-authoring-guide.md
  glossary.md

  appendix/
    A-template-library.md
    B-surface-cookbook.md
    C-later-roadmap.md
    D-migration-notes.md
```

## Main Documents

| Document | Owner role |
|---|---|
| [00-introduction.md](00-introduction.md) | shared mental model for users and implementers |
| [01-project-charter.md](01-project-charter.md) | project purpose, audience, values, scope, and non-goals |
| [02-strategy.md](02-strategy.md) | strategic thesis, failure model, principle groups, Design Stewardship Defaults |
| [03-kernel-spec.md](03-kernel-spec.md) | operating kernel, entities, lifecycle, gates, transitions, close semantics |
| [04-runtime-architecture.md](04-runtime-architecture.md) | three spaces, runtime home, Core, artifact, projection/reconcile architecture |
| [05-mcp-api-and-schemas.md](05-mcp-api-and-schemas.md) | MCP resources/tools, schemas, errors, validators, artifact refs |
| [06-reference-mvp.md](06-reference-mvp.md) | MVP implementation sequence, DDL, storage layout, validator skeleton |
| [07-document-projection.md](07-document-projection.md) | Markdown projection, managed/human-editable areas, template tiers |
| [08-design-quality-policy-pack.md](08-design-quality-policy-pack.md) | design-quality policies as policy contracts |
| [09-agent-integration.md](09-agent-integration.md) | agent surface integration and capability profile |
| [10-user-guide.md](10-user-guide.md) | user conversation phrases, status reading, judgments, resume |
| [11-operations-and-conformance.md](11-operations-and-conformance.md) | operator procedures and fixture-based conformance |
| [99-authoring-guide.md](99-authoring-guide.md) | document ownership and authoring rules |
| [glossary.md](glossary.md) | official terms |

## Appendices

| Document | Owner role |
|---|---|
| [appendix/A-template-library.md](appendix/A-template-library.md) | full template library and expanded report variants |
| [appendix/B-surface-cookbook.md](appendix/B-surface-cookbook.md) | surface-specific connector notes and profile examples |
| [appendix/C-later-roadmap.md](appendix/C-later-roadmap.md) | later automation and post-MVP roadmap |
| [appendix/D-migration-notes.md](appendix/D-migration-notes.md) | migration context only; not an active canonical owner |
