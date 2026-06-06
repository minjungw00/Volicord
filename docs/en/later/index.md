# Later Candidate Index

This is the single active index for later candidates. It replaces detailed later profile pages, the later schema appendix, the future fixture catalog, the old roadmap narrative, and later-profile template bodies.

Rows here are planning candidates only. They are not MVP-1 requirements, active API or schema contracts, fixture bodies, template bodies, runtime behavior, implementation tasks, generated artifacts, or acceptance evidence. Detailed body: none.

<a id="boundary"></a>
## 1. Boundary

| Boundary | Current status | Promotion requires | Active current MVP impact |
|---|---|---|---|
| Later candidate index | Active documentation index only. | A future owner must promote a narrow candidate before any detailed contract returns. | none |
| Current repository phase | Documentation-only planning and review baseline. | Documentation acceptance and a separate implementation-planning readiness decision before runtime work. | none |
| Candidate authority | Candidate rows do not create Core state, evidence, verification, QA, final acceptance, residual-risk acceptance, close readiness, conformance, or security guarantees. | Owner records, exact contracts, and proof in the promoted owner path. | none |
| Bilingual parity | English and Korean indexes carry the same candidate set and promotion boundary. | Same-batch paired updates for any meaning change. | none |

<a id="promotion-rule"></a>
## 2. Promotion Rule

| Rule | Current status | Promotion requires | Active current MVP impact |
|---|---|---|---|
| Owner decision | Candidate is parked before promotion. | A named future stage/profile owner, narrow scope, fallback behavior, and explicit non-claims. | none |
| Contract placement | No full later schema, template body, fixture body, or scenario script is active here. | Exact API, schema, storage, security, projection, fixture, template, and operator contracts in the appropriate owner document. | none |
| Proof expectation | Candidate listing is not proof. | Fixture/conformance target or other owner-defined evidence for the exact promoted behavior. | none |
| Non-substitution | Candidate output cannot replace Core state, user judgment, evidence, verification, Manual QA, final acceptance, residual-risk acceptance, or close readiness. | Core-owned records and owner-specific lifecycle rules must remain separate. | none |
| Early-stage inflation check | Later material stays outside Engineering Checkpoint and MVP-1 by default. | Proof that the promoted candidate does not add unsupported requirements to earlier stages. | none |

<a id="assurance-candidates"></a>
## 3. Assurance Candidates

| Candidate | Current status | Promotion requires | Active current MVP impact |
|---|---|---|---|
| assurance profile | Later candidate; detailed body removed. | Owner-scoped assurance profile with exact gates, fallback behavior, and proof expectations. | none |
| Evidence Manifest | Later candidate for detailed evidence coverage and gaps. | Evidence owner rules, artifact/ref handling, redaction behavior, and close impact. | none |
| Manual QA | Later candidate for human QA records, findings, waivers, and QA gate impact. | Manual QA owner policy, waiver route, artifact refs, and close-blocker behavior. | none |
| Eval / detached verification | Later candidate for independent verification result recording. | Eval owner path, independence semantics, baseline freshness, artifact integrity, and assurance update rules. | none |
| Decision Packet full-format presentation | Later candidate for expanded user-judgment display. | User-judgment owner must enable `presentation=full` without making it the default MVP path. | none |
| Risk review and residual-risk visibility | Later candidate for richer risk lifecycle display. | Core/user-judgment owner rules for visible risk, acceptance, expiry, and close impact. | none |
| Design-quality, stewardship, TDD trace, and context-hygiene assurance checks | Later candidate group. | Policy owners must define exact validators, severity, waiver, evidence, and fixture behavior. | none |

<a id="operations-candidates"></a>
## 4. Operations Candidates

| Candidate | Current status | Promotion requires | Active current MVP impact |
|---|---|---|---|
| operations profile | Later candidate; detailed body removed. | Owner-scoped operations profile with exact operator commands, diagnostics, fallback behavior, and proof expectations. | none |
| Export | Later candidate for task bundles, redaction/omission notes, retained/unavailable artifacts, and integrity summaries. | Operations/export owner contracts, storage/artifact rules, redaction rules, and non-leakage proof. | none |
| Release Handoff | Later candidate for report/export handoff only. | Handoff owner must keep deployment, merge, rollback, and production authority outside Harness unless separately promoted. | none |
| Recovery and reconcile | Later candidate for lock, projection, artifact, and managed-output repair paths. | Operations, Storage, Projection, Reconcile, and Security owner rules. | none |
| Operator readiness and `doctor` surfaces | Later candidate for local status, diagnostics, and next operator action. | Operations owner commands, capability checks, security posture wording, and unsupported-surface fallback. | none |
| Projection refresh and freshness diagnostics | Later candidate for derived view health. | Projection owner behavior that keeps projections non-authoritative. | none |
| Future conformance run entrypoint | Later candidate after runtime fixtures exist. | Exact runner, suite, assertion, request/response, storage/event/artifact/error, and reporting contracts. | none |

<a id="later-api-candidates"></a>
## 5. Later API Candidates

| Candidate | Current status | Promotion requires | Active current MVP impact |
|---|---|---|---|
| `harness.next` | Later/compatibility read candidate. | Owner activation for a separate next-action payload; MVP continues using `harness.status.next_actions`. | none |
| `harness.launch_verify` | Assurance candidate for detached verification launch or manual evaluator bundle. | Eval/verification owner path, capability handling, baseline freshness, and honest isolation wording. | none |
| `harness.record_eval` | Assurance candidate for verification result recording. | Eval owner contract, independence validation, artifact refs, and gate/assurance update rules. | none |
| `harness.record_manual_qa` | Assurance candidate for human QA outcome recording. | Manual QA owner contract, waiver route, artifacts, findings, and gate impact. | none |
| Later read-only resources | Later candidate for policy, evidence-manifest, surface, report, bundle, journey, and design reads. | Resource-specific owner contracts and no mutation side effects. | none |
| Later `harness.record_run` branches | Later candidate for verification input, feedback-loop updates, and TDD trace updates. | `record_run` owner activation and one-branch payload rules. | none |
| Later user-judgment branches | Later candidate for waiver, reconcile, residual-risk, and richer acceptance visibility. | User-judgment owner activation with non-substitution rules. | none |

<a id="later-schema-candidates"></a>
## 6. Later Schema Candidates

| Candidate | Current status | Promotion requires | Active current MVP impact |
|---|---|---|---|
| later schema extensions | Candidate only; full schema body removed. | A promoted owner must define exact fields and validators in the active owner contract. | none |
| Later close and assurance fields | Candidate for `verifying`, `qa`, `completed_verified`, `detached_verified`, verification gate, QA gate, and assurance blockers. | Core/API owner activation with close non-substitution rules. | none |
| Later next-action values | Candidate for `launch_verify`, `record_eval`, `record_manual_qa`, and `reconcile`. | Matching API/profile owner activation. | none |
| Recommended playbooks and judgment context | Candidate for richer read-only routing metadata. | Agent Integration/API owner rules that prevent metadata from mutating or satisfying state. | none |
| Later ref and artifact values | Candidate for bundle, manifest, QA capture, export component, design, Eval, Manual QA, TDD, projection, and related refs. | ArtifactRef, StateRecordRef, Storage, and profile owner activation. | none |
| ValidatorResult later stable IDs | Candidate for design, autonomy, feedback-loop, TDD, stewardship, residual-risk, shared-design, manual-QA, and context-hygiene checks. | Validator owner contracts, stable IDs, severity, waiver, and fixture proof. | none |
| Waiver, reconcile, and residual-risk branches | Candidate for richer user-judgment payloads. | User-judgment, Core, and close owner rules. | none |

<a id="later-template-candidates"></a>
## 7. Later Template Candidates

| Candidate | Current status | Promotion requires | Active current MVP impact |
|---|---|---|---|
| Decision Packet full-format presentation (`DEC`) | Later template candidate; detailed body removed. | User-judgment owner activation for complex/full presentation. | none |
| `APR` | Later template candidate; detailed body removed. | Sensitive-action Approval owner path and non-substitution wording. | none |
| Approval Card | Later template candidate; detailed body removed. | Approval display owner path. | none |
| `RUN-SUMMARY` | Later template candidate; detailed body removed. | Evidence/run owner path and compact MVP summary fallback. | none |
| `EVIDENCE-MANIFEST` | Later template candidate; detailed body removed. | Evidence Manifest owner path. | none |
| `EVAL` | Later template candidate; detailed body removed. | Eval owner path and detached-verification proof. | none |
| Verification Result Card | Later template candidate; detailed body removed. | Verification/Eval display owner path. | none |
| `MANUAL-QA` | Later template candidate; detailed body removed. | Manual QA owner path. | none |
| Manual QA Card | Later template candidate; detailed body removed. | Manual QA display owner path. | none |
| `TASK` | Later template candidate; detailed body removed. | Later continuity/report owner path. | none |
| `DIRECT-RESULT` | Later template candidate; detailed body removed. | Later direct-result report owner path. | none |
| `JOURNEY-CARD` | Later template candidate; detailed body removed. | Journey/current-position diagnostic owner path. | none |
| `DESIGN` | Later template candidate; detailed body removed. | Shared-design/design-quality owner path. | none |
| `DOMAIN-LANGUAGE` | Later template candidate; detailed body removed. | Domain language owner path. | none |
| `MODULE-MAP` | Later template candidate; detailed body removed. | Module map owner path. | none |
| `INTERFACE-CONTRACT` | Later template candidate; detailed body removed. | Interface contract owner path. | none |
| `TDD-TRACE` | Later template candidate; detailed body removed. | TDD/feedback-loop policy owner path. | none |
| `EXPORT` | Later template candidate; detailed body removed. | Operations/export owner path. | none |

<a id="future-fixture-families"></a>
## 8. Future Fixture Families

| Candidate | Current status | Promotion requires | Active current MVP impact |
|---|---|---|---|
| future fixture families | Later candidate index only; full scenario scripts removed. | Exact fixture body, runner behavior, assertions, and owner contracts after promotion. | none |
| Intake and decision routing | Future family candidate. | Core/API intake owner and user-judgment separation rules. | none |
| Core, evidence, verification, and close | Future family candidate. | Core, API, evidence, verification, and close owner assertions over state and refs. | none |
| Artifact redaction and export non-leakage | Future family candidate. | Artifact, redaction, storage, export, and security owner proof. | none |
| Agency and user-judgment separation | Future family candidate. | User-judgment, Approval, acceptance, residual-risk, and Autonomy Boundary owner rules. | none |
| Connector capability honesty | Future family candidate. | Agent Integration and Security owner capability profiles, fallback behavior, and guarantee proof. | none |
| Design-quality and stewardship | Future family candidate. | Design-quality, domain, module, interface, and validator owners. | none |
| Context hygiene and resume freshness | Future family candidate. | Agent Integration/context owner rules that keep retrieved context non-authoritative. | none |
| Projection, reconcile, and verification boundary | Future family candidate. | Projection/Reconcile/Core/Eval owner rules. | none |
| Operations diagnostics, export, recover, and handoff | Future family candidate. | Operations, Storage, Security, ArtifactRef, and conformance owners. | none |
| Browser QA Capture | Roadmap fixture-family candidate. | Browser capture profile, redaction policy, artifact retention, fallback behavior, and proof that capture does not replace Manual QA or acceptance. | none |

<a id="roadmap-candidates"></a>
## 9. Roadmap Candidates

| Candidate | Current status | Promotion requires | Active current MVP impact |
|---|---|---|---|
| Dashboard, hosted workflows, artifact dashboard, richer cards, richer visualizations | Roadmap candidate. | Read-only derived display rules; no authority, readiness, QA, verification, or acceptance effect. | none |
| Browser capture automation | Roadmap candidate. | Capture profile, redaction/PII handling, artifact retention, fallback behavior, and QA/acceptance non-substitution. | none |
| Cross-surface verification | Roadmap candidate. | Core-owned return records, Eval independence semantics, and unsupported-surface fallback. | none |
| Broader connectors, connector marketplace, hosted UI, hosted/remote runtime | Roadmap candidate. | Connector/API/security owners and local-authority boundary proof. | none |
| Native hooks, preventive guard expansion, advanced sidecar watcher | Roadmap candidate. | Proven covered mechanism before any preventive, isolation, or arbitrary-tool-control claim. | none |
| Context Index, local derived metrics, long-term metrics | Roadmap candidate. | Read-only retrieval/diagnostic owners and no authority or close effect. | none |
| Team workflows, permissions, shared profiles, orchestration, parallel lanes | Roadmap candidate. | Scope, authority, permission, and user-owned judgment owners. | none |
| Advanced exports, release/deployment/canary/rollback/merge/production-monitoring automation | Roadmap candidate. | Separate owner scope; deployment and production authority remain external unless explicitly promoted. | none |
| Advanced validators and language or interface checks | Roadmap candidate. | Exact policy, validator IDs, severity, waiver, and fixture behavior. | none |

<a id="explicitly-retired-material"></a>
## 10. Explicitly Retired Material

| Candidate | Current status | Promotion requires | Active current MVP impact |
|---|---|---|---|
| Full future designs in Later pages | Retired from active documentation; detailed body removed. | Reintroduce only in a promoted owner document with narrow scope. | none |
| Full later schema definitions | Retired from active documentation; detailed body removed. | Reintroduce only as active or profile-gated owner schema after promotion. | none |
| Full later template bodies | Retired from active documentation; detailed body removed. | Reintroduce only when a template owner activates the profile and defines source records. | none |
| Full YAML fixtures and scenario scripts | Retired from active documentation; detailed body removed. | Reintroduce only as exact-shape fixture material under conformance owners after promotion. | none |
| Old roadmap narratives, resolved goals, and historical cleanup notes | Retired from active documentation. | Reintroduce only as current candidate rows or maintainer-owned history if needed. | none |
| Archive copies, migration notes, and scratch files for this collapse | Not created and not retained. | No promotion path; recreate only if maintainers request a separate non-active migration record. | none |
