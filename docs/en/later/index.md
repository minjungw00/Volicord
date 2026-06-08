# Later Candidate Index

This page is the single current documentation owner for later candidates and promotion boundaries. It owns compact classification of values, API names, schema names, gates, validators, workflows, profiles, conformance ecosystems, and export/handoff formats removed from or excluded from the current active MVP.

Later candidates have no current active behavior. They do not affect current close readiness and do not create active API methods, enum values, storage tables, validators, or gates until promoted by an owner document.

Rows here are planning candidates only. They are not active MVP requirements, active API or schema contracts, fixture bodies, template bodies, runtime behavior, implementation tasks, generated artifacts, acceptance evidence, or permission to start runtime work. A candidate remains inert until explicitly promoted by an owner document.

Promotion must require explicit owner-document changes, not just mention in this `later/index.md`. Until that owner change, a name in this index creates no active behavior, active API method, schema field or enum value, storage table or record, gate, validator, report, template, fixture, connector behavior, generated artifact, close effect, guarantee claim, or implementation task. Names that look like methods, enum values, fields, validators, gates, templates, or commands are still inert until the promoted owner updates the active owner contract.

## 1. Boundary

For contrast, the active MVP boundary is closed in [MVP Plan](../build/mvp-plan.md). It includes only plain-language intake and Task creation, `update_scope`, user judgment recording, sensitive approval recording, path-level `prepare_write` and Write Authorization, `record_run`, staged artifact registration through `harness.stage_artifact` with `ArtifactInput.source_kind=staged_artifact`, `EvidenceSummary`, `close_task` blocker calculation, read-time status/projection, registered local surface access, cooperative guarantees, and detective guarantees only after the relevant active capability check has actually passed. Nothing on this page expands that list.

| Candidate | Status | Promotion boundary | Active MVP impact |
|---|---|---|---|
| Later candidate index | documentation only | A future owner must promote a narrow candidate before any detailed contract returns. | documentation only |
| Current repository phase | documentation-only planning | Documentation acceptance and a separate implementation-readiness decision in `docs/*/build/mvp-plan.md` before runtime work. | none |
| Candidate authority | names only | Owner assignment plus exact API, schema, storage, security, conformance, or evidence effects in the promoted owner document. | none until promoted |
| No current active behavior | required boundary | Later candidates do not affect current close readiness and create no active API methods, enum values, storage tables, validators, gates, or runtime behavior until promoted by an owner document. | none |
| Bilingual parity | paired active docs | Same-batch semantic updates for English and Korean. | documentation only |

## 2. Promotion Rule

| Candidate | Status | Promotion boundary | Active MVP impact |
|---|---|---|---|
| Owner assignment | required before promotion | Named owner, narrow scope, non-goals, and fallback behavior. | none |
| Explicit owner-document change | required before promotion | Mention in this index is not promotion; the owner document must update the active method, enum value, storage table, validator, gate, workflow, profile, or format contract before it exists as active behavior. | none until promoted |
| Contract placement | index boundary only | Exact API, schema, storage, projection, template, fixture, or operator contract in the owning active document. | none until promoted |
| No active behavior before promotion | required boundary | The promoted owner document must name scope, fallback behavior, and proof expectations before a candidate affects runtime behavior, API/schema values, storage, close, templates, fixtures, reports, connector behavior, or guarantee display. API/schema promotions must update the active Schema Core owner instead of relying on this index. | none until promoted |
| Active value-set ownership | active owner boundary | Current active method-name and schema enum value sets live in `docs/*/reference/api/schema-core.md`; later names listed here do not extend those sets. | none |
| Security wording | no active guarantee claim here | Honest cooperative, detective, preventive, or isolated wording matched to a proven mechanism. | none until promoted |
| Future proof-path expectation | listing is not current runtime proof | Conformance target, fixture, evidence expectation, or other owner-defined proof path for the promoted behavior. | none until promoted |
| Active-scope inheritance | disabled by default | Future owner proof that promotion does not add unsupported requirements to the active MVP or earlier smoke target. | must not affect active MVP |
| Non-substitution | required boundary | Core state, user judgment, evidence, verification, Manual QA, final acceptance, residual-risk acceptance, and close readiness stay separate. | none |

## 3. Assurance Candidates

| Candidate | Status | Promotion boundary | Active MVP impact |
|---|---|---|---|
| assurance hardening | later candidate | Owner-scoped gates, fallback behavior, and proof-path expectations for future promotion. | none until promoted |
| Full Evidence Manifest | later candidate | Evidence owner rules for artifact refs, redaction, close impact, and proof-path expectations for future promotion. | none until promoted |
| Discovery Brief as a persistent artifact, Question Queue, and Assumption Register | later shaping candidates | Core/API/storage owner rules for exact scope, persistence, non-substitution, close impact, and proof-path expectations. Active MVP shaping stays inside Task, Change Unit, `user_judgment`, evidence summary, blockers, and next safe action. | none until promoted |
| Manual QA workflow and `qa_gate` | later candidate | Manual QA owner policy for workflow steps, waivers, artifact refs, findings, exact `qa_gate` activation, and QA gate close impact. | none until promoted |
| Manual QA waiver `qa_waiver` | later user-judgment candidate | Manual QA and user-judgment owner rules for exact `qa_waiver` activation, allowed scope, non-substitution, residual-risk visibility, and close impact. | none until promoted |
| verification gate `verification_gate` | later candidate | Core/API/Eval owner rules for exact `verification_gate` fields, requiredness, fallback behavior, proof expectations, and close impact. | none until promoted |
| verification-risk acceptance `verification_risk_acceptance` | later user-judgment candidate | Verification and user-judgment owner rules for exact `verification_risk_acceptance` activation, allowed risk scope, non-substitution, and close impact. | none until promoted |
| Eval / detached verification / evaluation workflows | later candidate | Eval owner rules for independence, baseline freshness, artifact integrity, workflow effects, and assurance updates. | none until promoted |
| Full Decision Packet format and `presentation=full` | later candidate | User-judgment owner activation of `presentation=full` and the full Decision Packet format without making either the default MVP path. | none until promoted |
| Rich risk review and residual-risk lifecycle | later candidate | Core and user-judgment owner rules for rich risk records, review workflow, expiry, and close impact. Compact residual-risk visibility remains active only through the Core/API owners. | none until promoted |
| design gate and close category names: `design_gate`, `design_policy` | names only | Core/API/design-quality owner rules for exact fields, category values, fallback behavior, close non-substitution, and proof-path expectations before promotion. | none until promoted |
| Design-policy waiver | later waiver candidate | Core, user-judgment, QA/verification, and design-quality owner rules for allowed scope, non-substitution, residual-risk visibility, and exact recording behavior. | none until promoted |
| Broad design validators, design-policy validators, and severity-based blocking policy | later candidate | Validator and design-quality owner rules for exact IDs, severity meaning, close impact, fallback behavior, waiver boundary, and fixture proof expectation. | none until promoted |
| Full design-quality policy families: full `shared_design` policy, `domain_language`, `vertical_slice`, `feedback_loop`, `tdd_trace`, `deep_module_interface`, `codebase_stewardship`, detailed `manual_qa`, `two_stage_review_display`, detached-verification policy, steward policies | names only | Design-quality owner rules for exact scope, policy boundaries, evidence expectations, and proof-path expectations for future promotion. | none until promoted |

<a id="operations-candidates"></a>
## 4. Operations Candidates

| Candidate | Status | Promotion boundary | Active MVP impact |
|---|---|---|---|
| operations hardening | later candidate | Operations owner rules for commands, diagnostics, fallback behavior, security wording, and proof-path expectations for future promotion. | none until promoted |
| Future local operator command family: `harness connect`, `harness serve mcp`, `harness doctor`, `harness projection refresh`, `harness reconcile`, `harness recover`, `harness export`, `harness artifacts check`, `harness conformance run` | command names only | Operations owner rules for exact syntax, security posture, API/storage effects, reporting, fallback behavior, and proof-path expectations for future promotion. | none until promoted |
| Export | later candidate | Export owner contracts for storage/artifact handling, redaction, omissions, integrity, and future non-leakage proof expectation. | none until promoted |
| Release Handoff | later candidate | Handoff owner rules that keep deployment, merge, rollback, and production authority external unless separately promoted. | none until promoted |
| Export/handoff formats | later candidate | Export/Handoff owner rules for file formats, redaction, omissions, integrity, provenance, fallback behavior, and proof-path expectations. | none until promoted |
| Recovery and reconcile | later candidate | Operations, Storage, Projection, Reconcile, and Security owner rules. | none until promoted |
| Operator readiness and `doctor` surfaces | later candidate | Operations owner rules for diagnostics, capability checks, security posture, and unsupported-surface fallback. | none until promoted |
| Projection refresh and freshness diagnostics | later candidate | Projection owner behavior that keeps projections non-authoritative. | none until promoted |
| Persistent projection jobs and projection job storage | later candidate | Projection and Storage owners must define job lifecycle, storage rows, freshness, failure behavior, and proof expectations. Active MVP uses read-time compact status/projection only. | none until promoted |
| Projection reconcile and managed block drift repair | later candidate | Projection, Core, API, and Storage owners must define editable input handling, reconcile outcomes, repair candidates, state-change routing, non-substitution, and proof expectations. Human-edited projections are not active state. | none until promoted |
| Stronger local capability profiles: preventive profile, isolated profile, command observation, network observation, secret access observation, native artifact capture, pre-tool blocking, or isolation | later candidate | Agent Integration, Security, API, Storage, and Conformance owner rules for exact capability fields, covered operations, fallback behavior, errors, and proof paths. | none until promoted |
| Command execution observation, network observation, and secret access observation | later capability candidates | API, Core, Security, Agent Integration, and Conformance owners must define exact request fields, observation authority, fallback behavior, public errors, storage impact, and proof expectations. | none until promoted |
| Command/network/secret pre-tool blocking | later preventive candidate | A future preventive owner must define the exact blocking mechanism, covered operations, fallback behavior, user-visible guarantee wording, and proof path. | none until promoted |
| Future conformance run entrypoint | later candidate after runtime fixtures exist | Runner, suite, assertion, API/storage/event/artifact/error, and reporting contracts. | none until promoted |

## 5. Later API Candidates

| Candidate | Status | Promotion boundary | Active MVP impact |
|---|---|---|---|
| `harness.next` | method name only | Owner activation for a separate next-action payload; MVP keeps using `harness.status.next_actions`. | none until promoted |
| `harness.launch_verify` | method name only | Eval/verification owner rules for capability handling, baseline freshness, and honest isolation wording. | none until promoted |
| `harness.record_eval` | method name only | Eval owner contract for independence validation, artifact refs, and gate/assurance updates. | none until promoted |
| `harness.record_manual_qa` | method name only | Manual QA owner contract for waiver route, artifacts, findings, and gate impact. | none until promoted |
| Later read-only resources: policy, evidence-manifest, surface, report, bundle, journey, design | resource names only | Resource-specific owner contracts and no mutation side effects. | none until promoted |
| Later `harness.record_run` branches: verification input, feedback-loop updates, TDD trace updates | branch names only | `record_run` owner activation and one-branch payload rules. | none until promoted |
| Capability-gated `prepare_write` / `record_run` observation for command observation, network observation, and secret access observation | later candidate | API, Core, Security, Agent Integration, and Conformance owner rules for exact request fields, compatibility checks, validator behavior, public errors, storage impact, and proof expectations. | none until promoted |
| Later user-judgment branches: `qa_waiver`, `verification_risk_acceptance`, waiver, reconcile, residual-risk, richer acceptance visibility | branch names only | User-judgment owner activation with non-substitution rules. | none until promoted |

<a id="later-schema-candidates"></a>
## 6. Later Schema Candidates

| Candidate | Status | Promotion boundary | Active MVP impact |
|---|---|---|---|
| later schema extensions | schema names only | Promoted owner defines exact fields and validators in the active owner contract. | none until promoted |
| Capability-profile support fields: `command_observation_supported`, `network_observation_supported`, `secret_access_observation_supported`, `artifact_capture_supported`, `pre_tool_blocking_supported`, `isolation_supported` | field names only | Promoted Agent Integration, API/schema, Security, Storage, and Conformance owners define exact profile shape, covered operations, fallback behavior, validation, storage, errors, and proof expectations. Baseline `reference-local-mcp` omits these fields from the active profile and treats the capabilities as unsupported. | none until promoted |
| Capability-gated authorization observation fields: `intended_commands`, `intended_network`, `intended_secret_scope`; command observation, network observation, and secret access observation category names: `network_write`, `external_service_write`, `secret_access` | field and value names only | Promoted API/schema owner defines exact shapes, profile gates, validation, storage, and `record_run` compatibility semantics. Baseline `reference-local-mcp` does not include these fields or values in active `AuthorizedAttemptScope` or active `SensitiveCategory`. | none until promoted |
| Later actor, producer, and capture source values: `evaluator`, `operator`, `capture_adapter` | value names only | Promoted Eval, operations, capture, API/schema, and storage owners define exact request authority, artifact relation, fallback behavior, and proof expectations. Baseline current MVP active tables do not include these values. | none until promoted |
| `captured_artifact` and captured artifact handles | value names only | Promoted native-capture/API/schema/storage owners define exact handle source, capability profile gate, validation, storage, redaction, fallback behavior, and proof expectations. Active MVP uses `harness.stage_artifact` with `source_kind=staged_artifact` for new artifact bytes, plus `existing_artifact` refs for already registered artifacts. | none until promoted |
| Later close and assurance fields: `verifying`, `qa`, `completed_verified`, `detached_verified`, `design_gate`, `verification_gate`, `qa_gate`, Manual QA gate, design-policy blockers, assurance blockers | field names only | Core/API owner activation with close non-substitution rules, exact active schema fields, fallback behavior, and proof expectations. | none until promoted |
| Later next-action values: `launch_verify`, `record_eval`, `record_manual_qa`, `reconcile` | value names only | Matching API or owner activation. | none until promoted |
| Recommended playbooks and judgment context | metadata names only | Agent Integration/API owner rules that keep metadata read-only and non-satisfying. | none until promoted |
| Later ref and artifact values: bundle, manifest, QA capture, export component, design, Eval, Manual QA, TDD, projection, related refs | value names only | ArtifactRef, StateRecordRef, Storage, and relevant owner activation. | none until promoted |
| ValidatorResult later stable IDs and policy families: design, design-policy, autonomy, feedback-loop, TDD, stewardship, residual-risk, shared-design, manual-QA, context-hygiene checks | ID and family names only | Validator owner contracts for stable IDs, severity, waiver boundary, close impact, and future fixture proof expectation. | none until promoted |
| Waiver, reconcile, and residual-risk branches | branch names only | User-judgment, Core, and close owner rules. | none until promoted |

<a id="later-template-candidates"></a>
## 7. Later Template Candidates

| Candidate | Status | Promotion boundary | Active MVP impact |
|---|---|---|---|
| Decision Packet full-format presentation (`DEC`), `APR`, Approval Card, `RUN-SUMMARY`, `EVIDENCE-MANIFEST`, `EVAL`, Verification Result Card, `MANUAL-QA`, Manual QA Card, `TASK`, `DIRECT-RESULT`, `JOURNEY-CARD`, `DESIGN`, `DOMAIN-LANGUAGE`, `MODULE-MAP`, `INTERFACE-CONTRACT`, `TDD-TRACE`, `EXPORT` | template names only | Template owner assignment, source records, fallback behavior, non-substitution rules, freshness behavior, and proof-path expectations for future promotion. | none until promoted |

<a id="future-fixture-families"></a>
## 8. Future Fixture Families

The long row below preserves future fixture family names only. It is not a current MVP requirement or an executable conformance suite.

| Candidate | Status | Promotion boundary | Active MVP impact |
|---|---|---|---|
| Intake and decision routing; Core, evidence, verification, and close; Artifact redaction and export non-leakage; Agency and user-judgment separation; Connector capability honesty; Design-quality and stewardship; Context hygiene and resume freshness; Projection, reconcile, and verification boundary; Operations diagnostics, export, recover, and handoff; Browser QA Capture | fixture family names only | Conformance owner assignment, exact fixture shape, assertions, payload/API/storage/error effects, and proof-path expectations for future promotion. | none until promoted |

## 9. Broad Future Candidates

| Candidate | Status | Promotion boundary | Active MVP impact |
|---|---|---|---|
| Dashboard, hosted workflows, artifact dashboard, richer cards, richer visualizations | later candidate | Derived-display owner rules for read-only, non-authoritative behavior. | none until promoted |
| Verification Result Cards and richer verification workflows | later candidate | Projection/template, Core/API, Eval, and Manual QA owner rules for source records, freshness, non-substitution, fallback behavior, QA boundaries, and proof-path expectations. | none until promoted |
| Browser capture automation | later candidate | Capture owner rules for redaction/PII, retention, fallback behavior, and QA/acceptance non-substitution. | none until promoted |
| Cross-surface verification | later candidate | Core/Eval owner rules for return records, independence, and unsupported-surface fallback. | none until promoted |
| Broader connectors, connector marketplace, hosted UI, hosted/remote runtime | later candidate | Connector/API/security owners and future local-authority boundary proof expectation. | none until promoted |
| Connector conformance ecosystem | later candidate | Connector, API, Security, and Conformance owner rules for capability claims, connector assertions, suite/report formats, marketplace claims, and proof expectations. | none until promoted |
| Native hooks, preventive guard expansion, advanced sidecar watcher | later candidate | Owner-proven covered mechanism before any preventive, isolation, or arbitrary-tool-control claim. | none until promoted |
| Context Index, local derived metrics, long-term metrics | later candidate | Read-only retrieval/diagnostic owners and no authority or close effect. | none until promoted |
| Team workflows, permissions, shared capability sets, orchestration, parallel lanes | later candidate | Scope, authority, permission, and user-owned judgment owners. | none until promoted |
| Advanced exports, release/deployment/canary/rollback/merge/production-monitoring automation | later candidate | Separate owner scope; deployment and production authority remain external unless explicitly promoted. | none until promoted |
| Advanced validators, design-policy validators, and language or interface checks | later candidate | Validator owner rules for exact IDs, severity, waiver boundary, close impact, and fixture behavior. | none until promoted |
