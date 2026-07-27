# Runtime Home And Product Repository Design

## Purpose

This design explains how the implementation separates Volicord runtime state,
user product files, installed executables and host configuration, repository
maintenance tooling, and disposable test state.

## Design

`volicord-store` resolves and validates one canonical `Volicord Runtime Home`
for Registry data, project databases, artifacts, operational sessions, and
diagnostics. Product Repository registration stores the canonical product path
and Git layout identity without turning that repository into a runtime-data
directory.

CLI setup may manage owner-defined integration files in a Product Repository
or user configuration while holding mutation admission and setup transaction
state. Public Core method execution records owner-defined paths and
observations but does not write product files. `xtask` reads repository source,
documentation, metadata, and Cargo configuration without using Runtime Home
state.

## Invariants

- Runtime Home, Product Repository, source checkout, and installation location
  are distinct roles.
- A canonical Runtime Home identity is fixed after mutation admission.
- Runtime databases, logs, diagnostics, and generated records do not live in
  maintained documentation.
- Product-file writes remain outside the public Core method path.
- Tests use disposable Runtime Home and Product Repository locations.
- Repository tooling remains outside product runtime and reads the current
  source routes and workspace metadata directly.

## Responsibility boundaries

Store owns Runtime Home resolution, bootstrap, schema validation, project
registration, database access, and artifact paths. Platform filesystem code
owns safe path and publication primitives. CLI owns setup orchestration and
managed integration files. Core policy owns interpretation of owner-defined
product paths. `xtask` owns source-repository maintenance checks.

## Execution flow

1. The caller resolves and canonicalizes the Runtime Home.
2. Mutation-capable setup or ordinary writers acquire the applicable
   filesystem permit.
3. Store inspects or publishes the Runtime Home and validates its current
   manifest and physical schemas.
4. CLI registers the Product Repository and any explicit Connection
   membership.
5. Core and adapters use the registered product and Git coordinates for
   owner-defined work without using the repository as runtime storage.

## Failure behavior

Path alias mismatch, Runtime Home/Product Repository overlap, corrupt schema,
stale registration, publication ownership loss, or setup contention remains a
typed failure before dependent mutation. Setup rollback removes only state
whose publication guard still proves ownership; it does not recursively guess
at unrelated files.

## Scope exclusions

This design does not define path normalization contracts, security isolation,
storage layout, managed-file content, installation roots, or artifact
lifecycles. A location does not prove authority or actor identity.

## Implementation routes

- [`crates/volicord-store/src/runtime_home.rs`](../../../../crates/volicord-store/src/runtime_home.rs)
  and [`bootstrap.rs`](../../../../crates/volicord-store/src/bootstrap.rs):
  Runtime Home and project registration.
- [`crates/volicord-platform-fs/src/lib.rs`](../../../../crates/volicord-platform-fs/src/lib.rs)
  and [`mutation_lease.rs`](../../../../crates/volicord-platform-fs/src/mutation_lease.rs):
  canonical paths, publication, and mutation admission.
- [`crates/volicord-cli/src/setup_command/`](../../../../crates/volicord-cli/src/setup_command/)
  and [`connection_command/setup_transaction.rs`](../../../../crates/volicord-cli/src/connection_command/setup_transaction.rs):
  setup and managed-file transactions.
- [`crates/volicord-types/src/product_path.rs`](../../../../crates/volicord-types/src/product_path.rs):
  shared typed product-path normalization and containment helpers.
- [`xtask/src/repository.rs`](../../../../xtask/src/repository.rs):
  source-repository root and path handling.

## Reference owners

Exact behavior remains in
[Runtime Boundaries](../../reference/runtime-boundaries.md),
[Storage](../../reference/storage.md),
[Artifact Storage](../../reference/storage-artifacts.md),
[Administrative CLI](../../reference/admin-cli.md),
[Agent Connection](../../reference/agent-connection.md), and
[Security](../../reference/security.md).
