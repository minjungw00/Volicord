# Agent Working Rules — Reconstruction Branch

These rules apply across this reconstruction branch. They are repository work
instructions, not product contracts. The maintained implementation and
documentation that existed at the reconstruction baseline remain available for
reference, but they are not compatibility requirements for the replacement
product.

## Scope And Hierarchy

- Read this file before editing repository files.
- For any file under `rebuild/`, also read `rebuild/AGENTS.md` and the relevant
  documents under `rebuild/docs/design/`.
- `crates/AGENTS.md` and `docs/AGENTS.md` apply only when a task intentionally
  modifies the legacy paths in their scope. They do not govern new work under
  `rebuild/`.
- If a legacy scoped rule conflicts with the reconstruction boundary in this
  file, stop and report the conflict instead of adapting the replacement
  design to the legacy contract.
- Do not use product-generation or temporary reconstruction labels in public
  names, package names, commands, schemas, or user documentation. Conversation
  shorthand does not define repository naming.

## Repository Zones

### Reconstruction zone

- New product code, tests, design documents, fixtures, and temporary build
  configuration belong under `rebuild/` until cutover.
- The replacement implementation must build from `rebuild/Cargo.toml` without
  depending on the existing Volicord workflow crates.
- New durable product meaning is owned by `rebuild/docs/design/` during the
  reconstruction period.

### Legacy reference zone

The following paths are reference-only by default:

- `crates/`
- `tests/`
- `docs/en/`, `docs/ko/`, `docs/doc-index.yaml`, and
  `docs/terminology-map.yaml`
- `README.md` and `README.ko.md`
- the existing root Cargo workspace, release scripts, installers, Docker files,
  workflows, and `xtask/`

Do not modify a reference-only path unless the task explicitly does one of the
following:

1. fixes a critical issue needed to keep the baseline inspectable;
2. extracts an approved, responsibility-bounded primitive into the
   reconstruction zone;
3. prepares the final cutover after the replacement gate passes; or
4. updates the root boundary files required to keep the workspaces isolated.

When a legacy path is changed, state why the change is necessary and confirm
that it does not restore a removed workflow contract as a compatibility
requirement.

## Reconstruction Contract Ownership

- `rebuild/docs/design/product-charter.md` owns the accepted product purpose,
  user, values, principles, and non-goals.
- `rebuild/docs/design/open-decisions.md` owns accepted product decisions,
  their scope and revisit triggers, and any future product question.
- `rebuild/docs/design/acceptance-scenarios.md` owns the replacement usability
  and cutover scenarios.
- `rebuild/docs/design/legacy-asset-inventory.md` owns the provisional
  reuse/reference/removal classification.
- `rebuild/docs/design/validation-plan.md` owns risk spikes, fixture
  requirements, validation reports, and the gate for promoting experiment code.
- `rebuild/docs/design/architecture-inputs.md` owns Phase 3 evidence
  constraints, unsupported conclusions, and the architecture-document ownership
  plan. It does not own the target architecture.
- `rebuild/docs/design/architecture.md` owns the active logical subsystem map,
  cross-subsystem dependency direction, integration boundaries, and boundary
  conflict resolution.
- `rebuild/docs/design/domain-model.md` owns the active information classes,
  canonical entity meanings, core identity, provenance, relations, and lifecycle
  semantics.
- `rebuild/docs/design/repository-intelligence.md` owns active repository and
  analysis snapshot identity, inventory, normalized analysis envelopes,
  capability, coverage, freshness, provenance, and analyzer-adapter contracts.
- `rebuild/docs/design/privacy-and-provider-boundary.md` owns active local,
  interactive-host, and background-provider authority, opt-in, transmission,
  retention, and deletion boundaries.
- `rebuild/docs/design/inquiry-and-decision.md` owns active Question Candidate,
  frontier, response, terminal transition, Decision applicability, reuse, and
  Checkpoint-interaction contracts.
- `rebuild/docs/design/projections-and-documents.md` owns active Recall, map,
  document projection, grounding, preview, adoption, and output-format
  boundaries.
- `rebuild/docs/design/portable-context.md` owns active portable bundle,
  Project/clone binding, source-independent read, divergence, conflict,
  resolution, and merge-provenance boundaries.
- `rebuild/docs/design/versioning-policy.md` owns active canonical schema,
  portable bundle, Analysis Snapshot, Derived Index, and generated-document
  metadata version behavior and upgrade responsibility.
- `rebuild/docs/design/cutover-plan.md` owns the conditions and sequence for
  deleting the legacy implementation.
- Later target-architecture work must read all active Phase 3 owners and may
  not redefine `architecture.md` or `domain-model.md` in specialized documents.
  The remaining planned Phase 3 document becomes an active owner only after its
  file is created and routed here.
- Existing Reference, Architecture Guide, conformance, and SignalBox workflow
  documents describe the legacy baseline only. Do not infer replacement
  contracts from them.
- English/Korean parity and `docs/doc-index.yaml` routing do not apply to
  reconstruction design documents. Final maintained documentation policy will
  be established before cutover.

## Prohibited Compatibility Shortcuts

- Do not implement the replacement API as wrappers over legacy `status`,
  `intake`, UserAction, Write Ticket, Run, Evidence, final-acceptance, or close
  methods.
- Do not add replacement records to the existing Runtime Home or Store schema.
- Do not implement legacy Runtime Home detection, migration, import, historical
  export, backup guidance, command aliases, or dual-runtime compatibility.
- Do not make new crates depend on `volicord-core`, `volicord-store`,
  `volicord-types`, `volicord-user-action-service`, or other legacy workflow
  crates.
- Do not make legacy conformance tests the acceptance criteria for replacement
  behavior.
- Do not preserve Task phases, Change Units, ordinary-write admission, or close
  ceremony merely because the existing implementation contains them.
- Reuse code only after the needed primitive has a new responsibility, new
  tests, and no dependency on legacy workflow semantics.

## Runtime And Generated State

- Keep replacement runtime state separate from the existing `VOLICORD_HOME` and
  legacy database schema.
- Use ignored reconstruction-local paths such as `.local/` or
  `rebuild/.local/` for development runtime homes and generated analysis data.
- Canonical context, candidate observations, and rebuildable derived indexes
  must remain distinguishable in both design and implementation.
- Do not commit runtime homes, indexes, embeddings, generated analysis graphs,
  SQLite journals, logs, source copies, or local model output.

## Validation

For reconstruction work, run validation against the nested workspace rather
than the legacy root workspace:

```bash
cargo metadata --manifest-path rebuild/Cargo.toml --no-deps --format-version 1
cargo fmt --manifest-path rebuild/Cargo.toml --all -- --check
cargo clippy --manifest-path rebuild/Cargo.toml --workspace --all-targets --all-features
cargo test --manifest-path rebuild/Cargo.toml --workspace --all-targets --all-features
```

- Confirm that every reconstruction workspace package is under `rebuild/`.
- Confirm that no reconstruction package depends on a legacy Volicord crate.
- Confirm that product changes preserve the accepted polyglot capability
  contract rather than narrowing Repository Intelligence to the implementation
  language or the current dogfood repository.
- Run legacy validation only when legacy paths are intentionally changed.
- If required tools are unavailable, report the skipped command and reason;
  do not claim validation passed.
- Keep final reports in the conversation. Report changed files, checks and
  results, skipped checks, and remaining risks.
