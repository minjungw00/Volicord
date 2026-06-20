# Runtime Home and Product Repository separation

## Context

Harness needs a local place for Runtime Home records, project state, registry
metadata, artifact data, and operational setup. The user's product files live
in the `Product Repository`. Mixing those locations would make implementation
paths harder to reason about and could make generated runtime state look like
product work.

Harness Server source and installation files are a separate implementation
artifact role. They may contain or deploy the `harness` and `harness-mcp`
executables, but they are not the Runtime Home or Product Repository by
definition.

## Decision

The implementation keeps `Harness Runtime Home` and `Product Repository` as
separate location concepts:

- Store code owns Runtime Home path handling, registry/project databases,
  project Store access, migrations, inspection, and artifact data under Runtime
  Home.
- CLI setup registers a Product Repository path with Runtime Home records but
  does not turn that repository into runtime state.
- CLI setup and MCP startup may refer to Harness Server installation files, but
  the installation location does not become Runtime Home or Product Repository.
- Core method code may normalize and reason about Product Repository paths when
  a method owner defines such inputs, but public API execution does not write
  product files directly.

## Consequences

- Disposable tests can create Runtime Home state under temporary directories
  without writing maintained docs or user product data.
- Store and CLI setup code can validate Runtime Home state separately from
  Product Repository file paths and Harness Server executable paths.
- Product-file writes remain outside the public Harness API path, while Core
  can record compatibility, observations, artifact links, and authority state
  as owner-defined behavior.
- Documentation and tests must avoid storing runtime homes, SQLite databases,
  generated logs, or artifact output in maintained documentation.

## Non-Goals

- This decision does not define security isolation.
- It does not make Runtime Home location proof of authority.
- It does not define a mandatory Harness Server installation root.
- It does not define Product Repository path normalization rules; the runtime
  boundary owner does.
- It does not define storage record layout, DDL, or artifact lifecycle rules.

## Relevant Implementation

- [`crates/harness-store/src/runtime_home.rs`](../../../../crates/harness-store/src/runtime_home.rs):
  Runtime Home resolution.
- [`crates/harness-store/src/bootstrap.rs`](../../../../crates/harness-store/src/bootstrap.rs):
  Runtime Home initialization and project/surface registration.
- [`crates/harness-store/src/core_pipeline.rs`](../../../../crates/harness-store/src/core_pipeline.rs):
  `CoreProjectStore` project-local access.
- [`crates/harness-store/src/artifacts.rs`](../../../../crates/harness-store/src/artifacts.rs):
  Runtime Home artifact staging and persistent body verification.
- [`crates/harness-cli/src/setup.rs`](../../../../crates/harness-cli/src/setup.rs):
  local MCP setup planning and Runtime Home preparation.
- [`crates/harness-core/src/policy/path.rs`](../../../../crates/harness-core/src/policy/path.rs):
  Product Repository path normalization helpers used by Core policy.

## Related Tests And Reference Owners

- `harness_binary_local_mcp_setup_flow` and
  `harness_binary_json_dry_run_is_parseable_and_does_not_register` in
  [`crates/harness-cli/tests/binary_admin.rs`](../../../../crates/harness-cli/tests/binary_admin.rs).
- `disposable_runtime_home_stays_under_system_temp` in
  [`crates/harness-test-support/src/lib.rs`](../../../../crates/harness-test-support/src/lib.rs).
- `missing_write_authorization_grant_blocks_prepare_write` in
  [`tests/integration/mcp_surface.rs`](../../../../tests/integration/mcp_surface.rs)
  for cross-layer local access behavior.
- [Runtime Boundaries](../../reference/runtime-boundaries.md),
  [Storage](../../reference/storage.md), [Artifact Storage](../../reference/storage-artifacts.md),
  [Administrative CLI](../../reference/admin-cli.md), and
  [Security](../../reference/security.md).
