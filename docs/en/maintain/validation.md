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

## Changed-Owner Routing

Before broad discovery, derive the bounded route for the current Git changes:

```sh
cargo run -p xtask -- owner-route --changed
```

Use `--base <revision>` to include committed changes after an explicit series
base together with staged, unstaged, and untracked working-tree paths. Add
`--json` when another tool or agent will consume the result. The human and JSON
forms come from the same ordered report.

The command reads changed paths from Git, workspace package identity from Cargo
metadata, maintained document entries and language pairs from
`docs/doc-index.yaml`, and the validated instruction, direct-owner, and
validation-class associations in `docs/owner-routing.yaml`. It returns the
applicable root and scoped `AGENTS.md` files, changed packages, exact changed
document entries and their paired paths, direct owner documents, validation
classes, and paths not covered by a maintained route. Results are sorted and
duplicate-free. The command is read-only and does not infer ownership by
scanning arbitrary prose.

## Validation Profiles And Sequential Series

Before the first commit in a sequential change series, record its parent commit
as the series base. Use the same explicit revision for every profile invocation
in that series.

Run the focused profile for intermediate work:

```sh
cargo run -p xtask -- validate focused --base <revision>
```

The focused profile consumes `owner-route`, selects changed packages, direct
contract and generated-drift checks, documentation and architecture checks, and
repository hygiene. It does not run `cargo test --workspace` or another exact
workspace aggregate. Use `--json` when a tool or agent needs the exact summary.

After every planned commit is present and the worktree is ready for the final
gate, start one final-validation session:

```sh
cargo run -p xtask -- validate final --base <revision>
```

Do not run the final profile speculatively, concurrently, or once per
intermediate commit. The session owns the complete current repository policy,
including the exact aggregate and its bounded diagnostics. A failed final
session remains failed; report it and begin a corrected series instead of
describing decomposed results as closure.

The direct commands documented below remain stable focused checks and CI
building blocks. The validation profiles are authoritative for series-level
selection, sequencing, durable capture, aggregate handling, and summary status.

## Durable Command Results

Every profile creates an ignored
`target/volicord-validation/<run-id>/summary.json`. Each executed command has a
complete stdout log, stderr log, machine-readable result, exact invocation,
working directory, start and finish timestamps, and exit code under the same
run directory. Child stdout and stderr go directly to those files while the
command runs; they do not depend on a terminal buffer remaining attached.

The runner writes the initial summary before the first command and checkpoints
it before and after every command state change. If a terminal or process handle
is lost while the runner continues, recover the run by inspecting the reported
run ID and summary path. Pending work remains distinguishable from completed,
failed, and skipped commands. Validation output is ignored build output and is
not committed.

## Exact Aggregate And Bounded Decomposition

Only the final profile runs the exact aggregate:

```sh
cargo test --locked --workspace --all-targets --all-features
```

It runs once normally. When its output identifies one failing target in an
unchanged package, the runner may run that isolated target and the full package.
If both pass, it may retry the exact aggregate once. The exact aggregate never
runs more than twice in one final session.

After a second aggregate-only failure, the runner runs the workspace excluding
the identified unchanged package and runs that full package separately, then
stops. It does not add a permanent package exclusion and does not perform a
third exact attempt. It never applies this downgrade path to a changed-package
failure. No profile sets global `--test-threads=1` or `RUST_TEST_THREADS=1`
unless a maintained owner first requires that setting.

Passed, failed, decomposed, and skipped are independent summary categories. A
decomposed command can pass or fail, but its success never removes the failed
exact aggregate or changes the overall result to passed. Human and JSON output
are rendered from the same command records and category lists.

## Commit-Type Scope

The validation preflight checks commit subjects and practical file-scope
boundaries between the explicit base and `HEAD`. A `test:` commit must not
change production behavior. A `docs:` commit must not change production code or
runtime contracts. File-scope checks reject production implementation paths
where that distinction is machine-checkable; semantic review remains
responsible for contract meaning inside maintained documentation.

If test or documentation work exposes a production gap, put the production
change in a preceding `feat:`, `fix:`, or `refactor:` commit. Do not hide the
gap in a `test:` or `docs:` commit.

## Structural Checks

For documentation metadata, route, link, and terminology-path changes, run
`cargo run -p xtask -- docs-check` from the repository root. The command is
read-only and verifies the machine-checkable shape:

- `docs/doc-index.yaml` parses as YAML and has `version: 3`.
- Required top-level sections are present and unsupported top-level fields are
  rejected.
- Paired documents may declare duplicate-free semantic `contracts`. `DocIndex`
  resolves regular method pages to one request/response pair and requires
  non-conventional or multi-method pages to declare complete, duplicate-free
  `method_contracts`. Every declared or resolved contract exists in one
  machine-readable owner descriptor, and the normalized binding order is
  deterministic.
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
  `primary_audience`, `journeys`, `canonical_for`, `depends_on`, and
  `contracts`, and `method_contracts`.
- Required fields are present for each shared or paired entry; `applies_to` and
  paired-document `contracts` and `method_contracts` are optional.
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
- For each paired document with semantic contracts, every generator and
  validator consumes the same normalized `DocIndex` binding set and builds only
  those exact owner catalogs. Public request and response descriptors come from
  `volicord-types`; CLI syntax and values come from
  `volicord-command-model`; CLI inbox output comes from
  `volicord-user-action-presentation`; diagnostic codes come from the typed
  diagnostic registry; MCP identifiers come from the protocol registry.
  Deliberate adjacent-contract relationships are explicit in descriptors and
  never expand to a global public API catalog.
- The checker compares catalog members within corresponding parsed Markdown
  meaning units: heading coordinates, paragraphs, nested list items, table
  cells, definition entries, callouts, footnotes, and fenced examples. A unit's
  structural coordinate uses heading and block ordinals rather than translated
  heading text. Moving an identifier to another paragraph, list item, or table
  cell is therefore a mismatch even when it remains under the same heading.
- English and Korean units are validated independently against their document
  scope before valid units are compared for parity. An exact identifier owned
  by another contract is out of scope. A likely contract identifier absent from
  all current owners is invalid, including when both languages contain it.
- Recognition is exact catalog membership, including simple lowercase values,
  `snake_case` fields, hyphenated CLI tokens, dotted diagnostic codes, and
  protocol identifiers. Owner categories remain distinct. Arbitrary prose,
  paths, filenames, environment variables, and source identifiers are not
  treated as API fields.
- Every Reference JSON/YAML fence declares exactly one `shape=`. The resolved
  `DocIndex` binding set determines the available semantic contracts; when that
  shape is exposed by more than one available contract, the fence also declares
  exactly one `contract=<semantic_contract_id>` already bound to the document.
  Request and response descriptors remain separate.
- Before materializing an instance, the structured parser requires exact unique
  keys within every JSON object or YAML mapping at every nesting depth. YAML
  tags, anchors, aliases, merge keys, and non-string mapping keys are rejected.
- The selected exact schema validates the resulting unique-key JSON-compatible
  instance. Schema compilation errors are owner errors. Instance checks enforce
  required and unknown properties, types, nested objects and arrays, enum and
  const values, constraints, unions, references, and nullability. The exact
  `ToolError` schema also enforces the canonical public error-code/category
  relationship, so a mismatched pair is invalid even when both languages use
  it. A `schema` fence is reader-facing shape notation and is not treated as an
  instance.
- English and Korean instances are independently resolved, parsed, and
  schema-validated. Structural and exact-identifier parity runs only for a
  meaning unit that is valid in both languages.
- Shell examples and generated Administrative CLI regions use their routed
  command-model owner. Fuzzy matching may suggest nearby current identifiers
  in diagnostics but never accepts a spelling.
- Identifier diagnostics are deterministic and name the document pair,
  structural meaning unit, semantic contract and owner, and the out-of-scope,
  missing, or invalid identifier.
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
- `docs/owner-routing.yaml` resolves every instruction path, direct owner
  `doc_id`, workspace package, and supported validation class exactly once
  against the current instruction files, document index, and Cargo workspace.

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
with `workspace.metadata.architecture.packages` in the root `Cargo.toml`, which
is the single machine-readable owner for each package's semantic group,
bilingual responsibility, classification, production status, boundary, and
kind-specific internal dependency allowlists.

The check rejects undeclared or missing workspace packages, invalid or
duplicated responsibility groups, unresolved dependency owners, disallowed
kind-specific edges, production normal/build dependencies on test-support
packages, Core-facing dependencies on adapter or presentation packages, the
required UserAction service, Core, shared-types, and Store boundary violations,
and normal/build dependency cycles. CI runs this command as a focused workspace
check. Its tests use neutral synthetic graphs for general validator behavior
and read the current workspace directly for the repository graph case.

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
integrity. Check exact contract identifiers against the document's semantic
contracts and their current owner descriptors.

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

For the committed source distribution, use the canonical creation and
validation commands:

```sh
cargo run --locked -p xtask -- source-bundle --output /tmp/volicord-source.zip
cargo run --locked -p xtask -- source-bundle-validate --input /tmp/volicord-source.zip
```

The creation command selects `HEAD`, requires its tracked index and working
tree to be unchanged, reads entries and blobs from the selected Git tree, and
validates the completed ZIP before publishing it. `--commit <commit>` on both
commands selects another exact commit. The ZIP has forward-slash relative
paths, rejects duplicate and unsafe paths, stores regular files as `100644` or
`100755`, stores symbolic links as `120777` with their target bytes, and uses
normalized timestamps and stored compression. Entry ordering, content, modes,
link targets, and ZIP metadata are byte-for-byte deterministic for the same
selected commit and packaging implementation.

The validator compares the complete ZIP entry set, file types, modes, regular
file content, and symbolic-link targets with the Git tree. Because inclusion
comes only from Git tree entries, `.git` metadata, untracked files, local
databases, logs, runtime data, build and scratch output, and previously
generated untracked archives are not source-bundle inputs. Ordinary CI and
tagged release publication run the same creation command; release-integrity
tests verify that workflow routing.

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

After Rust implementation edits, use `validate focused` with the explicit
series base. It plans formatting, changed-package lint and tests, routed direct
checks, and hygiene. Use `validate final` once after the complete series; it
adds workspace lint, the complete repository checks, and the exact aggregate.
The durable summary is the owner for the commands actually run and any skipped
work.

## Generated Reference And Contract Drift Checks

Generated or source-derived reference surfaces use stable check commands:

- `cargo run -p xtask -- docs-sync` deterministically replaces the marked CLI
  syntax regions, schema-generated request and response structural regions in
  English and Korean API method owners, the canonical shared response
  structures in API Schema Core, and the bilingual package-responsibility and
  dependency-direction regions in Architecture. Run it after changing command,
  request, response, result, or shared response descriptors or workspace
  architecture metadata and review the generated diff. Run it a second time
  and require an empty update set.
- `cargo run -p xtask -- docs-check` checks maintained documentation structure,
  exact resolved request and response region bindings and schema drift,
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
- `cargo test -p volicord-user-action-presentation --test cli_output_contracts`
  checks the compact CLI inbox output descriptor artifact against its typed
  presentation owner. Regenerate an intentional change with
  `VOLICORD_UPDATE_CLI_OUTPUT_CONTRACTS=1 cargo test -p volicord-user-action-presentation --test cli_output_contracts`
  and review the generated fixture.
- To regenerate those public contract snapshots after an intentional source
  change, run
  `VOLICORD_UPDATE_CONTRACT_SNAPSHOTS=1 cargo test -p volicord-integration-tests --test public_contract_snapshots`
  and review the generated files under `tests/integration/snapshots/`.

The public contract snapshot and diagnostic registry files are generated test
artifacts marked with `_generated`. Do not edit them by hand; update the typed
owner first, then regenerate. CLI public command drift remains covered by
executable documentation examples and CLI help/output tests such as the
`volicord-cli` `binary_admin` and `mcp_transport` test targets. The separate
typed UserAction inbox JSON schema is owned and tested by
`volicord-user-action-presentation`; CLI tests additionally deserialize actual
`--json` output through that model.

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
changed files, run IDs, summary paths, and separate passed, failed, decomposed,
and skipped lists. Include skipped reasons and remaining documentation risks.
Do not state that validation passed when any exact aggregate attempt failed or
when the exact aggregate was not run.

Use `PASS`, `WARN`, `FAIL`, or `SKIP` only as documentation-maintenance or
implementation-check outcomes. Do not describe a passing validation step as
Volicord runtime conformance, product acceptance, QA completion, close readiness,
a security guarantee, or residual-risk acceptance.
