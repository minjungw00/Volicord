# Documentation Working Rules

These rules apply to maintained Volicord documentation and documentation
metadata under `docs/`. They add documentation-specific routing to the root
`AGENTS.md`. They do not define product behavior, public API behavior, storage
effects, security guarantees, schemas, Core authority semantics, conformance
results, QA results, acceptance decisions, close-readiness state, or
residual-risk decisions.

## First Reads

- Read the root `AGENTS.md` before changing documentation files.
- For English-facing documentation edits, read
  `docs/en/maintain/documentation-policy.md`.
- For Korean-facing documentation edits, read
  `docs/ko/maintain/documentation-policy.md`.
- For bilingual edits, translation review, parity review, or
  terminology-affecting edits, read both documentation policies, both
  translation policies, `docs/terminology-map.yaml`, and the relevant glossary
  entries.
- Use `docs/doc-index.yaml` as the machine-readable route for maintained
  document paths, paired-language routing, owner areas, applicability,
  dependencies, and focused `canonical_for` scopes.
- Use `docs/terminology-map.yaml` as the terminology and
  identifier-preservation source of truth.
- Use `docs/en/maintain/document-charters.md` or
  `docs/ko/maintain/document-charters.md` when deciding what major documents
  and document families should own, exclude, diagram, and link to.
- Use `docs/en/maintain/brand-guidelines.md` or
  `docs/ko/maintain/brand-guidelines.md` for Volicord spelling, official
  bilingual brand copy, component presentation, visual-presentation principles,
  and brand claim boundaries.
- Use `docs/en/maintain/diagram-policy.md` or
  `docs/ko/maintain/diagram-policy.md` when creating, reviewing, captioning,
  or maintaining diagrams.
- For work that crosses documentation and Rust implementation boundaries, also
  read `crates/AGENTS.md`.

## Routing By Change Type

- Exact API behavior, schema meaning, storage effects, security guarantees,
  error semantics, Core authority rules, close-readiness contracts, and
  value-set meanings route to the focused Reference owner selected from
  `docs/doc-index.yaml`, `docs/en/reference/README.md`, or
  `docs/ko/reference/README.md`.
- Public API method list changes route to
  `docs/en/reference/api/methods.md`,
  `docs/ko/reference/api/methods.md`, and the linked method owners.
- Administrative CLI changes route to `docs/en/reference/admin-cli.md` or
  `docs/ko/reference/admin-cli.md`.
- MCP transport, public tool exposure, and Agent Connection changes route to
  `docs/en/reference/mcp-transport.md`,
  `docs/ko/reference/mcp-transport.md`,
  `docs/en/reference/agent-connection.md`, and
  `docs/ko/reference/agent-connection.md`.
- Storage, Storage DDL, artifacts, records, effects, and versioning changes
  route to the matching Storage Reference owner under `docs/en/reference/` or
  `docs/ko/reference/`.
- Runtime home, product repository, local output, and generated host
  configuration changes route to
  `docs/en/reference/runtime-boundaries.md`,
  `docs/ko/reference/runtime-boundaries.md`, and adjacent CLI or MCP owners.
- Architecture Guide changes caused by durable source movement route to
  `docs/en/architecture-guide/architecture.md`,
  `docs/ko/architecture-guide/architecture.md`,
  `docs/en/architecture-guide/change-guide.md`,
  `docs/ko/architecture-guide/change-guide.md`, and the nearest focused Architecture Guide
  page.
- Onboarding, installation, and agent-host setup changes route through the
  relevant README or User Guide page listed in `doc-index.yaml`.
- Maintenance-process changes route to the applicable Maintain owner, not to a
  reader-facing guide.

## Content Boundaries

- Do not turn a guide, tutorial, how-to, explanation, index, README, Maintain
  page, `AGENTS.md`, example, generated output, implementation comment, test,
  fixture, or CLI help into a second contract body.
- When a documentation edit affects normative meaning, edit the canonical owner
  selected from `docs/doc-index.yaml`. If no focused owner exists, report the
  owner gap or route to the closest applicable owner instead of filling the gap
  in a non-owner document.
- Reader-facing documents may summarize, explain, teach, or sequence contract
  material, but they must link to the focused Reference owner when exact
  behavior matters.
- Keep baseline behavior separate from reserved, profile-gated, and
  out-of-scope material.
- Use stable product or user scenarios in examples. Do not use documentation
  maintenance, route reshaping, or section restructuring as the API example
  scenario unless the document is specifically about documentation maintenance.
- Keep API examples internally consistent across request data, visible response
  state, `state_version`, refs, paths, artifact refs, run refs, judgment refs,
  sensitive approval reasons, and close-readiness evidence.
- Do not add task history, PR notes, short-lived plans, implementation logs,
  migration narratives, scratch notes, generated runtime records, archive
  copies, or work logs to maintained documentation.
- Do not edit generated files directly. Change the source, generator, template,
  or fixture and regenerate with the repository-supported command.
- Do not add one-off tests, validation scripts, or local artifacts for a
  documentation-only edit. Add durable checks only when they protect current
  documentation structure, owner routing, links, examples, terminology, or
  contracts.

## Language And Terminology

- English and Korean documentation are both maintained. Neither language is an
  archive, appendix, or translation-only copy.
- Match English and Korean documents by meaning unit, not by line count or
  sentence count.
- Preserve reader purpose, normative strength, owner routing, baseline and
  out-of-scope boundaries, user-judgment boundaries, guarantee strength,
  headings, tables, lists, examples, links, and exact identifiers when editing
  paired documents.
- Korean documentation must use natural Korean technical prose.
- Preserve exact identifiers, file paths, API methods, schema names, field
  names, enum values, status values, product labels, anchors, and code literals
  where the terminology map requires them.

## Generated And Runtime Files

- Maintained documentation, shared metadata, README files, and `AGENTS.md`
  files are not Volicord runtime homes.
- Do not store runtime data, generated logs, SQLite files, product runtime
  homes, test runtime homes, generated projections, fixture output, QA results,
  acceptance records, close-readiness state, residual-risk records, work logs,
  archive copies, or local scratch notes in maintained documentation.
- If a documentation tool creates generated output during editing or
  validation, remove it before finishing unless it is ordinary ignored build
  output.

## Validation

- After documentation edits, use `docs/en/maintain/validation.md` or
  `docs/ko/maintain/validation.md` and run the checks that match the changed
  files.
- For documentation metadata, route, link, and terminology-path changes, run:
  - `cargo run -p xtask -- docs-check`
- For bilingual changes, compare English and Korean by meaning unit after
  automated checks.
- For contract-adjacent changes, confirm exact behavior remains in the focused
  Reference owner and non-owner pages only summarize or link.
- For Architecture Guide changes caused by code movement, confirm the relevant
  Architecture Guide documents describe durable crates, modules, entry points,
  execution stages, and responsibility boundaries without defining product
  contracts.
- Before finishing, confirm changed links, file paths, anchors,
  paired-language links, owner routing, terminology, and repository hygiene.
