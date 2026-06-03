# MODULE-MAP Template

## Used when

Use `MODULE-MAP` when module roles, public interfaces, internal complexity, dependencies, test boundaries, owner decisions, or watchpoints need a readable projection.

Boundary: projection template only; it does not authorize runtime/server implementation or generated operational outputs. Shared phase and projection rules live in [Template Reference](README.md#used-when).

Implementation tier: Future/diagnostic projections. Module Map output is a later stewardship/reference view, not required for the first runnable slice or v0.2 compact-card MVP.

## Source records

- `module_map_items`
- module-local watchpoints stored on module map items
- reconcile items that propose module map changes
- related Decision Packets and design refs
- design-quality validator results related to `deep_module_interface` or `codebase_stewardship`
- routed stewardship findings that affect module or boundary refs, when displayed
- projection freshness inputs

## Rendered sections

- Summary
- Modules
- Deep Module Candidates
- Module Watchpoint Rollup
- User Notes and Proposals

## Full template

````md
---
doc_type: module_map
project_id: PRJ-0001
status: active
projection_version: 1
source_state_version: 12
updated_at: 2026-05-06T09:30:15+09:00
---

# Module Map

> Projection view: rendered from `module_map_items` and related refs at `source_state_version` / `updated_at`. Managed sections are generated display; use `User Notes and Proposals` for reconcile input.

<!-- HARNESS:BEGIN managed -->
## Summary
- architecture state:
- latest review:
- stale conditions:

## Modules
| Module | Role | Public Interface | Internal Complexity | Dependencies | Test Boundary | Owner Decision | Watchpoints |
|---|---|---|---|---|---|---|---|
| AuthService | verifies auth and issues sessions | `login`, `logout` | credential validation, session issue | UserRepo, SessionStore | service interface tests | human_reviewed | session expiry drift |

## Deep Module Candidates
| Candidate | Current Pain | Proposed Boundary | Expected Test Boundary | Priority |
|---|---|---|---|---|

## Module Watchpoint Rollup
- source: `module_map_items.watchpoints_json`
- canonical owner: Module Map Item; dedicated architecture watchpoint refs only if a later DDL batch defines them
- shallow module growth:
- dependency direction risk:
- public interface drift:
<!-- HARNESS:END managed -->

## User Notes and Proposals
<!-- Human-editable: module proposals here are not canonical Module Map Items until accepted through reconcile/Core. -->
-
````

## Notes

This template is a rendered shape, not canonical state. Canonical module refs use `StateRecordRef.record_kind=module_map_item`. Review, watchpoint, and stewardship rollup text is display over owner records; it does not create Approval, evidence, QA, verification, work acceptance, residual-risk acceptance, close, or Write Authorization.

When a proposed module boundary change shifts product commitments, public interfaces, caller obligations, dependency direction, or architecture direction, route the judgment through the existing design-quality and Decision Packet paths. Rendering the proposal here does not resolve the `design_gate`, `decision_gate`, or close impact by itself.
