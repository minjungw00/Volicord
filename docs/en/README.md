# Harness Documentation

This is the English entry point for the active Harness documentation set. Harness is a planned local work-authority server for AI-assisted product work. Its planned authority is over Harness records and state transitions for scope, user-owned judgment, evidence, verification expectations, final acceptance, close readiness, and residual risk.

This repository is documentation-only today. It has no server/runtime implementation, runtime state, generated projections, generated operational artifacts, executable fixtures, conformance runner, or product implementation code. It is not the user's Product Repository, not a Harness Runtime Home, and not a running Harness instance.

Harness is not a prompt pack, operating-system permission control, arbitrary-tool sandboxing, tamper-proof storage, default pre-tool blocking, or security isolation. Treat the docs as planning source material for a future server unless the maintainer handoff status in [MVP Plan](build/mvp-plan.md) says otherwise.

## Current Routes

This entry point routes only to the compact active structure plus the route index.

| Purpose | Route |
|---|---|
| First model | [Start](start.md) |
| User workflow | [User Guide](use/user-guide.md) |
| Agent behavior | [Agent Guide](use/agent-guide.md) |
| User-owned judgment examples | [Judgment Examples](use/judgment-examples.md) |
| Current MVP plan and implementation-readiness decisions | [MVP Plan](build/mvp-plan.md) |
| Exact contract owner index | [Reference Index](reference/README.md) |
| Later candidates | [Later Index](later/index.md) |
| Documentation authoring rules | [Authoring Guide](maintain/authoring-guide.md) |
| Translation and semantic-parity rules | [Translation Guide](maintain/translation-guide.md) |
| Manual documentation checks | [Checks](maintain/checks.md) |
| Stable `doc_id` route metadata | [doc-index.yaml](../doc-index.yaml) |

## How To Read

Start with [Start](start.md), then use [User Guide](use/user-guide.md) or [Agent Guide](use/agent-guide.md) depending on the task. Use [MVP Plan](build/mvp-plan.md) for the current MVP scope and server-coding readiness decisions. Use [Reference Index](reference/README.md) to choose the single owner for exact schemas, API behavior, storage, state transitions, security wording, projection/template rules, conformance meaning, integration behavior, terminology, maintain checks, and translation rules.

For public API response branch questions, including `ToolResultBase`, `ToolRejectedResponse`, `ToolDryRunResponse`, `MethodResult`, `response_kind`, and `effect_kind`, Reference routes shared branches and active value sets to API Schema Core, method-specific response unions and state effects to MVP API, and rejected/blocked/dry-run error boundaries to API Errors.

`ToolDryRunResponse` is not the umbrella response for every `dry_run=true` request. The API owners distinguish valid dry-run previews for selected Core-commit or staging operations from read-only selections such as `harness.status dry_run=true` and `harness.close_task intent=check dry_run=true`, which return `StatusResult` or `CloseTaskResult` with `effect_kind=read_only`.

The Reference Index routes the active owners for the public `ErrorCode` contract and `STATE_VERSION_CONFLICT`, project-wide `project_state.state_version`, request-level `VerifiedSurfaceContext.access_class`, `run_recording`, `artifact_registration`, `artifact_read`, `harness.record_run`, `harness.stage_artifact`, `StagedArtifactHandle` promotion, persistent `existing_artifact` / `ArtifactRef` linking, separate artifact body reads, verified local surface access, `SensitiveActionScope`, product-file `AuthorizedAttemptScope`, `CompletionPolicy`, `EvidenceSummary`, `close_task` blockers, read-only projections, capability profiles, detective guarantee gating, user-owned judgments, and shaping readiness. Use Checks for error-code, access-class, and artifact-lifecycle consistency checks during documentation work.

Use [Later Index](later/index.md) only for material outside the active MVP path. Later candidate material does not become active delivery unless the relevant owner promotes it with scope and proof expectations.

Use [Authoring Guide](maintain/authoring-guide.md), [Translation Guide](maintain/translation-guide.md), and [Checks](maintain/checks.md) for documentation work. Checks are manual maintenance aids; their labels do not create runtime conformance, final acceptance, close readiness, implementation readiness, or permission to start server coding.

## Active MVP Boundary

The active MVP is closed to plain-language intake and Task creation, `harness.update_scope`, user judgment recording, sensitive approval recording, path-level `harness.prepare_write` and Write Authorization, `harness.record_run` with `access_class=run_recording`, artifact staging through `harness.stage_artifact` with `access_class=artifact_registration`, staged artifact promotion after `StagedArtifactHandle` provenance and scope validation, persistent `existing_artifact` / `ArtifactRef` linking, separate artifact body reads with `access_class=artifact_read`, compact `EvidenceSummary`, `harness.close_task` blocker calculation, read-time read-only status/projection output, verified local surface access through a registered surface, cooperative guarantee display, and detective guarantee display only after the relevant capability check has passed.

The active MVP does not include `captured_artifact`, native artifact capture, projection reconcile, persistent projection jobs, managed block drift repair, full Evidence Manifest, `qa_gate`, `verification_gate`, command execution observation, network observation, secret access observation, command/network/secret pre-tool blocking, Question Queue, Assumption Register, or Discovery Brief as a persistent artifact. Those remain later-only through [Later Index](later/index.md) until promoted.

## Quality Rules

Do not finish a meaning-changing documentation edit with only one language updated. Keep review history, cleanup notes, and temporary migration plans out of active docs.

Do not list profile-gated values as default active MVP values, describe later candidates as active requirements, or make unsupported preventive, isolation, sandboxing, tamper-proof, or default tool-blocking security claims.

## Bilingual Parity

English and Korean docs are both active. Major active docs should have paired paths under `docs/en` and `docs/ko`, including the Korean entry at [../ko/README.md](../ko/README.md).

Paired docs must preserve semantic parity, but they do not need line-by-line translation. Korean docs should read as natural Korean technical prose while preserving exact identifiers.

Agents should keep context small, pull owner docs only when needed, and avoid loading paired English/Korean docs for the same `doc_id` in one prompt unless translation or parity review requires comparison.
