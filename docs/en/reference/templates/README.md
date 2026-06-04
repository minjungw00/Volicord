# Template Reference

## Used when

Use this directory when you need the rendered shape for the small MVP-1 views. The projection rules, authority boundaries, managed-block behavior, artifact-ref rendering, and freshness behavior are owned by [Projection And Templates Reference](../projection-and-templates.md).

Authority rule:

- Templates are views, not authority state.
- User templates optimize readability.
- Agent templates optimize compact, accurate next-action context.
- Rendered views cannot create approval, work acceptance, residual-risk acceptance, evidence, close readiness, Write Authorization, or close.
- Chat, Markdown, status cards, agent packets, and reports cannot override Core state.
- A template existing in this repository does not make it required for MVP-1.

Owner boundary: this directory owns rendered template bodies and display card shapes. It does not define canonical kernel state, MCP schemas, SQLite DDL, gates, artifact storage, conformance, operations behavior, or implementation readiness. Current repository phase and handoff status are tracked in [Implementation Overview](../../build/implementation-overview.md#documentation-acceptance-status).

## Output tiers

| Tier | Template scope | Rule |
|---|---|---|
| Engineering Checkpoint status | Plain structured status/blocker output; optional [status-card](status-card.md) rendering | No projection job or full renderer is required. |
| MVP-1 User Work Loop views | [status-card](status-card.md), [agent-context-packet](agent-context-packet.md), [judgment-request](judgment-request.md), [run-evidence-summary](run-evidence-summary.md), [close-result](close-result.md) | This is the complete MVP-1 template/view set. Each view is derived from Core state and refs. |
| Later/full-profile templates | [later-profile/](later-profile/README.md) | Detailed assurance, diagnostic, operations, export, stewardship, and full-report templates stay later-profile unless an owner profile explicitly promotes them. |

## Template implementation classes

| Class | Templates | First active stage | Notes |
|---|---|---|---|
| User status | [status-card](status-card.md) | MVP-1 User Work Loop | Short user-visible current state. It is the default user-readable current-state view. |
| Agent next-action context | [agent-context-packet](agent-context-packet.md) | MVP-1 support view | Compact refs, blockers, source clocks, freshness, and owner-section pointers for the next safe action. |
| User-owned judgment prompt | [judgment-request](judgment-request.md) | MVP-1 User Work Loop | Concise prompt for Product/UX judgment, Technical judgment, Sensitive action approval, Work acceptance, or Residual risk acceptance. Full Decision Packet display is later/full-profile. |
| Run and evidence summary | [run-evidence-summary](run-evidence-summary.md) | MVP-1 User Work Loop | Minimal Run, check, evidence ref, artifact ref, redaction, and gap summary. Detailed Run Summary and Evidence Manifest are later/full-profile. |
| Close display | [close-result](close-result.md) | MVP-1 User Work Loop | Close readiness, acceptance, residual risk, blockers, smallest unblocker, and close result display. Detailed Journey, direct-result, export, and release-handoff reports are later/full-profile. |

## MVP-1 Template Set

MVP-1 templates/views are limited to:

- [status-card](status-card.md): short user-visible current state.
- [agent-context-packet](agent-context-packet.md): compact context for the next safe action.
- [judgment-request](judgment-request.md): user-owned judgment request.
- [run-evidence-summary](run-evidence-summary.md): minimal run and evidence summary.
- [close-result](close-result.md): close readiness, acceptance, residual risk, and blockers.

These five views can be returned as structured payloads, compact text, cards, or Markdown snippets depending on the surface. MVP-1 does not require persisted Markdown projection jobs, a full renderer, or every detailed report template.

## Later/Full-Profile Templates

Detailed templates are kept in [later-profile/](later-profile/README.md). They are useful for later profiles, but they are not MVP-1 requirements and their presence does not mean the runtime implements them.

| Bucket | Templates | Boundary |
|---|---|---|
| Assurance Profile | [DEC / Decision Packet](later-profile/decision-packet.md), [APR](later-profile/approval.md), [Approval Card](later-profile/approval-card.md), [EVIDENCE-MANIFEST](later-profile/evidence-manifest.md), [EVAL](later-profile/eval.md), [MANUAL-QA](later-profile/manual-qa.md), [Manual QA Card](later-profile/manual-qa-card.md), [Verification Result Card](later-profile/verification-result-card.md) | Verification strengthening, Manual QA, detailed evidence, risk review, and detailed evaluation output only when the owner profile is active. |
| Operations Profile | [EXPORT](later-profile/export.md) | Export, handoff, artifact availability, redaction/omission, and release-handoff displays only when the operations/export path is active. |
| Future/diagnostic profile material | [TASK](later-profile/task.md), [DIRECT-RESULT](later-profile/direct-result.md), [JOURNEY-CARD](later-profile/journey-card.md), [DESIGN](later-profile/design.md), [DOMAIN-LANGUAGE](later-profile/domain-language.md), [MODULE-MAP](later-profile/module-map.md), [INTERFACE-CONTRACT](later-profile/interface-contract.md), [RUN-SUMMARY](later-profile/run-summary.md), [TDD-TRACE](later-profile/tdd-trace.md) | Detailed continuity, stewardship, TDD, diagnostic, and reporting views stay later-profile unless an owner promotes them. |

Dashboard, hosted workflow, team workflow, broader connector, automation, and analytics views are [Roadmap](../../roadmap.md) candidates, not template requirements.

## Notes

If a source record or ref does not exist, render `none`, `unknown`, `not_required`, or a blocking/unavailable note. Do not invent placeholder state to satisfy a template.

Large logs, diffs, traces, screenshots, recordings, bundles, export components, and sensitive artifact bodies should be referenced by `ArtifactRef`, not embedded by default. Preserve `redaction_state` and show omission/block notes without reconstructing omitted or blocked raw values.
