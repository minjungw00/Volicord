# Later Candidate Index

This is the single active index for later candidates and promotion boundaries.

Rows here are planning candidates only. They are not active MVP requirements, active API or schema contracts, fixture bodies, template bodies, runtime behavior, implementation tasks, generated artifacts, acceptance evidence, or permission to start runtime work. A candidate remains inert until explicitly promoted by an owner document.

Until that promotion, a name in this index creates no active behavior, API or schema value, storage record, gate, validator, report, template, fixture, connector behavior, generated artifact, close effect, guarantee claim, or implementation task.

## 1. Boundary

| Candidate | Status | Promotion requires | Active MVP impact |
|---|---|---|---|
| Later candidate index | documentation only | A future owner must promote a narrow candidate before any detailed contract returns. | documentation only |
| Current repository phase | documentation-only planning | Documentation acceptance and a separate implementation-readiness decision in `docs/*/build/mvp-plan.md` before runtime work. | none |
| Candidate authority | names only | Owner assignment plus exact API, schema, storage, security, conformance, or evidence effects in the promoted owner document. | none until promoted |
| Bilingual parity | paired active docs | Same-batch semantic updates for English and Korean. | documentation only |

## 2. Promotion Rule

| Candidate | Status | Promotion requires | Active MVP impact |
|---|---|---|---|
| Owner assignment | required before promotion | Named owner, narrow scope, non-goals, and fallback behavior. | none |
| Contract placement | index boundary only | Exact API, schema, storage, projection, template, fixture, or operator contract in the owning active document. | none until promoted |
| No active behavior before promotion | required boundary | The promoted owner document must name scope, fallback behavior, and proof expectations before a candidate affects runtime behavior, API/schema values, storage, close, templates, fixtures, reports, connector behavior, or guarantee display. | none until promoted |
| Active value-set ownership | active owner boundary | Current active method-name and schema enum value sets live in `docs/*/reference/api/schema-core.md`; later names listed here do not extend those sets. | none |
| Security wording | no active guarantee claim here | Honest cooperative, detective, preventive, or isolated wording matched to a proven mechanism. | none until promoted |
| Future proof-path expectation | listing is not current runtime proof | Conformance target, fixture, evidence expectation, or other owner-defined proof path for the promoted behavior. | none until promoted |
| Active-scope inheritance | disabled by default | Future owner proof that promotion does not add unsupported requirements to the active MVP or earlier smoke target. | must not affect active MVP |
| Non-substitution | required boundary | Core state, user judgment, evidence, verification, Manual QA, final acceptance, residual-risk acceptance, and close readiness stay separate. | none |

## 3. Assurance Candidates

| Candidate | Status | Promotion requires | Active MVP impact |
|---|---|---|---|
| assurance hardening | later candidate | Owner-scoped gates, fallback behavior, and proof-path expectations for future promotion. | none until promoted |
| Full Evidence Manifest | later candidate | Evidence owner rules for artifact refs, redaction, close impact, and proof-path expectations for future promotion. | none until promoted |
| Manual QA | later candidate | Manual QA owner policy for waivers, artifact refs, findings, and QA gate impact. | none until promoted |
| QA waiver | later user-judgment candidate | Manual QA and user-judgment owner rules for exact `qa_waiver` activation, allowed scope, non-substitution, residual-risk visibility, and close impact. | none until promoted |
| verification gate | later candidate | Core/API/Eval owner rules for exact `verification_gate` fields, requiredness, fallback behavior, proof expectations, and close impact. | none until promoted |
| verification-risk acceptance | later user-judgment candidate | Verification and user-judgment owner rules for exact `verification_risk_acceptance` activation, allowed risk scope, non-substitution, and close impact. | none until promoted |
| Eval / detached verification / evaluation workflows | later candidate | Eval owner rules for independence, baseline freshness, artifact integrity, workflow effects, and assurance updates. | none until promoted |
| Full Decision Packet format and `presentation=full` | later candidate | User-judgment owner activation of `presentation=full` and the full Decision Packet format without making either the default MVP path. | none until promoted |
| Rich risk review and residual-risk lifecycle | later candidate | Core and user-judgment owner rules for rich risk records, review workflow, expiry, and close impact. Compact residual-risk visibility remains active only through the Core/API owners. | none until promoted |
| Design policy gate and close category names: `design_gate`, `design_policy` | names only | Core/API/design-quality owner rules for exact fields, category values, fallback behavior, close non-substitution, and proof-path expectations before promotion. | none until promoted |
| Design-policy waiver | later waiver candidate | Core, user-judgment, QA/verification, and design-quality owner rules for allowed scope, non-substitution, residual-risk visibility, and exact recording behavior. | none until promoted |
| Broad design-policy validators and severity-based blocking policy | later candidate | Validator and design-quality owner rules for exact IDs, severity meaning, close impact, fallback behavior, waiver boundary, and fixture proof expectation. | none until promoted |
| Full design-quality policy families: full `shared_design` policy, `domain_language`, `vertical_slice`, `feedback_loop`, `tdd_trace`, `deep_module_interface`, `codebase_stewardship`, detailed `manual_qa`, `two_stage_review_display`, detached-verification policy, steward policies | names only | Design-quality owner rules for exact scope, policy boundaries, evidence expectations, and proof-path expectations for future promotion. | none until promoted |

<a id="operations-candidates"></a>
## 4. Operations Candidates

| Candidate | Status | Promotion requires | Active MVP impact |
|---|---|---|---|
| operations hardening | later candidate | Operations owner rules for commands, diagnostics, fallback behavior, security wording, and proof-path expectations for future promotion. | none until promoted |
| Future local operator command family: `harness connect`, `harness serve mcp`, `harness doctor`, `harness projection refresh`, `harness reconcile`, `harness recover`, `harness export`, `harness artifacts check`, `harness conformance run` | command names only | Operations owner rules for exact syntax, security posture, API/storage effects, reporting, fallback behavior, and proof-path expectations for future promotion. | none until promoted |
| Export | later candidate | Export owner contracts for storage/artifact handling, redaction, omissions, integrity, and future non-leakage proof expectation. | none until promoted |
| Release Handoff | later candidate | Handoff owner rules that keep deployment, merge, rollback, and production authority external unless separately promoted. | none until promoted |
| Export and handoff formats | later candidate | Export/Handoff owner rules for file formats, redaction, omissions, integrity, provenance, fallback behavior, and proof-path expectations. | none until promoted |
| Recovery and reconcile | later candidate | Operations, Storage, Projection, Reconcile, and Security owner rules. | none until promoted |
| Operator readiness and `doctor` surfaces | later candidate | Operations owner rules for diagnostics, capability checks, security posture, and unsupported-surface fallback. | none until promoted |
| Projection refresh and freshness diagnostics | later candidate | Projection owner behavior that keeps projections non-authoritative. | none until promoted |
| Stronger `preventive` / `isolated` local capability profiles for command observation, network observation, secret-access observation, native artifact capture, pre-tool blocking, or isolation | later candidate | Agent Integration, Security, API, Storage, and Conformance owner rules for exact capability fields, covered operations, fallback behavior, errors, and proof paths. | none until promoted |
| Future conformance run entrypoint | later candidate after runtime fixtures exist | Runner, suite, assertion, API/storage/event/artifact/error, and reporting contracts. | none until promoted |

## 5. Later API Candidates

| Candidate | Status | Promotion requires | Active MVP impact |
|---|---|---|---|
| `harness.next` | method name only | Owner activation for a separate next-action payload; MVP keeps using `harness.status.next_actions`. | none until promoted |
| `harness.launch_verify` | method name only | Eval/verification owner rules for capability handling, baseline freshness, and honest isolation wording. | none until promoted |
| `harness.record_eval` | method name only | Eval owner contract for independence validation, artifact refs, and gate/assurance updates. | none until promoted |
| `harness.record_manual_qa` | method name only | Manual QA owner contract for waiver route, artifacts, findings, and gate impact. | none until promoted |
| Later read-only resources: policy, evidence-manifest, surface, report, bundle, journey, design | resource names only | Resource-specific owner contracts and no mutation side effects. | none until promoted |
| Later `harness.record_run` branches: verification input, feedback-loop updates, TDD trace updates | branch names only | `record_run` owner activation and one-branch payload rules. | none until promoted |
| Capability-gated `prepare_write` / `record_run` observation for commands, network effects, and secret access | later candidate | API, Core, Security, Agent Integration, and Conformance owner rules for exact request fields, compatibility checks, validator behavior, public errors, storage impact, and proof expectations. | none until promoted |
| Later user-judgment branches: `qa_waiver`, `verification_risk_acceptance`, waiver, reconcile, residual-risk, richer acceptance visibility | branch names only | User-judgment owner activation with non-substitution rules. | none until promoted |

<a id="later-schema-candidates"></a>
## 6. Later Schema Candidates

| Candidate | Status | Promotion requires | Active MVP impact |
|---|---|---|---|
| later schema extensions | schema names only | Promoted owner defines exact fields and validators in the active owner contract. | none until promoted |
| Capability-gated authorization observation fields: `intended_commands`, `intended_network`, `intended_secret_scope` | field names only | Promoted API/schema owner defines exact shapes, profile gates, validation, storage, and `record_run` compatibility semantics. Baseline `reference-local-mcp` does not include these fields in active `AuthorizedAttemptScope`. | none until promoted |
| Later close and assurance fields: `verifying`, `qa`, `completed_verified`, `detached_verified`, `design_gate`, `verification_gate`, `qa_gate`, Manual QA gate, design-policy blockers, assurance blockers | field names only | Core/API owner activation with close non-substitution rules, exact active schema fields, fallback behavior, and proof expectations. | none until promoted |
| Later next-action values: `launch_verify`, `record_eval`, `record_manual_qa`, `reconcile` | value names only | Matching API or owner activation. | none until promoted |
| Recommended playbooks and judgment context | metadata names only | Agent Integration/API owner rules that keep metadata read-only and non-satisfying. | none until promoted |
| Later ref and artifact values: bundle, manifest, QA capture, export component, design, Eval, Manual QA, TDD, projection, related refs | value names only | ArtifactRef, StateRecordRef, Storage, and relevant owner activation. | none until promoted |
| ValidatorResult later stable IDs and policy families: design, design-policy, autonomy, feedback-loop, TDD, stewardship, residual-risk, shared-design, manual-QA, context-hygiene checks | ID and family names only | Validator owner contracts for stable IDs, severity, waiver boundary, close impact, and future fixture proof expectation. | none until promoted |
| Waiver, reconcile, and residual-risk branches | branch names only | User-judgment, Core, and close owner rules. | none until promoted |

<a id="later-template-candidates"></a>
## 7. Later Template Candidates

| Candidate | Status | Promotion requires | Active MVP impact |
|---|---|---|---|
| Decision Packet full-format presentation (`DEC`), `APR`, Approval Card, `RUN-SUMMARY`, `EVIDENCE-MANIFEST`, `EVAL`, Verification Result Card, `MANUAL-QA`, Manual QA Card, `TASK`, `DIRECT-RESULT`, `JOURNEY-CARD`, `DESIGN`, `DOMAIN-LANGUAGE`, `MODULE-MAP`, `INTERFACE-CONTRACT`, `TDD-TRACE`, `EXPORT` | template names only | Template owner assignment, source records, fallback behavior, non-substitution rules, freshness behavior, and proof-path expectations for future promotion. | none until promoted |

<a id="future-fixture-families"></a>
## 8. Future Fixture Families

The long row below preserves future fixture family names only. It is not a current MVP requirement or an executable conformance suite.

| Candidate | Status | Promotion requires | Active MVP impact |
|---|---|---|---|
| Intake and decision routing; Core, evidence, verification, and close; Artifact redaction and export non-leakage; Agency and user-judgment separation; Connector capability honesty; Design-quality and stewardship; Context hygiene and resume freshness; Projection, reconcile, and verification boundary; Operations diagnostics, export, recover, and handoff; Browser QA Capture | fixture family names only | Conformance owner assignment, exact fixture shape, assertions, payload/API/storage/error effects, and proof-path expectations for future promotion. | none until promoted |

## 9. Broad Future Candidates

| Candidate | Status | Promotion requires | Active MVP impact |
|---|---|---|---|
| Dashboard, hosted workflows, artifact dashboard, richer cards, richer visualizations | later candidate | Derived-display owner rules for read-only, non-authoritative behavior. | none until promoted |
| Verification Result Cards and richer verification/QA workflows | later candidate | Projection/template, Core/API, Eval, and Manual QA owner rules for source records, freshness, non-substitution, fallback behavior, and proof-path expectations. | none until promoted |
| Browser capture automation | later candidate | Capture owner rules for redaction/PII, retention, fallback behavior, and QA/acceptance non-substitution. | none until promoted |
| Cross-surface verification | later candidate | Core/Eval owner rules for return records, independence, and unsupported-surface fallback. | none until promoted |
| Broader connectors, connector marketplace, hosted UI, hosted/remote runtime | later candidate | Connector/API/security owners and future local-authority boundary proof expectation. | none until promoted |
| Connector conformance ecosystem | later candidate | Connector, API, Security, and Conformance owner rules for capability claims, connector assertions, suite/report formats, marketplace claims, and proof expectations. | none until promoted |
| Native hooks, preventive guard expansion, advanced sidecar watcher | later candidate | Owner-proven covered mechanism before any preventive, isolation, or arbitrary-tool-control claim. | none until promoted |
| Context Index, local derived metrics, long-term metrics | later candidate | Read-only retrieval/diagnostic owners and no authority or close effect. | none until promoted |
| Team workflows, permissions, shared capability sets, orchestration, parallel lanes | later candidate | Scope, authority, permission, and user-owned judgment owners. | none until promoted |
| Advanced exports, release/deployment/canary/rollback/merge/production-monitoring automation | later candidate | Separate owner scope; deployment and production authority remain external unless explicitly promoted. | none until promoted |
| Advanced validators, design-policy validators, and language or interface checks | later candidate | Validator owner rules for exact IDs, severity, waiver boundary, close impact, and fixture behavior. | none until promoted |
