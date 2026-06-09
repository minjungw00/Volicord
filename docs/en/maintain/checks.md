# Checks

Use these checks after documentation edits and before major review handoff. They are read-only Markdown documentation checks, not runtime checks.

Use `PASS`, `WARN`, and `FAIL` only as docs-maintenance labels. They help reviewers decide what to inspect next; they do not decide acceptance or readiness.

## 1. What This Checks

These checks look for documentation drift:

- compact route drift, stale route wording, broken links, anchors, and README routes
- bilingual semantic parity problems
- Korean prose that reads like a literal English translation
- Korean negative coordination that reverses blocker meaning
- exact identifier versus explanatory prose confusion
- duplicate strict contracts outside their owner
- active/later boundary drift and active/profile-gated value confusion
- later-only concepts that are not marked later-only, or reference docs that imply later-only features are active MVP requirements
- unsupported security claims that overstate the guarantee level
- `surface_id` wording that treats an identifier as proof of authority, access, binding, or capability
- active artifact-input wording that promotes `captured_artifact`
- user judgment routes that substitute for each other
- `sensitive_approval` / `SensitiveActionScope` wording that collapses into `AuthorizedAttemptScope`
- residual-risk close blocker wording that hides one of several negative requirements
- projection-derived display wording that treats generated views as source authority
- state-clock wording that exposes both task-scoped and project-scoped `state_version` as public conflict clocks
- projection reconcile wording that treats `reconcile` as an active Core state mutation path
- final acceptance or residual-risk acceptance wording that substitutes for missing required evidence
- user-facing documentation or templates that expose internal enum, schema, or error-code terms unnecessarily
- public error-code wording that uses any public state-version conflict code other than `STATE_VERSION_CONFLICT`
- `access_class`, `record_run`, `run_recording`, `artifact_registration`, `stage_artifact`, `existing_artifact`, and staged artifact promotion wording that blurs active MVP contracts
- staged handle provenance or scope validation wording that maps validation failure to `LOCAL_ACCESS_MISMATCH` or `CAPABILITY_INSUFFICIENT` instead of `VALIDATION_FAILED`
- response-branch wording that leaks method-result-only fields into `ToolRejectedResponse` or generated refs and side effects into `dry_run` responses
- `dry_run=true` wording that treats every valid dry-run request as `ToolDryRunResponse` or requires `ToolDryRunResponse` for a read-only selected intent
- mixed intent method wording that chooses response branches by method name instead of the selected intent's state effect
- `harness.close_task intent=check` with `dry_run=true` wording that creates `task_events`, replay rows, close-state mutations, Write Authorization changes, staged-handle consumption, or `state_version` increments
- `close_task` wording that turns preflight `STATE_VERSION_CONFLICT`, stale `WriteAuthorization.basis_state_version`, or `idempotency_key` request-hash conflict into committed close blockers
- one-language-per-`doc_id` agent retrieval problems
- stale rewrite/history notes, closed issue records, and obsolete review prose

## 2. What This Does Not Prove

This page does not prove runtime behavior, runtime conformance, implementation readiness, documentation acceptance, development readiness, final acceptance, close readiness, QA, evidence sufficiency, residual-risk acceptance, or permission to start server coding.

Do not use these checks to create runtime state, `task_events`, generated projections, generated operational artifacts, executable fixtures, conformance reports, QA records, acceptance records, close records, residual-risk records, or product writes.

`PASS` means only that the checked documentation appears internally consistent for that item. `WARN` means a human should review uncertain wording. `FAIL` means docs-maintenance drift was found and should be routed to the owner.

## 3. Compact Route Check

Inspect README files, Maintain docs, route tables, navigation summaries, paired-language links, and retrieval guidance.

Pass when README and Maintain routes point only to:

- `docs/doc-index.yaml`
- `docs/*/start.md`
- `docs/*/use/user-guide.md`
- `docs/*/use/agent-guide.md`
- `docs/*/use/judgment-examples.md`
- `docs/*/build/mvp-plan.md`
- `docs/*/reference/README.md`
- `docs/*/later/index.md`
- `docs/*/maintain/authoring-guide.md`
- `docs/*/maintain/translation-guide.md`
- `docs/*/maintain/checks.md`

Fail when active routing points to deleted files, stale route families, inactive migration records, wrong-language owners, stale structure labels, or deep owner files instead of the compact owner index.

## 4. Link And Anchor Check

Inspect relative Markdown links, paired-language links, owner routes, heading anchors, stale path names, deleted route names, and stale structure labels.

Pass when every active link resolves to a current file and current anchor. Fail when active docs point to a missing file, stale heading, inactive migration record, wrong-language owner, stale route family, or stale structure name.

## 5. Bilingual Semantic Parity Check

Inspect `docs/en` and `docs/ko` for the same active file map, reader purpose, section coverage, owner routing, and exact identifiers.

Pass when paired files preserve the same meaning while Korean remains natural. Fail when a Korean file omits active English meaning, translates an exact identifier, changes an owner route, compresses negative coordination so a blocker condition reverses meaning, or moves active material into later scope or later material into active scope.

## 6. Korean Natural Prose Check

Inspect Korean explanatory prose, headings, examples, and maintain guidance.

Pass when Korean reads as natural Korean technical documentation, separates exact identifiers from explanatory prose, keeps exact identifiers unchanged, and does not leave English noun phrases in Korean prose unless they are exact identifiers or intentional Harness labels. Fail when Korean is a literal line-by-line English translation, preserves English prose where natural Korean explanation is required, treats an explanatory English noun phrase as an identifier, preserves English noun phrases as prose, compresses negative coordination in a meaning-changing way, or changes meaning to follow English sentence order.

## 7. Owner-Boundary Check

Inspect schemas, DDL, enum values, state transitions, gate rules, algorithms, fixture body shapes, template bodies, storage rules, security guarantees, validator IDs, and official definitions.

Pass when each strict contract is defined in one owner and non-owner docs use a short local consequence plus compact owner route. Fail when Start, Use, Build, Maintain, README, or a non-owner Reference summary creates a second normative definition.

## 8. Active/Profile-Gated Value Check

Inspect active schemas, API docs, DDL, Build scope wording, Reference docs, Later docs, later candidates, profile/capability tables, connector modes, artifact-input wording, and examples.

Pass when default active blocks contain only active MVP material, profile-gated values are clearly labeled and owned, later-only concepts are marked later-only at the point of use, and later candidates stay in the Later index or promoted owners. Fail when later enum values, methods, tables, commands, templates, assurance behavior, operations behavior, fixture families, or profile-gated values are presented as default active requirements. Fail when Reference docs imply that later-only features are required for active MVP implementation. Fail when active MVP text describes `captured_artifact` as an active artifact input path.

## 9. Unsupported Security Claim Check

Inspect claims using cooperative, detective, preventive, isolated, guard, freeze, careful-mode, sandbox, permission, blocking, tamper-proof, isolation, `surface_id`, capability, local access, or surface binding language.

Pass when the claim matches the documented guarantee level, `detective` display is backed by passed capability verification for the covered observable scope, and the text names the owner/proof path for preventive or isolated behavior. Fail when cooperative or detective behavior is described as OS permission, arbitrary-tool sandboxing, tamper-proof storage, universal pre-tool blocking, or security isolation without a proven owner path. Fail when a copied `surface_id` is treated as proof of authority, access, binding, or capability. Fail when any text displays or claims a `detective` guarantee without passed capability verification.

## 10. User Judgment Boundary Check

Inspect judgment prompts, examples, close wording, approval wording, final acceptance wording, residual-risk wording, evidence wording, `SensitiveActionScope`, `AuthorizedAttemptScope`, and any later/reserved QA waiver or verification-risk wording.

Pass when product decisions, technical decisions, scope decisions, sensitive-action approval, final acceptance, residual-risk acceptance, cancellation, and later/reserved QA waiver or verification-risk acceptance stay distinct. Pass when `sensitive_approval` / `SensitiveActionScope` stays separate from product-file `AuthorizedAttemptScope` and Write Authorization. Fail when broad approval, sensitive-action approval, final acceptance, later QA waiver, evidence, verification, or residual-risk acceptance silently substitutes for another route. Fail when final acceptance or residual-risk acceptance substitutes for missing required evidence.

## 11. Residual-Risk Close Blocker Wording Check

Inspect residual-risk close blocker text, Korean translations of blocker conditions, and examples that combine visibility, acceptance, waiver, evidence, or required judgment.

Pass when each negative requirement is stated explicitly and residual-risk close blockers preserve the meaning "not visible, or not accepted when required." Korean should use a clear form such as "보이지 않거나, 요구될 때 수락되지 않은 경우." Fail when wording drops the first negative requirement or when residual-risk acceptance substitutes for final acceptance, later QA waiver, evidence, or verification.

## 12. Projection-Derived-Display Check

Inspect projection and template wording, generated-display examples, status cards, summaries, user-facing views, diagrams, projection reconcile wording, and `reconcile` references.

Pass when projections and rendered displays are described as derived views with freshness and owner boundaries, and projection reconcile remains later-only unless an owner has promoted it. Fail when generated displays are treated as source-of-truth records, runtime state, evidence, QA, acceptance, close records, residual-risk records, Write Authorization, or permission to perform product/runtime writes. Fail when active MVP text treats projection reconcile or `reconcile` as a Core state mutation path.

## 13. One-Language-Per-`doc_id` Agent Retrieval Check

Inspect agent guidance, context-loading advice, README routes, Reference routes, and any always-on context examples.

Pass when agent-facing docs retrieve only one language for a given `doc_id` during normal work, load paired languages only for translation or parity review, retrieve only the owner section needed for the next action, and keep always-on context compact. Fail when docs instruct agents to load both languages for the same `doc_id` by default, broad reference sets, full schemas, full templates, historical logs, generated artifacts, or stale migration records.

## 14. State-Version Conflict Clock Check

Inspect `state_version`, `project_state.state_version`, `tasks.state_version`, task-scoped state clocks, project-scoped state clocks, conflict wording, concurrency wording, public `ErrorCode` lists, and public API examples.

Pass when active MVP public conflict wording uses the project-wide `project_state.state_version` basis unless an owner explicitly promotes another clock, and when project-wide mismatch uses the single public `ErrorCode` `STATE_VERSION_CONFLICT`. Fail when active MVP text exposes both task-scoped and project-scoped `state_version` as public conflict clocks, asks clients or agents to choose between them, treats `tasks.state_version` as an active public conflict/concurrency basis, or documents any alternate public code, synonym, deprecated alias, alternate spelling, or storage-layer public error name for that mismatch.

## 15. User-Facing Internal-Term Check

Inspect public user-facing documentation, templates, status cards, examples, close summaries, judgment prompts, Korean renderings, English renderings, error displays, and enum displays.

Pass when user-facing docs and templates use reader-facing display wording and expose exact enum, schema, or error-code terms only when the contract value itself is being explained. Fail when user-facing documentation or templates unnecessarily expose internal terms such as `EvidenceSummary`, `CloseBlocker.category`, `judgment_kind`, `guarantee_level`, raw enum values, schema field names, internal error codes, or internal blocker categories instead of natural display text.

## 16. Active MVP API And Artifact Contract Check

Inspect public `ErrorCode` lists and examples, `access_class` request wording, `record_run`, `stage_artifact`, `run_recording`, `artifact_registration`, staged artifact promotion wording, `existing_artifact`, and staged handle provenance or scope validation failure mapping.

Pass when public error codes follow the API Errors document, active state-version conflict uses only `STATE_VERSION_CONFLICT`, active MVP prose describes one method-level verified `access_class` per request, `record_run` uses `run_recording`, `stage_artifact` uses `artifact_registration`, staged artifact promotion validates handle provenance and scope before linking, staged handle provenance or scope validation failure maps to `VALIDATION_FAILED`, and artifact lifecycle prose separates staging, promotion, persistent artifact linking, and artifact body read.

Fail when any of these conditions appear in active MVP documentation:

- A state-version conflict public error is described with any value other than `STATE_VERSION_CONFLICT`.
- Any alternate public code, alias, deprecated spelling, or storage-layer public error name is documented for project-wide state-version mismatch.
- A single request is described as carrying multiple active `access_class` values, or `access_class` is described as something other than method-level verified request context.
- `artifact_registration` is described as including `record_run`.
- `record_run` is described as requiring both `run_recording` and `artifact_registration`.
- Staged artifact promotion appears allowed by `handle_id` alone, without provenance and scope validation.
- Staged handle provenance or scope validation failure is mapped to `LOCAL_ACCESS_MISMATCH`.
- Staged handle provenance or scope validation failure is mapped to `CAPABILITY_INSUFFICIENT`.
- `existing_artifact` is described as registering new artifact body bytes.

## 17. Response-Branch Shape Check

Inspect public response unions, method-specific result branches, rejected-response prose, `dry_run` prose, examples, representative conformance rows, and smoke-target wording.

Pass when public method responses are written as a method-specific `MethodResult` branch, `ToolDryRunResponse` when the selected operation has a distinct state-effecting dry-run preview branch, or `ToolRejectedResponse`; strictly read-only methods may omit `ToolDryRunResponse` only by explicit contract. For mixed intent methods, branch selection must follow the selected intent's state effect, not the method name alone. A selected read-only operation with `dry_run=true` may return the method-specific `MethodResult` with `base.dry_run=true` and `effect_kind=read_only`; method-specific fields appear only on that method result branch.

Pass when `harness.close_task intent=check` with `dry_run=true` is documented as `CloseTaskResult` with `base.dry_run=true` and `effect_kind=read_only`, and as creating no `task_events`, replay rows, close-state mutations, Write Authorization changes, staged-handle consumption, or `state_version` increments. Pass when `harness.close_task intent=complete`, `intent=cancel`, or `intent=supersede` with `dry_run=true` is documented as `ToolDryRunResponse` when otherwise valid and previewable. Pass when pre-commit failure with `dry_run=true` is documented as `ToolRejectedResponse`.

Valid `ToolDryRunResponse` branches contain only preview data and no generated refs. `ToolRejectedResponse` and `ToolDryRunResponse` have `effect_kind=no_effect`: no replay row, no state-version increment, no staged-handle consumption, and no Write Authorization creation or consumption.

Fail when any of these conditions appear in active MVP documentation:

- `dry_run=true` is described as always returning `ToolDryRunResponse`.
- `ToolDryRunResponse` is required for a selected read-only operation, including `harness.close_task intent=check` with `dry_run=true`.
- A mixed intent method chooses response branches by method name instead of by the selected intent's state effect.
- `harness.close_task intent=check` with `dry_run=true` is described as creating `task_events`, replay rows, close-state mutations, Write Authorization changes, staged-handle consumption, or `state_version` increments.
- `ToolRejectedResponse` is described as requiring method-specific result-only fields such as `decision`, `task_ref`, `run_summary`, `staged_artifact_handle`, `write_authorization_ref`, `user_judgment_ref`, or `close_state`.
- `dry_run`, `ToolDryRunResponse`, or `DryRunSummary` is described as requiring real generated refs, including generated `task_ref`, `run_summary`, `staged_artifact_handle`, `write_authorization_ref`, `user_judgment_ref`, event refs, artifact refs, or authority for records that do not exist.
- `STATE_VERSION_CONFLICT` is described as a `PrepareWriteResult.decision` value instead of a public `ErrorCode` on `ToolRejectedResponse`.
- `StageArtifactResponse` failure is described as requiring `staged_artifact_handle`.
- `RecordRunResponse` rejection is described as requiring `run_summary`.
- `ToolRejectedResponse`, `ToolDryRunResponse`, or read-only `dry_run` `MethodResult` branches are described as creating replay rows, events, `state_version` increments, staged-handle consumption, artifact promotion, or Write Authorization creation or consumption.

## 18. `close_task` Preflight And Blocked-Close Check

Inspect `close_task` prose, response examples, error lists, close blocker matrices, write-compatibility wording, recovery wording, storage effects, smoke examples, and authoring guidance.

Pass when `close_task` defines preflight rejection before the close matrix, preflight rejection returns `ToolRejectedResponse`, and semantic close matrix blockers return `CloseTaskResult(close_state=blocked)` only after a valid matrix evaluation. Pass when `STATE_VERSION_CONFLICT` is documented only as a `ToolRejectedResponse` preflight error, stale `WriteAuthorization.basis_state_version` is preflight rejection before consumption, and `idempotency_key` reuse with a different request hash is preflight rejection that preserves the existing replay row.

Fail when any of these conditions appear in active MVP documentation:

- `STATE_VERSION_CONFLICT` is used as `CloseBlocker.code`.
- `STATE_VERSION_CONFLICT` is described as `CloseTaskResult(close_state=blocked).errors[0]`, the primary error of `CloseTaskResult(close_state=blocked)`, or the primary error for any committed blocked close result.
- `STATE_VERSION_CONFLICT` is described as a committed blocked close result rather than a `ToolRejectedResponse`.
- Stale `WriteAuthorization.basis_state_version` is described as a committed `write_compatibility` blocker.
- `idempotency_key` reuse with a different request hash is described as a `recovery` blocker.
- Close preflight rejection is described as creating a `CloseBlocker`, `task_event`, `task_events` append, replay row, `tool_invocations.response_json`, `close_state` mutation, Write Authorization creation or consumption, staged-handle consumption, artifact promotion or link, evidence update, or `project_state.state_version` increment.
- The same state effects are assigned to `ToolRejectedResponse` and committed blocked `CloseTaskResult`.

## 19. Stale Content Check

Inspect Maintain docs and nearby routes for historical rewrite reviews, closed issue records, obsolete acceptance records, obsolete delivery-label explanations, prior stage label history, obsolete alias history, later-candidate localization audit records, past translation problem records, past audit result narrative, and temporary migration plans.

Pass when Maintain docs contain only living editing rules and current checks. Prior stage label history may remain only as a minimal compatibility rule when a current owner needs it. Fail when obsolete review prose is preserved as active guidance, issue-resolution or audit-result narrative remains, archive copies are created, or scratch migration files remain after the edit.
