# Approval Card Template

## Used when

Use the approval card when a pending approval needs a compact user-facing display of requested scope, purpose, boundaries, risks, alternatives, and recommendation.

## Source records

- approval record
- approval-shaped Decision Packet
- sensitive category and requested scope
- allowed paths, tools, commands, network targets, and secrets
- baseline ref
- risks, alternatives, and recommendation

Coverage placeholders such as `{approval_covers}` and `{approval_does_not_cover}` are derived display summaries from approval scope, linked Approval records, related Decision Packet refs, and current write or close context. They show the approval boundary only; the Approval record and decision path remain authoritative.

## Rendered sections

- approval requirement
- request identity
- purpose
- allowed paths
- allowed tools
- allowed commands (`allowed_commands`)
- network
- required secrets
- baseline
- risks
- alternatives
- recommendation
- what this approval covers
- what this approval does not cover
- approval question

## Full template

````text
Approval is required.
Display only: approval must still be recorded through the canonical approval decision path.

{approval_id} {category}
Request: {summary}
Purpose: {why_needed}
This approval would cover:
{approval_covers}

This approval would not cover:
{approval_does_not_cover}

Allowed paths:
{allowed_paths}

Allowed tools:
{allowed_tools}

Allowed commands:
{allowed_commands}

Network:
{allowed_network}

Required secrets:
{required_secrets}

Baseline:
{baseline_ref}

Risks:
{risks}

Alternatives:
{alternatives}

Recommendation:
{recommendation}

Do you approve this sensitive action and scope only?
````

## Notes

This template is a rendered card shape, not approval authority. Approval still requires the canonical approval decision path.

Approval does not resolve user-owned product or material technical judgment, prove correctness, replace verification, replace Manual QA, imply acceptance, accept residual risk, or create Write Authorization.

Approval cards should make the Approval boundary explicit. For example, dependency install Approval is not an architecture decision; secret access Approval is not permission to expose secret values; auth or system Approval is not a session/JWT/social-login choice; and final acceptance is not permission for additional writes.
