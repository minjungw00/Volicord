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

## Test layers

| Layer | Actual package or path | Use it for | Avoid using it as |
|---|---|---|---|
| Module unit tests | Colocated tests in implementation modules such as [`crates/volicord-types/src/lib.rs`](../../../crates/volicord-types/src/lib.rs), [`crates/volicord-core/src/pipeline.rs`](../../../crates/volicord-core/src/pipeline.rs), [`crates/volicord-store/src/core_pipeline.rs`](../../../crates/volicord-store/src/core_pipeline.rs), [`crates/volicord-store/src/sqlite.rs`](../../../crates/volicord-store/src/sqlite.rs), setup, connection, guard, host integration, and MCP modules. | Local helper behavior, typed parsing, canonical hashing, policy helpers, Store transaction edges, schema validation, setup workflow branches, connection selection/output helpers, guard integration planning/audit helpers, and small branch checks close to the code. | A cross-layer acceptance test or a product contract source. |
| Platform filesystem facade tests | Colocated tests in [`crates/volicord-platform-fs/src/lib.rs`](../../../crates/volicord-platform-fs/src/lib.rs), package `volicord-platform-fs`. | Safe result classification around platform-native namespace operations and target-specific facade behavior. | Managed-file ownership policy, caller recovery behavior, a filesystem-wide portability claim, or a security proof. |
| Core method tests | [`crates/volicord-core/src/methods/tests/`](../../../crates/volicord-core/src/methods/tests/) in package `volicord-core`, split into method-focused files such as `status.rs`, `intake.rs`, and `prepare_write.rs`. | Method planning, shared preflight through `CoreService`, dry-run/no-effect/commit branches, replay, state-version effects, artifact staging distinction, and method-visible Store effects. | MCP transport coverage or full public behavior authority. |
| Storage DDL contract test | [`crates/volicord-store/tests/storage_ddl_contract.rs`](../../../crates/volicord-store/tests/storage_ddl_contract.rs), target `storage_ddl_contract`, package `volicord-store`. | Owner-to-implementation consistency for Storage DDL, canonical SQL sources, schema initialization, schema validation, tables, columns, constraints, indexes, and maintained triggers. | General storage-effect behavior or runtime conformance. |
| Binary tests for administrative CLI | [`crates/volicord-cli/tests/binary_admin.rs`](../../../crates/volicord-cli/tests/binary_admin.rs), target `binary_admin`, package `volicord-cli`. | The `volicord` binary, Runtime Home setup through `volicord init`, setup workflow effects that must be observed through the binary, project detection, `volicord status`, `volicord connection add`, `volicord connection list`, `volicord connection status/verify/mode/remove`, connection output, `volicord inbox ...`, zero-write dry runs, host-state verification, connected-project membership lifecycle, generated guard output drift through guarded init/status cases, residual-effect reporting, host config writes, preflight failure handling, doctor diagnostics, and command-line error paths. | Public API method behavior. |
| Guard command tests | [`crates/volicord-cli/tests/guard_command.rs`](../../../crates/volicord-cli/tests/guard_command.rs), target `guard_command`, package `volicord-cli`. | Guard hook lifecycle behavior for session start, pre-tool, post-tool, prompt capture, and stop; recorded observations; expected-write matching; write-ticket coverage; host-native rendering; prompt-capture command handling; and guarded lifecycle fixtures. | A security proof, human approval record, product acceptance record, or replacement for Core method tests. |
| Binary tests for MCP transport | [`crates/volicord-cli/tests/mcp_transport.rs`](../../../crates/volicord-cli/tests/mcp_transport.rs), target `mcp_transport`, package `volicord-cli`. | The `volicord mcp` subcommand, help/version, `--check`, stdio framing, JSON-RPC behavior, reconnect cases, and response wrapping. | Core method semantics. |
| Local HTTP serve transport tests | [`crates/volicord-cli/tests/serve_transport.rs`](../../../crates/volicord-cli/tests/serve_transport.rs), target `serve_transport`, package `volicord-cli`. | The `volicord serve --transport local-http` process path, loopback listener startup, token and origin checks, HTTP session behavior, defensive headers, and MCP request routing through the local HTTP transport. | A general MCP method test or security proof. |
| Opt-in live host smoke tests | [`crates/volicord-cli/tests/live_host_smoke.rs`](../../../crates/volicord-cli/tests/live_host_smoke.rs), target `live_host_smoke`, package `volicord-cli`. | Explicit checks against an installed Codex or Claude Code executable in an environment prepared for that host. Configuration checks use `VOLICORD_RUN_*_SMOKE=1`; interactive Judgment round trips use `VOLICORD_RUN_*_JUDGMENT_SMOKE=1`. Every live check is ignored by default. | A default workspace-test signal, portable host conformance, host trust, credential availability, network availability, or a security proof. |
| MCP integration tests | [`tests/integration/mcp_connection.rs`](../../../tests/integration/mcp_connection.rs), target `mcp_connection`, package `volicord-integration-tests`. | Cross-layer MCP, Core, Store, connection binding, `operation_category` derivation, tool exposure, replay-context binding, and storage no-effect checks visible through MCP. | A replacement for focused method tests or Reference owners. |
| Public contract snapshot tests | [`tests/integration/public_contract_snapshots.rs`](../../../tests/integration/public_contract_snapshots.rs), target `public_contract_snapshots`, package `volicord-integration-tests`. | Generated API request-schema and MCP tool-contract snapshot drift against the current source projection. | Hand-edited generated snapshots, semantic Reference review, or proof that the public contract is correct. |
| Conformance implementation tests | [`tests/conformance/baseline.rs`](../../../tests/conformance/baseline.rs), target `baseline`, package `volicord-conformance-tests`. | Baseline cross-method scenarios through Core-facing APIs, including replay, write tickets, artifacts, judgments, close readiness, error routing, and corruption handling. | Product acceptance, security proof, close readiness, or the sole source of a product rule. |
| Shared test support | [`crates/volicord-test-support/src/lib.rs`](../../../crates/volicord-test-support/src/lib.rs), package `volicord-test-support`. | Disposable Runtime Home fixtures, registered project and Agent Connection setup, request builders, Store inspection helpers, and shared fixture composition. | Production behavior or a durable runtime home. |
| CLI integration support | [`crates/volicord-cli/tests/support/`](../../../crates/volicord-cli/tests/support/). | Binary fixtures, fake hosts, fake MCP processes, guard lifecycle fixtures, JSON helpers, and assertion helpers reused by `binary_admin`, `guard_command`, `mcp_transport`, and `serve_transport`. | A source of product contracts or durable runtime state. |
| Documentation maintenance tooling tests | [`xtask/tests/docs_check.rs`](../../../xtask/tests/docs_check.rs), package `xtask`. | The read-only documentation validator, metadata parsing, bilingual coverage, local link and anchor checks, terminology path and role checks, command-example validation, public-language checks, and temporary fixture behavior. | Semantic translation review, technical-accuracy review, or a product contract source. |

## Fixture and support structure

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

## Opt-in live host smoke tests

`live_host_smoke` is a normal Cargo test target whose four live checks carry
`#[ignore]`, so an ordinary workspace test run reports those checks as ignored.
Run one host check only in an environment where that host executable is
installed and the matching opt-in variable is set:

```sh
VOLICORD_RUN_CODEX_SMOKE=1 cargo test -p volicord-cli --test live_host_smoke codex_live_smoke_is_opt_in -- --ignored --nocapture
VOLICORD_RUN_CLAUDE_SMOKE=1 cargo test -p volicord-cli --test live_host_smoke claude_code_live_smoke_is_opt_in -- --ignored --nocapture
VOLICORD_RUN_CODEX_JUDGMENT_SMOKE=1 cargo test -p volicord-cli --test live_host_smoke codex_live_judgment_round_trip_is_opt_in -- --ignored --nocapture
VOLICORD_RUN_CLAUDE_JUDGMENT_SMOKE=1 cargo test -p volicord-cli --test live_host_smoke claude_code_live_judgment_round_trip_is_opt_in -- --ignored --nocapture
```

The Judgment variants are human-in-the-loop checks. They create a disposable
Runtime Home and Product Repository, configure the selected host, then launch
the installed host interactively with an initial no-write instruction. They
reuse the runner's normal host authentication environment; the fixture does
not copy credentials into its isolated Runtime Home. The operator must approve
the project/MCP entry when the host requires it, choose the answer in the
host-native MCP elicitation UI, and exit the host after status is reported.

A passing Judgment variant verifies marker Task and Judgment creation,
host-native prompt/response recording with
`mcp_elicitation_user_channel`, the resulting Task-state transition, authority
events, and the matching content-free session diagnostic. If native
elicitation is unavailable, the harness verifies that the pending Judgment is
visible through `volicord inbox`, prints an exact `volicord inbox answer`
command, and fails the native-round-trip check rather than treating fallback as
native success. The operator may use that command for recovery, but doing so
does not turn the failed live-native check into a passing result.

Before publishing a release that claims the maintained Codex or Claude Code
Judgment path, the manual release-validation checklist must run the matching
Judgment variant against the release candidate and retain the host version,
Volicord `build_id`, and pass/fail result. An unavailable host, authentication
environment, or native elicitation surface is a reported skipped validation,
not a passing round trip.

An explicitly selected check fails when its opt-in variable or host executable
is unavailable. Passing confirms only the assertions that the installed host
and local test environment allowed the smoke test to observe. It does not
prove portable host behavior, host trust, approval, credential availability,
network availability, security enforcement, or general product correctness.

## Generated output and documentation validation

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

## Validation map by change area

Use this map after the Codebase Tour or Architecture page has identified the
affected crates or documents. It names likely checks; it is not a rule that
every small edit runs every listed test.

| Change area | Likely code or doc area | Start with | Add when |
|---|---|---|---|
| Architecture Guide, documentation routes, metadata, links, or terminology | `docs/en/`, `docs/ko/`, `docs/doc-index.yaml`, `docs/terminology-map.yaml`; `xtask` when validator behavior changes. | `cargo run -p xtask -- docs-check`, plus manual semantic parity, owner-routing, and terminology review. | `xtask` tests when adding or changing deterministic docs-check rules. |
| Public schemas, shared request/result types, value sets, identifiers, or request hashing | `crates/volicord-types/src/` and the applicable Reference owners. | `volicord-types` unit tests. | Core method tests when method planning changes; `public_contract_snapshots` or MCP integration when tool schemas or exposure change; docs-check when maintained docs change. |
| Platform filesystem facade or adapter-managed conditional file replacement | `crates/volicord-platform-fs/src/lib.rs`, the calling adapter such as `crates/volicord-cli/src/guard_integration/files.rs`, and the applicable CLI/runtime/system owners. | `volicord-platform-fs` and caller-module unit tests. | Target-specific compile or test coverage when native code changes; `binary_admin` when the administrative result is binary-visible; docs-check when maintained owners or Architecture Guide pages change. |
| Public method behavior, Core pipeline behavior, policy helpers, replay, or effect branches | `crates/volicord-core/src/pipeline.rs`, `crates/volicord-core/src/methods/`, and `crates/volicord-core/src/policy/`. | Core colocated unit tests and the method-focused files under `crates/volicord-core/src/methods/tests/`. | `tests/conformance/baseline.rs` for cross-method baseline scenarios; `tests/integration/mcp_connection.rs` for adapter-visible context or tool exposure. |
| Store DDL, canonical SQL, persistence helpers, transaction boundaries, storage effects, or artifact storage | `crates/volicord-store/src/`, [`crates/volicord-store/tests/storage_ddl_contract.rs`](../../../crates/volicord-store/tests/storage_ddl_contract.rs), and storage Reference owners. | Store colocated unit tests; `cargo test -p volicord-store --test storage_ddl_contract` for Storage DDL, canonical SQL, or schema validation changes. | Core method, conformance, or MCP integration tests when public-method-visible storage behavior changes. |
| MCP startup, stdio or local HTTP transport, tool listing, `tools/call`, project selection, or Agent Connection invocation context | `crates/volicord-mcp/src/`, `crates/volicord-cli/tests/mcp_transport.rs`, `crates/volicord-cli/tests/serve_transport.rs`, and `tests/integration/mcp_connection.rs`. | `volicord-mcp` unit tests, `mcp_transport`, or `serve_transport` for the transport being changed. | `public_contract_snapshots` when generated API or MCP tool projections change; `mcp_connection` when Core/Store behavior must be observed through MCP; docs-check when MCP docs change. |
| Setup workflow behavior and output | `crates/volicord-cli/src/setup_command.rs`, `setup_command/`, `doctor_command.rs`, and `crates/volicord-cli/tests/binary_admin.rs`. | Setup module tests for workflow branches and rendering; `binary_admin` when setup behavior must be observed through the binary. | Store tests when bootstrap, registry, inspection, schema initialization, or installation-profile persistence changes; docs-check when setup docs change. |
| Connection provisioning, status, verification, and output | `crates/volicord-cli/src/connection_command.rs`, `connection_command/`, `crates/volicord-store/src/bootstrap.rs`, `agent_connections.rs`, and `crates/volicord-cli/tests/binary_admin.rs`. | Connection command module tests and `binary_admin`. | `mcp_transport` when process/preflight behavior changes; `tests/integration/mcp_connection.rs` when MCP/Core/Store behavior must be observed through MCP; docs-check when CLI docs change. |
| Guard integration files, capability records, and audit facts | `crates/volicord-cli/src/guard_integration/`, connection status/output code, `doctor_command.rs`, and guarded init/status tests in `binary_admin`. | Guard integration module tests and `binary_admin` guarded init/status cases for generated guard output drift. | `guard_command` when lifecycle observations consume the generated capability facts; docs-check when guard CLI docs change. |
| Guard hook lifecycle behavior and host-native rendering | `crates/volicord-cli/src/guard_command.rs`, `guard_command/`, and `crates/volicord-cli/tests/guard_command.rs`. | `guard_command` tests plus colocated parsing/rendering tests. | Core method, conformance, or storage tests when the hook path depends on owner-defined Core or Store behavior. |
| Host config adapters | `crates/volicord-cli/src/host_integration/`, especially `host_integration/codex/`, `host_integration/claude_code/`, `config_edit.rs`, `contracts.rs`, `generic.rs`, and `verification.rs`. | Host adapter module tests and `binary_admin`. | `guard_command` when host-native hook output changes; `mcp_transport` when launch or preflight behavior changes. |
| Conformance scenario or shared fixture behavior | `tests/conformance/baseline.rs`, `crates/volicord-test-support/src/lib.rs`, and `crates/volicord-cli/tests/support/` for CLI integration fixtures. | The focused crate/unit tests for the behavior first, then the affected conformance or CLI scenario. | Consuming integration or method tests when fixture behavior changes what another layer observes. |

## Durable tests and one-time audits

Use [Validation](../maintain/validation.md) for the full rule that separates a
durable test from a cleanup-only audit. At the implementation layer, test the
current supported shape rather than the history of removed text or options.

Current examples show the intended style:

- `binary_help_options_match_supported_contracts` checks the current CLI help
  allowlists in `crates/volicord-cli/tests/binary_admin.rs`.
- `initial_schemas_satisfy_connection_storage_contract` checks current schema
  structure in `crates/volicord-store/tests/storage_ddl_contract.rs`.
- `public_mcp_arguments_reject_internal_envelope_and_invocation_fields` checks
  the public MCP schema boundary in `tests/integration/mcp_connection.rs`.
- `reports_required_terminology_role_failure` and
  `accepts_supported_volicord_shell_command_examples` protect current
  documentation validation rules in `xtask/tests/docs_check.rs`.

These tests protect implementation or maintenance boundaries. The focused
Reference owner still defines the product fact being checked.

## Tests that demonstrate boundaries

Some tests are especially useful for understanding architecture boundaries:

- `tool_sets_follow_connection_mode_and_exclude_user_only_recording`,
  `volicord_mcp_subcommand_tools_list_respects_connection_mode_and_schema_boundary`,
  `generated_mcp_workflow_tool_contract_snapshot_matches_sources`, and
  `generated_mcp_read_only_tool_contract_snapshot_matches_sources` cover
  tool-set projection and stdio `tools/list` exposure at their respective
  layers.
- `status_is_read_only_including_dry_run` and
  `status_include_false_omits_optional_sections_without_effect` cover the Core
  status branches; `mcp_status_succeeds_with_readonly_storage` and
  `mcp_status_does_not_advance_state_version` cover the corresponding
  MCP-visible read-only properties without asserting full response equivalence.
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

## Validation defaults

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
