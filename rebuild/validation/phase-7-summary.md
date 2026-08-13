# Phase 7 product-surface validation state

- Phase 7 capability gate before the final aggregate: `ready`
- Scope: qualified local platform primitives, source-grounded Project
  projections and documents, Project-scoped provider privacy, Local Operations
  and CLI, exact Guarded confirmation, local viewer, clean Linux installation,
  and high-level Codex/MCP host integration
- Excluded claim: this summary does not claim the reconstruction final
  aggregate, V11, Phase 8 dogfood quality, Phase 9 cutover, or legacy deletion

## Maintained validation status

| Validation | Status | Maintained evidence | Boundary retained |
| --- | --- | --- | --- |
| V06 | passed | `projections/source-grounded-documents` | four source-grounded documents, read-only Project projections, explicit publication/adoption |
| V07 | passed | `privacy/local-only-boundary` | local-first utility, Project opt-in, truthful transmission and deletion state |
| V08 | passed | `linux-codex-integration` | viewer, clean install, real Codex discovery, high-level MCP, lifecycle and preservation |
| V10 | passed | `local-platform-primitives` | qualified Linux process/filesystem/storage primitives without legacy API promotion |

The V08 report deliberately records that the reconstruction final aggregate has
not yet been run. Its status is based on the maintained focused journey and
mapped semantic oracles, not on a future aggregate result.

## Phase 7 capability conclusion

The smallest complete local surface is now present. A user can initialize and
bind a Project, inspect health, Recall, repository structure, Decisions,
Checkpoints, Candidates, canonical context, privacy state, and grounded
documents through the CLI, local viewer, or high-level host adapter. Read
surfaces do not gain mutation authority. Corrections, supersession, forgetting,
adoption/publication, Checkpoints, Inquiry responses, and Guarded confirmation
remain routed through Local Operations and their canonical Source requirements.

The viewer supports English and Korean fixed product text, overview/working/deep
explanations, explicit degraded states, and arbitrary requested generated
language. The Codex integration exposes product capabilities rather than raw
database operations. When host elicitation is unavailable, it returns the same
Guarded request identity, revision, and fingerprint to viewer/CLI fallback.
Connection launch failure remains separate from a connected but degraded
Project capability.

## Legacy exclusion audit

Cargo metadata places every reconstruction package below `rebuild/`. No package
depends on `volicord-core`, `volicord-store`, `volicord-types`,
`volicord-user-action-service`, a legacy MCP protocol/server crate, or another
legacy workflow crate. The clean installer accepts only its explicit current
prefix and replacement Runtime Home, registers the current `volicord-mcp`
binary, and contains no `VOLICORD_HOME`, legacy schema, migration, import,
backup, conversion, old command alias, or dual decoder path.

The V08 clean journey supplies a bait `VOLICORD_HOME` that contains a sentinel.
Install, Project use, uninstall, and reinstall leave the sentinel byte and
timestamp identical while the separate replacement canonical store survives.
This is an exclusion observation, not legacy compatibility or migration support.

## Accepted-Decision revisit triggers

No accepted Q1–Q13 Decision revisit trigger is active. Phase 7 retains the
Project as the user-facing unit, local-first canonical authority, current-host
user Source provenance, semantic Decision correction/supersession distinction,
Candidate non-canonical lifecycle, explicit document adoption/publication,
Project-scoped provider consent, and exact Guarded-effect confirmation.

The current Codex CLI registration syntax is an adapter fact, not a durable
domain decision. A future CLI syntax or elicitation capability change may
require adapter work without changing canonical identity or confirmation
meaning. V08's small deterministic repository does not trigger the accepted
large-context or multi-repository revisit conditions; those remain for V11.

## Known limits and next boundary

- Real Codex CLI registration and discovery are qualified, but an authenticated
  model-driven session and external network behavior are not required or claimed.
- The Linux journey does not qualify other operating systems, hostile filesystem
  races, concurrent MCP clients, abrupt power loss, large-repository latency,
  accessibility, or long-duration resource ceilings.
- V06 narrative quality, V07 commercial-provider behavior, and V10 platform
  portability retain the known limits documented in their individual reports.
- V11/Phase 8 remains the next independent multi-repository dogfood and product
  quality boundary after, and only after, the single Phase 7 final aggregate
  genuinely passes.

## Maintained references

- `rebuild/validation/projections/source-grounded-documents/report.md`
- `rebuild/validation/privacy/local-only-boundary/report.md`
- `rebuild/validation/linux-codex-integration/report.md`
- `rebuild/validation/local-platform-primitives/report.md`
- `rebuild/validation/shared/fixture-manifest.json`
- `rebuild/docs/design/validation-plan.md`
- `rebuild/docs/design/architecture.md`
- `rebuild/docs/design/failure-and-recovery.md`
- `rebuild/docs/design/cutover-plan.md`
