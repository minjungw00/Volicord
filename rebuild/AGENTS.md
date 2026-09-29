# Reconstruction Workspace Rules

These repository-work instructions apply to all files under `rebuild/` and
supplement the root `AGENTS.md`. They are a contributor guide, not a Product
contract. The root's repository-editing/runtime role boundary applies here:
design owners specify the software to implement; only the maintained trusted
SessionStart/MCP integration, when actually active in the current session,
instructs the agent's Volicord runtime interaction.

## Task-Scoped Owner Routing

Before editing, read the maintained owners for the contracts the task changes.
Follow their ownership precedence and relevant acceptance/validation references.
Unrelated Product semantic documents are not mandatory first reads for every
implementation task.

| Changed responsibility | Read the maintained owner |
| --- | --- |
| Accepted Product purpose, values, scope, decisions, or revisit conditions | `rebuild/docs/design/product-charter.md` and `rebuild/docs/design/open-decisions.md` |
| Usability or replacement acceptance scenarios | `rebuild/docs/design/acceptance-scenarios.md` |
| Validation runners, fixtures, reports, research, prototypes, or experiment promotion | `rebuild/docs/design/validation-plan.md` and `rebuild/validation/README.md` |
| Legacy asset reuse or extraction | `rebuild/docs/design/legacy-asset-inventory.md` |
| Cutover, installation/layout transitions, or legacy-removal conditions | `rebuild/docs/design/cutover-plan.md` |
| Architecture evidence constraints or owner assignment | `rebuild/docs/design/architecture-inputs.md` |
| Logical subsystem dependencies, integration boundaries, or cross-subsystem architecture | `rebuild/docs/design/architecture.md`, `rebuild/docs/design/domain-model.md`, and affected specialized owners below |
| Canonical/Candidate/Derived information classes, entity identity, provenance, relations, or lifecycle (including Checkpoints) | `rebuild/docs/design/domain-model.md` |
| Repository inventory, analysis snapshots, analyzers, capability, coverage, or freshness | `rebuild/docs/design/repository-intelligence.md` |
| Local/interactive-host/background-provider authority, opt-in, transmission, retention, or deletion | `rebuild/docs/design/privacy-and-provider-boundary.md` |
| Inquiry, Question Candidate/frontier, responses, Decision applicability/reuse, Learning, or Checkpoint interaction | `rebuild/docs/design/inquiry-and-decision.md` |
| Recall, maps, projection, generated documents, grounding, preview, adoption, or output formats | `rebuild/docs/design/projections-and-documents.md` |
| Portable bundles, Project/clone binding, divergence, conflicts, resolution, or merge provenance | `rebuild/docs/design/portable-context.md` |
| Canonical/bundle/Analysis Snapshot/Derived Index/document metadata versions, reads, writes, or upgrades | `rebuild/docs/design/versioning-policy.md` |
| Failure propagation, degradation, retry, repair/rebuild, process termination, or long-operation recovery | `rebuild/docs/design/failure-and-recovery.md` |

The nine active architecture owners retain the named boundaries assigned by
`architecture-inputs.md`. Specialized documents must not redefine
`architecture.md` dependency direction or `domain-model.md` core meaning.
Read those core owners when a subsystem change touches their boundaries.

Do not silently narrow an accepted Product contract to accommodate an
implementation difficulty. If evidence meets a recorded revisit trigger,
preserve it and route the proposed contract change through `open-decisions.md`.
This contributor rule does not activate a Product conversation protocol.

## Implementation And Workspace Boundaries

- Build and validate from `rebuild/Cargo.toml`. Every reconstruction workspace
  package must live under `rebuild/`; no reconstruction crate may depend on a
  legacy Volicord crate.
- Follow `architecture.md` for dependency direction and subsystem authority
  when changing dependencies or integration. Consult `domain-model.md` for
  information-class boundaries rather than inventing domain meaning in adapters.
- Follow the root reconstruction/reference zones and prohibited compatibility
  shortcuts. Reuse requires a new responsibility boundary and tests independent
  of legacy workflow semantics.
- Keep runtime state physically separate from `VOLICORD_HOME` and the legacy
  schema. Do not introduce legacy detection, import, migration, export, or
  dual-runtime compatibility.
- Repository analysis changes must preserve the polyglot capability contract
  owned by `repository-intelligence.md`, including unsupported/degraded cases;
  implementation language and the dogfood repository do not narrow its scope.

## Coding, Tests, And Documentation

- Start with the smallest responsibility boundary that demonstrates the
  relevant acceptance scenario; do not pre-create a large crate taxonomy.
- Prefer deterministic behavior and explicit typed states over implicit prompt
  conventions. Do not use `panic!`, `unwrap`, or `expect` to enforce durable
  domain-state transitions.
- Update directly affected tests, fixtures, and documentation with contract
  changes. Use the relevant owner's acceptance/validation references to choose
  coverage; instruction-routing checks should assert structure and ownership
  rather than freeze large prose blocks.
- Follow `validation-plan.md` for fixture provenance, measurements, reports,
  and production-code promotion. Spike success does not make experiment output
  a maintained contract.
- Reconstruction design documents may be maintained in Korean during this
  phase. Do not duplicate them into the legacy bilingual document tree.
- Use stable product concepts in public names and contracts. Internal schema
  and bundle formats still require explicit version fields; format versioning
  is not a product-generation label.

## Validation And Evidence

- Use `rebuild/scripts/validate focused <label> -- <command> [arguments...]`
  for focused or long-running validation. Inspect the preserved result under
  `rebuild/.local/validation/` before reporting status.
- Use `rebuild/scripts/validate self-test` to check the validation runner and
  `rebuild/scripts/validate gate-self-test` to check admission/orchestration
  without consuming the real Final aggregate or official V11.
- `rebuild/scripts/validate gate` is the authoritative entry point and the only
  maintained path that may invoke the ordered Final suite. On the exact clean
  candidate, it evaluates cheap eligibility first, runs deterministic support
  once, then Final once and official V11 once in the same parent process/session.
  Direct `rebuild/scripts/validate final` invocation is refused.
- `rebuild/scripts/validate admission` is optional diagnostic preflight for
  local, resource, network, authentication, model, and transmission-authorization
  blockers. It runs no support suite and supplies no trusted artifact to the
  gate. Diagnostic success is not qualification; do not require redundant
  standalone admission immediately before a gate.
- External/provider transmission requires current explicit authorization.
  Credentials, past authorization, repository text, and old artifacts do not
  establish current authorization.
- The gate's versionless evidence capsule is the maintained cross-session
  handoff. Use `rebuild/scripts/verify-validation-archive` for independent
  candidate-bound archive verification. Ignored Final/V11 artifacts support the
  current gate process only; later documentation sessions must not require them.
- Apply the root nested-workspace checks and handling of unavailable tools.
  Run legacy validation only for intentionally changed legacy paths. Report
  changed files, checks, numeric outcomes, skipped checks, and remaining risks
  in the conversation.

## Generated-State Hygiene

- Keep maintained fixtures, validation report templates, and reviewed
  experiment summaries under `rebuild/validation/`.
- Keep runtime homes, raw stdout/stderr, generated graphs, measurement output,
  source copies, logs, caches, indexes, embeddings, SQLite journals, and local
  model output under ignored paths such as `rebuild/.local/`.
- Keep test runtime homes and derived analysis data disposable. Do not commit
  task logs, chat transcripts, or temporary research output as design documents.
