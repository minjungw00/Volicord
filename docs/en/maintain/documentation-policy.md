# Documentation policy

Use this policy when changing maintained Volicord documentation. It defines the
documentation governance model for authors, reviewers, translators, and agents.

This is a maintenance policy. It does not define product behavior, API
behavior, storage effects, security guarantees, runtime behavior, schemas, Core
authority semantics, conformance results, QA results, acceptance decisions,
close-readiness state, or residual-risk decisions.

## Metadata And Document Kinds

Use [`docs/doc-index.yaml`](../../doc-index.yaml) as the machine-readable route
for maintained documentation. Version 3 metadata records `doc_id`, maintained
paths, document `kind`, summary, normative level, translation policy, primary
audience, reader journeys, focused `canonical_for` ownership where needed,
maintenance `owner_area`, `created_on`, `last_updated_on`, `last_verified_on`,
root `default_applicability`, optional entry-specific `applies_to`, and
`depends_on` relationships.

`canonical_for` names the information or contract area owned by a document.
`owner_area` names the durable maintenance responsibility domain for keeping
that entry accurate. The two fields are related but not interchangeable.

Dates use `YYYY-MM-DD`. `created_on` records the earliest verifiable
introduction date for the maintained file or bilingual pair. `last_updated_on`
records the latest verifiable content-update date for that file or pair.
`last_verified_on` records maintenance verification of the indexed paths,
metadata, links, pairing, and owner routing; it is not product acceptance,
runtime conformance, QA completion, close readiness, a security proof, or
residual-risk acceptance. Applicability catalog keys use stable semantic
identifiers without release, schema, or toolchain numbers. Each catalog entry
names the owning file or registry in `version_source`; maintenance tooling reads
the current value from that owner. `default_applicability` applies its non-empty
list to every entry. An entry uses `applies_to` only for non-empty additional
catalog values and does not repeat a root default.

Use these document kinds by reader purpose:

- `landing`: introduces a product, repository, or documentation area.
- `tutorial`: leads a reader through an executable sequence.
- `how_to`: explains how to complete a concrete task.
- `explanation`: teaches concepts, architecture, rationale, or code structure.
- `reference`: owns exact product contracts or routes readers within Reference.
- `maintenance`: guides documentation authors, translators, reviewers, and
  agents.

Most landing, tutorial, how-to, and explanation documents should not carry
`canonical_for`. Use `canonical_for` only when the document is a stable owner of
a defined subject, especially focused Reference contracts and maintenance
policies.

Use [`docs/terminology-map.yaml`](../../terminology-map.yaml) as the structured
terminology and identifier-preservation source of truth. The terminology map
does not define API, storage, schema, security, projection, or runtime behavior.

Use [Brand Guidelines](brand-guidelines.md) as the maintenance owner for
Volicord brand spelling, official bilingual brand copy, component presentation,
project-local visual principles, and brand claim boundaries. The brand
guidelines do not define product behavior, API behavior, storage effects,
schemas, security guarantees, or Core authority semantics.

Use [Product and Maintenance Charter](product-maintenance-charter.md) as the
durable maintenance charter for Volicord product identity, service planning
principles, documentation roles, code-guidance boundaries, test philosophy,
compatibility discipline, and length-gate rejection. The charter does
not move exact contracts out of focused Reference owners.

Use [Document Charters](document-charters.md) when deciding what major documents
and document families should own, exclude, diagram, and link to. The charters
turn the metadata model into practical scope guidance for high-traffic
documents; they do not move exact product contracts out of focused Reference
owners.

Use [Diagram Policy](diagram-policy.md) when creating, reviewing, captioning, or
maintaining diagrams. It defines diagram categories, caption expectations,
arrow-semantics guidance, accuracy-owner expectations, and placement boundaries
so workflow diagrams, component maps, runtime sequences, authority models,
storage lifecycles, connection setup flows, and dependency graphs stay distinct.
It also keeps authoring and review metadata separate from reader-facing prose:
ordinary README, guide, Reference, and Architecture Guide pages should explain a
diagram's purpose in natural captions or surrounding prose instead of exposing
literal labels such as `Diagram role:`.

<a id="surface-stability-labels"></a>
## Surface Stability Labels

Use this small vocabulary when a maintained owner needs to distinguish public
contracts, evolving local integration surfaces, implementation details, and
diagnostic output. These labels classify documented surfaces; they do not create
schema versions, migration versions, alternate API versions, legacy
compatibility paths, storage upgrade paths, or fallback behavior.

| Label | Meaning |
|---|---|
| `stable` | A documented public or baseline contract intended for implementation and integration reliance. |
| `beta` | A documented surface that is supported in the current workspace but still expected to evolve within its owner-defined boundary. |
| `internal` | A documented implementation, storage, process-binding, generated-wrapper, or adapter detail that is not a public contract or ordinary user-facing selector. |
| `diagnostic` | Human-readable summaries, reports, health views, disclosure text, or troubleshooting output. Structured fields and stable IDs are contracts only when the focused owner explicitly says so. |

Focused Reference owner pages that define public, beta, internal, or diagnostic
surfaces must include a short `## Surface Stability` section with the stable
anchor `<a id="surface-stability"></a>`. Keep the section compact, use labels at
the narrowest practical surface level, and link back to this vocabulary instead
of repeating long explanations. Do not use stability labels to downgrade a
currently stable contract without support from the focused owner documents.

## Ownership Boundaries

Exact product contracts stay in the focused Reference owners selected from
`doc-index.yaml` or the [Reference Index](../reference/README.md). This
includes baseline scope, API behavior, schema meaning, error meaning, storage
effects, security wording, access boundaries, close-readiness meaning, product
terminology, out-of-scope promotion rules, and value-set meaning.

Reader-facing documents may summarize, explain, teach, or sequence contract
material, but they must link to the focused Reference owner when exact behavior
matters. Do not turn a guide, tutorial, how-to, explanation, README, route page,
Maintain page, `AGENTS.md`, example, implementation comment, test, fixture, CLI
help, or generated output into a second contract body.

Treat duplication by information ownership, not by wording similarity. Repeating
short orientation prose can be useful. Repeating API behavior, schema fields,
storage effects, security guarantees, value meanings, or owner maps creates
competing authority unless the repeated material belongs to that document.

If no focused owner exists for a needed normative meaning, report the owner gap
or update the applicable owner first. Do not fill the gap in a non-owner
document.

Keep baseline behavior separate from reserved, profile-gated, and out-of-scope
material. A value name can appear in schemas, examples, storage notes, or route
pages without becoming baseline behavior.

<a id="architecture-design-references"></a>
## Architecture Design References

The maintained current-state architecture-design family lives under
[`docs/en/architecture-guide/design/`](../architecture-guide/design/README.md)
and
[`docs/ko/architecture-guide/design/`](../../ko/architecture-guide/design/README.md).
Its individual design pages describe the implementation that exists now. They
are not chronological decision records, review reports, migration guides, or
release histories.

Except for the family index, every current architecture-design page uses one H1
title followed by this exact H2 sequence:

1. `Purpose`
2. `Design`
3. `Invariants`
4. `Responsibility boundaries`
5. `Execution flow`
6. `Failure behavior`
7. `Scope exclusions`
8. `Implementation routes`
9. `Reference owners`

The paired Korean pages use the semantically equivalent sequence `목적`, `설계`,
`불변 조건`, `책임 경계`, `실행 흐름`, `실패 동작`, `범위 제외`, `구현 경로`,
`참조 담당 문서`. The family index remains a reader route rather than an
individual design page. Individual design pages do not add nested heading
sections outside this positive schema.

Design pages own current implementation structure, module responsibility,
execution flow, and durable implementation invariants. Exact product behavior,
schema meaning, storage effects, security guarantees, and Core authority
semantics remain in focused Reference owners and are linked from `Reference
owners`.

Architecture-design pages contain only the current implementation structure
described by the positive heading schema. Each current design has one maintained
route and no duplicate or numeric-version-selected representation. Structural
validation enforces the heading schema; prose quality remains an owner and
review concern.

## Examples And Source Links

Examples should be stable, self-contained product or user scenarios. They show
the documented shape without creating product policy.

Explain example paths, placeholder values, and sample filenames affirmatively:
say what the value represents for the reader, such as an example Product
Repository path or an authority bundle output path. Reserve negative wording
for safety, authority, routing, persistence, or user-decision boundaries.

API method reference examples must be method-local. Introduce every required
ref, `state_version` fact, artifact ref, run ref, judgment ref, blocker ref, and
file path inside the method document or state it as a method-local precondition.
Do not build a shared cross-method scenario spine across method reference pages.

Review examples against method owners, schema owners, value-set owners, and
storage-effect owners where relevant. Unsupported enum-like values, stale
response shapes, mismatched required fields, and inconsistent response branches
are documentation failures.

Source-code links and Architecture Guide prose should describe durable crates,
modules, entry points, execution stages, and responsibility boundaries. Avoid
line-number-dependent explanations, private helper catalogs, and implementation
history. When code structure changes durably, update the relevant Architecture Guide
document, especially [Architecture](../architecture-guide/architecture.md), in the same
documentation batch.

## Durable Maintained Content

Maintained documentation should describe the stable current model. Do not store
task history, PR notes, migration narratives, scratch notes, generated runtime
records, archive copies, conversion notes, unresolved review notes, work logs,
or task-specific follow-up plans in maintained documentation.

Maintained documentation, shared metadata, README files, and `AGENTS.md` files
are not Volicord runtime homes. Do not store runtime data, generated logs, SQLite
files, product runtime homes, test runtime homes, generated projections, fixture
output, QA results, acceptance records, close-readiness state, or residual-risk
records in them.

If a documentation tool creates generated output during editing or validation,
remove it before finishing unless it is ordinary ignored build output.

## Scoped Working Rules

Read the root [`AGENTS.md`](../../../AGENTS.md) before changing repository files.
Under `docs/`, also read [`docs/AGENTS.md`](../../AGENTS.md). For work that
crosses documentation and Rust implementation boundaries, also read
[`crates/AGENTS.md`](../../../crates/AGENTS.md).

`AGENTS.md` files are repository working guidance. They do not define product
contracts, runtime behavior, API behavior, storage effects, security guarantees,
or Core authority semantics.

When adding, removing, renaming, or repurposing a maintained document, update
`doc-index.yaml`, paired-language routes, reader navigation, terminology paths,
and links in the same change.
