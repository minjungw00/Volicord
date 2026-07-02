# Agent Working Rules

These rules apply across the repository to agents and maintainers who edit,
review, validate, or report on Volicord repository work. They are work
instructions only. They do not define product behavior, public API behavior,
storage effects, security guarantees, schemas, Core authority semantics,
conformance results, QA results, acceptance decisions, close-readiness state,
residual-risk decisions, or implementation output.

## Scope And Hierarchy

- Read this file before editing repository files.
- Read the closest applicable `AGENTS.md` before changing files in its scope.
  Under `docs/`, read `docs/AGENTS.md`. For Rust workspace implementation
  work, including crate code and supporting tests or fixtures, read
  `crates/AGENTS.md`.
- When a change crosses documentation and implementation boundaries, apply all
  relevant scoped rules together. Stop and report if scoped rules cannot be
  reconciled.
- If another scoped `AGENTS.md` is added later, it applies only to files under
  that directory and does not replace this root file.
- Stop and report if the repository structure no longer matches the maintained
  shape described by this file, `docs/AGENTS.md`, `crates/AGENTS.md`,
  `docs/doc-index.yaml`, `docs/en`, `docs/ko`, and `crates`.
- Stop and report before broad documentation edits if `docs/doc-index.yaml` is
  missing or malformed enough that the applicable shared or paired document
  entry cannot be identified.

## Stable Owner Routes

- Use `docs/doc-index.yaml` for machine-readable owner routing, maintained
  paths, paired-language routing, maintenance responsibility, and applicability.
  It is not runtime configuration and not product contract data.
- Use `docs/terminology-map.yaml` for terminology and identifier-preservation
  rules.
- For Rust implementation work, start with
  `docs/en/development/change-guide.md` or
  `docs/ko/development/change-guide.md`, then use the matching
  `docs/en/development/architecture.md` or
  `docs/ko/development/architecture.md` before changing durable
  implementation structure.
- For public API, CLI, MCP, storage, schema, runtime-boundary, security, Core
  model, conformance, close-readiness, value-set, or blocker behavior, use the
  focused Reference owner selected through `docs/doc-index.yaml` or the
  human-readable Reference Index at `docs/en/reference/README.md` or
  `docs/ko/reference/README.md`.
- For the supported public API method list and method-owner routing, use
  `docs/en/reference/api/methods.md` and
  `docs/ko/reference/api/methods.md`. Administrative CLI commands are not
  public API methods.
- For product naming, brand copy, visual presentation, or broad claim wording,
  use `docs/en/maintain/brand-guidelines.md` or
  `docs/ko/maintain/brand-guidelines.md` together with the applicable
  Reference owner when exact behavior matters.
- For documentation policy and validation, use
  `docs/en/maintain/documentation-policy.md`,
  `docs/ko/maintain/documentation-policy.md`,
  `docs/en/maintain/validation.md`, and
  `docs/ko/maintain/validation.md`.

## Contract And Implementation Boundaries

- Do not define API behavior, storage behavior, schema meaning, security
  guarantees, Core authority semantics, or other product contracts in an
  `AGENTS.md`, README, Maintain page, route page, implementation comment, test,
  fixture, generated output, or CLI help.
- If implementation work needs behavior that the maintained owners do not
  define, update the applicable owner document first or report the owner gap.
- If implementation and documentation disagree, treat that as an owner-routing
  or implementation gap to resolve. Do not infer a new contract from current
  code, examples, generated output, logs, or metadata.
- Keep product implementation code, tests, fixtures, and build configuration in
  ordinary implementation paths. Do not put product implementation code under
  `docs/`.
- Core-facing code must stay independent of CLI and MCP adapter layers. CLI and
  MCP adapters may call Core-facing interfaces.
- Keep tests and fixtures aligned to owner-defined facts. A test, fixture,
  helper, or snapshot must not become the only place a product contract is
  defined.
- Do not add legacy compatibility code, old aliases, fallback behavior, or
  adapter paths unless a stable Reference or Development owner explicitly
  requires them. When compatibility is required, test the current supported
  behavior and owner-defined boundary.
- When public contracts change, update the focused owner, durable contract
  tests, affected generated documentation or schema output, and reader-facing
  routes in the same change.

## Generated And Runtime Files

- Do not edit generated files directly. Change the source, generator, template,
  or fixture that owns the generated content, then regenerate it with the
  repository-supported command.
- Maintained documentation, shared metadata, README files, and `AGENTS.md`
  files are not Volicord runtime homes and are not places for generated runtime
  state.
- Do not store runtime data, generated logs, SQLite files, product runtime
  homes, test runtime homes, generated projections, fixture output, QA results,
  acceptance records, close-readiness state, residual-risk records, work logs,
  archive copies, or local scratch notes in maintained documentation or
  repository guidance files.
- For local test runs, use Cargo build output, another ignored test-output
  location already used by the repository, or `/tmp`. If a test needs a runtime
  home, point it at a disposable per-test path.
- Do not add one-off tests, persistent scratch artifacts, or local output
  directories. Add durable tests only when they protect current behavior,
  owner-defined contracts, stable validation rules, or user value.
- If a tool creates generated output during editing or validation, remove it
  before finishing unless it is ordinary ignored build output.

## Language And Terminology

- English and Korean documentation are both maintained. Neither language is an
  archive, appendix, or translation-only copy.
- For ordinary lookup, read the language that matches the request or the
  default language recorded in `docs/doc-index.yaml`.
- Do not finish a meaning-changing documentation batch with only one language
  updated when the changed document has a maintained paired path.
- Preserve exact identifiers, file paths, API methods, schema names, field
  names, enum values, status values, product labels, anchors, and code literals
  where the terminology map requires them.
- Korean documentation must use natural Korean technical prose.

## Validation And Reporting

- After documentation edits, run the checks in
  `docs/en/maintain/validation.md` or `docs/ko/maintain/validation.md`. For
  route and entry changes, include structure, links, terminology, and
  language-parity checks when applicable.
- After Rust implementation edits, run the applicable Rust validation from the
  workspace or changed crate:
  - `cargo fmt`
  - `cargo clippy --all-targets --all-features`
  - `cargo test --all-targets --all-features`
- Use narrower Cargo commands only when the repository structure or task scope
  clearly calls for them, and report the reason.
- If validation cannot run because the relevant workspace, crate, toolchain,
  dependency, or network access is unavailable, report it as skipped validation
  with the reason.
- Before finishing, confirm changed links, paths, anchors, paired-language
  links, owner routing, terminology, and repository hygiene.
- Final reports stay in the conversation, not in repository files. Include
  changed files, validation performed and results, skipped validation with
  reasons, and remaining risks or out-of-scope issues.
