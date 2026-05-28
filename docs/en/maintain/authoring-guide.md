# Authoring Guide

## What this document helps you do

Use this guide when you add, rewrite, split, rename, or review Harness documentation.

It helps you keep the current documentation readable for the intended reader, clear about where each kind of detail belongs, and aligned between English and Korean.

This is maintenance documentation. It governs documentation maintenance only and does not authorize runtime/server implementation, product state changes, generated operational files, executable fixtures, runtime data, evidence records, QA results, acceptance decisions, or task closure before the documentation set is accepted for implementation planning. The first product MVP target is v0.1 Kernel MVP, exercised by Kernel Smoke as its narrow conformance profile. v0.2 through v0.4 are staged packs toward the Agency-Hardened MVP reference conformance target, and v1+ Expansion remains roadmap scope unless owner docs promote and prove it.

## Read this when

- You are adding, splitting, renaming, or reviewing documentation.
- You need to decide which document owns a strict contract.
- You are checking English/Korean parity, links, TODO hygiene, or duplicate owner text.

## Before you read

For exact runtime contracts, use the Reference owner documents linked below. For Korean wording rules, use [Translation Guide](translation-guide.md).

## Main idea

Keep each document useful for its reader and keep exact contracts in their owner Reference docs. Documentation-maintenance checks make drift visible, but they do not create runtime state, evidence, QA, acceptance, close readiness, or implementation readiness.

## Documentation principles

Write from the reader's next useful step. A document should make it easier for the reader to understand, decide, use, build, verify, or maintain something specific.

Prefer a small number of strong ideas over a complete inventory of internal machinery. Move strict contracts to Reference docs, and link to them when another document needs precision.

Introduce unfamiliar concepts with a concrete situation first. Readers should understand why a concept exists before they are asked to memorize its exact definition.

Keep the opening of every document predictable. A reader should quickly know what the document helps them do, when to read it, what they need first, and what idea will organize the rest of the page.

Write current documentation as current truth. Migration history, removed structures, and old names must stay out of the main explanation. If a dedicated migration note exists during a migration, keep that history there; otherwise rely on Git history or a clearly labeled non-active migration record.

## Document types

### Learn

Learn docs build the reader's mental model.

They explain purpose, concepts, examples, and trade-offs before implementation details. Use them when the reader needs orientation more than a command, schema, or checklist.

### Use

Use docs help a person operate Harness during an AI-assisted work session.

They should emphasize user-facing flow, status interpretation, decisions, and recovery paths. Mention internal gates only when they explain why the user sees a block or next action.

### Build

Build docs help an implementer construct the reference system after the documentation set is accepted for implementation planning.

They should explain implementation order, module boundaries, runnable slices, and verification strategy. Link to Reference docs for exact schemas, DDL, and invariants.

### Reference

Reference docs own exact contracts.

They are the place for strict schemas, gates, DDL, enum values, state transitions, invariants, API shapes, storage rules, projection rules, fixture semantics, and official definitions.

### Maintain

Maintain docs govern the documentation system itself.

They define authoring rules, translation policy, review checklists, link hygiene, ownership maps, and documentation-maintenance expectations. They must not become runtime conformance specs or product implementation plans.

## Entrypoint rule

README pages are routing pages before they are explanations. They should briefly say what Harness is and is not, then route first-time readers, users, implementers, reference readers, and maintainers to the right owner docs.

Keep entrypoints current and compact. Do not use them to preserve migration history, removed names, inactive paths, or old structures unless a section is clearly labeled as a non-active migration record.

README pages may summarize path ownership, but they should not copy strict contracts. Link to Reference owners for exact schemas, DDL, gates, state transitions, fixture semantics, template bodies, and official definitions.

First-time reader routes should include the fast practical tour before deeper Learn and Reference paths. Use routes should include practical Decision Packet examples near the User Guide so readers can understand judgment prompts before reading strict Reference contracts.

## Standard opening pattern

Every active document should begin with a short, predictable opening. Keep it compact, but make the reader's path clear. Template reference files under `reference/templates` use the template-specific opening below instead of the general opening headings.

### What this document helps you do

State the useful outcome in plain language. Avoid saying only what the document "covers."

### Read this when

Name the situation that makes the document relevant. This can be a short list.

### Before you read

Name the assumed context, prior document, or prerequisite. If there is no prerequisite, say so briefly.

### Main idea

Give the reader the central model or claim that will make the rest of the page easier to follow.

### Reference scope, only for reference docs

Reference docs should state the exact contract they own and what they deliberately do not own. This prevents strict details from spreading across Learn, Use, and Build docs.

### Template reference opening, only for `reference/templates`

Template reference files are the explicit exception to the general opening headings above. Docs-maintenance should identify them by path: `docs/*/reference/templates/README.md` for the directory index and non-README Markdown files under `docs/*/reference/templates/` for individual templates.

The directory README should begin with `Used when`, then `Template tiering`. It should explain that the directory owns rendered template bodies and display card shapes while projection rules, freshness behavior, and authority boundaries stay with their reference owners.

Each individual template file should begin with these sections, in this order:

- `Used when`: the reader purpose and projection or display situation.
- `Source records`: the owner records, refs, gates, artifacts, or summaries the renderer may read.
- `Rendered sections`: the display shape readers should expect.
- `Full template`: the complete rendered body or card body.

Template files must also make the non-authority boundary visible, either in the opening explanation or near `Notes`: a template is rendered display, not canonical state, gate authority, approval, acceptance, evidence, schema, DDL, or runtime behavior.

## Concept introduction rule

Introduce concepts through examples before strict definitions.

Start with a concrete situation, show what problem the concept solves, and then name the concept. Put the strict definition after the reader has seen why it matters.

Preferred shape:

```text
When an agent wants to change product state, Harness first needs to know which scoped implementation unit the write belongs to. That unit is the Change Unit. The larger user-value item the user wants finished or answered is the Task.
```

Avoid opening a Learn doc with a dense definition list unless the page is explicitly a glossary or reference table.

## Reference contract rule

Strict schemas, gates, DDL, enum values, state transitions, invariants, API shapes, storage rules, projection contract details, and fixture semantics belong in Reference docs.

Learn, Use, Build, and Maintain docs may summarize a contract in one or two sentences when needed, then link to the owning Reference document. They should not duplicate full tables, schema bodies, transition matrices, DDL blocks, or fixture mini-languages.

Runtime conformance fixture body shape, assertion modes, isolated execution behavior, JSON `TEXT` validation, and owner-bound enum/status validation are owned by [Operations And Conformance](../reference/operations-and-conformance.md#conformance-fixture-format). Other docs should summarize that conformance is executable-state-based and link to the owner instead of restating the full contract.

## Repetition rule

Do not repeat long source-of-truth paragraphs across docs.

When another document needs the same idea, write a short local summary and link to the owner. If the source text changes, update the owner first, then check summaries for drift.

Repeated explanatory examples are allowed when they serve different readers, but repeated normative contract language is a drift risk.

For the non-authority boundaries that are easy to repeat, use these owners:

| Boundary | Owner for exact wording |
|---|---|
| Context Index and retrieved/indexed context | [Roadmap: Context Index](../roadmap.md#context-index) for future feature boundary; [Agent Integration: Context Push/Pull Principles](../reference/agent-integration.md#context-pushpull-principles) for connector context handling |
| Local Derived Metrics | [Roadmap: Local Derived Metrics](../roadmap.md#local-derived-metrics) |
| Role Lens | [Agent Integration: Role Lens Behavior](../reference/agent-integration.md#role-lens-behavior) |
| Review Stages | [Design Quality Policies: Two-stage Review Display](../reference/design-quality-policies.md#two-stage-review-display) |
| Release Handoff and export | [Operations And Conformance: Release Handoff Export Profile](../reference/operations-and-conformance.md#release-handoff-export-profile); rendered shape in [EXPORT Template](../reference/templates/export.md) |
| Docs-maintenance | [Authoring Guide: Docs-maintenance checks](#docs-maintenance-checks) for rule bodies; [Operations And Conformance: Docs-maintenance profile](../reference/operations-and-conformance.md#docs-maintenance-profile) for operator reporting |
| Projection and report surfaces | [Document Projection Reference](../reference/document-projection.md); rendered shapes in [Template Reference](../reference/templates/README.md) |
| Security assets, trust boundaries, threat categories, control categories, and high-risk cooperative/detective/preventive/isolated security expectations | [Security Threat Model Reference](../reference/security-threat-model.md) for threat concepts; exact API, storage, kernel, connector, operations, and conformance behavior stays with those owners |

## Owner-link summary pattern

When you find duplicated normative language outside its owner, do not polish the duplicate in place. First decide which document owns the exact contract. Update that owner if the contract needs to change, then replace non-owner copies with:

- one ordinary-language sentence naming what the reader needs to know now
- one link to the owner document or owner section for exact rules
- any local consequence for the current reader path

Example:

```text
Product writes need current Change Unit scope and Write Authorization. Exact write-gate behavior is owned by [Kernel Reference](../reference/kernel.md), and the public request shape is owned by [MCP API And Schemas](../reference/mcp-api-and-schemas.md).
```

Do not paste the gate matrix, request schema, DDL block, fixture body, template body, enum table, or glossary definition into Learn, Use, Build, or Maintain docs.

## Diagram rule

Use diagrams only when they reduce cognitive load.

A diagram is useful when it shows a relationship, sequence, boundary, or lifecycle more clearly than prose. Do not add a diagram as decoration, as a second copy of an already clear list, or as a way to hide unresolved structure.

Every diagram should have nearby prose that explains what to notice. If a diagram and the prose disagree, the owning prose or reference contract is the source to fix first.

## English/Korean semantic parity rule

English and Korean docs must preserve the same active file map, semantic section coverage, and contractual detail.

Paired English/Korean files keep the same active file map and semantic section coverage. Heading text and minor grouping may be idiomatic when owner links, stable identifiers, and reviewability remain clear. Korean headings and prose may be natural Korean; different but semantically equivalent Korean headings are not an automatic docs-maintenance failure. Official identifiers, API names, schema names, enum values, DDL names, file names, error codes, validator IDs, code identifiers, and product terms listed in the translation guide must remain exact.

Any semantic change in `docs/en` must be mirrored in `docs/ko` in the same batch, and the reverse is also true.

## Link and rename rule

When you rename, move, split, or merge a document, update links in both languages in the same batch.

Prefer links to the owner document or owner section instead of links to secondary summaries. Do not point active owner links to removed migration context.

If old names, old structures, or migration decisions must be retained for review, keep them in a clearly labeled non-active migration record. Active docs should describe the current structure and link to current owners.

After a rename, search for old paths, old anchors, old headings, and old title text. Update the README path, nearby cross-references, template/reference links, and paired-language links together.

## Docs-maintenance checks

Docs-maintenance checks are read-only documentation maintenance. They may report documentation drift, owner mismatch, English/Korean parity issues, duplicate normative text outside the owner, broken links or anchors, and TODO hygiene problems. They are not Core fixture conformance, runtime validators, canonical state transitions, projection refresh, generated operational reports, QA results, acceptance records, evidence artifacts, residual-risk acceptance, close readiness, or implementation readiness. They do not execute fixture actions, seed runtime state, compare runtime state/events/artifacts/projections/errors, or count toward runtime fixture pass/fail.

Maintain docs may define documentation review rules, category labels, and reviewer expectations. They must not define runtime conformance pass/fail, runtime fixture semantics, Core state effects, gate behavior, or implementation readiness. When a docs-maintenance finding touches a runtime contract, the finding should point to the owner Reference document instead of restating that contract.

### Final pre-acceptance review

Before maintainers accept the documentation set for implementation planning, do one final docs-maintenance pass. Check English/Korean active file map parity, semantic section parity in paired files, broken links and anchors, owner-boundary drift, non-owner duplicate contracts, terminology drift for Approval, Decision Packet, Evidence, Verification, Manual QA, Acceptance, Residual Risk, Projection, and Guarantee Level, and TODO hygiene.

This final review is documentation maintenance only. It does not create runtime conformance, evidence, QA, acceptance, residual-risk acceptance, close readiness, implementation readiness, or canonical state. Use the existing docs-maintenance reporting expectations when recording findings; do not create a new required report format for this final pass.

A docs-maintenance review or future checker should report:

- category
- result: `PASS`, `WARN`, or `FAIL`
- file path
- heading or anchor when available
- owner document and expected source section
- observed drift
- suggested fix
- runtime effect statement: none; no canonical state transition was performed and no runtime fixture result was recorded

Resolve drift in this order:

1. Identify the owner document or owner section for the exact contract.
2. Update the owner first when the contract itself is wrong or incomplete.
3. Replace non-owner duplicate contracts with a short reader-focused summary plus owner link.
4. Mirror any English/Korean semantic change in the paired file during the same batch.
5. Repair links, anchors, TODO metadata, or glossary phrasing only after the owner boundary is clear.

Use these result meanings:

| Result | Meaning |
|---|---|
| `FAIL` | Drift can make active docs contradictory or non-actionable, such as broken owner links, schema/DDL/enum/stable event/`ValidatorResult`/`ProjectionKind` mismatch, missing paired active files, missing semantic section coverage, or non-owner text redefining an owner contract. Idiomatic heading text or minor grouping differences are not failures when owner links, stable identifiers, and reviewability remain clear. |
| `WARN` | Drift should be cleaned up but does not yet contradict an owner contract, such as minor glossary phrasing drift, duplicate explanatory prose that is not normative, stale but non-blocking cross-reference wording, or incomplete TODO metadata that is still understandable. |
| `PASS` | No relevant drift is found for the category. |

Required check categories:

| Category | Required check |
|---|---|
| English/Korean file structure parity | `docs/en` and `docs/ko` keep the same active document paths, README entries, and paired route expectations unless an exception is explicitly documented. |
| English/Korean semantic section parity | Paired files keep the same active file map, reader purpose, semantic section coverage, owner links, and contractual detail. Heading text and minor grouping may be idiomatic when stable identifiers, schema names, enum values, DDL names, validator IDs, code identifiers, and reviewability remain clear. |
| Opening convention compliance | Non-template active docs use the standard opening pattern. `docs/*/reference/templates/README.md` uses `Used when` plus `Template tiering`; individual template files under `docs/*/reference/templates/` other than `README.md` use `Used when`, `Source records`, `Rendered sections`, and `Full template`, plus a visible non-authority boundary. |
| Broken cross-reference detection | Markdown links, heading anchors, template/reference links, same-language README routes, paired-language entry links, and owner-section links resolve to active docs and current anchors. |
| Owner-boundary drift | Exact contracts and active owner concepts stay in their active owners, including `reference/kernel.md`, `reference/mcp-api-and-schemas.md`, `reference/storage-and-ddl.md`, `reference/document-projection.md`, `reference/templates/*.md`, `reference/design-quality-policies.md`, `reference/security-threat-model.md`, `reference/operations-and-conformance.md`, and `reference/glossary.md`; non-owner docs summarize and link rather than redefining those contracts. |
| Fixture/action schema drift | Operations fixture examples keep `action` and executable `input` aligned with public MCP request schemas in `reference/mcp-api-and-schemas.md` and the `ToolEnvelope` expansion convention in `reference/operations-and-conformance.md`; docs-maintenance may flag drift but does not execute fixture actions or restate fixture semantics here. |
| Enum, event, validator, and projection drift | State/gate/result values and Kernel Stable Event Catalog names match `reference/kernel.md`; error and stable `ValidatorResult` IDs match `reference/mcp-api-and-schemas.md`; storage values match `reference/storage-and-ddl.md`; `ProjectionKind` tiers and template ownership match `reference/document-projection.md` and `reference/templates/*.md`. |
| Glossary and source-of-truth phrasing drift | Official terms, capitalization, record ID prefixes, source-of-truth wording, and authority-boundary phrases match `reference/glossary.md` and the relevant owner docs without implying extra state authorities. |
| TODO compliance | `TODO_DECISION` and `TODO_IMPLEMENT` use the allowed meanings, name the gap clearly, include enough owner/context to act on, and do not leave `TODO_REWRITE` markers in finished canonical sections. |
| Non-owner duplicate full contracts | Full schemas, DDL, transition tables, fixture mini-languages, template bodies, enum tables, validator tables, projection tables, or glossary definitions outside the owner doc are replaced with a short summary plus owner link. |

## Review checklist

```text
[ ] Does the document serve a clear reader situation?
[ ] Do README entrypoints route first-time readers, users, implementers, reference readers, and maintainers quickly?
[ ] Does the opening follow the standard pattern, or the template-specific pattern for `reference/templates` files?
[ ] Are concepts introduced through examples before strict definitions?
[ ] Are strict schemas, gates, DDL, enums, and invariants kept in Reference docs?
[ ] Are long source-of-truth paragraphs and duplicated normative contract blocks summarized and linked instead of repeated?
[ ] Do diagrams reduce cognitive load?
[ ] Are English and Korean files semantically aligned?
[ ] Are official identifiers preserved exactly?
[ ] Are renamed paths, anchors, and README links updated in both languages?
[ ] Is current truth separated from migration history?
[ ] Are Maintain docs limited to documentation governance, not runtime behavior?
```

## Reference ownership map

Use this map when deciding where exact detail belongs. It identifies the active owner in the current documentation structure, so inactive paths do not remain part of the authoring workflow.

| Subject | Active owner |
|---|---|
| Repo and docs entrypoints, reader routes, language choice, document list, target tree summary | repo root `README.md`; docs root `docs/README.md`; language entrypoints `docs/en/README.md` and `docs/ko/README.md` |
| Shared reader mental model and three-space overview | `learn/overview.md` |
| Fast first-reader practical tour and short usage scenarios | `learn/harness-in-15-minutes.md` |
| Small core concept introduction | `learn/concepts.md` |
| Project purpose, target users, values, scope, non-goals, automation philosophy | `learn/purpose-and-principles.md` |
| Strategic thesis, failure model, MVP boundary, principle groups | `learn/purpose-and-principles.md` for reader explanation; `reference/design-quality-policies.md` and `reference/kernel.md` for exact contract impact |
| Kernel entities, lifecycle, gates, state transitions, close semantics, `prepare_write`, `close_task` | `reference/kernel.md` |
| Runtime architecture, three spaces in implementation detail, Core process model, artifact architecture, projection/reconcile architecture, guarantee levels | `reference/runtime-architecture.md` |
| Security assets, trust boundaries, threat categories, control categories, and high-risk cooperative/detective/preventive/isolated security expectations | `reference/security-threat-model.md` for threat concepts; exact enforcement, API, storage, kernel, connector, operations, and conformance behavior stays with those owners |
| MCP resources/tools, request/response schemas, error taxonomy, validator result schema, artifact ref shape | `reference/mcp-api-and-schemas.md` |
| SQLite DDL, migrations, storage layout, lock policy, artifact directory layout, baseline capture format, projection job table | `reference/storage-and-ddl.md` |
| MVP implementation order and stage exit criteria | `build/mvp-plan.md` |
| First runnable implementation slice | `build/first-runnable-slice.md` |
| Markdown projection principles, authority matrix, managed blocks, human-editable sections, artifact reference rendering, template tiers, projection freshness/failure rules | `reference/document-projection.md` |
| All projection template bodies and display card shapes | `reference/templates/*.md` |
| Design-quality policy contracts, validators, severity composition, waiver semantics, evidence expectations, close impact | `reference/design-quality-policies.md` |
| User-facing conversation, status reading, user judgments, close checklist | `use/user-guide.md` |
| Practical Decision Packet examples and user-facing judgment prompt patterns | `use/decision-packet-cookbook.md` for examples; `reference/kernel.md` and `reference/mcp-api-and-schemas.md` for exact Decision Packet behavior |
| User/agent session procedure | `use/agent-session-flow.md` |
| Agent surface capability profiles, common connector contract, fallback semantics, Role Lens, connector conformance overview | `reference/agent-integration.md` |
| Surface-specific recipes | `reference/surface-cookbook.md` |
| Generic capability profile examples | `reference/agent-integration.md` |
| Operator procedures, conformance fixture bodies, fixture assertion semantics, doctor/recover/reconcile/export/artifact integrity, docs-maintenance reporting | `reference/operations-and-conformance.md` |
| Official term definitions and capitalization | `reference/glossary.md` |
| v1+ Expansion roadmap | `roadmap.md` |
| Documentation authoring rules | `maintain/authoring-guide.md` |
| Translation and bilingual prose rules | `maintain/translation-guide.md` |
