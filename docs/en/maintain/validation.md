# Validation

Use this policy after maintained documentation edits. It separates structural
checks, human semantic review, Rust implementation validation, and result
reporting.

This is maintenance validation. It is not Harness runtime conformance, product
acceptance, QA completion, close readiness, a security proof, or residual-risk
acceptance. The repository-local automated documentation validator is:

```sh
cargo run -p xtask -- docs-check
```

## Structural Checks

For documentation metadata, route, link, and terminology-path changes, run
`cargo run -p xtask -- docs-check` from the repository root. The command is
read-only and verifies the machine-checkable shape:

- `docs/doc-index.yaml` parses as YAML and has `version: 2`.
- Every shared entry uses only `doc_id`, `path`, `kind`, `summary`,
  `normative_level`, `primary_audience`, `journeys`, `canonical_for`, and
  `depends_on`.
- Every paired entry uses only `doc_id`, `path_en`, `path_ko`, `kind`,
  `summary`, `normative_level`, `translation_policy`, `primary_audience`,
  `journeys`, `canonical_for`, and `depends_on`.
- Required fields are present for each shared or paired entry.
- `kind` values are only `landing`, `tutorial`, `how_to`, `explanation`,
  `reference`, or `maintenance`.
- `normative_level` values are only `contract`, `guide`, `example`, or
  `maintenance`.
- `translation_policy` is `semantic_parity` for maintained English/Korean
  pairs.
- `primary_audience`, `journeys`, `canonical_for`, and `depends_on` are lists
  when present.
- `doc_id` values are unique.
- Every indexed path exists.
- Every `depends_on` value resolves to an indexed `doc_id`.
- Every maintained paired Markdown file under `docs/en/` and `docs/ko/` is
  represented in the index with matching relative structure.
- Relative links resolve to existing files.
- Fragment links and hidden anchors resolve where they are used.
- `docs/terminology-map.yaml` primary-owner and related-reference paths exist
  and are represented in `doc-index.yaml`.
- README, route-page, Reference, Development, `AGENTS.md`, and terminology
  links do not point to retired documentation paths.

After automated structural validation, manually confirm repository hygiene:

- No generated records, runtime homes, SQLite files, generated logs, archive
  copies, conversion notes, scratch notes, temporary inventories, or work logs
  remain in maintained documentation.

## Human Semantic Review

For bilingual changes, compare English and Korean by meaning unit. Preserve
reader purpose, normative strength, owner routing, baseline and out-of-scope
boundaries, user-judgment boundaries, negative clauses, non-claims, guarantee
strength, headings, tables, lists, examples, links, and exact identifiers.

For contract-adjacent edits, confirm exact API behavior, schema meaning, error
meaning, storage effects, security wording, access boundaries, close-readiness
meaning, value-set meaning, and Core authority semantics remain in the focused
Reference owner. Non-owner pages should summarize and link, not become second
contract bodies.

For terminology changes, check the terminology map for exact identifiers,
preferred expressions, avoid expressions, Korean mixed-language controls, and
owner path integrity.

For API and Reference examples, check method-local consistency, request and
response shape, field names, required fields, nullability, enum-like values,
`state_version`, refs, artifact refs, run refs, judgment refs, close-readiness
blockers, response branches, and links to applicable owners where relevant.

For developer-learning changes caused by code movement, confirm the relevant
Development documents describe durable crates, modules, entry points, execution
stages, and responsibility boundaries without turning implementation detail into
product contract text.

The automated `docs-check` command does not perform semantic bilingual review,
contract-owner review, technical-accuracy review, translation judgment, API
example consistency review, or product meaning review. Those checks remain
manual and owner-routed.

## Rust Implementation Validation

If no Rust source, Cargo manifest, test, fixture, or build configuration is
changed, Rust validation is not required.

After Rust implementation edits, run the applicable Rust validation from the
workspace or changed crate:

- `cargo fmt`
- `cargo clippy --all-targets --all-features`
- `cargo test --all-targets --all-features`

Use narrower Cargo commands only when the repository structure or task scope
clearly calls for them, and report the reason.

## Reporting

Report validation results in the conversation, not in repository files. Include
changed files, checks performed, results, skipped checks with reasons, and
remaining documentation risks.

Use `PASS`, `WARN`, `FAIL`, or `SKIP` only as documentation-maintenance or
implementation-check outcomes. Do not describe a passing validation step as
Harness runtime conformance, product acceptance, QA completion, close readiness,
a security guarantee, or residual-risk acceptance.
