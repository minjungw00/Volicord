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
`applies_to`, and `depends_on` relationships.

`canonical_for` names the information or contract area owned by a document.
`owner_area` names the durable maintenance responsibility domain for keeping
that entry accurate. The two fields are related but not interchangeable.

Dates use `YYYY-MM-DD`. `created_on` records the earliest verifiable
introduction date for the maintained file or bilingual pair. `last_updated_on`
records the latest verifiable content-update date for that file or pair.
`last_verified_on` records maintenance verification of the indexed paths,
metadata, links, pairing, and owner routing; it is not product acceptance,
runtime conformance, QA completion, close readiness, a security proof, or
residual-risk acceptance. `applies_to` uses stable identifiers from the
top-level applicability catalog rather than ambiguous values such as current,
latest, or all versions.

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
ordinary README, guide, Reference, and Development pages should explain a
diagram's purpose in natural captions or surrounding prose instead of exposing
literal labels such as `Diagram role:`.

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

## Examples And Source Links

Examples should be stable, self-contained product or user scenarios. They show
the documented shape without creating product policy.

Explain example paths, placeholder values, and sample filenames affirmatively:
say what the value represents for the reader, such as an example Product
Repository path or an exported MCP config output path. Reserve negative wording
for safety, authority, routing, persistence, or user-decision boundaries.

API method reference examples must be method-local. Introduce every required
ref, `state_version` fact, artifact ref, run ref, judgment ref, blocker ref, and
file path inside the method document or state it as a method-local precondition.
Do not build a shared cross-method scenario spine across method reference pages.

Review examples against method owners, schema owners, value-set owners, and
storage-effect owners where relevant. Unsupported enum-like values, stale
response shapes, mismatched required fields, and inconsistent response branches
are documentation failures.

Source-code links and developer-learning prose should describe durable crates,
modules, entry points, execution stages, and responsibility boundaries. Avoid
line-number-dependent explanations, private helper catalogs, and implementation
history. When code structure changes durably, update the relevant Development
document, especially [Architecture](../development/architecture.md), in the same
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

If a documentation tool creates temporary output during editing or validation,
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
