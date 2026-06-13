# API errors

This document is the API error reference index. It routes public error-code, precedence, routing, and machine-readable detail questions to the focused error owner documents.

It does not define rendered labels, message copy, templates, storage rows, runtime output, or method-specific result payloads.

## Error owner documents

| Question | Owner |
|---|---|
| Public `ErrorCode` identifiers, meanings, and occurrence summaries | [API error codes](error-codes.md) |
| Primary public-error selection, precedence, stale-state conflict, and idempotency conflict behavior | [API error precedence](error-precedence.md) |
| Rejected responses, blocked results, `dry_run` previews, forbidden blocker-code use, and `close_task` blocker mapping | [API error routing](error-routing.md) |
| `ToolError.details`, detail fields, helper values, and machine-readable detail constraints | [API error details](error-details.md) |

## Related owners

- Method payload schemas, response branch shapes, and common envelopes: [API Schema Core](schema-core.md), method owners routed from [API Methods](methods.md), and the API schema owners.
- Core authority checks, user-owned judgment meaning, and close-readiness meaning: [Core Model](../core-model.md), [User-judgment methods](method-user-judgment.md), and [Close-task method](method-close-task.md).
- `CloseReadinessBlocker`, `WriteDecisionReason`, `PlannedBlocker`, and value-set field definitions: [API State Schemas](schema-state.md), [API Schema Core](schema-core.md), and [API Value Sets](schema-value-sets.md).
- Storage rows, replay rows, DDL, locks, migrations, and storage effects: [Storage Records](../storage-records.md), [Storage Effects](../storage-effects.md), and [Storage Versioning](../storage-versioning.md).
- Security guarantee wording and access-boundary claims: [Security](../security.md).
- User-facing labels, rendered message phrasing, and template wording: [Template Bodies](../template-bodies.md).
