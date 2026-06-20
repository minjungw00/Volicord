# Agent Working Rules

These rules apply across the repository to agents and maintainers who edit,
review, validate, or report on Harness repository work. They are repository
working guidance only. They do not define Harness runtime behavior, public API
behavior, storage effects, security guarantees, schemas, Core authority
semantics, conformance results, QA results, acceptance decisions,
residual-risk decisions, or implementation output.

Harness is the local work-authority product/system for AI-assisted product
work. Core is the local authority record for Harness state.

## Repository Priorities

Use this priority order when repository guidance, documentation shape, and
implementation workflow all matter:

1. Product onboarding must be understandable and executable.
2. Developer-learning documentation must accurately reflect the source code.
3. Reference contracts must remain precise and authoritative.
4. Maintenance metadata and validation rules must support those goals without
   becoming an ordinary reader entry point.

## Instruction Hierarchy

- Read this file before changing repository documentation or implementation
  files.
- The nearest applicable `AGENTS.md` adds scope-specific rules. Under `docs/`,
  read `docs/AGENTS.md`. For Rust workspace implementation work, including
  crate code and supporting tests or fixtures, read `crates/AGENTS.md`.
- When work crosses documentation and implementation boundaries, apply all
  relevant scoped rule files together. Stop and report if the scoped rules
  cannot be reconciled with this repository-wide file.
- Treat `docs/doc-index.yaml` as maintenance metadata for owner routing. It is
  not runtime configuration, not product contract data, and not an ordinary
  reader's mandatory first read.
- Stop and report before broad edits if `docs/doc-index.yaml` is missing or
  malformed enough that the applicable shared or paired document entry cannot
  be identified.
- Stop and report if the repository structure no longer matches the maintained
  shape described by this file, `docs/AGENTS.md`, `crates/AGENTS.md`,
  `docs/doc-index.yaml`, `docs/en`, `docs/ko`, and `crates`.
- Stop and report if a requested edit would require defining API behavior,
  storage behavior, schema meaning, security guarantees, or Core authority
  semantics directly in an `AGENTS.md`, README, Maintain page, route page,
  implementation comment, test, fixture, or other non-owner location.

## First Reads

- Use `docs/terminology-map.yaml` as the terminology and
  identifier-preservation source of truth.
- For documentation edits, read `docs/AGENTS.md` and the applicable authoring
  guidance under `docs/*/maintain/`.
- For bilingual edits, translation review, parity review, or
  terminology-affecting edits, read both language guides,
  `docs/terminology-map.yaml`, and the relevant glossary entries.
- For Rust implementation work, read `crates/AGENTS.md`, then start with
  `docs/en/build/implementation-guide.md` or
  `docs/ko/build/implementation-guide.md` according to the working language.
- For public API work, use `docs/en/reference/api/methods.md` and
  `docs/ko/reference/api/methods.md` for the supported public method list and
  method-owner routing.

## Contract-First Work

- Exact product contracts live in the applicable Reference owners selected from
  `docs/doc-index.yaml` or the human-readable Reference Index. This includes
  baseline scope, API behavior, schema meaning, error meaning, storage effects,
  security wording, access boundaries, close-readiness meaning, product
  terminology, out-of-scope promotion rules, and value-set meaning.
- If implementation work needs behavior that the maintained owners do not
  define, update the applicable owner document first or report the owner gap.
  Do not encode the behavior only in code, tests, fixtures, CLI help, adapter
  behavior, examples, generated output, or comments.
- If implementation and documentation appear to disagree, treat that as an
  owner-routing or implementation gap to resolve. Do not infer a new contract
  from existing code, examples, logs, generated output, or route metadata.
- Public Harness API methods are limited to the documented public method list
  in `docs/*/reference/api/methods.md`. Admin CLI commands are not public API
  methods.
- Keep user-owned judgment, evidence, verification criteria, ordinary approval,
  write approval, sensitive-action approval, `Write Authorization`, final
  acceptance, close readiness, and residual-risk acceptance distinct in
  documentation, code, tests, and reports.

## Language And Terminology

- English and Korean documentation are both maintained. Neither language is an
  archive, appendix, or translation-only copy.
- For ordinary lookup, read the language that matches the request or the
  default language in `docs/doc-index.yaml`.
- Do not finish a meaning-changing documentation batch with only one language
  updated when the changed document has a maintained paired path. Preserve the
  same reader purpose, normative strength, owner routing, baseline and
  out-of-scope boundaries, user-judgment boundary, and security guarantee level
  by meaning unit.
- Preserve exact identifiers, file paths, API methods, schema names, field
  names, enum values, status values, product labels, anchors, and code literals
  exactly where the terminology map requires it.
- Korean documentation must use natural Korean technical prose.

## Implementation Boundaries

- Keep product implementation code, tests, fixtures, and build configuration in
  ordinary implementation paths selected by the repository structure. Do not
  put product implementation code under `docs/`.
- Core-facing code must stay independent of CLI and MCP adapter layers. CLI and
  MCP adapters may call into Core-facing interfaces.
- Keep tests aligned to owner-defined facts. A test fixture or assertion must
  not become the only place a product contract is defined.
- Detailed Rust workspace placement, dependency, fixture, and validation rules
  live in `crates/AGENTS.md`.

## Runtime Output And Scratch Artifacts

- Maintained documentation under `docs/`, shared metadata, README files, and
  `AGENTS.md` files are not Harness runtime homes and are not places for
  generated runtime state.
- Do not store runtime data, generated logs, SQLite files, product runtime
  homes, test runtime homes, generated projections, fixture output, QA results,
  acceptance records, close-readiness state, residual-risk records, or work
  notes in maintained documentation or repository guidance files.
- For local test runs, use Cargo build output, another ignored test-output
  location already used by the repository, or `/tmp`. If a test needs a runtime
  home, point it at a disposable per-test path, not at maintained
  documentation, shared metadata, or user product data.
- Do not add persistent output directories, generated records, or local runtime
  homes to the repository unless the user asks for a durable implementation
  artifact and the repository's ignore rules and documentation owners support
  it.
- Keep planning in the conversation unless the user explicitly asks for a
  maintained documentation artifact. Scratch notes, archive copies, conversion
  notes, unresolved review notes, generated runtime records, implementation
  logs, PR notes, migration records, and work logs do not belong in maintained
  documentation.
- If a tool creates generated output during editing or validation, remove it
  before finishing unless it is ordinary ignored build output.

## Validation

- After documentation edits, run or perform the checks that match the changed
  files. For route and entry changes, include structure, links/indexes,
  terminology, and language parity checks when applicable.
- After Rust implementation edits, run the applicable Rust validation from the
  workspace or changed crate:
  - `cargo fmt`
  - `cargo clippy --all-targets --all-features`
  - `cargo test --all-targets --all-features`
- Use narrower Cargo commands only when the repository structure or task scope
  clearly calls for them, and report the reason.
- If a validation command cannot run because the relevant workspace, crate,
  toolchain, dependency, or network access is unavailable, report that as
  skipped validation with the reason.
- Before finishing, confirm changed links, file paths, anchors, paired-language
  links, owner routing, and terminology. Confirm no scratch files, archive
  copies, generated records, runtime homes, SQLite files, generated logs, or
  work notes remain from the edit.

## Reporting

Final reports stay in the conversation, not in repository files. Include changed
files, the summary of changes, validation performed and results, skipped
validation with reasons, and remaining risks or out-of-scope issues.

Treat validation results as repository maintenance or implementation-check
results only. Do not describe them as Harness runtime conformance, product
acceptance, QA completion, close readiness, security proof, or residual-risk
acceptance.
