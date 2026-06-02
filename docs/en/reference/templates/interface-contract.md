# INTERFACE-CONTRACT Template

## Used when

Use `INTERFACE-CONTRACT` when a module interface, caller impact, compatibility risk, or test boundary needs a readable projection.

Boundary: projection template only; it does not authorize runtime/server implementation or generated operational outputs. Shared phase and projection rules live in [Template Reference](README.md#used-when).

Implementation tier: Future/diagnostic projections. Interface Contract output is a later reference view unless an owner profile explicitly promotes it.

## Source records

- `interface_contracts`
- impacted caller refs
- related module map items
- related Decision Packets or design refs
- boundary, integration, or contract test refs
- design-quality validator results related to `deep_module_interface`
- routed stewardship findings that affect interface or compatibility refs, when displayed
- projection freshness inputs

## Rendered sections

- Identity
- Contract
- Callers Impacted
- Test Boundary
- Review
- References
- User Notes and Proposals

## Full template

````md
---
doc_type: interface_contract
interface_contract_id: IFACE-0001
task_id: TASK-0001
review_status: pending
projection_version: 1
source_state_version: 42
updated_at: 2026-05-06T09:30:15+09:00
---

# IFACE-0001 Interface Title

> Projection view: rendered from `interface_contracts` and related refs at `source_state_version` / `updated_at`. Managed sections are generated display; use `User Notes and Proposals` for reconcile input.

<!-- HARNESS:BEGIN managed -->
## Identity
- module:
- interface:
- change type: new | changed | deprecated | removed

## Contract
- inputs:
- outputs:
- errors:
- side effects:
- compatibility impact: none | minor | breaking

## Callers Impacted
- caller:

## Test Boundary
- boundary tests:
- integration tests:
- contract tests:

## Review
- review_status: pending | reviewed
- reviewed by:
- decision:
- waiver reason:

## References
- TASK:
- DESIGN:
- DEC:
- EVIDENCE-MANIFEST:
<!-- HARNESS:END managed -->

## User Notes and Proposals
<!-- Human-editable: interface proposals here are not canonical Interface Contract records until accepted through reconcile/Core. -->
-
````

## Notes

This template is a rendered shape, not canonical state. Canonical interface refs use `StateRecordRef.record_kind=interface_contract`. The `Review` section is projection display over interface, validator, and decision refs; it is not Approval, evidence, QA, verification, work acceptance, residual-risk acceptance, close, or Write Authorization.

When a public interface change, compatibility risk, breaking change, or caller-impact choice requires user-owned product judgment or material technical judgment, route it through the existing design-quality and Decision Packet paths. Rendering the contract here does not resolve the `design_gate`, `decision_gate`, or close impact by itself.
