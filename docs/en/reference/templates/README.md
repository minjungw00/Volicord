# Template Reference

## Used when

Use this directory when you need the rendered shape for the small MVP-1 views. The projection rules, authority boundaries, managed-block behavior, artifact-ref rendering, and freshness behavior are owned by [Projection And Templates Reference](../projection-and-templates.md).

Authority rule:

- Templates are views, not authority state.
- User templates optimize readability and decision support.
- Agent templates optimize compact, accurate next-action context.
- User-facing templates and agent-facing packets are separate audiences; do not reuse one as the other.
- Rendered views cannot create approval, final acceptance, residual-risk acceptance, evidence, close readiness, Write Authorization, or close.
- Chat, Markdown, status cards, agent packets, and reports cannot override Core state.
- A template existing in this repository does not make it required for MVP-1.

Owner boundary: this directory owns rendered template bodies and display card shapes. It does not define canonical kernel state, MCP schemas, SQLite DDL, gates, artifact storage, conformance, operations behavior, or implementation readiness. Current repository phase and handoff status are tracked in [MVP Plan](../../build/mvp-plan.md#documentation-acceptance-status).

## Output tiers

| Tier | Template scope | Rule |
|---|---|---|
| Engineering Checkpoint status | Plain structured status/blocker output; optional [status-card](status-card.md) rendering | No projection job or full renderer is required. |
| MVP-1 user-facing compact outputs | [status-card](status-card.md), [judgment-request](judgment-request.md), [run-evidence-summary](run-evidence-summary.md), [close-result](close-result.md) | This is exactly the complete MVP-1 user-facing output set. Each output is derived from Core state and refs, uses ordinary language, and avoids unnecessary internal schema fields. |
| MVP-1 agent-facing compact output | [agent-context-packet](agent-context-packet.md) | This is the only active MVP-1 agent-facing packet. It carries only task/change-unit refs, state/source refs, active scope, unresolved user judgments, blockers, one next safe action, evidence gaps, close blockers, residual-risk summary when active, and guarantee level. |
| Later/full-profile templates | [Later template candidates](../../later/index.md#later-template-candidates) | Detailed assurance, diagnostic, operations, export, stewardship, and full-report templates stay candidate-only unless an owner profile explicitly promotes them. |

## Template implementation classes

| Class | Templates | First active stage | Notes |
|---|---|---|---|
| User status | [status-card](status-card.md) | MVP-1 User Work Loop | Short user-visible current state. It is the default user-readable current-state view. |
| Agent next-action context | [agent-context-packet](agent-context-packet.md) | MVP-1 support view | Compact refs, blockers, source clocks, freshness, evidence gaps, close blockers, residual-risk summary when active, guarantee level, and one next safe action. It is not user prose. |
| User-owned judgment prompt | [judgment-request](judgment-request.md) | MVP-1 User Work Loop | Concise prompt for Product decision, Technical decision, Scope decision, Sensitive action approval, QA waiver, Verification risk acceptance, Final acceptance, Residual risk acceptance, or Cancellation. Full Decision Packet display is later/full-profile. |
| Run and evidence summary | [run-evidence-summary](run-evidence-summary.md) | MVP-1 User Work Loop | Minimal Run, check, evidence ref, artifact ref, redaction, availability, and gap summary. Detailed Run Summary and detailed evidence reports are later/full-profile. |
| Close display | [close-result](close-result.md) | MVP-1 User Work Loop | Active MVP close readiness, final acceptance status, residual risk, blockers, smallest unblocker, and close result display. Detailed Journey, direct-result, export, release-handoff, and later assurance reports are later/full-profile. |

## MVP-1 Template Set

MVP-1 active compact outputs are limited exactly to these audience-specific shapes:

- User-facing [status-card](status-card.md): short current state.
- User-facing [judgment-request](judgment-request.md): user-owned judgment request.
- User-facing [run-evidence-summary](run-evidence-summary.md): minimal run and evidence summary.
- User-facing [close-result](close-result.md): close readiness, acceptance, residual risk, and blockers.
- Agent-facing [agent-context-packet](agent-context-packet.md): compact Core-derived refs for the next safe action.

The four user-facing outputs can be returned as compact text, cards, Markdown snippets, or structured payloads depending on the surface. The agent-facing packet can be structured or prompt-sized text. MVP-1 does not require persisted Markdown projection jobs, a full renderer, or every detailed report template.

User-facing outputs should be readable without Harness-internal expertise. They may show short refs and freshness when those help the user decide, understand a blocker, or inspect evidence, but they should not expose full schemas, field inventories, DDL, event logs, full report bodies, full artifact inventories, or future/profile catalogs.

The agent-facing packet stays smaller than a status card in prose terms. It may carry refs that a user card omits, but it must not inject full reference docs, full schemas, full DDL, historical logs, full projection bodies, full artifact contents, unrelated templates, or future catalogs.

MVP-1 outputs may show design-quality findings only through the routed action owned by [Design Quality Policies](../design-quality-policies.md#impact-classes-and-allowed-routes): block write, block close, ask one focused user judgment, request evidence, mark residual risk, show advisory next action, or no action. Do not render the full policy catalog as a default close checklist.

## Later/Full-Profile Templates

Detailed template bodies are retired from active documentation and summarized in [Later template candidates](../../later/index.md#later-template-candidates). They may be useful for later profiles, but they are not MVP-1 requirements and their listing does not mean the runtime implements them.

| Bucket | Templates | Boundary |
|---|---|---|
| Assurance Profile | [DEC / Decision Packet](../../later/index.md#later-template-candidates), [APR](../../later/index.md#later-template-candidates), [Approval Card](../../later/index.md#later-template-candidates), [EVIDENCE-MANIFEST](../../later/index.md#later-template-candidates), [EVAL](../../later/index.md#later-template-candidates), [MANUAL-QA](../../later/index.md#later-template-candidates), [Manual QA Card](../../later/index.md#later-template-candidates), [Verification Result Card](../../later/index.md#later-template-candidates) | Verification strengthening, Manual QA, detailed evidence, risk review, and detailed evaluation output only when the owner profile is active. |
| Operations Profile | [EXPORT](../../later/index.md#later-template-candidates) | Export, handoff, artifact availability, redaction/omission, and release-handoff displays only when the operations/export path is active. |
| Future/diagnostic profile material | [TASK](../../later/index.md#later-template-candidates), [DIRECT-RESULT](../../later/index.md#later-template-candidates), [JOURNEY-CARD](../../later/index.md#later-template-candidates), [DESIGN](../../later/index.md#later-template-candidates), [DOMAIN-LANGUAGE](../../later/index.md#later-template-candidates), [MODULE-MAP](../../later/index.md#later-template-candidates), [INTERFACE-CONTRACT](../../later/index.md#later-template-candidates), [RUN-SUMMARY](../../later/index.md#later-template-candidates), [TDD-TRACE](../../later/index.md#later-template-candidates) | Detailed continuity, stewardship, TDD, diagnostic, and reporting views stay later-profile unless an owner promotes them as non-required display or later-stage scope. |

Dashboard, hosted workflow, team workflow, broader connector, automation, and analytics views are [Roadmap](../../later/index.md#roadmap-candidates) candidates, not template requirements.

## Notes

If a source record or ref does not exist, render `none`, `unknown`, `not_required`, or a blocking/unavailable note. Do not invent placeholder state to satisfy a template.

Large logs, diffs, traces, screenshots, recordings, bundles, export components, and sensitive artifact bodies should be referenced by `ArtifactRef`, not embedded by default. User-facing MVP-1 outputs may summarize integrity, ownership, availability, and redaction in plain language; diagnostic and later/full-profile outputs may show exact integrity metadata when that detail is needed. Always show omission/block notes without reconstructing omitted or blocked raw values.
