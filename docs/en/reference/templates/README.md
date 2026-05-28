# Template Reference

## Used when

Use these files when you need the rendered Markdown shape for projection templates and display cards. The projection rules, authority boundaries, and freshness behavior are defined in [Document Projection Reference](../document-projection.md).

Owner boundary: templates are rendered shapes, not canonical state. They do not authorize runtime/server implementation, generated operational files, executable fixtures, or runtime data before the documentation set is accepted for implementation planning. The first product MVP target is v0.1 Kernel MVP, exercised by Kernel Smoke as its narrow conformance profile. v0.2 through v0.4 are staged packs toward the Agency-Hardened MVP reference conformance target, and v1+ Expansion remains roadmap scope unless owner docs explicitly promote and prove it.

## Template tiering

Projection templates match the API `ProjectionKind` staged/reference support tiers:

| Tier | Templates | Rule |
|---|---|---|
| Reference-required | `TASK`, `APR`, `RUN-SUMMARY`, `EVIDENCE-MANIFEST`, `EVAL`, `DIRECT-RESULT` | Staged/reference projection support must enqueue/render these when their source records exist or change. |
| Reference-optional | `MANUAL-QA`, `TDD-TRACE`, `DOMAIN-LANGUAGE`, `MODULE-MAP`, `INTERFACE-CONTRACT` | Render when policy applies, records exist, or the user/operator enables the projection. |
| Extension / optional | `DEC`, `DESIGN`, `EXPORT`, `JOURNEY-CARD` | Render only when the corresponding optional projection is enabled. |

`Reference-required` means required by staged/reference projection support after the relevant owner records exist; it does not mean every v0.1 Kernel MVP run must render every kind. v0.1 Kernel MVP requires only a minimal `TASK` projection or durable projection enqueue. v0.2+ expands evidence/projection support; Agency-Hardened/reference projection support covers the full Reference-required projection set when source records exist or change.

`TASK`, `APR`, `RUN-SUMMARY`, `EVIDENCE-MANIFEST`, `EVAL`, `DIRECT-RESULT`, and other report projections are readable views from owner records and refs. They must not redefine kernel fields, MCP schemas, SQLite DDL, gate behavior, or artifact integrity rules.

Rendered placeholders, labels, table columns, and example front matter keys are template bindings for display. A binding must either show an existing owner record field or ref, or be a derived display summary from the source records named by the template. If the source record or ref does not exist, render `none`, `unknown`, `not_required`, or an unavailable/blocking note instead of inventing state.

Compact authority displays should prefer a short refs line when several sources are relevant: `write=`, `decision=`, `approval=`, `evidence=`, `eval=`, `manual_qa=`, `acceptance=`, `residual_risk=`, `artifacts=`, `redaction=`, and `freshness=`. These labels point to existing refs, redaction state, and projection freshness only; they are not schema fields or authority records.

Derived display summaries include approval boundary lines such as `approval_covers`, `approval_does_not_cover`, and `secret_exposure_boundary`; close context, close blockers, waiver path, projection freshness, redaction availability, compact context, Journey Card, Review Stages, and judgment-context-related summaries. These names are not new canonical records, schema fields, DDL columns, `ProjectionKind` values, gates, authority inputs, or authority paths. They must not be used as validator input except through the owner records, refs, gates, artifacts, or Decision Packets they summarize.

Rendered examples should make that boundary visible to the reader. `source_state_version` names the state clock used for the render, `projection_version` or projection status names the render/template/job view, and `updated_at` names when the view was produced. Freshness lines say whether the view still matches its source records; they are not task results, gate values, approval, acceptance, evidence, close readiness, or Core state rollback.

Managed blocks are projector-owned display. Direct edits inside managed blocks are drift and should become reconcile candidates, not state changes. Human-editable sections such as `User Notes and Proposals` are proposal surfaces: they become state only through proposal -> reconcile item -> accepted Core state-changing action with the relevant `state.sqlite.task_events` row, or they remain rejected, deferred, or note-only content.

Any template that renders artifact refs must preserve `redaction_state`. Large or sensitive artifact bodies are not embedded by default. `secret_omitted` entries may show safe notes or handles and may support only visible nonsecret evidence; `blocked` entries show the committed metadata-only notice as unavailable input. Templates must not inline, reconstruct, summarize, or export omitted secret/PII values or blocked raw payload bytes.

Display fields such as `redaction_availability_summary`, omitted or blocked impact lines, and `Downstream Effect` columns are rendered summaries only. They are derived from `ArtifactRef.redaction_state`, owner records, and downstream gate, evidence, QA, verification, projection, export, or Release Handoff status.

Decision Packet visibility does not depend on a standalone `DEC` Markdown projection. Required surfaces must still show active Decision Packets through `TASK`, status/next responses, judgment-context resources, and decision-packet resources. Standalone `DEC` is only an optional rendered view when that projection is enabled.

Decision Packet displays may include reader-facing shape fields such as decision title, display judgment type, why this is needed now, what the user is deciding, options, trade-offs, recommendation, uncertainty, deferral consequence, and residual risk when relevant. Display judgment type should use the primary display categories Product / UX, Technical architecture, Security / privacy, QA / acceptance, Residual risk, or Scope / autonomy. If a decision is cross-cutting, templates should render secondary considerations in trade-offs, affected gates, risk, evidence, or follow-up instead of treating the category as exclusive. These labels help readers; they are derived display metadata, not new schema fields, `ProjectionKind` values, gates, owner records, validator inputs, or authority paths.

Display cards should distinguish three different problems: a stale projection means the readable view may lag behind its source records, stale state or stale evidence means the underlying state, baseline, or artifact inputs have moved or become insufficient, and MCP unavailable means the surface cannot reach the required Harness/Core capability. Only the owner records and Core transitions can change state.

Close and assurance displays must keep distinct labels for self-checked work, `detached_verified` assurance, waived verification, QA waiver, and residual-risk accepted `completed_with_risk_accepted` close. They may appear in the same compact card, but should not be collapsed into "done," "verified," or "accepted" without the owner refs that support each state.

## Reference-required templates

- [TASK](task.md)
- [APR](approval.md)
- [RUN-SUMMARY](run-summary.md)
- [EVIDENCE-MANIFEST](evidence-manifest.md)
- [EVAL](eval.md)
- [DIRECT-RESULT](direct-result.md)

## Reference-optional templates

- [MANUAL-QA](manual-qa.md)
- [TDD-TRACE](tdd-trace.md)
- [DOMAIN-LANGUAGE](domain-language.md)
- [MODULE-MAP](module-map.md)
- [INTERFACE-CONTRACT](interface-contract.md)

## Extension templates

- [DEC](decision-packet.md)
- [DESIGN](design.md)
- [EXPORT](export.md)
- [JOURNEY-CARD](journey-card.md)

## Display cards

- [Compact Status Card](compact-status-card.md)
- [Approval Card](approval-card.md)
- [Verification Result Card](verification-result-card.md)
- [Manual QA Card](manual-qa-card.md)

## Notes

This directory is the active reference location for projection template bodies and display card shapes.
