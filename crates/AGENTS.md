# Rust Implementation Working Rules

These rules apply to Rust workspace implementation work in `crates/` and to
supporting implementation tests or fixtures that exercise crate behavior. They
add implementation-specific routing to the root `AGENTS.md`. They do not
define product behavior, public API behavior, storage effects, security
guarantees, schemas, Core authority semantics, conformance results, QA results,
acceptance decisions, close-readiness state, or residual-risk decisions.

## First Reads

- Read the root `AGENTS.md` before editing implementation files.
- Start with `docs/en/development/change-guide.md` or
  `docs/ko/development/change-guide.md` according to the working language.
- Use `docs/en/development/architecture.md` or
  `docs/ko/development/architecture.md` before changing crate placement,
  dependency direction, execution flow, Store boundaries, or adapter
  boundaries.
- Use `docs/en/development/testing-strategy.md` or
  `docs/ko/development/testing-strategy.md` before adding or moving tests.
- Use `docs/doc-index.yaml` for exact owner routing and
  `docs/en/reference/README.md` or `docs/ko/reference/README.md` for
  reader-facing Reference navigation.
- For implementation-facing text, CLI help, generated guidance, or examples
  that need Volicord brand presentation, use
  `docs/en/maintain/brand-guidelines.md` or
  `docs/ko/maintain/brand-guidelines.md` together with the applicable
  Reference owner when exact behavior matters.

## Contract Routes

- Public API method work starts at `docs/en/reference/api/methods.md` or
  `docs/ko/reference/api/methods.md`; follow the linked method, schema, error,
  blocker, storage-effect, and value-set owners as needed.
- Administrative CLI behavior routes to `docs/en/reference/admin-cli.md` or
  `docs/ko/reference/admin-cli.md`.
- MCP transport, tool exposure, and connection-context behavior route to
  `docs/en/reference/mcp-transport.md`,
  `docs/ko/reference/mcp-transport.md`,
  `docs/en/reference/agent-connection.md`, and
  `docs/ko/reference/agent-connection.md`.
- Store behavior routes to the applicable Storage Reference owner, including
  `storage.md`, `storage-effects.md`, `storage-records.md`,
  `storage-ddl.md`, `storage-artifacts.md`, and `storage-versioning.md`.
- Runtime home, product repository, generated host configuration, and local
  output location questions route to
  `docs/en/reference/runtime-boundaries.md`,
  `docs/ko/reference/runtime-boundaries.md`, and adjacent CLI or MCP owners.
- Conformance scenario meaning routes to `docs/en/reference/conformance.md` or
  `docs/ko/reference/conformance.md`. Test code still asserts the focused
  owner-defined facts behind each scenario.

## Implementation Boundaries

- Implement owner-defined behavior. If a method, schema, storage effect,
  security guarantee, runtime boundary, error meaning, scope rule, or Core
  authority rule is missing or unclear, update the applicable Reference owner
  first or report the owner gap.
- Do not add or expose a new public API method, request field, response field,
  storage effect, error meaning, security guarantee, or Core authority rule
  solely in Rust code, tests, fixtures, CLI help, adapter behavior, generated
  output, or comments.
- Keep product code in ordinary implementation paths under the Rust workspace,
  not under `docs/`.
- Place durable crate behavior in the crate that owns the responsibility
  described by the Development docs. Keep Core-facing code independent of CLI
  and MCP adapter layers.
- Keep shared type, schema representation, identifier, and value-set code in
  the workspace areas documented for shared types rather than duplicating
  shapes in adapters.
- Keep fixtures, snapshots, and helpers aligned with owner-defined facts. They
  must not become the only place a product contract is defined.
- Do not add legacy compatibility code, old flags, aliases, request shapes, or
  adapter paths unless a stable owner explicitly requires them.
- Do not edit generated files directly. Change the implementation source,
  generator, template, or fixture that owns the output and regenerate with the
  repository-supported command.

## Tests And Artifacts

- Place integration tests in the repository's established integration-test
  areas, colocated unit tests near the crate behavior they exercise, and shared
  test helpers in the established test-support crate.
- Add durable tests for current behavior, stable contracts, validation rules,
  state transitions, and user value. Do not add one-off tests whose only
  purpose is to prove that a removed string or obsolete path no longer exists.
- Update durable contract tests and generated schema or documentation outputs
  when public API, CLI, MCP, storage, or schema contracts change.
- Do not store runtime data, generated logs, SQLite files, product runtime
  homes, test runtime homes, generated projections, fixture output, QA results,
  acceptance records, close-readiness state, residual-risk records, work logs,
  archive copies, or local scratch notes in maintained documentation or
  repository guidance files.
- For local test runs, use Cargo build output, another ignored test-output
  location already used by the repository, or `/tmp`. If a test needs a runtime
  home, point it at a disposable per-test path.

## Validation

- After Rust implementation edits, inspect the Cargo workspace or changed crate
  layout before choosing validation commands.
- Default Rust validation from the workspace root is:
  - `cargo fmt`
  - `cargo clippy --all-targets --all-features`
  - `cargo test --all-targets --all-features`
- Use narrower Cargo commands only when the repository structure or task scope
  clearly calls for them, and report the reason.
- When editing Storage DDL, canonical SQL, or schema validation code, also run:
  - `cargo test -p volicord-store --test storage_ddl_contract`
- When implementation work also changes maintained documentation, run the
  applicable documentation validation from `docs/en/maintain/validation.md` or
  `docs/ko/maintain/validation.md`.
- If no Rust source, Cargo manifest, test, fixture, or build configuration is
  changed, Rust validation is not required.
