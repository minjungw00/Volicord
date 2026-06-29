# Testing strategy

This guide explains which implementation test layer to use for common Volicord
Rust changes. Tests verify owner-defined facts; they do not define product
contracts, prove security, complete QA, establish close readiness, or record
product acceptance.

For exact behavior, use the [Reference Index](../reference/README.md). For
crate-by-crate source orientation, use the [Codebase Tour](codebase-tour.md).
For workspace structure and the Cargo dependency graph, use
[Implementation Architecture](architecture.md). For change workflow, use the
[Implementation Guide](change-guide.md). For documentation command-example
validation, terminology role validation, bilingual link parity, and validation
reporting boundaries, use the [Validation](../maintain/validation.md) policy.

## Test Layers

| Layer | Actual package or path | Use it for | Avoid using it as |
|---|---|---|---|
| Module unit tests | Colocated tests in implementation modules such as [`crates/volicord-types/src/lib.rs`](../../../crates/volicord-types/src/lib.rs), [`crates/volicord-core/src/pipeline.rs`](../../../crates/volicord-core/src/pipeline.rs), [`crates/volicord-store/src/core_pipeline.rs`](../../../crates/volicord-store/src/core_pipeline.rs), [`crates/volicord-store/src/sqlite.rs`](../../../crates/volicord-store/src/sqlite.rs), and CLI or MCP modules. | Local helper behavior, typed parsing, canonical hashing, policy helpers, Store transaction edges, schema validation, and small branch checks close to the code. | A cross-layer acceptance test or a product contract source. |
| Core method tests | [`crates/volicord-core/src/methods/tests.rs`](../../../crates/volicord-core/src/methods/tests.rs) in package `volicord-core`. | Method planning, shared preflight through `CoreService`, dry-run/no-effect/commit branches, replay, state-version effects, artifact staging distinction, and method-visible Store effects. | MCP transport coverage or full public behavior authority. |
| Storage DDL contract test | [`crates/volicord-store/tests/storage_ddl_contract.rs`](../../../crates/volicord-store/tests/storage_ddl_contract.rs), target `storage_ddl_contract`, package `volicord-store`. | Owner-to-implementation consistency for Storage DDL, executable migrations, schema validation, tables, columns, constraints, indexes, and maintained triggers. | General storage-effect behavior or runtime conformance. |
| Binary tests for administrative CLI | [`crates/volicord-cli/tests/binary_admin.rs`](../../../crates/volicord-cli/tests/binary_admin.rs), target `binary_admin`, package `volicord-cli`. | The `volicord` binary, Runtime Home setup commands, project detection, `volicord connect`, `volicord connections`, `volicord connection status/verify/mode/remove`, `volicord export mcp-config`, `volicord user ...`, zero-write dry runs, host-state verification, connected-project membership lifecycle, residual-effect reporting, host config writes, preflight failure handling, and command-line error paths. | Public API method behavior. |
| Binary tests for MCP transport | [`crates/volicord-mcp/tests/binary_transport.rs`](../../../crates/volicord-mcp/tests/binary_transport.rs), target `binary_transport`, package `volicord-mcp`. | The `volicord-mcp` binary, help/version, `--check`, stdio framing, JSON-RPC behavior, reconnect cases, and response wrapping. | Core method semantics. |
| MCP integration tests | [`tests/integration/mcp_connection.rs`](../../../tests/integration/mcp_connection.rs), target `mcp_connection`, package `volicord-integration-tests`. | Cross-layer MCP, Core, Store, connection binding, `operation_category` derivation, tool exposure, replay-context binding, and storage no-effect checks visible through MCP. | A replacement for focused method tests or Reference owners. |
| Conformance implementation tests | [`tests/conformance/baseline.rs`](../../../tests/conformance/baseline.rs), target `baseline`, package `volicord-conformance-tests`. | Baseline cross-method scenarios through Core-facing APIs, including replay, Write Check, artifacts, judgments, close readiness, error routing, and corruption handling. | Product acceptance, security proof, close readiness, or the sole source of a product rule. |
| Shared test support | [`crates/volicord-test-support/src/lib.rs`](../../../crates/volicord-test-support/src/lib.rs), package `volicord-test-support`. | Disposable Runtime Home fixtures, registered project and Agent Connection setup, request builders, Store inspection helpers, and shared fixture composition. | Production behavior or a durable runtime home. |
| Documentation maintenance tooling tests | [`xtask/tests/docs_check.rs`](../../../xtask/tests/docs_check.rs), package `xtask`. | The read-only documentation validator, metadata parsing, bilingual coverage, local link and anchor checks, terminology path checks, retired-path detection, and temporary fixture behavior. | Semantic translation review, technical-accuracy review, or a product contract source. |

## Validation Map By Change Area

Use this map after the Codebase Tour or Architecture page has identified the
affected crates or documents. It names likely checks; it is not a rule that
every small edit runs every listed test.

| Change area | Likely code or doc area | Start with | Add when |
|---|---|---|---|
| Developer documentation, documentation routes, metadata, links, or terminology | `docs/en/`, `docs/ko/`, `docs/doc-index.yaml`, `docs/terminology-map.yaml`; `xtask` when validator behavior changes. | `cargo run -p xtask -- docs-check`, plus manual semantic parity, owner-routing, and terminology review. | `xtask` tests when adding or changing deterministic docs-check rules. |
| Public schemas, shared request/result types, value sets, identifiers, or request hashing | `crates/volicord-types/src/` and the applicable Reference owners. | `volicord-types` unit tests. | Core method tests when method planning changes; MCP integration when tool schemas or exposure change; docs-check when maintained docs change. |
| Public method behavior, Core pipeline behavior, policy helpers, replay, or effect branches | `crates/volicord-core/src/pipeline.rs`, `crates/volicord-core/src/methods/`, and `crates/volicord-core/src/policy/`. | Core colocated unit tests and `crates/volicord-core/src/methods/tests.rs`. | `tests/conformance/baseline.rs` for cross-method baseline scenarios; `tests/integration/mcp_connection.rs` for adapter-visible context or tool exposure. |
| Store DDL, migrations, persistence helpers, transaction boundaries, storage effects, or artifact storage | `crates/volicord-store/src/`, [`crates/volicord-store/tests/storage_ddl_contract.rs`](../../../crates/volicord-store/tests/storage_ddl_contract.rs), and storage Reference owners. | Store colocated unit tests; `cargo test -p volicord-store --test storage_ddl_contract` for Storage DDL, migrations, or schema validation changes. | Core method, conformance, or MCP integration tests when public-method-visible storage behavior changes. |
| MCP startup, stdio transport, tool listing, `tools/call`, project selection, or Agent Connection invocation context | `crates/volicord-mcp/src/`, `crates/volicord-mcp/tests/binary_transport.rs`, and `tests/integration/mcp_connection.rs`. | `volicord-mcp` unit tests and `binary_transport`. | `mcp_connection` when Core/Store behavior must be observed through MCP; docs-check when MCP docs change. |
| Administrative CLI, host setup, managed host configuration, registration, or `volicord user` commands | `crates/volicord-cli/src/` and `crates/volicord-cli/tests/binary_admin.rs`. | CLI colocated unit tests and `binary_admin`. | Store tests when bootstrap, registry, inspection, migration, Agent Connection, or project membership behavior changes; docs-check when CLI docs change. |
| Conformance scenario or shared fixture behavior | `tests/conformance/baseline.rs` and `crates/volicord-test-support/src/lib.rs`. | The focused crate/unit tests for the behavior first, then the affected conformance scenario. | Consuming integration or method tests when fixture behavior changes what another layer observes. |

## Choosing A Layer

| Change category | Start with | Add when |
|---|---|---|
| Shared request, response, value, identifier, or canonical-hash type | `volicord-types` unit tests. | Add Core method or integration tests when the shape changes method planning or adapter exposure. |
| Store read helper, mutation application, transaction, migration, or artifact storage behavior | Store module tests near the changed code. | Add Core method tests when a public method effect changes; add conformance or MCP integration when cross-layer behavior is affected. |
| Storage DDL reference, executable migration, or schema validation behavior | `cargo test -p volicord-store --test storage_ddl_contract` plus nearby Store tests. | Add docs-check when maintained Storage DDL documentation changes; add Core, conformance, or MCP integration tests when public-method-visible storage effects change. |
| Core method behavior | `crates/volicord-core/src/methods/tests.rs`. | Add `tests/conformance/baseline.rs` for cross-method baseline scenarios and `tests/integration/mcp_connection.rs` when MCP exposure or `operation_category` derivation matters. |
| Common Core preflight, branch routing, replay, freshness, or access policy | `crates/volicord-core/src/pipeline.rs` unit tests and method tests. | Add MCP integration when adapter-derived invocation context or session binding is involved. |
| MCP adapter startup, tool schema, `tools/call`, or stdio transport | `crates/volicord-mcp/src/lib.rs` tests and `binary_transport`. | Add `tests/integration/mcp_connection.rs` for cross-layer Core/Store behavior through MCP. |
| Administrative agent setup behavior | `binary_admin` and colocated CLI module tests for `connection_command.rs`, host adapters, managed host configuration, and registration helpers. | Add Store tests when bootstrap, inspection, registry, migration, Agent Connection, project membership, or managed host configuration state inventory behavior changes. |
| Test fixture behavior | `volicord-test-support` tests or the consuming package's tests. | Add owner-focused documentation checks if the fixture exposes a missing contract owner. |
| Documentation validator behavior | `xtask` tests and `cargo run -p xtask -- docs-check`. | Add fixture cases when a new deterministic structural rule is introduced. |
| Developer documentation only | `cargo run -p xtask -- docs-check` plus manual semantic parity, owner-routing, and terminology review. | Run Cargo tests only when requested or when the documentation change depends on source behavior that needs fresh validation. |

## Durable Contract Tests And One-Time Audits

Durable repository tests should verify the current public contract, storage
contract, schema contract, or maintained documentation rule. A one-time audit
checks whether a cleanup was completed. Keep those separate.

A repository test is durable when it would still be useful after the cleanup or
rename that prompted it. Prefer positive assertions against the current allowed
shape: current command options, documented command examples, current storage
tables and columns, current MCP-visible schemas, and terminology roles defined
by `docs/terminology-map.yaml`. String searches for removed artifacts are audit
procedures. Use them during the change when helpful, report the result, and do
not turn them into persistent tests whose only value is proving that an old
string disappeared.

For CLI help, assert the current option allowlist exposed by each command
rather than checking for removed flags by name. A help test such as
`connect_help_exposes_only_public_connect_options` should compare the parsed
help options for `volicord connect` with the supported option set. Documentation
command-example validation should check executable `volicord` examples against
the public CLI command contract, as in
`documented_volicord_commands_match_public_cli_contract`.

For storage, MCP, and terminology checks, assert the stable abstraction that
current contributors must preserve. Storage schema tests should name the
current records, columns, indexes, and constraints they expect, as in
`storage_registry_contains_current_contract_columns`. MCP preflight and public
schema tests should check current startup and schema behavior; MCP-visible
schema projection should remain an abstraction contract that hides internal
envelope fields, as in `mcp_public_schema_hides_internal_envelope_fields`.
Terminology checks should validate identity-sensitive role metadata such as
storage internals, MCP process bindings, diagnostics, and public selectors, as
in `terminology_map_defines_identity_sensitive_roles`; they should not become
prose-wide bans on identifiers such as `connection_id` or `project_id`.

Name tests after the current product contract they protect. Preferred examples
include:

- `connect_help_exposes_only_public_connect_options`
- `documented_volicord_commands_match_public_cli_contract`
- `export_mcp_config_uses_default_file_when_output_is_omitted`
- `mcp_public_schema_hides_internal_envelope_fields`
- `terminology_map_defines_identity_sensitive_roles`
- `storage_registry_contains_current_contract_columns`

Avoid test names or structures that describe cleanup history instead of the
current contract:

- `removed_options_are_gone`
- `legacy_flags_are_removed`
- `old_strings_do_not_remain`
- `cleanup_removed_project_id`

## Tests That Demonstrate Boundaries

Some tests are especially useful for understanding architecture boundaries:

- `mcp_exposes_exactly_the_documented_public_methods` and
  `stdio_tools_list_exposes_exactly_the_public_method_set` show MCP exposure of
  the public method set.
- `adapter_and_direct_core_status_have_equivalent_response_meaning` and
  `mcp_and_direct_status_omit_same_excluded_projection_fields` compare adapter
  and direct Core behavior.
- `rejected_branch_has_no_storage_effect`, `dry_run_branch_has_no_storage_effect`,
  and `read_only_branch_has_no_storage_effect` protect no-commit branches.
- `committed_mutation_increments_state_version_once` and Store transaction
  replay tests protect the atomic commit boundary.
- `stage_artifact_creates_transient_handle_without_core_commit` protects the
  staging path from being confused with normal Core mutation commit.
- `no_effect_branches_state_version_and_idempotency_are_stable` demonstrates
  cross-method no-effect and replay stability through Core-facing APIs.

These tests are implementation checks. They are not Volicord runtime
conformance claims, product acceptance records, QA completion, security proof,
close-readiness results, or residual-risk acceptance.

## Validation Defaults

For Rust implementation edits, the repository default is:

```sh
cargo fmt
cargo clippy --all-targets --all-features
cargo test --all-targets --all-features
```

For documentation-only edits, use the applicable documentation checks. When a
documentation task asks for source verification, `cargo metadata --no-deps
--format-version 1`, repository search, and the requested test command are
appropriate implementation checks.

For maintained documentation structural checks, run:

```sh
cargo run -p xtask -- docs-check
```

Then complete the manual semantic bilingual review, contract-owner review, and
technical-accuracy review that match the changed documents.
