# Later: Security and Assurance

## What this document owns

This document owns inactive later candidates about stronger security posture, assurance claims, capability labels, observation, blocking, and verification confidence. It keeps those candidates grouped so [Later Candidate Index](index.md) can remain a router and short summary.

Every candidate here is future-facing. The candidate details are documentation source material only and do not activate runtime behavior.

## What this document does not own

This document does not define current MVP security guarantees, active access classes, active API methods, storage effects, UI behavior, connector behavior, executable conformance, or implementation readiness.

It also does not decide that a stronger guarantee is possible. Any preventive, detective, isolation, redaction, observation, or blocking claim must be re-owned by the current security and active-scope owners during promotion.

## Category boundary

This category is for candidates whose main question is "what assurance can Harness honestly claim?" It includes preventive-control candidates, isolation labels, capability-profile hardening, command/network/secret observation, pre-tool blocking, and stronger verification-confidence claims.

It does not own native artifact capture as a storage mechanism, connector surface design, team workflow, or validator catalog detail unless the candidate is specifically about an assurance claim. Cross-cutting candidates may also appear in another category later, but this document owns only the security-and-assurance framing before promotion.

## Candidate summary

| Candidate | Summary |
|---|---|
| Assurance hardening | Stronger evidence, verification, and close-readiness assurance claims beyond the current MVP. |
| Operations hardening | Future operator diagnostics and stronger security posture for local operation. |
| Stronger local capability profiles | Future profile labels for observation, capture, isolation, or blocking capabilities. |
| Command, network, and secret-access observation | Future observation of selected command, network, or secret-access intent. |
| Command, network, and secret pre-tool blocking | Future preventive blocking claims before tool execution. |
| Capability-gated `prepare_write` and `record_run` observation | Future command, network, or secret-access observation around write preparation and run recording. |
| Capability-profile support fields | Future support fields for observation, capture, pre-tool blocking, and isolation capabilities. |
| Capability-gated authorization observation fields | Future fields such as `intended_commands`, `intended_network`, `network_write`, and `secret_access`. |
| Later close and assurance fields | Future close, gate, verification, QA, design, and assurance fields. |
| Native hooks and advanced sidecar watcher | Future native hook or sidecar watcher claims for broader tool visibility. |

## Candidate details

<a id="assurance-hardening"></a>
### Assurance hardening

- Status: Later candidate; currently inactive.
- Current MVP non-effect: Not part of the current MVP. Does not create active gates, validators, or close-readiness requirements.
- Promotion focus: assurance owners, schema owners, API behavior, and conformance checks if stronger assurance becomes normative.

<a id="operations-hardening"></a>
### Operations hardening

- Status: Later candidate; currently inactive.
- Current MVP non-effect: Not part of the current MVP. Does not create active operator commands, diagnostics, or security guarantees.
- Promotion focus: operations owners, security wording, API behavior, and conformance checks if local-operation hardening becomes normative.

<a id="stronger-local-capability-profiles"></a>
### Stronger local capability profiles

- Status: Later candidate; currently inactive.
- Current MVP non-effect: Not part of the current MVP. Does not create active preventive, isolated, observation, capture, pre-tool blocking, or isolation guarantees.
- Promotion focus: agent-integration owners, security owners, schema owners, and conformance checks for any promoted capability profile.

<a id="command-network-and-secret-access-observation"></a>
### Command, network, and secret-access observation

- Status: Later candidate; currently inactive.
- Current MVP non-effect: Not part of the current MVP. Does not create active command observation, network observation, or secret-access observation authority.
- Promotion focus: agent-integration owners, security owners, API behavior, and conformance checks for any promoted observation claim.

<a id="command-network-and-secret-pre-tool-blocking"></a>
### Command, network, and secret pre-tool blocking

- Status: Later candidate; currently inactive.
- Current MVP non-effect: Not part of the current MVP. Does not create active preventive blocking, isolation, or arbitrary-tool-control guarantees.
- Promotion focus: security owners, agent-integration owners, API behavior, and conformance checks for any promoted blocking claim.

<a id="capability-gated-prepare-write-and-record-run-observation"></a>
### Capability-gated `prepare_write` and `record_run` observation

- Status: Later candidate; currently inactive.
- Current MVP non-effect: Not part of the current MVP. Does not create active command, network, or secret-access observation for `prepare_write` or `record_run`.
- Promotion focus: API behavior, security owners, schema owners, and conformance checks for any promoted observation branch.

<a id="capability-profile-support-fields"></a>
### Capability-profile support fields

- Status: Later candidate; currently inactive.
- Current MVP non-effect: Not part of the current MVP. Does not activate `command_observation_supported`, `network_observation_supported`, `secret_access_observation_supported`, `artifact_capture_supported`, `pre_tool_blocking_supported`, or `isolation_supported`.
- Promotion focus: schema owners, agent-integration owners, security owners, and conformance checks for any supported profile field.

<a id="capability-gated-authorization-observation-fields"></a>
### Capability-gated authorization observation fields

- Status: Later candidate; currently inactive.
- Current MVP non-effect: Not part of the current MVP. Does not activate `intended_commands`, `intended_network`, `intended_secret_scope`, `network_write`, `external_service_write`, or `secret_access`.
- Promotion focus: schema owners, API behavior, security owners, and conformance checks for any promoted observation field.

<a id="later-close-and-assurance-fields"></a>
### Later close and assurance fields

- Status: Later candidate; currently inactive.
- Current MVP non-effect: Not part of the current MVP. Does not activate `verifying`, `qa`, `completed_verified`, `detached_verified`, `design_gate`, `verification_gate`, `qa_gate`, Manual QA gate fields, design-policy blockers, or assurance blockers.
- Promotion focus: schema owners, core owners, API behavior, and conformance checks for any promoted close or assurance field.

<a id="native-hooks-and-advanced-sidecar-watcher"></a>
### Native hooks and advanced sidecar watcher

- Status: Later candidate; currently inactive.
- Current MVP non-effect: Not part of the current MVP. Does not create active preventive guard expansion, native hook, sidecar watcher, or arbitrary-tool-control guarantees.
- Promotion focus: security owners, agent-integration owners, API behavior, and conformance checks for any promoted hook or watcher claim.

## Promotion rule

Promotion is not a local edit to this file. A candidate becomes active only when current active scope and the relevant current owner documents are updated in the same documentation-only batch.

If no current owner exists for the promoted behavior, the promotion batch must create or designate that owner before defining active API, storage, security, UI, or conformance requirements.

## Active-scope non-effect

This document has no effect on the current MVP. Mentioning a candidate here does not activate a guarantee, profile, field, method, access class, validator, fixture, UI, or runtime control.

## Related owners

- [Later Candidate Index](index.md)
- [Active MVP Scope](../reference/active-mvp-scope.md)
- [Reference Index](../reference/README.md)
- [Security](../reference/security.md)
- [Agent Integration](../reference/agent-integration.md)
- [Runtime Boundaries](../reference/runtime-boundaries.md)
- [API Value Sets](../reference/api/schema-value-sets.md)
