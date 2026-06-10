# Later candidate index

This index routes inactive later candidates to their category documents. It is a summary table, not a contract owner.

For current scope, see [Active MVP Scope](../reference/active-mvp-scope.md).

This page does not define current MVP behavior, implementation tasks, runtime permission, active API contracts, active storage effects, active security guarantees, or permission to start runtime/server coding.

Mention here or in a category file is not promotion. A candidate becomes active only after current scope and the relevant owner documents are updated in the same documentation-only batch.

Profile-gated values are distinct from later candidates. A value is profile-gated only when active scope and its owner document name the profile and supported value set. Values on this page remain later candidates until that promotion happens.

## Candidate summary

| Candidate | Category | Summary | Details link |
|---|---|---|---|
| Assurance hardening | Security and assurance | Stronger evidence, verification, and close-readiness assurance claims beyond the current MVP. | [details](security-and-assurance.md#assurance-hardening) |
| Full `Evidence Manifest` | Artifacts and evidence | Future manifest-level evidence records and rendered summaries. | [details](artifacts-and-evidence.md#full-evidence-manifest) |
| Discovery brief, question queue, and assumption register | Workflow and collaboration | Future shaping records for open questions, assumptions, and task context. | [details](workflow-and-collaboration.md#discovery-brief-question-queue-and-assumption-register) |
| Manual QA workflow and `qa_gate` | Policy and conformance | Future Manual QA gate policy and close-readiness relationship. | [details](policy-and-conformance.md#manual-qa-workflow-and-qa-gate) |
| Manual QA waiver `qa_waiver` | Policy and conformance | Future waiver route for Manual QA policy without replacing user-owned judgment. | [details](policy-and-conformance.md#manual-qa-waiver-qa-waiver) |
| Verification gate `verification_gate` | Policy and conformance | Future verification gate policy and close-readiness relationship. | [details](policy-and-conformance.md#verification-gate-verification-gate) |
| Verification-risk acceptance `verification_risk_acceptance` | Workflow and collaboration | Future user-judgment route for accepting verification risk. | [details](workflow-and-collaboration.md#verification-risk-acceptance-verification-risk-acceptance) |
| Eval and detached verification workflows | Workflow and collaboration | Future evaluation and detached verification workflows. | [details](workflow-and-collaboration.md#eval-and-detached-verification-workflows) |
| Full `Decision Packet` and `presentation=full` | Workflow and collaboration | Future full-format decision presentation. | [details](workflow-and-collaboration.md#full-decision-packet-and-presentation-full) |
| Rich risk review and residual-risk lifecycle | Workflow and collaboration | Future richer risk review records, residual-risk lifecycle, and expiry behavior. | [details](workflow-and-collaboration.md#rich-risk-review-and-residual-risk-lifecycle) |
| Design gates and policy blockers: `design_gate`, `design_policy` | Policy and conformance | Future design gates, policy blocker categories, and design-quality policy. | [details](policy-and-conformance.md#design-gates-and-policy-blockers) |
| Design-policy waiver | Policy and conformance | Future waiver route for design-policy blockers. | [details](policy-and-conformance.md#design-policy-waiver) |
| Broad design validators and severity-based blocking | Policy and conformance | Future validator IDs, severity meanings, and blocking policy. | [details](policy-and-conformance.md#broad-design-validators-and-severity-based-blocking) |
| Full design-quality policy families | Policy and conformance | Future design-quality policy families such as `shared_design`, `domain_language`, and `codebase_stewardship`. | [details](policy-and-conformance.md#full-design-quality-policy-families) |
| Operations hardening | Security and assurance | Future operator diagnostics and stronger security posture for local operation. | [details](security-and-assurance.md#operations-hardening) |
| Future local operator command family | Connectors and surfaces | Future local command surfaces such as `harness doctor`, `harness export`, and `harness conformance run`. | [details](connectors-and-surfaces.md#future-local-operator-command-family) |
| Export | Artifacts and evidence | Future export behavior, export artifacts, and redaction boundaries. | [details](artifacts-and-evidence.md#export) |
| Release handoff | Workflow and collaboration | Future release handoff workflow without production authority. | [details](workflow-and-collaboration.md#release-handoff) |
| Export and handoff formats | Artifacts and evidence | Future file formats, bundle contracts, and provenance requirements for export or handoff. | [details](artifacts-and-evidence.md#export-and-handoff-formats) |
| Recovery and reconcile | Workflow and collaboration | Future recovery, reconcile, and state-repair workflow. | [details](workflow-and-collaboration.md#recovery-and-reconcile) |
| Operator readiness and `doctor` surfaces | Connectors and surfaces | Future local readiness and diagnostic surfaces. | [details](connectors-and-surfaces.md#operator-readiness-and-doctor-surfaces) |
| Projection refresh and freshness diagnostics | Connectors and surfaces | Future refresh and freshness visibility for projection material. | [details](connectors-and-surfaces.md#projection-refresh-and-freshness-diagnostics) |
| Persistent projection jobs | Workflow and collaboration | Future projection job lifecycle and job storage. | [details](workflow-and-collaboration.md#persistent-projection-jobs) |
| Projection reconcile and editable projection areas | Workflow and collaboration | Future projection reconcile, managed-block repair, and editable projection areas. | [details](workflow-and-collaboration.md#projection-reconcile-and-editable-projection-areas) |
| Stronger local capability profiles | Security and assurance | Future profile labels for observation, capture, isolation, or blocking capabilities. | [details](security-and-assurance.md#stronger-local-capability-profiles) |
| Command, network, and secret-access observation | Security and assurance | Future observation of selected command, network, or secret-access intent. | [details](security-and-assurance.md#command-network-and-secret-access-observation) |
| Command, network, and secret pre-tool blocking | Security and assurance | Future preventive blocking claims before tool execution. | [details](security-and-assurance.md#command-network-and-secret-pre-tool-blocking) |
| Future conformance run entrypoint | Policy and conformance | Future executable conformance runner, suite, and reporting contract. | [details](policy-and-conformance.md#future-conformance-run-entrypoint) |
| `harness.next` | Workflow and collaboration | Future next-action API method. | [details](workflow-and-collaboration.md#harness-next) |
| `harness.launch_verify` | Workflow and collaboration | Future verification-launch API method. | [details](workflow-and-collaboration.md#harness-launch-verify) |
| `harness.record_eval` | Workflow and collaboration | Future evaluation-recording API method. | [details](workflow-and-collaboration.md#harness-record-eval) |
| `harness.record_manual_qa` | Workflow and collaboration | Future Manual QA recording API method. | [details](workflow-and-collaboration.md#harness-record-manual-qa) |
| Later read-only resources | Connectors and surfaces | Future read-only resources such as `policy`, `evidence-manifest`, `surface`, `report`, `bundle`, `journey`, and `design`. | [details](connectors-and-surfaces.md#later-read-only-resources) |
| Later `harness.record_run` branches | Workflow and collaboration | Future `harness.record_run` branches for verification input, feedback-loop updates, or TDD trace updates. | [details](workflow-and-collaboration.md#later-harness-record-run-branches) |
| Capability-gated `prepare_write` and `record_run` observation | Security and assurance | Future command, network, or secret-access observation around write preparation and run recording. | [details](security-and-assurance.md#capability-gated-prepare-write-and-record-run-observation) |
| Later user-judgment branches | Workflow and collaboration | Future `qa_waiver`, `verification_risk_acceptance`, waiver, reconcile, residual-risk, and richer acceptance branches. | [details](workflow-and-collaboration.md#later-user-judgment-branches) |
| Later schema extensions | Policy and conformance | Future cross-cutting fields, enum values, and validators. | [details](policy-and-conformance.md#later-schema-extensions) |
| Capability-profile support fields | Security and assurance | Future support fields for observation, capture, pre-tool blocking, and isolation capabilities. | [details](security-and-assurance.md#capability-profile-support-fields) |
| Capability-gated authorization observation fields | Security and assurance | Future fields such as `intended_commands`, `intended_network`, `network_write`, and `secret_access`. | [details](security-and-assurance.md#capability-gated-authorization-observation-fields) |
| Later actor, producer, and capture source values | Artifacts and evidence | Future producer, actor, and capture-source values such as `evaluator`, `operator`, and `capture_adapter`. | [details](artifacts-and-evidence.md#later-actor-producer-and-capture-source-values) |
| Native artifact capture | Artifacts and evidence | Capture artifact data directly rather than only staging references. | [details](artifacts-and-evidence.md#native-artifact-capture) |
| Later close and assurance fields | Security and assurance | Future close, gate, verification, QA, design, and assurance fields. | [details](security-and-assurance.md#later-close-and-assurance-fields) |
| Later next-action values | Workflow and collaboration | Future next-action values such as `launch_verify`, `record_eval`, `record_manual_qa`, and `reconcile`. | [details](workflow-and-collaboration.md#later-next-action-values) |
| Read-only playbook and judgment context metadata | Artifacts and evidence | Future read-only metadata that can support evidence review without satisfying judgment by itself. | [details](artifacts-and-evidence.md#read-only-playbook-and-judgment-context-metadata) |
| Later reference and artifact value families | Artifacts and evidence | Future value families for bundles, manifests, QA capture, export components, design, Eval, Manual QA, TDD, `Projection`, and related references. | [details](artifacts-and-evidence.md#later-reference-and-artifact-value-families) |
| `ValidatorResult` stable IDs and policy families | Policy and conformance | Future stable validator identity, policy family, severity, and waiver vocabulary. | [details](policy-and-conformance.md#validatorresult-stable-ids-and-policy-families) |
| Waiver, reconcile, and residual-risk branches | Workflow and collaboration | Future waiver, reconcile, and residual-risk branches. | [details](workflow-and-collaboration.md#waiver-reconcile-and-residual-risk-branches) |
| Later template names | Artifacts and evidence | Future template names for richer decision, evidence, run, design, and export displays. | [details](artifacts-and-evidence.md#later-template-names) |
| Future fixture families | Policy and conformance | Future executable fixture families, conformance suites, assertions, and report formats. | [details](policy-and-conformance.md#future-fixture-families) |
| Dashboard and hosted workflows | Connectors and surfaces | Future dashboard, hosted workflow, visualization, card, and artifact dashboard surfaces. | [details](connectors-and-surfaces.md#dashboard-and-hosted-workflows) |
| Verification result cards and richer verification workflows | Workflow and collaboration | Future verification cards and richer verification workflow without substituting for QA. | [details](workflow-and-collaboration.md#verification-result-cards-and-richer-verification-workflows) |
| Browser capture automation | Artifacts and evidence | Future browser screenshots, recordings, or captured UI state as evidence material. | [details](artifacts-and-evidence.md#browser-capture-automation) |
| Cross-surface verification | Connectors and surfaces | Future verification visibility across IDE, CLI, chat, MCP, or hosted surfaces. | [details](connectors-and-surfaces.md#cross-surface-verification) |
| Broader connectors and hosted runtime | Connectors and surfaces | Future connector marketplace, hosted UI, hosted runtime, and remote runtime candidates. | [details](connectors-and-surfaces.md#broader-connectors-and-hosted-runtime) |
| Connector conformance ecosystem | Connectors and surfaces | Future connector-facing compatibility claims, marketplace signals, and report surfaces. | [details](connectors-and-surfaces.md#connector-conformance-ecosystem) |
| Native hooks and advanced sidecar watcher | Security and assurance | Future native hook or sidecar watcher claims for broader tool visibility. | [details](security-and-assurance.md#native-hooks-and-advanced-sidecar-watcher) |
| Context index and derived metrics | Workflow and collaboration | Future context indexing and derived metrics that support workflow review without becoming authority by themselves. | [details](workflow-and-collaboration.md#context-index-and-derived-metrics) |
| Team workflows and orchestration | Workflow and collaboration | Future team permissions, shared capability sets, orchestration, and parallel-lane behavior. | [details](workflow-and-collaboration.md#team-workflows-and-orchestration) |
| Advanced release and deployment automation | Workflow and collaboration | Future deployment, canary, rollback, merge, and production-monitoring automation. | [details](workflow-and-collaboration.md#advanced-release-and-deployment-automation) |
| Advanced validators and interface checks | Policy and conformance | Future advanced validators, design-policy validators, language checks, and interface checks. | [details](policy-and-conformance.md#advanced-validators-and-interface-checks) |
