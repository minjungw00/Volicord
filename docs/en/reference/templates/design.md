# DESIGN Template

## Used when

Use `DESIGN` when shared design, domain language impact, module/interface planning, alternatives, recommendation, and verification considerations need a standalone readable projection.

Boundary: projection template only; it does not authorize runtime/server implementation or generated operational outputs. Shared phase and projection rules live in [Template Reference](README.md#used-when).

Implementation tier: Future/diagnostic projections. Standalone design projection is later-profile scope; early user judgment context can appear in the user judgment request display.

## Source records

- shared design records and events
- Task and Change Unit refs
- related Decision Packets and approvals
- `domain_terms`
- `module_map_items`
- `interface_contracts`
- feedback loop, TDD, Manual QA, and evidence refs
- design-quality or stewardship findings routed through existing owner paths, when displayed
- projection freshness inputs

## Rendered sections

- Problem
- Goals
- Non-Goals
- Constraints
- Shared Design Summary
- Domain Language Impact
- Module And Interface Plan
- Proposed Shape
- Alternatives
- Recommendation
- Verification Considerations
- References

## Full template

````md
---
doc_type: design
design_id: DESIGN-0001
task_id: TASK-0001
status: draft
source_state_version: 42
updated_at: 2026-05-06T09:30:15+09:00
---

# DESIGN-0001 Design Title

> Projection view: rendered from `source_state_version` at `updated_at`; summarizes owner records and proposals. Editing it does not replace Domain Language, Module Map, Interface Contract, Decision Packet, or Task state.

## Problem
- design problem:

## Goals
- goal:

## Non-Goals
- non-goal:

## Constraints
- technical:
- operational:
- compatibility:
- security/privacy:

## Shared Design Summary
- resolved questions:
- remaining assumptions:
- rejected options:

## Domain Language Impact
| Term | Impact | Action |
|---|---|---|

## Module And Interface Plan
| Module | Current Role | Proposed Change | Public Interface | Test Boundary | Risk |
|---|---|---|---|---|---|

## Proposed Shape
- components:
- boundaries and responsibilities:
- data flow:
- dependency direction:

## Alternatives
### Alternative A
- benefits:
- drawbacks:

### Alternative B
- benefits:
- drawbacks:

## Recommendation
- recommendation:
- remaining trade-off:

## Verification Considerations
- success criteria:
- regression watchpoint:
- selected feedback loop:
- required TDD trace:
- required Manual QA:
- required evidence:

## References
- TASK:
- DEC:
- APR:
- design-support owner refs:
  - domain term refs:
  - module map item refs:
  - interface contract refs:
- rendered projection refs, if shown:
  - DOMAIN-LANGUAGE:
  - MODULE-MAP:
  - INTERFACE-CONTRACT:
- EVIDENCE-MANIFEST:
````

## Notes

This template is a rendered shape, not canonical state. It may summarize design-support owner refs and routed stewardship findings, but it must not replace those owner records or the owner paths that Review Stages point to. It does not satisfy or block close, grant Approval, create evidence, record QA or verification, accept results, accept residual risk, or create Write Authorization.
