# Rust Implementation Working Rules

These rules apply to Rust workspace implementation work in `crates/` and to
supporting implementation tests or fixtures that exercise crate behavior. They
add implementation-specific guidance to the root `AGENTS.md`. They do not
define product behavior, API behavior, storage effects, security guarantees,
runtime behavior, schemas, Core authority semantics, conformance results, QA
results, acceptance decisions, close-readiness state, or residual-risk
decisions.

## First Reads

- Start with `docs/en/build/implementation-guide.md` or
  `docs/ko/build/implementation-guide.md` according to the working language.
- Use `docs/en/build/architecture.md` or `docs/ko/build/architecture.md` for
  durable workspace shape, crate roles, execution-flow maps, and
  implementation-boundary guidance.
- Use `docs/doc-index.yaml` only when exact machine-readable owner routing is
  needed. Use the Reference Index for reader-facing owner navigation.
- For public API work, use `docs/*/reference/api/methods.md` for the supported
  public method list and method-owner routing.

## Contract-First Implementation

- Implement owner-defined behavior. If a method, schema, storage effect,
  security guarantee, runtime boundary, error meaning, scope rule, or Core
  authority rule is missing or unclear, update the applicable Reference owner
  first or report the owner gap.
- Do not add or expose a new public API method, request field, response field,
  storage effect, error meaning, security guarantee, or Core authority rule
  solely in Rust code, tests, fixtures, CLI help, adapter behavior, or comments.
- Use examples as reading aids, not as complete schemas or behavior sources.
  Implementation decisions come from the focused owners for scope, methods,
  schemas, storage, security, runtime boundaries, errors, blockers, and
  conformance.
- If code and documentation disagree, do not treat current code as the new
  contract. Resolve product meaning through the applicable Reference owner and
  implementation.

## Placement And Dependencies

- Keep product code in ordinary implementation paths under the Rust workspace,
  not under `docs/`.
- Place durable crate behavior in the crate that owns the implementation
  responsibility described by the implementation guide and architecture guide.
- Keep Core-facing code independent of CLI and MCP adapter layers. CLI and MCP
  adapters may call into Core-facing interfaces; Core-facing code must not
  depend on those adapters.
- Keep shared type, schema representation, identifier, and value-set code in
  the workspace areas documented for shared types rather than duplicating
  shapes in adapters.
- Place integration tests in the repository's established integration-test
  areas, colocated unit tests near the crate behavior they exercise, and shared
  test helpers in the established test-support crate.
- Keep fixtures aligned with owner-defined facts. A fixture shape, helper API,
  snapshot, or assertion must not become the only place a product contract is
  defined.

## Judgment And Authority Boundaries

- Keep user-owned judgment, evidence, verification criteria, ordinary approval,
  write approval, sensitive-action approval, `Write Authorization`, final
  acceptance, close readiness, and residual-risk acceptance distinct in code,
  tests, fixtures, and API examples.
- Do not collapse evidence collection, verification criteria, QA, acceptance,
  waivers, close readiness, or residual-risk decisions into one broad approval
  path.
- Route exact behavior for these distinctions to the focused Reference owners
  rather than duplicating an owner map in implementation guidance.

## Runtime Data And Generated Output

- Do not store runtime data, generated logs, SQLite files, product runtime
  homes, test runtime homes, generated projections, fixture output, QA results,
  acceptance records, close-readiness state, residual-risk records, or work
  notes in maintained documentation or repository guidance files.
- For local test runs, use Cargo build output, another ignored test-output
  location already used by the repository, or `/tmp`.
- If a test needs a runtime home, point it at a disposable per-test path, not
  at maintained documentation, shared metadata, or user product data.

## Validation

- After Rust implementation edits, inspect the Cargo workspace or changed crate
  layout before choosing validation commands.
- Default Rust validation from the workspace root is:
  - `cargo fmt`
  - `cargo clippy --all-targets --all-features`
  - `cargo test --all-targets --all-features`
- Use narrower Cargo commands only when the repository structure or task scope
  clearly calls for them, and report the reason.
- When implementation work also changes maintained documentation, run the
  applicable documentation checks from `docs/*/maintain/checks.md`.
- If no Rust source, Cargo manifest, test, fixture, or build configuration is
  changed, Rust validation is not required.
