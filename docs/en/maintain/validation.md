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

## Structural Checks

For documentation metadata, route, link, and terminology-path changes, run
`cargo run -p xtask -- docs-check` from the repository root. The command is
read-only and verifies the machine-checkable shape:

- `docs/doc-index.yaml` parses as YAML and has `version: 3`.
- Required top-level sections are present and unsupported top-level fields are
  rejected.
- The `owner_areas` catalog and `applicability` catalog use stable identifiers
  with string descriptions.
- Every shared entry uses only `doc_id`, `path`, `kind`, `summary`,
  `normative_level`, `owner_area`, `created_on`, `last_updated_on`,
  `last_verified_on`, `applies_to`, `primary_audience`, `journeys`,
  `canonical_for`, and `depends_on`.
- Every paired entry uses only `doc_id`, `path_en`, `path_ko`, `kind`,
  `summary`, `normative_level`, `translation_policy`, `owner_area`,
  `created_on`, `last_updated_on`, `last_verified_on`, `applies_to`,
  `primary_audience`, `journeys`, `canonical_for`, and `depends_on`.
- Required fields are present for each shared or paired entry.
- `owner_area` resolves to the top-level owner-area catalog.
- `applies_to` is a non-empty duplicate-free list and every value resolves to
  the top-level applicability catalog.
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
- Executable `volicord` command examples in shell fences use supported public
  CLI command shapes and options.
- Terminology role metadata for identity-sensitive terms uses the allowed role
  set and includes the required roles for public selectors, storage internals,
  MCP process bindings, and diagnostics.
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

For brand-presentation or broad-claim changes, check the [Brand Guidelines](brand-guidelines.md)
for Volicord spelling, official bilingual brand copy, component presentation,
test harness term boundaries, visual principles, and claim restrictions. Confirm
exact product behavior, API behavior, storage effects, schemas, security
guarantees, and Core authority semantics still route to their Reference owners.

For API and Reference examples, check method-local consistency, request and
response shape, field names, required fields, nullability, enum-like values,
`state_version`, refs, artifact refs, run refs, judgment refs, close-readiness
blockers, response branches, and links to applicable owners where relevant.

For developer-learning changes caused by code movement, confirm the relevant
Development documents describe durable crates, modules, entry points, execution
stages, and responsibility boundaries without turning implementation detail into
product contract text.

The automated `docs-check` command includes local documentation-link parity for
maintained English/Korean pairs, but it does not perform semantic bilingual
review, contract-owner review, technical-accuracy review, translation judgment,
API example consistency review, or product meaning review. A passing local-link
parity check only confirms the machine-comparable local reader routes. The
remaining checks stay manual and owner-routed.

## Durable Tests And One-Time Audits

When a documentation or implementation change suggests a new automated check,
decide whether it is a durable contract test or a one-time audit. A durable test
belongs in the repository when it asserts the current allowed contract or
maintained validation rule. A one-time audit belongs in the change process when
it only proves that cleanup-specific text, flags, fields, or examples were
removed.

For implementation-layer placement and test-authoring examples, use
[Testing Strategy](../development/testing-strategy.md). This validation policy
owns the maintenance-check, review, and reporting boundaries for those checks.

Do not add permanent tests whose only assertion is a cleanup-specific string
search such as "the old option name no longer appears." Run those searches as
audits when useful, then report them outside repository files. If the absence
matters as a durable contract, test the positive current shape instead:

- CLI help exposes only the current public option allowlist for the command.
- Maintained shell examples use supported `volicord` commands and options.
- Storage schema checks assert current tables, columns, indexes, constraints,
  migrations, and validation behavior.
- MCP preflight and transport/schema checks assert current startup behavior,
  public tool exposure, and public schema projection. Public MCP schemas must
  keep hiding internal envelope and invocation fields as a stable abstraction
  contract.
- Terminology validation checks identity-sensitive role metadata instead of
  adding broad prose forbidden-word searches for identifiers such as
  `connection_id` or `project_id`.

Name durable tests after the current contract, for example
`connect_help_exposes_only_public_connect_options`,
`documented_volicord_commands_match_public_cli_contract`,
`export_mcp_config_uses_default_file_when_output_is_omitted`,
`mcp_public_schema_hides_internal_envelope_fields`,
`terminology_map_defines_identity_sensitive_roles`, or
`storage_registry_contains_current_contract_columns`. Avoid names and structures
such as `removed_options_are_gone`, `legacy_flags_are_removed`,
`old_strings_do_not_remain`, and `cleanup_removed_project_id`.

## Onboarding Usability Validation

Use representative-user usability validation when maintained onboarding,
installation, agent-host setup, troubleshooting, or owner-routing documentation
is added or materially changed. This is human usability testing with actual
participants. It is separate from automated `docs-check`, Rust implementation
tests, conformance checks, human semantic review, and an agent-performed desk
review. An agent desk review may find documentation-maintenance blockers, but it
is not evidence that first-time human readers can complete the flow.

The participant set must include at least:

- two technically capable users with no prior Volicord experience
- one MCP host operator with no prior Volicord experience
- one implementer who needs to navigate API or schema Reference material

The tasks must cover whether participants can:

1. Determine whether their environment is documented as suitable.
2. Build or select the executables.
3. Verify executable readiness.
4. Choose and follow one Codex or Claude Code setup path.
5. Interpret `action_required` and identify the required next action.
6. Recover from an unavailable or incorrectly selected executable.
7. Interpret a state with no allowed project or ambiguous project selection.
8. Explain what remains after safe removal.
9. Find the detailed schema owner for `StateRecordRef` or `EvidenceSummary`.

Record observations needed to improve the maintained documentation, including
where participants stop, questions they ask without prompting, incorrect state
interpretations, unsafe write or deletion attempts, whether success was
self-verified, whether recovery completed, the number and type of document
transitions, and search terms that failed.

Passing usability validation requires first-time users to complete executable
preparation and one host path without author explanation, identify documented
success independently, avoid treating `action_required` as an unexplained fatal
failure, recover without deleting unrelated user configuration or product data,
and find the detailed schema owner without author assistance. Critical blockers
include any issue that prevents task completion, causes an unsafe write or
deletion attempt, produces a wrong success interpretation, or breaks an owner
route. Correct critical blockers in the applicable maintained owner documents,
keep paired English and Korean meaning aligned when a paired document changes,
rerun matching automated and manual maintenance checks, and retest the affected
task with the relevant participant profile before treating the blocker as
resolved.

Report usability validation results in the conversation or another
repository-approved durable research location, not as individual test records in
maintained documentation. Do not commit participant notes, screenshots,
recordings, session logs, work logs, fabricated completion rates, fabricated
quotations, or private participant data to maintained docs. Do not claim
representative-user testing occurred unless actual representative participants
performed the tasks and their participation is verifiable. Automated validation
proves only the machine-checkable properties it owns, Rust tests prove only
implementation checks, and an agent desk review proves only that a maintainer
reviewed the documents for objective blockers.

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

## Storage DDL Contract Check

When editing Storage DDL, `volicord-store` migrations, or schema validation code,
run the focused owner-to-implementation consistency check:

```sh
cargo test -p volicord-store --test storage_ddl_contract
```

This check compares the authoritative English and Korean Storage DDL SQL with
the latest schemas produced by executable migrations in in-memory SQLite
databases. It checks schema semantics such as tables, columns, defaults,
constraints, foreign keys, indexes, partial indexes, and maintained triggers
without comparing Markdown prose or SQL formatting.

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
