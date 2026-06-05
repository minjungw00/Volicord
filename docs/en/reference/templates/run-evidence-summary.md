# Run Evidence Summary Template

## Used when

Use `run-evidence-summary` after advice, a run, a check, or a change needs a minimal summary of what happened and what evidence now supports the current claim.

Implementation tier: MVP-1 User Work Loop view. Detailed [RUN-SUMMARY](later-profile/run-summary.md) and [EVIDENCE-MANIFEST](later-profile/evidence-manifest.md) reports are later/full-profile templates.

Boundary: this template displays Run and evidence refs only. It is not the evidence itself, not a full Evidence Manifest, not verification, not Manual QA, not final acceptance, not residual-risk acceptance, and not close readiness authority.

## Source records

- Run refs and command/check summaries
- changed paths or no-file outcome
- consumed Write Authorization ref, no-write basis, or attempted invalid authorization context when relevant
- ArtifactRefs, `evidence_ref` refs, `redaction_state`, and integrity or availability notes
- acceptance criteria, completion claims, or close-relevant claims supported by the evidence
- evidence gaps, stale inputs, or unresolved support
- next safe evidence action

## Rendered sections

- run or action
- changed paths
- checks
- evidence refs
- supported claims
- gaps or stale support
- redaction and availability
- next evidence action

## Full template

````text
Run/evidence summary
Display only: refs and summaries; not evidence, verification, QA, final acceptance, residual-risk acceptance, or close.

Action: {run_or_action_summary}
Changed paths: {changed_paths|none}
Checks: {checks_run_or_reason_not_run}
Write authority: {consumed_write_authorization_ref|no_product_write|attempted_invalid_ref_only|none}
Evidence summary: status={evidence_summary.status}; summary={evidence_summary.summary}
Evidence refs: {evidence_refs|none}
Artifact refs: {artifact_refs|none}; integrity={sha256_size_content_type_summary|none}; redaction={redaction_summary|none}
Supports: {supported_claims_or_criteria|none}
Still missing or stale: {evidence_gaps_or_stale_inputs|none}
Agent can safely do next: {next_evidence_action|none}
Sources/freshness: state={source_state_version}; refs={source_refs}; rendered={updated_at}; freshness={freshness_state}
````

## Notes

Evidence sufficiency is coverage, not volume. If a claim has no current supporting ref, or a critical artifact ref lacks owner relation, `sha256`, `size_bytes`, `content_type`, or `redaction_state`, show the gap and `evidence_summary.status` instead of treating a long artifact list or report prose as proof.

Only a compatible consumed Write Authorization may be displayed as write authority for a product-write Run. Attempted invalid authorization refs may be shown only as violation/audit or validator-finding context, and they must not be rendered as consumed authority or completion evidence.
