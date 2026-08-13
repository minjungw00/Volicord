# V09 — Recall and Checkpoint accuracy

## Status

Passed for the maintained Phase 6 Production Rust acceptance matrix. The evidence maps 83 requirements to 47 discovered Production tests across Canonical Context, Inquiry, and Projections and executes those semantic owners without a second implementation.

## Goal

Verify Candidate lifecycle and inspection, staged Question promotion and frontier behavior, exact current-host response and Decision applicability, current-grounded Checkpoints, historical Source degradation, bounded Recall, and restart/resume without mutation or repeated resolved meaning.

## Accepted decisions being validated

The evidence validates product-charter sections 8–13; accepted decisions Q1, Q9, Q10, Q11, and Q13; acceptance scenarios F–I and L; and the active Domain, Inquiry/Decision, Projection, Privacy, Failure/Recovery, Validation, and Versioning owners. It changes no accepted product Decision.

## Input repositories and revisions

The Production semantic oracle is the nested reconstruction workspace at `62d8a7ccf754cb6dfa79b1472bf46b1e7a8cbeb2` plus this maintained evidence layer. The self-authored `v09-phase-6-matrix` fixture maps stable requirement identities to named Production Rust tests and is cataloged in the shared fixture manifest.

## Environment and tool versions

- Linux `6.18.33.2-microsoft-standard-WSL2`, x86-64.
- Rust and Cargo `1.97.1`.
- Python `3.12.3` for orchestration and evidence accounting only.
- No network service, LLM, background provider, or legacy Runtime Home is used.

## Candidate approaches

The selected approach discovers named Production tests, rejects a missing or ambiguous requirement mapping, and executes the three Production Rust owners. A separate Python domain engine was rejected because it would duplicate Candidate, frontier, Decision, Checkpoint, and Recall semantics instead of validating the implementation users run.

## Commands and configuration

The maintained focused commands are:

```text
rebuild/scripts/validate focused v09-fixture-manifest -- rebuild/scripts/check-fixture-manifest rebuild/validation/shared/fixture-manifest.json
rebuild/scripts/validate focused v09-assertions -- rebuild/validation/inquiry/phase-6-acceptance/assertions.py
rebuild/scripts/validate focused v09-assertions-repeat -- rebuild/validation/inquiry/phase-6-acceptance/assertions.py
rebuild/scripts/validate focused v09-report-shape -- rebuild/scripts/check-validation-report rebuild/validation/inquiry/phase-6-acceptance/report.md
```

The orchestrator uses normal Cargo parallelism and does not add repository-wide test-thread serialization.

## Observed results

The maintained runner discovered 19 Inquiry tests, 4 Projection tests, and 24 selected Canonical Context tests. All three Production commands passed. The 83 mapped requirements are distributed across Candidate lifecycle (16), promotion/frontier (17), response/Decision (19), Checkpoint (17), and Recall/inspection (14).

Independent pre-maintenance probes also passed against public Production APIs. They exercised promoted, dismissed, and pending finite-retention cleanup after reopen; idempotent cleanup; canonical target preservation; supporting, repository, verification, review, and acceptance Source roles across current, stale, unavailable, unknown, missing, and mixed bases; rejected-write no-mutation; and historical Checkpoint read degradation.

## Coverage and failures

Candidate evidence covers Project scope, automatic and explicit collection, opt-out/re-enable, immutable read bases, bounded content, promotion, dismissal, explicit deletion, retention cleanup, restart, reconciliation, and Candidate Inspection. Inquiry evidence covers materiality, research before asking, all seven terminal outcomes, exact dependencies, deterministic diagnostics/order, response rejection/batches/replay, Decision reuse/review, and supersession. Checkpoint/Recall evidence covers current Source roles and kinds, dirty attribution, boundary false positives, independent work/verification/review/acceptance facts, historical degradation, one-per-session triggering, bounds, omissions, state distinctions, and projection purity.

No maintained assertion failed. The evidence intentionally excludes V07, V10, V11, Phase 7 behavior, host UI rendering, and LLM-dependent Question relevance.

## Performance and resource observations

The first maintained acceptance run completed in about 0.95 seconds including discovery and 47 selected Production tests. The independent blocker probe completed in about 0.44 seconds. These are small local deterministic observations, not throughput or scale claims.

## Privacy and external transmission

Fixtures and test repositories are self-authored and local. Candidate and canonical databases, repository snapshots, logs, and command results remain under disposable temporary directories or ignored `rebuild/.local/` state. No source or Candidate content is transmitted externally.

## Acceptance results

- Pass: Candidate disposition, cleanup, retention, promotion target, opt-out, and inspection state remain independent and restart-safe.
- Pass: repository facts may resolve a branch without fabricating a user Decision; all seven terminal outcomes retain their Decision requirements.
- Pass: frontier prerequisite meaning, ordering, diagnostics, historical observation, and restart recomputation are deterministic.
- Pass: exact current-host identity/revision/Source linkage is enforced; rejected or stale responses do not gain partial canonical success.
- Pass: Decision reuse is distinguished from evidence-driven review and semantic-choice supersession.
- Pass: only current, kind-correct Source basis makes a new Checkpoint eligible; missing and non-current failures remain distinct.
- Pass: existing dirty changes are not attributed to bounded work, and work, verification, review, and acceptance remain independent.
- Pass: historical Checkpoints remain readable with stale or unavailable Sources and expose degradation without canonical rewriting.
- Pass: unrelated requests do not Recall; first project-scoped requests do; bounded Recall reports deterministic omissions and is read-only.
- Pass: restart preserves terminal Questions, Decisions, Candidate lifecycle, and Checkpoint observations while current frontier and Recall derive from current canonical state.

## Known limits

- Fixtures are deterministic, local, and small; they do not measure natural-language Question quality, large-context relevance, or production-scale resource ceilings.
- Session Recall triggering is an in-memory logical contract; Codex, CLI, MCP, and viewer transport are later integration responsibilities.
- Automatic Candidate discovery quality and LLM interpretation are not evaluated.
- V07 privacy journeys, V10 primitive qualification, and V11 multi-repository rehearsal are not complete or claimed.

## Recommended implementation choice

Keep Production Rust as the only semantic authority. Retain Candidate storage as physically separate local state, recompute frontier and Recall from immutable current read bases, require current semantic Source grounding for new Checkpoints, and preserve historical canonical observations while exposing degraded Source state in projections.

## Rejected alternatives and reasons

- Reject a Python shadow engine: it could agree with itself while Production behavior diverges.
- Reject persisted Checkpoint frontier as resume authority: canonical Question state already provides the deterministic current basis.
- Reject treating unavailable historical Sources as deleted: degradation is inspectable history, not forgetting.
- Reject weakening currentness assertions for a green gate: stale, unavailable, unknown, and missing bases have distinct user meaning.

## Reusable primitive decision

`production_evidence`. The fixture and Python runner are maintained orchestration and accounting; all Candidate, Inquiry, Decision, Checkpoint, Recall, and inspection meaning remains in the Production Rust crates and their integration tests.

## Decision revisit trigger status

Not triggered. The maintained evidence supports the accepted Candidate, Inquiry, Decision, Checkpoint, and Recall boundaries without narrowing product scope or changing Source authority. No unresolved accepted-Decision revisit trigger is present in this phase evidence.

## Follow-up work

Use this V09 result for the Phase 6 conclusion only. V07, V10, V11, host integration, document output, and Phase 7 implementation remain separate work and must not be inferred from this report.

## Artifacts

Maintained artifacts are the `v09-phase-6-matrix` manifest entry, `fixtures/phase-6-matrix.json`, `assertions.py`, and this report. Raw reproducibility evidence is preserved under ignored `rebuild/.local/validation/`, including the independent blocker recheck and focused pre-commit assertion runs; it is not a maintained product surface.
