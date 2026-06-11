# Later candidate index

This index routes inactive later candidates to their category documents. It is a route list, not a contract owner.

For current scope, see [Active MVP Scope](../reference/active-mvp-scope.md). This page does not define current MVP behavior, implementation tasks, runtime permission, active API contracts, active storage effects, active security guarantees, or permission to start runtime/server coding.

Mention here or in a category file is not promotion. A candidate becomes active only after current scope and the relevant current owner document, or a new owner document created during promotion, are updated in the same documentation-only batch.

Promotion-time owner update means that the needed owner work happens at promotion time. It does not create active requirements before promotion, and a candidate entry is not itself an active owner document.

Profile-gated values are distinct from later candidates. A value is profile-gated only when active scope and its owner document name the profile and supported value set. Values on this page remain later candidates until that promotion happens.

## Candidate routes

### Artifacts and evidence

- [Full `Evidence Manifest`](artifacts-and-evidence.md#full-evidence-manifest): manifest-level evidence records and rendered summaries.
- [Export](artifacts-and-evidence.md#export): export behavior, export artifacts, and redaction boundaries.
- [Export and handoff formats](artifacts-and-evidence.md#export-and-handoff-formats): file formats, bundle contracts, and provenance requirements.
- [Later actor, producer, and capture source values](artifacts-and-evidence.md#later-actor-producer-and-capture-source-values): producer, actor, and capture-source vocabulary.
- [Native artifact capture](artifacts-and-evidence.md#native-artifact-capture): direct artifact capture beyond reference staging.
- [Read-only playbook and judgment context metadata](artifacts-and-evidence.md#read-only-playbook-and-judgment-context-metadata): evidence-review metadata that does not satisfy judgment by itself.
- [Later reference and artifact value families](artifacts-and-evidence.md#later-reference-and-artifact-value-families): bundle, manifest, QA, export, design, Eval, Manual QA, TDD, `Projection`, and related reference values.
- [Later template names](artifacts-and-evidence.md#later-template-names): richer decision, evidence, run, design, and export display names.
- [Browser capture automation](artifacts-and-evidence.md#browser-capture-automation): browser screenshots, recordings, or captured UI state as evidence material.

### Connectors and surfaces

- [Future local operator command family](connectors-and-surfaces.md#future-local-operator-command-family): local command surfaces such as `harness doctor`, `harness export`, and `harness conformance run`.
- [Operator readiness and `doctor` surfaces](connectors-and-surfaces.md#operator-readiness-and-doctor-surfaces): local readiness and diagnostic surfaces.
- [Projection refresh and freshness diagnostics](connectors-and-surfaces.md#projection-refresh-and-freshness-diagnostics): refresh and freshness visibility for projection material.
- [Later read-only resources](connectors-and-surfaces.md#later-read-only-resources): read-only resources such as `policy`, `evidence-manifest`, `surface`, `report`, `bundle`, `journey`, and `design`.
- [Dashboard and hosted workflows](connectors-and-surfaces.md#dashboard-and-hosted-workflows): dashboard, hosted workflow, visualization, card, and artifact dashboard surfaces.
- [Cross-surface verification](connectors-and-surfaces.md#cross-surface-verification): verification visibility across IDE, CLI, chat, MCP, or hosted surfaces.
- [Broader connectors and hosted runtime](connectors-and-surfaces.md#broader-connectors-and-hosted-runtime): connector marketplace, hosted UI, hosted runtime, and remote runtime candidates.
- [Connector conformance ecosystem](connectors-and-surfaces.md#connector-conformance-ecosystem): connector-facing compatibility claims, marketplace signals, and report surfaces.

### Policy and conformance

- [Manual QA workflow and `qa_gate`](policy-and-conformance.md#manual-qa-workflow-and-qa-gate): Manual QA gate policy and close-readiness relationship.
- [Manual QA waiver `qa_waiver`](policy-and-conformance.md#manual-qa-waiver-qa-waiver): waiver route for Manual QA policy without replacing user-owned judgment.
- [Verification gate `verification_gate`](policy-and-conformance.md#verification-gate-verification-gate): verification gate policy and close-readiness relationship.
- [Design gates and policy blockers](policy-and-conformance.md#design-gates-and-policy-blockers): design gates, policy blocker categories, and design-quality policy.
- [Design-policy waiver](policy-and-conformance.md#design-policy-waiver): waiver route for design-policy blockers.
- [Broad design validators and severity-based blocking](policy-and-conformance.md#broad-design-validators-and-severity-based-blocking): validator IDs, severity meanings, and blocking policy.
- [Full design-quality policy families](policy-and-conformance.md#full-design-quality-policy-families): policy families such as `shared_design`, `domain_language`, and `codebase_stewardship`.
- [Future conformance run entrypoint](policy-and-conformance.md#future-conformance-run-entrypoint): executable conformance runner, suite, and reporting contract.
- [Later schema extensions](policy-and-conformance.md#later-schema-extensions): cross-cutting fields, enum values, and validators.
- [`ValidatorResult` stable IDs and policy families](policy-and-conformance.md#validatorresult-stable-ids-and-policy-families): stable validator identity, policy family, severity, and waiver vocabulary.
- [Future fixture families](policy-and-conformance.md#future-fixture-families): executable fixture families, conformance suites, assertions, and report formats.
- [Advanced validators and interface checks](policy-and-conformance.md#advanced-validators-and-interface-checks): advanced validators, design-policy validators, language checks, and interface checks.

### Security and assurance

- [Assurance hardening](security-and-assurance.md#assurance-hardening): stronger evidence, verification, and close-readiness assurance claims.
- [Operations hardening](security-and-assurance.md#operations-hardening): operator diagnostics and stronger security posture for local operation.
- [Stronger local capability profiles](security-and-assurance.md#stronger-local-capability-profiles): profile labels for observation, capture, isolation, or blocking capabilities.
- [Command, network, and secret-access observation](security-and-assurance.md#command-network-and-secret-access-observation): observation of selected command, network, or secret-access intent.
- [Command, network, and secret pre-tool blocking](security-and-assurance.md#command-network-and-secret-pre-tool-blocking): preventive blocking claims before tool execution.
- [Capability-gated `prepare_write` and `record_run` observation](security-and-assurance.md#capability-gated-prepare-write-and-record-run-observation): observation around write preparation and run recording.
- [Capability-profile support fields](security-and-assurance.md#capability-profile-support-fields): support fields for observation, capture, pre-tool blocking, and isolation capabilities.
- [Capability-gated authorization observation fields](security-and-assurance.md#capability-gated-authorization-observation-fields): fields such as `intended_commands`, `intended_network`, `network_write`, and `secret_access`.
- [Later close and assurance fields](security-and-assurance.md#later-close-and-assurance-fields): close, gate, verification, QA, design, and assurance fields.
- [Native hooks and advanced sidecar watcher](security-and-assurance.md#native-hooks-and-advanced-sidecar-watcher): native hook or sidecar watcher claims for broader tool visibility.

### Workflow and collaboration

- [Discovery brief, question queue, and assumption register](workflow-and-collaboration.md#discovery-brief-question-queue-and-assumption-register): shaping records for open questions, assumptions, and task context.
- [Verification-risk acceptance `verification_risk_acceptance`](workflow-and-collaboration.md#verification-risk-acceptance-verification-risk-acceptance): user-judgment route for accepting verification risk.
- [Eval and detached verification workflows](workflow-and-collaboration.md#eval-and-detached-verification-workflows): evaluation and detached verification workflows.
- [Full `Decision Packet` and `presentation=full`](workflow-and-collaboration.md#full-decision-packet-and-presentation-full): full-format decision presentation.
- [Rich risk review and residual-risk lifecycle](workflow-and-collaboration.md#rich-risk-review-and-residual-risk-lifecycle): richer risk review records, residual-risk lifecycle, and expiry behavior.
- [Release handoff](workflow-and-collaboration.md#release-handoff): release handoff workflow without production authority.
- [Recovery and reconcile](workflow-and-collaboration.md#recovery-and-reconcile): recovery, reconcile, and state-repair workflow.
- [Persistent projection jobs](workflow-and-collaboration.md#persistent-projection-jobs): projection job lifecycle and job storage.
- [Projection reconcile and editable projection areas](workflow-and-collaboration.md#projection-reconcile-and-editable-projection-areas): projection reconcile, managed-block repair, and editable projection areas.
- [`harness.next`](workflow-and-collaboration.md#harness-next): next-action API method.
- [`harness.launch_verify`](workflow-and-collaboration.md#harness-launch-verify): verification-launch API method.
- [`harness.record_eval`](workflow-and-collaboration.md#harness-record-eval): evaluation-recording API method.
- [`harness.record_manual_qa`](workflow-and-collaboration.md#harness-record-manual-qa): Manual QA recording API method.
- [Later `harness.record_run` branches](workflow-and-collaboration.md#later-harness-record-run-branches): verification input, feedback-loop update, and TDD trace update branches.
- [Later user-judgment branches](workflow-and-collaboration.md#later-user-judgment-branches): `qa_waiver`, `verification_risk_acceptance`, waiver, reconcile, residual-risk, and richer acceptance branches.
- [Later next-action values](workflow-and-collaboration.md#later-next-action-values): next-action values such as `launch_verify`, `record_eval`, `record_manual_qa`, and `reconcile`.
- [Waiver, reconcile, and residual-risk branches](workflow-and-collaboration.md#waiver-reconcile-and-residual-risk-branches): waiver, reconcile, and residual-risk branches.
- [Verification result cards and richer verification workflows](workflow-and-collaboration.md#verification-result-cards-and-richer-verification-workflows): verification cards and richer verification workflow without substituting for QA.
- [Context index and derived metrics](workflow-and-collaboration.md#context-index-and-derived-metrics): workflow-review context that does not become authority by itself.
- [Team workflows and orchestration](workflow-and-collaboration.md#team-workflows-and-orchestration): team permissions, shared capability sets, orchestration, and parallel-lane behavior.
- [Advanced release and deployment automation](workflow-and-collaboration.md#advanced-release-and-deployment-automation): deployment, canary, rollback, merge, and production-monitoring automation.
