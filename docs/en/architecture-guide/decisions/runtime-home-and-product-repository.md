# Runtime Home and Product Repository separation

## Context

Volicord needs a local place for Runtime Home records, project state, registry
metadata, artifact data, and operational setup. The user's product files live
in the `Product Repository`. Mixing those locations would make implementation
paths harder to reason about and could make generated runtime state look like
product work.

Volicord source and installation files are a separate implementation
artifact role. They may contain or deploy the `volicord` executable and
implementation crates such as `volicord-mcp`, but they are not the Runtime Home
or Product Repository by definition.

## Decision

The implementation keeps `Volicord Runtime Home` and `Product Repository` as
separate location concepts:

- Store code owns Runtime Home path handling, registry/project databases,
  project Store access, schema initialization and validation, inspection, and artifact data under Runtime
  Home.
- CLI connection provisioning registers a Product Repository path with Runtime
  Home records but does not turn that repository into runtime state.
- CLI setup and MCP startup may refer to Volicord installation files, but
  the installation location does not become Runtime Home or Product Repository.
- Core method code may normalize and reason about Product Repository paths when
  a method owner defines such inputs, but public API execution does not write
  product files directly.

## Consequences

- Disposable tests can create Runtime Home state under temporary directories
  without writing maintained docs or user product data.
- Store and CLI setup code can validate Runtime Home state separately from
  Product Repository file paths and Volicord executable paths.
- Product-file writes remain outside the public Volicord API path, while Core
  can record compatibility, observations, artifact links, and authority state
  as owner-defined behavior.
- Documentation and tests must avoid storing runtime homes, SQLite databases,
  generated logs, or artifact output in maintained documentation.

## Non-goals

- This decision does not define security isolation.
- It does not make Runtime Home location proof of authority.
- It does not define a mandatory Volicord installation root.
- It does not define Product Repository path normalization rules; the runtime
  boundary owner does.
- It does not define storage record layout, DDL, or artifact lifecycle rules.

## Relevant implementation

- [`crates/volicord-store/src/runtime_home.rs`](../../../../crates/volicord-store/src/runtime_home.rs):
  Runtime Home resolution.
- [`crates/volicord-store/src/bootstrap.rs`](../../../../crates/volicord-store/src/bootstrap.rs):
  Runtime Home initialization and project registration.
- [`crates/volicord-store/src/agent_connections.rs`](../../../../crates/volicord-store/src/agent_connections.rs):
  Agent Connection records and Connection Project memberships.
- [`crates/volicord-store/src/core_pipeline.rs`](../../../../crates/volicord-store/src/core_pipeline.rs):
  `CoreProjectStore` project-local store access.
- [`crates/volicord-store/src/artifacts.rs`](../../../../crates/volicord-store/src/artifacts.rs):
  Runtime Home artifact staging and persistent body verification.
- [`crates/volicord-cli/src/connection_command/service.rs`](../../../../crates/volicord-cli/src/connection_command/service.rs):
  connection provisioning, Runtime Home preparation, project registration, and
  Agent Connection membership orchestration.
- [`crates/volicord-core/src/policy/path.rs`](../../../../crates/volicord-core/src/policy/path.rs):
  Product Repository path normalization helpers used by Core policy.

## Related tests and Reference owners

- `init_dry_run_does_not_write_runtime_or_repo_files`,
  `init_rejects_invalid_profile_without_artifacts`,
  `init_codex_guarded_writes_policy_mcp_and_guard_status_idempotently`, and
  `project_commands_use_current_git_repository_without_user_ids` in
  [`crates/volicord-cli/tests/binary_admin.rs`](../../../../crates/volicord-cli/tests/binary_admin.rs)
  cover focused initialization, zero-write, and project-selection behavior.
- `disposable_runtime_home_stays_under_system_temp` in
  [`crates/volicord-test-support/src/lib.rs`](../../../../crates/volicord-test-support/src/lib.rs).
- `read_only_mode_rejects_agent_workflow_methods_before_core` in
  [`tests/integration/mcp_connection.rs`](../../../../tests/integration/mcp_connection.rs)
  for cross-layer invocation-context behavior.
- [Runtime Boundaries](../../reference/runtime-boundaries.md),
  [Storage](../../reference/storage.md), [Artifact Storage](../../reference/storage-artifacts.md),
  [Administrative CLI](../../reference/admin-cli.md), and
  [Security](../../reference/security.md).
