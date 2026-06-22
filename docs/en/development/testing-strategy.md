# Testing strategy

This guide explains which implementation test layer to use for common Harness
Rust changes. Tests verify owner-defined facts; they do not define product
contracts, prove security, complete QA, establish close readiness, or record
product acceptance.

For exact behavior, use the [Reference Index](../reference/README.md). For
workspace structure, use [Implementation Architecture](architecture.md). For
change workflow, use the [Implementation Guide](change-guide.md).

## Test Layers

| Layer | Actual package or path | Use it for | Avoid using it as |
|---|---|---|---|
| Module unit tests | Colocated tests in implementation modules such as [`crates/harness-types/src/lib.rs`](../../../crates/harness-types/src/lib.rs), [`crates/harness-core/src/pipeline.rs`](../../../crates/harness-core/src/pipeline.rs), [`crates/harness-store/src/core_pipeline.rs`](../../../crates/harness-store/src/core_pipeline.rs), [`crates/harness-store/src/sqlite.rs`](../../../crates/harness-store/src/sqlite.rs), and CLI or MCP modules. | Local helper behavior, typed parsing, canonical hashing, policy helpers, Store transaction edges, schema validation, and small branch checks close to the code. | A cross-layer acceptance test or a product contract source. |
| Core method tests | [`crates/harness-core/src/methods/tests.rs`](../../../crates/harness-core/src/methods/tests.rs) in package `harness-core`. | Method planning, shared preflight through `CoreService`, dry-run/no-effect/commit branches, replay, state-version effects, artifact staging distinction, and method-visible Store effects. | MCP transport coverage or full public behavior authority. |
| Binary tests for administrative CLI | [`crates/harness-cli/tests/binary_admin.rs`](../../../crates/harness-cli/tests/binary_admin.rs), target `binary_admin`, package `harness-cli`. | The `harness` binary, Runtime Home setup commands, `harness agent` install/status/verify/uninstall/guidance behavior, local MCP compatibility setup, zero-write dry runs, host-state verification, project default and final-membership lifecycle, compensation and residual-effect reporting, host config writes, repository guidance safety, preflight failure handling, and command-line error paths. | Public API method behavior. |
| Binary tests for MCP transport | [`crates/harness-mcp/tests/binary_transport.rs`](../../../crates/harness-mcp/tests/binary_transport.rs), target `binary_transport`, package `harness-mcp`. | The `harness-mcp` binary, help/version, `--check`, stdio framing, JSON-RPC behavior, reconnect cases, and response wrapping. | Core method semantics. |
| MCP integration tests | [`tests/integration/mcp_surface.rs`](../../../tests/integration/mcp_surface.rs), target `mcp_surface`, package `harness-integration-tests`. | Cross-layer MCP, Core, Store, surface binding, access derivation, tool exposure, replay-context binding, and storage no-effect checks visible through MCP. | A replacement for focused method tests or Reference owners. |
| Conformance implementation tests | [`tests/conformance/baseline.rs`](../../../tests/conformance/baseline.rs), target `baseline`, package `harness-conformance-tests`. | Baseline cross-method scenarios through Core-facing APIs, including replay, write authorization, artifacts, judgments, close readiness, error routing, and corruption handling. | Product acceptance, security proof, close readiness, or the sole source of a product rule. |
| Shared test support | [`crates/harness-test-support/src/lib.rs`](../../../crates/harness-test-support/src/lib.rs), package `harness-test-support`. | Disposable Runtime Home fixtures, registered project and surface setup, request builders, Store inspection helpers, and shared fixture composition. | Production behavior or a durable runtime home. |
| Documentation maintenance tooling tests | [`xtask/tests/docs_check.rs`](../../../xtask/tests/docs_check.rs), package `xtask`. | The read-only documentation validator, metadata parsing, bilingual coverage, local link and anchor checks, terminology path checks, retired-path detection, and temporary fixture behavior. | Semantic translation review, technical-accuracy review, or a product contract source. |

## Choosing A Layer

| Change category | Start with | Add when |
|---|---|---|
| Shared request, response, value, identifier, or canonical-hash type | `harness-types` unit tests. | Add Core method or integration tests when the shape changes method planning or adapter exposure. |
| Store read helper, mutation application, transaction, migration, or artifact storage behavior | Store module tests near the changed code. | Add Core method tests when a public method effect changes; add conformance or MCP integration when cross-layer behavior is affected. |
| Core method behavior | `crates/harness-core/src/methods/tests.rs`. | Add `tests/conformance/baseline.rs` for cross-method baseline scenarios and `tests/integration/mcp_surface.rs` when MCP exposure or access derivation matters. |
| Common Core preflight, branch routing, replay, freshness, or access policy | `crates/harness-core/src/pipeline.rs` unit tests and method tests. | Add MCP integration when adapter-derived invocation context or session binding is involved. |
| MCP adapter startup, tool schema, `tools/call`, or stdio transport | `crates/harness-mcp/src/lib.rs` tests and `binary_transport`. | Add `tests/integration/mcp_surface.rs` for cross-layer Core/Store behavior through MCP. |
| Administrative agent setup behavior | `binary_admin` and colocated CLI module tests for `agent_command.rs`, host adapters, repository guidance, and compatibility setup. | Add Store tests when bootstrap, inspection, registry, migration, Agent Integration Profile, project membership, or Host Installation inventory behavior changes. |
| Test fixture behavior | `harness-test-support` tests or the consuming package's tests. | Add owner-focused documentation checks if the fixture exposes a missing contract owner. |
| Documentation validator behavior | `xtask` tests and `cargo run -p xtask -- docs-check`. | Add fixture cases when a new deterministic structural rule is introduced. |
| Developer documentation only | `cargo run -p xtask -- docs-check` plus manual semantic parity, owner-routing, and terminology review. | Run Cargo tests only when requested or when the documentation change depends on source behavior that needs fresh validation. |

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

These tests are implementation checks. They are not Harness runtime
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
