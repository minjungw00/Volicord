# Validation

Use this policy after maintained documentation edits. It separates structural
checks, human semantic review, Rust implementation validation, and result
reporting.

This is maintenance validation. It is not Volicord runtime conformance, product
acceptance, QA completion, close readiness, a security proof, or residual-risk
acceptance. The repository-local automated documentation validator is:

```sh
cargo run -p xtask -- docs-check
```

Regenerate the syntax-only Administrative CLI regions from the Clap command
model with:

```sh
cargo run -p xtask -- docs-sync
```

Validate the current workspace package graph with:

```sh
cargo run -p xtask -- architecture-check
```

## Structural Checks

For documentation metadata, route, link, and terminology-path changes, run
`cargo run -p xtask -- docs-check` from the repository root. The command is
read-only and verifies the machine-checkable shape:

- `docs/doc-index.yaml` parses as YAML and has `version: 3`.
- Required top-level sections are present and unsupported top-level fields are
  rejected.
- Every `contract_sources` entry declares one supported current owner kind,
  owner location, and non-empty semantic document selectors. Each selector
  resolves to maintained paired documents, and each owner kind has one catalog.
- The `owner_areas` catalog uses stable identifiers with string descriptions.
  Applicability keys use lowercase semantic words separated by underscores and
  contain no embedded version numbers.
- Every applicability entry declares one supported `version_source`.
  `docs-check` reads current workspace package and Rust values from the root
  `Cargo.toml`, MCP production revisions from `ProtocolRegistry`, and metadata
  schema values from `docs/doc-index.yaml` or `docs/terminology-map.yaml`.
- `default_applicability` is a non-empty duplicate-free list whose values
  resolve to the applicability catalog.
- `entry_schema` declares exactly the current applicability descriptions,
  required shared and paired fields, optional fields, maintenance fields,
  document kinds, reader journeys, normative levels, and translation policy.
- Every shared entry uses only `doc_id`, `path`, `kind`, `summary`,
  `normative_level`, `owner_area`, `created_on`, `last_updated_on`,
  `last_verified_on`, `applies_to`, `primary_audience`, `journeys`,
  `canonical_for`, and `depends_on`.
- Every paired entry uses only `doc_id`, `path_en`, `path_ko`, `kind`,
  `summary`, `normative_level`, `translation_policy`, `owner_area`,
  `created_on`, `last_updated_on`, `last_verified_on`, `applies_to`,
  `primary_audience`, `journeys`, `canonical_for`, and `depends_on`.
- Required fields are present for each shared or paired entry; `applies_to` is
  optional.
- `owner_area` resolves to the top-level owner-area catalog.
- When present, `applies_to` is a non-empty duplicate-free list of additional
  catalog values and does not repeat a root default.
- `created_on`, `last_updated_on`, and `last_verified_on` use valid
  `YYYY-MM-DD` calendar dates ordered as
  `created_on <= last_updated_on <= last_verified_on`.
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
- The exact root pair `README.md` and `README.ko.md` is accepted as the only
  maintained root-level semantic-parity pair.
- If `README.ko.md` exists, it must be indexed with `README.md` as the root
  README pair; missing indexed root README paths are reported by the normal
  path-existence rules.
- Paired documents preserve the same heading-level sequence.
- Every individual current architecture-design document under
  `docs/en/architecture-guide/design/` and
  `docs/ko/architecture-guide/design/` uses the language-specific exact H2
  sequence defined by the Documentation Policy and has no nested heading
  sections outside that positive schema.
- For each paired document selected by `contract_sources`, `docs-check` builds
  owner catalogs directly from the generated public JSON Schemas, the
  `volicord-command-model` command tree, the generated typed diagnostic
  registry artifact, and the current protocol registry. Public-schema catalogs
  include property names, named schema definitions and titles, enum strings,
  and constant strings.
- The checker compares catalog members within corresponding parsed Markdown
  meaning units: heading coordinates, paragraphs, nested list items, table
  cells, definition entries, callouts, footnotes, and fenced examples. A unit's
  structural coordinate uses heading and block ordinals rather than translated
  heading text. Moving an identifier to another paragraph, list item, or table
  cell is therefore a mismatch even when it remains under the same heading.
- Recognition is exact catalog membership, including simple lowercase values,
  `snake_case` fields, hyphenated CLI tokens, dotted diagnostic codes, and
  protocol identifiers. Inline code is contract-bearing only when an exact
  owner catalog contains the token; arbitrary prose and unrelated inline code
  are ignored. Structured JSON/YAML examples contribute parsed keys and literal
  values. A contract-bound JSON/YAML fence can declare
  `contract=<source_id>` to reject unknown keys against that current owner.
  The `<!-- contract-source: <source_id> -->` metadata immediately before a
  table or other block binds that block's inline identifiers to the selected
  current owner and rejects unknown tokens. Shell examples and generated
  Administrative CLI regions use their routed command-model owner. Fuzzy
  matching may suggest nearby current identifiers in diagnostics but never
  accepts a spelling.
- Identifier diagnostics are deterministic and name the document pair,
  structural meaning unit, current contract source and owner, and the missing
  or invalid identifier.
- Existing-file and duplicate-path rules apply to the root README pair in the
  same way they apply to other indexed paths.
- Relative links resolve to existing files.
- Fragment links and hidden anchors resolve where they are used.
- Maintained English/Korean pairs use equivalent local Markdown reader links
  after indexed targets are normalized to `doc_id`, valid non-indexed
  repository targets are normalized to repository-relative paths, and fragments
  are preserved. The exact root README pair uses this same local semantic-link
  and fragment parity mechanism. External links, images, and fenced-code text
  are ignored for this parity check.
- Executable `volicord` examples use an explicit `sh cli-example` fence and
  parse through the actual public Clap command model. Generic shell fences,
  `text` fences, and displayed output are not inferred to be executable CLI
  examples.
- Generated Administrative CLI synopsis regions match the current public Clap
  command tree and exclude hidden internal commands. The maintained owner paths
  come from the `reference.admin-cli` entry in `docs/doc-index.yaml`.
- Terminology role metadata for identity-sensitive terms uses the allowed role
  set and includes the required roles for public selectors, storage internals,
  MCP process bindings, and diagnostics.
- `docs/terminology-map.yaml` primary-owner and related-reference paths exist
  and are represented in `doc-index.yaml`.
- The operation-category table in the paired API value-set owners matches
  the generated JSON Schema for
  `volicord_types::values::OperationCategory`.
- Focused Reference owner pages that the documentation policy marks as needing
  surface labels include a `Surface Stability` section, link to the canonical
  vocabulary, and use only `stable`, `beta`, `internal`, or `diagnostic` labels.
  These paths resolve from their focused `doc_id` entries.
- The paired Storage DDL owner paths resolve from
  `reference.storage-ddl`, and their marked SQL regions match the canonical
  Store SQL sources.
- A tracked file must not match the repository artifact-exclusion rules owned
  by `.gitignore`.

`docs-check` does not search Rust or Markdown lines for prohibited words or
phrases. Prose quality, brand claims, security wording, and host-support wording
remain owner and review concerns. Diagnostic identity is covered by typed
diagnostic registries and rendering tests, not by documentation vocabulary
searches.

After automated structural validation, manually confirm the remaining
repository hygiene:

- No generated records, runtime homes, SQLite files, generated logs, archive
  copies, conversion notes, scratch notes, local inventories, or work logs
  remain in maintained documentation, including untracked working files that
  are outside the Git-index check.

## Workspace Architecture Validation

Run `cargo run -p xtask -- architecture-check` after changing the root
`Cargo.toml`, a workspace member manifest, package placement, or an internal
dependency. The command reads actual workspace package identity and normal,
development, and build dependency edges from Cargo metadata. It compares them
with `workspace.metadata.architecture` in the root `Cargo.toml`, which is the
single machine-readable owner for package-to-group assignments and allowed
internal dependency directions.

The check rejects undeclared or missing workspace packages, unknown groups,
disallowed kind-specific edges, production normal/build dependencies on
test-support groups, and dependencies from Core-facing groups to adapter
groups. CI runs this command as a focused workspace check. Its tests use neutral
synthetic graphs for general validator behavior and read the current workspace
directly for the repository graph case.

## Human Semantic Review

For bilingual changes, compare English and Korean by meaning unit. Preserve
reader purpose, normative strength, owner routing, baseline and out-of-scope
boundaries, user-judgment boundaries, negative clauses, non-claims, guarantee
strength, headings, tables, lists, examples, links, and exact identifiers.
For a contract-bearing meaning unit, also preserve the identifier in the same
parsed structural coordinate. The current policy defines no exception that
allows moving an owner-derived identifier between paragraphs, list items, or
table cells.

For contract-adjacent edits, confirm exact API behavior, schema meaning, error
meaning, storage effects, security wording, access boundaries, close-readiness
meaning, value-set meaning, and Core authority semantics remain in the focused
Reference owner. Non-owner pages should summarize and link, not become second
contract bodies.

For terminology changes, check the terminology map for identifier-presentation
policy, preferred and contextual forms, natural Korean guidance, and owner path
integrity. Check exact contract identifiers against the current owners selected
by `contract_sources`.

For brand-presentation or broad-claim changes, check the [Brand Guidelines](brand-guidelines.md)
for Volicord spelling, official bilingual brand copy, component presentation,
test harness term boundaries, visual principles, and claim restrictions. Confirm
exact product behavior, API behavior, storage effects, schemas, security
guarantees, and Core authority semantics still route to their Reference owners.

For managed-host claims, classify each statement separately as environment
applicability, setup or configuration state, behavioral connection verification,
or operational session authority. Check those layers against
[System Requirements](../reference/system-requirements.md),
and [Agent Connection](../reference/agent-connection.md#validated-agent-session).
A setup, configuration, implementation, fixture, or test fact does not establish
a current managed session. A successful observation describes only the behavior
tested for the current configuration and environment; future behavior requires
new observation.

For API and Reference examples, check method-local consistency, request and
response shape, field names, required fields, nullability, enum-like values,
`state_version`, refs, artifact refs, run refs, judgment refs, close-readiness
blockers, response branches, and links to applicable owners where relevant.

For Architecture Guide changes caused by code movement, confirm the relevant
Architecture Guide documents describe durable crates, modules, entry points, execution
stages, and responsibility boundaries without turning implementation detail into
product contract text. For current architecture-design references, also compare
the English and Korean pages section by section, confirm each page describes
only the present implementation, and verify every implementation route and
focused Reference-owner link.

The automated `docs-check` command includes local documentation-link parity,
heading-level structure parity, and current-owner-derived exact-identifier
parity by parsed structural meaning unit for maintained English/Korean pairs.
It proves neither that prose with no recognized identifier says the same thing
nor that a shared identifier is used with the same meaning. It also does not
perform full semantic bilingual review, contract-owner review,
technical-accuracy review, translation judgment, API example consistency
review, or product meaning review. Those responsibilities stay with human
semantic review and the focused owners.

## Durable Tests

When a documentation or implementation change suggests a new automated check,
keep it in the repository when it asserts current durable behavior, a contract,
a state transition, user value, a stable abstraction boundary, or a maintained
validation rule.

File length, document length, and LOC counts are not durable quality checks.
Use [Product and Maintenance Charter](product-maintenance-charter.md) for the
quality-gate boundary, and prefer checks that validate owner routing,
contracts, links, examples, state transitions, and reader usability.

For implementation-layer placement and test-authoring examples, use
[Testing Strategy](../architecture-guide/testing-strategy.md). This validation policy
owns the maintenance-check, review, and reporting boundaries for those checks.

Tests for a closed surface assert its positive current shape:

- CLI help exposes only the current public option allowlist for the command.
- Maintained shell examples use supported `volicord` commands and options.
- Storage schema checks assert current canonical SQL, tables, columns, indexes,
  constraints, initialization, and validation behavior.
- MCP preflight and transport/schema checks assert current startup behavior,
  public tool exposure, and public schema projection. Public MCP schemas must
  keep hiding internal envelope and invocation fields as a stable abstraction
  contract.
- Terminology validation checks identity-sensitive role metadata instead of
  adding broad prose forbidden-word searches for identifiers such as
  `connection_id` or `project_id`.
- Diagnostic tests construct typed registry entries and compare human and
  structured rendering from those entries instead of searching arbitrary
  source lines for diagnostic words.

Name durable tests after the current contract, for example
`connect_help_exposes_only_public_connect_options`,
`documented_volicord_commands_match_public_cli_contract`,
`export_help_lists_authority_bundle`,
`mcp_public_schema_hides_internal_envelope_fields`,
`terminology_map_defines_identity_sensitive_roles`, or
`storage_registry_contains_current_contract_columns`.

## Maintainability Report

Use the maintainability report when reviewers need quick visibility into large
or complex repository surfaces:

```sh
cargo run -p xtask -- maintainability-report
```

The report is reviewer guidance. It lists signals such as the largest Rust,
test, and Markdown files, heuristic command parsing/execution/rendering mixes,
and obvious test-coverage hints where those can be inferred cheaply. It does
not define LOC limits, LOC exception allowlists, invalid long-file states, or a
requirement to split cohesive files by line count. Treat reported sizes and
signals as prompts for review questions about ownership, readability, test
coverage, and source structure.

When the report runs in CI, a failing exit status means the command could not
inspect the repository. A large file, long document, mixed signal, or coverage
hint is not a CI failure by itself.

## Onboarding Usability Validation

Use representative-user usability validation when onboarding, installation,
Codex setup, troubleshooting, or owner routing changes materially. This is
human usability testing; automated checks and an agent desk review do not
substitute for actual participants.

Confirm that a first-time operator can:

1. determine whether the documented platform and repository topology are
   supported;
2. install or select the exact `volicord` and Codex executables;
3. provision a `codex` Agent Connection with the `record` profile at
   `personal` or `shared` scope;
4. verify the connection and interpret each non-success result;
5. find a pending `UserActionRequest` with `volicord inbox`, resolve it with
   `volicord inbox resolve`, and resume agent work;
6. repair or remove only Volicord-managed configuration; and
7. find the focused schema or storage owner for an exact contract.

Record results outside maintained documentation. Do not commit participant
notes, screenshots, recordings, credentials, transcripts, Runtime Homes, or
fabricated completion claims.

## Release And Host Smoke Validation

Volicord release validation covers the ordinary five-target build, package,
checksum, binary-smoke, platform, Docker, and publication paths. Operational
Codex interoperability validation is a separate observation of the current
managed configuration and environment.

The durable repository checks for release packaging are:

```sh
cargo test --locked -p volicord-release-integrity-tests --all-targets --all-features
cargo run --locked -p volicord-release-smoke -- --bin <path-to-built-volicord>
```

These tests protect target coverage, version consistency, canonical text bytes,
archive shape, packaged-binary identity, checksum output, and workflow
semantics. Workflow validation inspects parsed action identity, matrix inputs,
step ordering, and invocation counts; it does not compare one complete shell
command.

The publish-disabled `tests/release-smoke` package owns the cross-platform
actual-binary harness. It uses a disposable Product Repository, Runtime Home,
and stable test-owned Codex fixture while delegating bounded process execution
and cleanup to `volicord-test-process`. The local composite action
`.github/actions/volicord-release-smoke` is the single workflow invocation
boundary. Ordinary CI passes its built debug binary exactly once. Every native
release packaging matrix entry passes the exact Linux, macOS, or Windows binary
already built for that target exactly once, before artifact staging. The smoke
uses public `volicord mcp serve`, so its session remains `manual_cli` and is not
managed-host evidence.

An optional smoke run with a real Codex installation may exercise managed
configuration, MCP initialization, required-tool discovery, safe tool round
trips, and Guard observations. Treat its result as an operational observation
for that configuration and environment. A reported Codex version is diagnostic;
a version change requires repeating the operational observation. Missing smoke
infrastructure is reported as skipped or unavailable and does not change the
ordinary Volicord release result.

Repository operational tests accept arbitrary bounded version strings through
connection verification, exercise initialize and tool-list milestones, check
required tools and safe calls, audit Guard artifacts and required-phase
observations, and verify session ownership and revision isolation.

Run live smoke only with disposable Product Repository and Runtime Home paths.
Keep credentials, prompts, transcripts, tokens, screenshots, and runtime data
outside the repository. Preserve the cooperative-host boundaries in
[Agent Connection](../reference/agent-connection.md): successful behavior does
not prove human identity, process identity, future host behavior, or policy
compliance outside the observed round trips.

## Rust Implementation Validation

If no Rust source, Cargo manifest, test, fixture, or build configuration is
changed, Rust validation is not required.

After Rust implementation edits, run the applicable Rust validation from the
workspace or changed crate:

- `cargo fmt`
- `cargo run -p xtask -- architecture-check` when workspace metadata, Cargo
  manifests, package placement, or internal dependencies change
- `cargo clippy --all-targets --all-features`
- `cargo test --all-targets --all-features`

Use narrower Cargo commands only when the repository structure or task scope
clearly calls for them, and report the reason.

## Generated Reference And Contract Drift Checks

Generated or source-derived reference surfaces use stable check commands:

- `cargo run -p xtask -- docs-sync` deterministically replaces only the marked
  syntax regions in the English and Korean Administrative CLI owners. Run it
  after changing the command model and review the generated diff.
- `cargo run -p xtask -- docs-check` checks maintained documentation structure,
  generated/source-derived documentation surfaces, executable `volicord`
  command examples, bilingual link/heading/exact-identifier parity, terminology
  metadata owner paths and roles, and canonical Storage DDL SQL blocks against
  `crates/volicord-store/src/schema/registry.sql` and
  `crates/volicord-store/src/schema/project.sql`.
- `cargo test -p volicord-integration-tests --test public_contract_snapshots`
  checks generated public contract snapshots for API request schema projections
  and MCP `workflow`/read-only tool projections against their Rust sources.
- `cargo test -p volicord-cli --test diagnostic_registry_contract` checks the
  generated machine-readable diagnostic-code artifact against the current typed
  registries. After an intentional registry change, regenerate it with
  `VOLICORD_UPDATE_DIAGNOSTIC_REGISTRY=1 cargo test -p volicord-cli --test diagnostic_registry_contract`
  and review `crates/volicord-cli/tests/fixtures/diagnostic-registry.json`.
- To regenerate those public contract snapshots after an intentional source
  change, run
  `VOLICORD_UPDATE_CONTRACT_SNAPSHOTS=1 cargo test -p volicord-integration-tests --test public_contract_snapshots`
  and review the generated files under `tests/integration/snapshots/`.

The public contract snapshot and diagnostic registry files are generated test
artifacts marked with `_generated`. Do not edit them by hand; update the typed
owner first, then regenerate. CLI public command drift remains covered by executable
documentation examples and CLI help/output tests such as the `volicord-cli`
`binary_admin` and `mcp_transport` test targets rather than by a separate CLI
JSON schema.

## Storage DDL Contract Check

When editing Storage DDL, `volicord-store` canonical SQL, or schema validation
code, run the focused owner-to-implementation consistency check:

```sh
cargo test -p volicord-store --test storage_ddl_contract
```

This check compares the authoritative English and Korean Storage DDL SQL with
the schemas initialized from canonical registry/project SQL in in-memory SQLite
databases. It checks schema semantics such as tables, columns, defaults,
constraints, foreign keys, indexes, partial indexes, and maintained triggers
without comparing Markdown prose or SQL formatting.

The repository documentation check also validates that the marked canonical SQL
blocks in English and Korean Storage DDL match the canonical registry/project
SQL source files.

This is a repository maintenance and implementation consistency check. It is
distinct from general documentation structure validation, public runtime
conformance, product acceptance, QA completion, close readiness, security
proof, and residual-risk acceptance.

## Reporting

Report validation results in the conversation, not in repository files. Include
changed files, checks performed, results, skipped checks with reasons, and
remaining documentation risks.

Use `PASS`, `WARN`, `FAIL`, or `SKIP` only as documentation-maintenance or
implementation-check outcomes. Do not describe a passing validation step as
Volicord runtime conformance, product acceptance, QA completion, close readiness,
a security guarantee, or residual-risk acceptance.
