# Conformance Reference

## 1. Current Status

This repository is documentation-only and still in documentation review. No Harness Server runtime, conformance runner, executable fixture files, generated conformance reports, generated runtime artifacts, or current runtime conformance results exist here.

This page is the current conformance owner for planning. It is not a runnable suite, not a test catalog, and not evidence that any future server behavior has run. Current phase and handoff status remain owned by [MVP Plan](../build/mvp-plan.md#documentation-acceptance-status).

## 2. What Conformance Means

Conformance means a future Harness implementation can prove specific behavior against Harness-owned authority records. A future check must drive an owner-defined Core, API, or operator action and compare captured facts with structured expectations.

Keep these layers separate:

| Layer | Meaning | Current status |
|---|---|---|
| Documentation checks | Read-only Markdown maintenance checks for links, terminology, owner boundaries, active/later wording, security wording, and bilingual parity. | Current docs-maintenance aid only; not runtime conformance. |
| Behavior examples | Compact examples of expected first-smoke and active MVP behavior. | Planning reference only; not fixture files and not pass/fail criteria. |
| Runtime conformance | Future executable checks over implemented Core/API/storage/operator behavior. | Does not exist yet. |

Conformance does not judge generated prose. It will judge owner-state effects, response facts, storage effects, stable events when promoted, artifact refs, blockers, errors, and forbidden side effects.

## 3. What Does Not Exist Yet

The following are future implementation work, not current repository contents:

- executable fixture files or a fixture directory
- a conformance runner or `harness conformance run` implementation
- generated conformance artifacts, reports, projections, runtime state, or Harness Runtime Home data
- current runtime `PASS`, `WARN`, or `FAIL` results
- active fixture suites for the active MVP or later candidates
- current proof of preventive blocking, OS permission control, arbitrary-tool sandboxing, tamper-proof storage, security isolation, or profile-gated `preventive` / `isolated` guarantees

Documentation examples here may guide implementation planning, but they do not create runtime state, acceptance evidence, close readiness, or implementation readiness.

## 4. Fixture Shape

Future fixtures should be ordinary structured inputs for a runner after the Harness Server exists. This page records the intended shape only; it does not provide full YAML bodies.

A promoted fixture should include these parts:

| Part | Purpose |
|---|---|
| `scenario_id` | Stable identifier for the behavior under review. |
| owner scope | Task, Change Unit, surface, state-version, and owner refs needed to interpret the action. |
| action | One public Core/API/operator request using the owner request schema. |
| initial authority context | The relevant Core state, storage rows, artifact refs, and surface capabilities before the action. |
| expected authoritative assertions | Structured response, state, storage, event, artifact, blocker, error, guarantee, or forbidden-side-effect facts. |
| owner links | The API, Core, Storage, Security, ArtifactRef, and policy owners that define the exact values. |

Materialized fixtures must use public owner schemas. They must not invent fixture-only enum values, pseudo-fields, localized display labels as state, prose-only expectations, or later-candidate-only values.

## 5. Assertion Authority

Assertion authority is narrower than scenario prose.

Authoritative future assertions:

- response facts returned by public owner APIs
- Core-owned Task, Change Unit, user judgment, Write Authorization, Run, evidence summary, blocker, close, and residual-risk state
- Storage-owned row effects, JSON `TEXT` owner fields, idempotency/replay facts, and state-version effects
- stable `task_events` only after the Core owner promotes their names
- `ArtifactRef`, artifact-link, `sha256`, `size_bytes`, `content_type`, `redaction_state`, retention, availability, and file-integrity facts when artifact proof matters
- primary `ErrorCode`, error details, and structured blocker fields from API/Core owners
- guarantee-level facts that match the Security and Agent Integration owners
- absence assertions for forbidden side effects, such as no durable authorization, no Run row, no artifact mutation, or no close-state change

Current active examples may assert `cooperative` and supported `detective` facts. `preventive` or `isolated` assertions are valid only for a promoted profile with owner-defined proof; conformance planning text does not make those guarantees currently executable or proven.

Non-authoritative material:

- prose scenario descriptions
- comments and author notes
- rendered Markdown, status prose, Journey Card prose, close report prose, or agent summaries
- documentation-check `PASS`, `WARN`, or `FAIL` labels
- projections, except for freshness or availability assertions when projection support is explicitly in scope

## 6. Representative Active Examples

These examples are compact behavior references. They are not fixture files, not full YAML, not a current runnable suite, and not runtime pass/fail criteria.

| Example | Behavior | Structured assertions a future fixture would use |
|---|---|---|
| `MVP-ACTIVE-prepare-write-blocked-or-dry-run-no-durable-authorization` | `prepare_write` returns blocked or dry-run information without creating a durable authorization. | Response has no consumable Write Authorization; `write_authorizations` has no inserted active row; no Run, artifact, evidence, close, final-acceptance, or residual-risk effect is created; blocker or dry-run facts match API/Core owners. |
| `MVP-ACTIVE-prepare-write-committed-scoped-authorization` | A committed allowed `prepare_write` records a scoped single-use Write Authorization. | Response authorization scope, Core state, and `write_authorizations.attempt_scope_json` agree on Task, Change Unit, state version, surface, intended paths/tools/commands/network/secrets/sensitive categories, baseline refs, related judgments, and guarantee level. |
| `MVP-ACTIVE-close-task-blocks-missing-acceptance-or-risk-condition` | `close_task` blocks when required final acceptance is missing, or when close-relevant residual risk is not visible or accepted as required. | Response blockers use owner categories and `required_judgment_kind` where applicable; Task is not completed; no close record substitutes for missing acceptance or risk acceptance; evidence, final acceptance, and residual-risk state remain separate. |

## 7. Catalog-Only Future Boundary

Future fixture families belong in [Later Candidate Index: Future Fixture Families](../later/index.md#future-fixture-families). That index lists names only as future candidates. It must not contain full scenario scripts, fixture bodies, active API payload examples, or suite requirements.

Current future family names are:

- Intake and decision routing
- Core, evidence, verification, and close
- Artifact redaction and export non-leakage
- Agency and user-judgment separation
- Connector capability honesty
- Design-quality and stewardship
- Context hygiene and resume freshness
- Projection, reconcile, and verification boundary
- Operations diagnostics, export, recover, and handoff
- Browser QA Capture

Listing a family does not make it an active MVP or later-candidate requirement. A future owner must promote a narrow behavior with scope, fallback behavior, exact contracts, and proof expectations before executable fixture material exists.

## 8. Metrics Boundary

Metrics are not conformance authority in the current documentation set. Future local derived metrics may be useful for diagnostics or planning, but they remain read-only derived displays until an owner promotes them.

Metrics must not create Core state, satisfy evidence, pass QA or verification, authorize writes, accept final results, accept residual risk, close work, prove implementation readiness, or replace runtime conformance. If a future metric is promoted, its owner must define the source records, freshness boundary, display wording, and non-substitution rule.
