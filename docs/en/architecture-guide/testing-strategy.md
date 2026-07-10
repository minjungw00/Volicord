# Testing strategy

This guide explains which implementation test layer to use for common Volicord
Rust changes. Tests verify owner-defined facts; they do not define product
contracts, prove security, complete QA, establish close readiness, or record
product acceptance.

For exact behavior, use the [Reference Index](../reference/README.md). For
crate-by-crate source orientation, use the [Codebase Tour](codebase-tour.md).
For workspace shape and the dependency-boundary overview, use
[Implementation Architecture](architecture.md). For exact Cargo dependency
edges, read the workspace and crate `Cargo.toml` manifests. For change workflow,
use the [Implementation Guide](change-guide.md). For documentation
command-example validation, terminology role validation, bilingual link parity,
and validation reporting boundaries, use the [Validation](../maintain/validation.md)
policy.

## Test Layers

| Layer | Actual package or path | Use it for | Avoid using it as |
|---|---|---|---|
| Module unit tests | Colocated tests in implementation modules such as [`crates/volicord-types/src/lib.rs`](../../../crates/volicord-types/src/lib.rs), [`crates/volicord-core/src/pipeline.rs`](../../../crates/volicord-core/src/pipeline.rs), [`crates/volicord-store/src/core_pipeline.rs`](../../../crates/volicord-store/src/core_pipeline.rs), [`crates/volicord-store/src/sqlite.rs`](../../../crates/volicord-store/src/sqlite.rs), setup, connection, guard, host integration, and MCP modules. | Local helper behavior, typed parsing, canonical hashing, policy helpers, Store transaction edges, schema validation, setup workflow branches, connection selection/output helpers, guard integration planning/audit helpers, and small branch checks close to the code. | A cross-layer acceptance test or a product contract source. |
| Core method tests | [`crates/volicord-core/src/methods/tests/mod.rs`](../../../crates/volicord-core/src/methods/tests/mod.rs) in package `volicord-core`. | Method planning, shared preflight through `CoreService`, dry-run/no-effect/commit branches, replay, state-version effects, artifact staging distinction, and method-visible Store effects. | MCP transport coverage or full public behavior authority. |
| Storage DDL contract test | [`crates/volicord-store/tests/storage_ddl_contract.rs`](../../../crates/volicord-store/tests/storage_ddl_contract.rs), target `storage_ddl_contract`, package `volicord-store`. | Owner-to-implementation consistency for Storage DDL, canonical SQL sources, schema initialization, schema validation, tables, columns, constraints, indexes, and maintained triggers. | General storage-effect behavior or runtime conformance. |
| Binary tests for administrative CLI | [`crates/volicord-cli/tests/binary_admin.rs`](../../../crates/volicord-cli/tests/binary_admin.rs), target `binary_admin`, package `volicord-cli`. | The `volicord` binary, Runtime Home setup through `volicord init`, setup workflow effects that must be observed through the binary, project detection, `volicord status`, `volicord connection add`, `volicord connection list`, `volicord connection status/verify/mode/remove`, connection output, `volicord inbox ...`, zero-write dry runs, host-state verification, connected-project membership lifecycle, generated guard output drift through guarded init/status cases, residual-effect reporting, host config writes, preflight failure handling, doctor diagnostics, and command-line error paths. | Public API method behavior. |
| Guard command tests | [`crates/volicord-cli/tests/guard_command.rs`](../../../crates/volicord-cli/tests/guard_command.rs), target `guard_command`, package `volicord-cli`. | Guard hook lifecycle behavior for session start, pre-tool, post-tool, prompt capture, and stop; recorded observations; expected-write matching; write-ticket coverage; host-native rendering; prompt-capture command handling; and guarded lifecycle fixtures. | A security proof, human approval record, product acceptance record, or replacement for Core method tests. |
| Binary tests for MCP transport | [`crates/volicord-cli/tests/mcp_transport.rs`](../../../crates/volicord-cli/tests/mcp_transport.rs), target `mcp_transport`, package `volicord-cli`. | The `volicord mcp` subcommand, help/version, `--check`, stdio framing, JSON-RPC behavior, reconnect cases, and response wrapping. | Core method semantics. |
| Local HTTP serve transport tests | [`crates/volicord-cli/tests/serve_transport.rs`](../../../crates/volicord-cli/tests/serve_transport.rs), target `serve_transport`, package `volicord-cli`. | The `volicord serve --transport local-http` process path, loopback listener startup, token and origin checks, HTTP session behavior, defensive headers, and MCP request routing through the local HTTP transport. | A general MCP method test or security proof. |
| Opt-in live host smoke tests | [`crates/volicord-cli/tests/live_host_smoke.rs`](../../../crates/volicord-cli/tests/live_host_smoke.rs), target `live_host_smoke`, package `volicord-cli`. | Explicit checks against an installed Codex or Claude Code executable in an environment prepared for that host. The tests are ignored by default and require the matching `VOLICORD_RUN_*_SMOKE=1` selector. | A default workspace-test signal, portable host conformance, host trust, credential availability, network availability, or a security proof. |
| MCP integration tests | [`tests/integration/mcp_connection.rs`](../../../tests/integration/mcp_connection.rs), target `mcp_connection`, package `volicord-integration-tests`. | Cross-layer MCP, Core, Store, connection binding, `operation_category` derivation, tool exposure, replay-context binding, and storage no-effect checks visible through MCP. | A replacement for focused method tests or Reference owners. |
| Public contract snapshot tests | [`tests/integration/public_contract_snapshots.rs`](../../../tests/integration/public_contract_snapshots.rs), target `public_contract_snapshots`, package `volicord-integration-tests`. | Generated API request-schema and MCP tool-contract snapshot drift against the current source projection. | Hand-edited generated snapshots, semantic Reference review, or proof that the public contract is correct. |
| Conformance implementation tests | [`tests/conformance/baseline.rs`](../../../tests/conformance/baseline.rs), target `baseline`, package `volicord-conformance-tests`. | Baseline cross-method scenarios through Core-facing APIs, including replay, write tickets, artifacts, judgments, close readiness, error routing, and corruption handling. | Product acceptance, security proof, close readiness, or the sole source of a product rule. |
| Shared test support | [`crates/volicord-test-support/src/lib.rs`](../../../crates/volicord-test-support/src/lib.rs), package `volicord-test-support`. | Disposable Runtime Home fixtures, registered project and Agent Connection setup, request builders, Store inspection helpers, and shared fixture composition. | Production behavior or a durable runtime home. |
| CLI integration support | [`crates/volicord-cli/tests/support/`](../../../crates/volicord-cli/tests/support/). | Binary fixtures, fake hosts, fake MCP processes, guard lifecycle fixtures, JSON helpers, and assertion helpers reused by `binary_admin`, `guard_command`, `mcp_transport`, and `serve_transport`. | A source of product contracts or durable runtime state. |
| Documentation maintenance tooling tests | [`xtask/tests/docs_check.rs`](../../../xtask/tests/docs_check.rs), package `xtask`. | The read-only documentation validator, metadata parsing, bilingual coverage, local link and anchor checks, terminology path and role checks, command-example validation, public-language checks, and temporary fixture behavior. | Semantic translation review, technical-accuracy review, or a product contract source. |

## Fixture and Support Structure

Shared fixture structure is part of test strategy. `volicord-test-support`
owns disposable Runtime Home fixtures, registered project and Agent Connection
setup, request builders, Store inspection helpers, controlled-corruption
helpers, and shared assertions used by Core, conformance, and integration
tests. Its fixtures support implementation verification; they are not
production runtime state or contract owners.

`crates/volicord-cli/tests/support/` owns CLI integration helpers for binary
execution, fake hosts, fake MCP processes, guard lifecycle setup, JSON parsing,
and reusable assertions. Host contract fixtures under
`crates/volicord-cli/tests/fixtures/host_contracts/` support host-adapter and
guard lifecycle tests. Assertions made through those helpers still route to the
Reference owner for the fact being checked.

## Opt-in Live Host Smoke Tests

`live_host_smoke` is registered as an ignored test target, so an ordinary
workspace test run reports that the live checks were not executed. Run one
host check only in an environment where that host executable is installed and
the matching opt-in variable is set:

```sh
VOLICORD_RUN_CODEX_SMOKE=1 cargo test -p volicord-cli --test live_host_smoke codex_live_smoke_is_opt_in -- --ignored --nocapture
VOLICORD_RUN_CLAUDE_SMOKE=1 cargo test -p volicord-cli --test live_host_smoke claude_code_live_smoke_is_opt_in -- --ignored --nocapture
```

An explicitly selected check fails when its opt-in variable or host executable
is unavailable. Passing confirms only the assertions that the installed host
and local test environment allowed the smoke test to observe. It does not
prove portable host behavior, host trust, approval, credential availability,
network availability, security enforcement, or general product correctness.

## Generated Output and Documentation Validation

Generated output drift checks verify that generated or projected repository
artifacts still match their current sources. `public_contract_snapshots`
checks generated API request-schema and MCP tool-contract snapshots. CLI
binary and guard tests check generated host and guard outputs where those
outputs are visible through supported test paths. A drift failure means the
source change, owner document, validation expectation, or regeneration step
needs review; it is not a correctness proof.

For maintained documentation, `cargo run -p xtask -- docs-check` is the
repository structural check for `docs/doc-index.yaml`, maintained paths,
links and anchors, bilingual local-link parity, terminology owner paths and
roles, command-example shape, and public-language checks. It is documentation
and public-language validation; it does not replace semantic bilingual review,
technical-accuracy review, Reference-owner review, or product conformance.

## Validation Map By Change Area

Use this map after the Codebase Tour or Architecture page has identified the
affected crates or documents. It names likely checks; it is not a rule that
every small edit runs every listed test.

| Change area | Likely code or doc area | Start with | Add when |
|---|---|---|---|
| Architecture Guide, documentation routes, metadata, links, or terminology | `docs/en/`, `docs/ko/`, `docs/doc-index.yaml`, `docs/terminology-map.yaml`; `xtask` when validator behavior changes. | `cargo run -p xtask -- docs-check`, plus manual semantic parity, owner-routing, and terminology review. | `xtask` tests when adding or changing deterministic docs-check rules. |
| Public schemas, shared request/result types, value sets, identifiers, or request hashing | `crates/volicord-types/src/` and the applicable Reference owners. | `volicord-types` unit tests. | Core method tests when method planning changes; `public_contract_snapshots` or MCP integration when tool schemas or exposure change; docs-check when maintained docs change. |
| Public method behavior, Core pipeline behavior, policy helpers, replay, or effect branches | `crates/volicord-core/src/pipeline.rs`, `crates/volicord-core/src/methods/`, and `crates/volicord-core/src/policy/`. | Core colocated unit tests and `crates/volicord-core/src/methods/tests/mod.rs`. | `tests/conformance/baseline.rs` for cross-method baseline scenarios; `tests/integration/mcp_connection.rs` for adapter-visible context or tool exposure. |
| Store DDL, canonical SQL, persistence helpers, transaction boundaries, storage effects, or artifact storage | `crates/volicord-store/src/`, [`crates/volicord-store/tests/storage_ddl_contract.rs`](../../../crates/volicord-store/tests/storage_ddl_contract.rs), and storage Reference owners. | Store colocated unit tests; `cargo test -p volicord-store --test storage_ddl_contract` for Storage DDL, canonical SQL, or schema validation changes. | Core method, conformance, or MCP integration tests when public-method-visible storage behavior changes. |
| MCP startup, stdio or local HTTP transport, tool listing, `tools/call`, project selection, or Agent Connection invocation context | `crates/volicord-mcp/src/`, `crates/volicord-cli/tests/mcp_transport.rs`, `crates/volicord-cli/tests/serve_transport.rs`, and `tests/integration/mcp_connection.rs`. | `volicord-mcp` unit tests, `mcp_transport`, or `serve_transport` for the transport being changed. | `public_contract_snapshots` when generated API or MCP tool projections change; `mcp_connection` when Core/Store behavior must be observed through MCP; docs-check when MCP docs change. |
| Setup workflow behavior and output | `crates/volicord-cli/src/setup_command.rs`, `setup_command/`, `doctor_command.rs`, and `crates/volicord-cli/tests/binary_admin.rs`. | Setup module tests for workflow branches and rendering; `binary_admin` when setup behavior must be observed through the binary. | Store tests when bootstrap, registry, inspection, schema initialization, or installation-profile persistence changes; docs-check when setup docs change. |
| Connection provisioning, status, verification, and output | `crates/volicord-cli/src/connection_command.rs`, `connection_command/`, `registration.rs`, and `crates/volicord-cli/tests/binary_admin.rs`. | Connection command module tests and `binary_admin`. | `mcp_transport` when process/preflight behavior changes; `tests/integration/mcp_connection.rs` when MCP/Core/Store behavior must be observed through MCP; docs-check when CLI docs change. |
| Guard integration files, capability records, and audit facts | `crates/volicord-cli/src/guard_integration/`, connection status/output code, `doctor_command.rs`, and guarded init/status tests in `binary_admin`. | Guard integration module tests and `binary_admin` guarded init/status cases for generated guard output drift. | `guard_command` when lifecycle observations consume the generated capability facts; docs-check when guard CLI docs change. |
| Guard hook lifecycle behavior and host-native rendering | `crates/volicord-cli/src/guard_command.rs`, `guard_command/`, and `crates/volicord-cli/tests/guard_command.rs`. | `guard_command` tests plus colocated parsing/rendering tests. | Core method, conformance, or storage tests when the hook path depends on owner-defined Core or Store behavior. |
| Host config adapters | `crates/volicord-cli/src/host_integration/`, especially `host_integration/codex/`, `host_integration/claude_code/`, `config_edit.rs`, `contracts.rs`, `generic.rs`, and `verification.rs`. | Host adapter module tests and `binary_admin`. | `guard_command` when host-native hook output changes; `mcp_transport` when launch or preflight behavior changes. |
| Conformance scenario or shared fixture behavior | `tests/conformance/baseline.rs`, `crates/volicord-test-support/src/lib.rs`, and `crates/volicord-cli/tests/support/` for CLI integration fixtures. | The focused crate/unit tests for the behavior first, then the affected conformance or CLI scenario. | Consuming integration or method tests when fixture behavior changes what another layer observes. |

## Choosing A Layer

| Change category | Start with | Add when |
|---|---|---|
| Shared request, response, value, identifier, or canonical-hash type | `volicord-types` unit tests. | Add Core method or integration tests when the shape changes method planning or adapter exposure. |
| Store read helper, mutation application, transaction, schema initialization, or artifact storage behavior | Store module tests near the changed code. | Add Core method tests when a public method effect changes; add conformance or MCP integration when cross-layer behavior is affected. |
| Storage DDL reference, canonical SQL, or schema validation behavior | `cargo test -p volicord-store --test storage_ddl_contract` plus nearby Store tests. | Add docs-check when maintained Storage DDL documentation changes; add Core, conformance, or MCP integration tests when public-method-visible storage effects change. |
| Core method behavior | `crates/volicord-core/src/methods/tests/mod.rs`. | Add `tests/conformance/baseline.rs` for cross-method baseline scenarios and `tests/integration/mcp_connection.rs` when MCP exposure or `operation_category` derivation matters. |
| Common Core preflight, branch routing, replay, freshness, or access policy | `crates/volicord-core/src/pipeline.rs` unit tests and method tests. | Add MCP integration when adapter-derived invocation context or session binding is involved. |
| MCP adapter startup, tool schema, `tools/call`, stdio transport, or local HTTP transport | `crates/volicord-mcp/src/tests.rs` tests plus `mcp_transport` or `serve_transport` for the affected process path. | Add `public_contract_snapshots` when generated API or MCP tool projections change; add `tests/integration/mcp_connection.rs` for cross-layer Core/Store behavior through MCP. |
| Setup workflow behavior and output | Setup module tests near `setup_command/` and `binary_admin` for binary-visible setup flows. | Add Store tests when bootstrap, inspection, registry, schema initialization, or installation-profile persistence changes. |
| Connection provisioning, status, verification, and output | Connection command module tests and `binary_admin`. | Add `mcp_transport` for preflight/process changes and `tests/integration/mcp_connection.rs` for MCP-visible cross-layer behavior. |
| Guard integration files, capability records, and audit facts | Guard integration module tests and `binary_admin` guarded init/status cases. | Add `guard_command` when recorded observations or hook lifecycle paths consume the generated facts. |
| Guard hook lifecycle behavior and host-native rendering | `guard_command` and colocated guard module tests. | Add Core, Store, conformance, or MCP tests when the hook behavior depends on owner-defined behavior outside the CLI. |
| Host config adapters | Host adapter module tests and `binary_admin`. | Add `guard_command` for host-native hook rendering or `mcp_transport` for launch/preflight behavior. |
| Test fixture behavior | `volicord-test-support` tests, `crates/volicord-cli/tests/support/`, or the consuming package's tests. | Add owner-focused documentation checks if the fixture exposes a missing contract owner. |
| Generated public contract snapshot behavior | `cargo test -p volicord-integration-tests --test public_contract_snapshots`. | Regenerate snapshots only through the recorded update command when the owner-approved source projection changes. |
| Documentation validator behavior | `xtask` tests and `cargo run -p xtask -- docs-check`. | Add fixture cases when a new deterministic structural rule is introduced. |
| Architecture Guide only | `cargo run -p xtask -- docs-check` plus manual semantic parity, owner-routing, and terminology review. | Run Cargo tests only when requested or when the documentation change depends on source behavior that needs fresh validation. |

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
help options for `volicord connection add` with the supported option set. Documentation
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
- `export_help_lists_authority_bundle`
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
