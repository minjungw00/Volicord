# Reconstruction Workspace Rules

These rules apply to all files under `rebuild/`. They supplement the root
`AGENTS.md`. They guide reconstruction work and do not replace the product
charter or create public behavior by themselves.

## Required First Reads

Before changing reconstruction design or implementation, read:

1. `rebuild/docs/design/product-charter.md`
2. `rebuild/docs/design/open-decisions.md`
3. `rebuild/docs/design/acceptance-scenarios.md`
4. `rebuild/docs/design/validation-plan.md` before research, prototypes, or
   promoting experiment code
5. `rebuild/docs/design/legacy-asset-inventory.md` when considering reuse
6. `rebuild/docs/design/cutover-plan.md` when changing repository layout,
   installation, data handling, or legacy-removal conditions
7. `rebuild/docs/design/architecture-inputs.md` before Phase 3 target
   architecture work; it owns evidence constraints and the architecture-document
   ownership plan, not the target architecture

Planned Phase 3 architecture documents are not active contracts until their
files are created. Once created, `architecture.md` owns cross-subsystem
dependency direction and resolves boundary conflicts; each specialized document
owns the named domain routed by `architecture-inputs.md`.

The required product decisions are accepted in `open-decisions.md`. Do not
quietly narrow them because an implementation is difficult. When validation
meets a recorded revisit trigger, add a new product question, preserve the
evidence, and wait for the user's decision before changing the contract.

## Product Direction

- Optimize for user understanding, meaningful judgment, shared memory,
  learning, and reliable resumption across sessions and environments.
- Do not optimize the product around ceremony, automatic orchestration, token
  reduction, or maximum tool-call capture.
- Ask as many questions as materially necessary, but do not ask questions that
  the repository, environment, or an experiment can answer.
- Separate a user decision from an agent recommendation, inferred preference,
  observed fact, and generated explanation.
- A user should answer a named question once in the current host interaction.
  Stronger confirmation is reserved for high-risk effects.
- Ordinary repository edits do not require a Write Ticket or replacement
  equivalent.

## Canonical And Derived Boundaries

The design uses three information classes:

- **Canonical context:** portable, user-inspectable project identity, sources,
  questions, decisions, context items, checkpoints, revisions, and
  supersession.
- **Session candidates:** temporary observations, interpretations, possible
  questions, and checkpoint candidates that have not been promoted.
- **Derived state:** rebuildable indexes, code graphs, embeddings, cached
  summaries, fingerprints, rankings, layouts, and generated previews.

Rules:

- Deleting derived state must not damage canonical context.
- Derived analysis must not silently create, resolve, revise, supersede, or
  delete a user decision.
- Candidate promotion must preserve provenance and information class.
- Users must be able to inspect, correct, supersede, and forget canonical
  records.
- Access frequency may affect retrieval order, not the validity of a decision.

## Dependency Direction

- The canonical context kernel must not depend on repository analyzers, LLM
  providers, MCP, CLI, web UI, or document rendering.
- Repository Intelligence may refer to canonical source and decision IDs, but
  it must not own user judgment.
- Inquiry may read canonical context and repository analysis, but only an
  explicitly linked user response may become a user decision.
- Projection and document generation may read canonical and derived data, but
  must not mutate source records as a side effect.
- Adapters translate host input and output; they do not invent domain meaning.
- No reconstruction crate may depend on a legacy Volicord crate.

## Repository Intelligence

- Treat repository-wide code understanding as a first-party product
  responsibility implemented in a separable subsystem.
- Do not restrict the product contract to Rust because Volicord is implemented
  in Rust or dogfoods a Rust repository.
- Preserve per-language and per-area capability states for inventory,
  agent-assisted, structural, semantic, and ecosystem analysis.
- The first structural gate covers Java, Python, JavaScript, TypeScript, C,
  C++, and Rust; at least three ecosystems also require semantic validation.
- Distinguish parser- or repository-derived structural facts from LLM-produced
  semantic annotations in types, storage, output, and tests.
- Every explanation must identify its source snapshot, coverage, unsupported or
  excluded areas, freshness, and uncertainty where applicable.
- Do not claim complete semantic knowledge of unsupported languages, macros,
  generated code, dynamic behavior, external services, or runtime-only state.
- Structural mode must remain useful when semantic analysis is disabled or
  unavailable.
- Source code must not be sent to an external background provider without an
  explicit Project opt-in and inspectable source scope. Interactive use by the
  active host and background transmission must remain distinguishable.

## Inquiry And Decision Work

- Model open questions independently from decisions.
- Track dependencies so only the current material question frontier is shown.
- For each question, explain why it matters now, established facts, options,
  recommendation, trade-offs, uncertainty, and what the answer unlocks.
- A branch ends through decision, delegation, research, prototype, deferment,
  exclusion, or supersession.
- Preserve progress after every inquiry round so a new session can resume
  without repeating answered questions.
- Do not coerce an answer when the user says they do not know. Convert the
  branch to research, prototype, or deferment as appropriate.

## Checkpoints, Recall, And Documents

- A checkpoint records current state, meaningful changes, applied decisions,
  verification, known limits, open questions, and the next recommended step.
- Work state, automated verification, user review, and user acceptance are
  independent facts.
- User and agent recall views may differ in depth, but must use the same record
  identities, sources, freshness, uncertainty, and supersession state.
- Generated documents are source-grounded projections by default. They become
  preserved sources only through an explicit adoption action.
- Generated documents must record source snapshot, included decisions,
  analysis coverage, known gaps, generation time, and generator identity.

## Coding And Testing

- Start with the smallest responsibility boundary that demonstrates an
  acceptance scenario; do not pre-create a large crate taxonomy.
- Use `rebuild/scripts/validate focused <label> -- <command> [arguments...]`
  for focused or long-running validation. Inspect the preserved result under
  `rebuild/.local/validation/` before reporting its status.
- Use `rebuild/scripts/validate self-test` to verify the repository-local
  validation runner. `rebuild/scripts/validate final` owns the ordered
  aggregate suite from the root `AGENTS.md`; run it only when the task's
  validation role is final.
- Keep maintained fixtures, validation report templates, and experiment
  summaries under `rebuild/validation/`. Keep raw command output, generated
  graphs, and measurement artifacts under ignored `rebuild/.local/` state.
- Follow `validation-plan.md` for fixtures, measurements, reports, and
  production-code promotion. Spike success does not make experiment output a
  maintained contract.
- Prefer deterministic behavior and explicit typed states over implicit prompt
  conventions.
- Do not use `panic!`, `unwrap`, or `expect` to enforce durable domain-state
  transitions.
- Add tests for portable serialization, restart recovery, provenance,
  correction, supersession, deletion, coverage reporting, and degraded
  operation as those capabilities are introduced.
- Keep test runtime homes and derived analysis data disposable.
- Validate from `rebuild/Cargo.toml`; do not run legacy workspace tests as a
  substitute for reconstruction acceptance.

## Documentation And Naming

- Reconstruction design documents may be maintained in Korean during this
  phase. Do not duplicate them into the legacy bilingual document tree.
- Use stable product concepts rather than implementation-history or temporary
  reconstruction labels in public contracts.
- Internal schema and bundle formats still require explicit version fields;
  format versioning is not a product-generation label.
- Do not commit task logs, chat transcripts, runtime state, generated graphs,
  or temporary research output as maintained design documentation.
- The replacement product does not detect, import, migrate, export, or provide
  compatibility for the legacy Runtime Home. Use a clean, physically separate
  runtime during reconstruction and after cutover.
