# Template Reference

## Used when

Use these files when you need the rendered Markdown shape for projection templates and display cards. The projection rules, authority boundaries, and freshness behavior are defined in [Document Projection Reference](../document-projection.md).

## Template tiering

Projection templates match the API `ProjectionKind` tiers:

| Tier | Templates | Rule |
|---|---|---|
| MVP-required | `TASK`, `APR`, `RUN-SUMMARY`, `EVIDENCE-MANIFEST`, `EVAL`, `DIRECT-RESULT` | MVP projector must render these. |
| MVP-optional | `MANUAL-QA`, `TDD-TRACE`, `DOMAIN-LANGUAGE`, `MODULE-MAP`, `INTERFACE-CONTRACT` | Render when policy applies, records exist, or the user/operator enables the projection. |
| Extension / optional | `DEC`, `DESIGN`, `EXPORT`, `JOURNEY-CARD` | Render only when the corresponding optional projection is enabled. |

Templates are rendered shapes, not canonical state. They must not redefine kernel fields, MCP schemas, SQLite DDL, gate behavior, or artifact integrity rules.

Rendered placeholders, labels, table columns, and example front matter keys are template bindings for display. A binding must either show an existing owner record field or ref, or be a derived display summary from the source records named by the template. If the source record or ref does not exist, render `none`, `unknown`, `not_required`, or an unavailable/blocking note instead of inventing state.

Derived display summaries include approval boundary lines such as `approval_covers`, `approval_does_not_cover`, and `secret_exposure_boundary`; close context, close blockers, waiver path, projection freshness, redaction availability, compact context, Journey Card, and judgment-context-related summaries. These names are not new canonical records, schema fields, DDL columns, `ProjectionKind` values, gates, authority inputs, or authority paths. They must not be used as validator input except through the owner records, refs, gates, artifacts, or Decision Packets they summarize.

Rendered examples should make that boundary visible to the reader. `source_state_version` names the state clock used for the render, `projection_version` or projection status names the render/template/job view, and `updated_at` names when the view was produced. Freshness lines say whether the view still matches its source records; they are not task results, gate values, approval, acceptance, or evidence.

Managed blocks are projector-owned display. Direct edits inside managed blocks are drift and should become reconcile candidates, not state changes. Human-editable sections such as `User Notes and Proposals` are proposal surfaces: they become state only after reconcile or another Core state-changing path appends the relevant `state.sqlite.task_events` row.

Any template that renders artifact refs must preserve `redaction_state`. Large or sensitive artifact bodies are not embedded by default. `secret_omitted` entries may show safe notes or handles and may support only visible nonsecret evidence; `blocked` entries show the committed metadata-only notice as unavailable input. Templates must not inline, reconstruct, summarize, or export omitted secret/PII values or blocked raw payload bytes.

Display fields such as `redaction_availability_summary`, omitted or blocked impact lines, and `Downstream Effect` columns are rendered summaries only. They are derived from `ArtifactRef.redaction_state`, owner records, and downstream gate, evidence, QA, verification, projection, export, or Release Handoff status.

Decision Packet visibility does not depend on a standalone `DEC` Markdown projection. MVP surfaces must still show active Decision Packets through `TASK`, status/next responses, judgment-context resources, and decision-packet resources. Standalone `DEC` is only an optional rendered view when that projection is enabled.

Display cards should distinguish three different problems: a stale projection means the readable view may lag behind its source records, stale state or stale evidence means the underlying state, baseline, or artifact inputs have moved or become insufficient, and MCP unavailable means the surface cannot reach the required Harness/Core capability. Only the owner records and Core transitions can change state.

## MVP-required templates

- [TASK](task.md)
- [APR](approval.md)
- [RUN-SUMMARY](run-summary.md)
- [EVIDENCE-MANIFEST](evidence-manifest.md)
- [EVAL](eval.md)
- [DIRECT-RESULT](direct-result.md)

## MVP-optional templates

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
