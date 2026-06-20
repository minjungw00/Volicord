# Documentation Working Rules

These rules apply to maintained Harness documentation and documentation
metadata under `docs/`. They add documentation-specific guidance to the root
`AGENTS.md`. They do not define product behavior, API behavior, storage effects,
security guarantees, runtime behavior, schemas, Core authority semantics,
conformance results, QA results, acceptance decisions, close-readiness state, or
residual-risk decisions.

## Documentation Priorities

- Product onboarding should help readers understand and execute real first
  steps.
- Developer-learning documentation should explain the durable implementation
  shape accurately enough for implementers to orient themselves in the source.
- Reference documentation remains the authoritative home for exact product
  contracts.
- Maintenance metadata and checks support documentation quality and owner
  routing; they are not ordinary reader entry points.

## First Reads

- Use `docs/doc-index.yaml` as the machine-readable owner route for maintained
  documentation. It owns `doc_id`, paired paths, document role, owner scope,
  non-owner scope, dependencies, normative level, and audience metadata.
- Use `docs/terminology-map.yaml` as the terminology and
  identifier-preservation source of truth.
- For English-facing documentation edits, read
  `docs/en/maintain/authoring-guide.md`.
- For Korean-facing documentation edits, read
  `docs/ko/maintain/authoring-guide.md`.
- For bilingual edits, translation review, parity review, or
  terminology-affecting edits, read both authoring guides, the translation
  guides, `docs/terminology-map.yaml`, and the relevant glossary entries.
- For ordinary readers, prefer reader-facing entry points such as README,
  Start, Use, Build, and Reference pages. Do not require ordinary readers to
  start from `docs/doc-index.yaml`.

## Documentation Types

Classify documents by their maintained purpose and owner metadata, not by a
broad filename family.

- Landing documents introduce the product, repository, or documentation area.
  They may own substantial onboarding, orientation, and first-step content.
- Tutorial documents lead readers through executable sequences. They may own
  durable setup flow, prerequisites, expected checkpoints, and troubleshooting
  at guide level.
- How-to documents explain how to complete a concrete task. They may own
  substantial workflow guidance, operator steps, and reader-facing
  consequences.
- Explanation documents teach concepts, architecture, rationale, or code
  structure. They may own durable developer-learning explanations that reflect
  the source code.
- Reference documents own exact product contracts for their focused scope.
- Maintenance documents guide authors, translators, reviewers, and agents.
  They may own documentation process, metadata use, and check procedures.
- Index and route-only documents help readers choose a next document. Limit
  navigation-only behavior to pages whose purpose and owner metadata are
  actually index or route-only.

README, Start, Use, Build, and Maintain pages are not automatically
navigation-only. If their owner metadata gives them landing, tutorial, how-to,
explanation, or maintenance responsibility, they may carry substantial durable
reader-facing content within that scope.

## Owner And Contract Boundaries

- Exact API behavior, schema meaning, storage effects, security guarantees,
  error semantics, Core authority rules, close-readiness contracts, and
  value-set meanings stay in their focused Reference owners.
- Non-Reference documents may summarize, explain, teach, or sequence contract
  material for a reader, but they must link precisely to the applicable
  Reference owner when exact behavior matters.
- Do not turn a guide, tutorial, how-to, explanation, index, README, Maintain
  page, or `AGENTS.md` into a second contract body for API behavior, storage
  effects, schemas, security guarantees, Core authority semantics, or detailed
  owner maps.
- When a documentation edit affects normative meaning, edit the canonical owner
  selected from `docs/doc-index.yaml`. If no focused owner exists, report the
  owner gap or route to the closest applicable owner instead of filling the gap
  in a non-owner document.
- Contract owner pages may contain the detail that belongs to their `owner_for`
  scope. They should still avoid duplicating adjacent owners' API behavior,
  schema fields, storage effects, security guarantees, or other focused
  contracts.
- Keep baseline behavior separate from reserved, profile-gated, and
  out-of-scope material. Do not describe out-of-scope capabilities as baseline
  requirements.

## Documentation Content Rules

- Keep index and route-only pages short and navigational. If they start to need
  field tables, status-value tables, storage-effect detail, error behavior,
  guarantee levels, or long lists of prohibitions and exceptions, move that
  material to the applicable owner and leave a route link.
- Developer-learning documents may explain implementation structure, crate
  roles, source-module maps, execution stages, and durable responsibility
  boundaries. Exact product behavior still routes to the applicable Reference
  owner.
- Use stable product or user scenarios in examples. Do not make documentation
  maintenance, route reshaping, or section restructuring the API example
  scenario unless the document is specifically about documentation maintenance.
- Keep API examples internally consistent across request data, visible response
  state, `state_version`, refs, paths, artifact refs, run refs, judgment refs,
  sensitive approval reasons, and close-readiness evidence.
- Prohibit task history, PR notes, short-lived plans, implementation logs,
  migration narratives, scratch notes, generated runtime records, archive
  copies, and work logs in maintained documentation.
- Treat path allowlists and documentation batch boundaries as maintainer
  editing controls, not Harness runtime override capabilities.
- Use durable maintenance wording. Avoid task-specific, PR-specific, or
  short-lived wording in maintained documentation.

## Language And Terminology

- English and Korean documentation are both maintained. Neither language is an
  archive, appendix, or translation-only copy.
- Match English and Korean documents by meaning unit, not by line count or
  sentence count. Preserve reader purpose, normative strength, owner routing,
  baseline and out-of-scope boundaries, user-judgment boundary, and security
  guarantee level.
- Korean documentation must use natural Korean technical prose.
- Preserve exact identifiers, file paths, API methods, schema names, field
  names, enum values, status values, product labels, anchors, and code literals
  exactly where the terminology map requires it.
- Use the terminology map's Harness/Core distinction: Harness is the local
  work-authority product/system for AI-assisted product work, and Core is the
  local authority record for Harness state.

## Validation

- After documentation edits, start from `docs/*/maintain/checks.md`, then use
  focused check guidance such as structure, links and indexes, language parity,
  terminology, and API examples checks as applicable.
- For route and metadata edits, confirm changed paths exist, `doc_id` values
  remain unique, link targets are valid, and owner routing remains consistent.
- Before finishing, confirm no scratch files, archive copies, generated
  records, runtime homes, SQLite files, generated logs, or work notes remain
  from the edit.
