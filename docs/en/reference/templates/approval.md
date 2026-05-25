# APR Template

## Used when

Use `APR` after an approval request has been committed and Harness needs a readable approval request and decision record for sensitive change.

## Source records

- approval record
- related approval-shaped Decision Packet
- optional decision request routing/replay record, if implementation keeps one
- Change Unit scope
- sensitive categories
- allowed paths, tools, commands, network targets, and secrets
- baseline, expiry, alternatives, and decision note

A non-mutating `approval_request_candidate` returned by `prepare_write` is not an `APR` source and must be displayed, if at all, as candidate display.

Boundary Summary is a derived display block from approval scope, linked Approval records, related Decision Packet refs, and current write or close context. It is a user-facing boundary reminder, not an independent authority source or gate.

## Rendered sections

- Request Summary
- Boundary Summary
- Related Decision Packet
- Requested Scope
- Why This Is Needed
- Impact
- Risks
- Alternatives
- Recommendation
- Decision
- Boundary

## Full template

````md
---
doc_type: approval
approval_id: APR-0001
task_id: TASK-0001
category: dependency_change
status: pending
source_state_version: 42
updated_at: 2026-05-06T09:30:15+09:00
---

# APR-0001 Approval Request

> Projection view: rendered from `source_state_version` at `updated_at`; displays the approval request and boundary. Approval still requires the canonical approval decision path, and writes still require compatible `prepare_write`.

## Request Summary
- proposed action:
- sensitive action being approved:
- what the word "approved" means here:

## Boundary Summary
- this request covers:
- this request does not decide:
- if granted, still requires later:
- secret exposure boundary:

## Related Decision Packet
- approval-shaped Decision Packet:
- separate Decision Packet for product or material technical judgment, if required:
- decision gate impact:
- approval gate impact:

## Requested Scope
- sensitive categories:
- allowed paths:
- allowed tools:
- allowed commands:
- allowed network targets:
- required secrets:
- baseline ref:
- expected diff envelope:
- expires on scope drift:

## Why This Is Needed
- purpose:
- relation to current task:

## Impact
- code/docs:
- user/operations:
- security/privacy:
- cost/deployment:
- domain language:
- module/interface:

## Risks
- main risk:
- failure impact:
- scope drift condition:

## Alternatives
### Alternative A
- description:
- benefit:
- cost/risk:

### Alternative B
- description:
- benefit:
- cost/risk:

## Recommendation
- recommendation:
- reason:

## Decision
- status: pending | granted | denied | expired
- decision note:
- decided by:
- decided at:

## Boundary
- approval does not resolve user-owned product or material technical judgment, prove correctness, replace verification, replace Manual QA, imply acceptance, or accept residual risk.
- approval is not Write Authorization; a later compatible `prepare_write` retry must allow the write before implementation or direct `record_run` can consume authorization.
- dependency install approval does not decide the architecture direction for using that dependency.
- secret access approval does not permit exposing secret values in artifacts, projections, exports, logs, screenshots, or summaries.
- auth, permission, or system-change approval does not decide session auth, JWT, social login, role model, lockout behavior, or user notice.
- public API direction, deployment, merge, final acceptance, and additional write attempts each need their own applicable recorded decision or authority when required.
````

## Notes

The Boundary section is the user-facing reminder. Decision request routing records are not decision authority and cannot affect `decision_gate` except through a linked compatible Decision Packet.
