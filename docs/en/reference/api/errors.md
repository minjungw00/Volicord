# API errors

Use this error-family router as the first hop to focused API error owners. For exact machine-readable owner routing, use [`docs/doc-index.yaml`](../../../doc-index.yaml).

This page does not define public error code meaning, error precedence, response branch routing, close-readiness blocker/API boundaries, machine-readable error details, rendered labels, storage effects, or method-specific result payloads.

## Error Routes

| Question | Owner |
|---|---|
| What a public `ErrorCode` means | [API Error Codes](error-codes.md) |
| Which public error is selected | [API Error Precedence](error-precedence.md) |
| Which API response branch is used | [API Error Routing](error-routing.md) |
| Where close-readiness blockers meet API responses | [API Blocker Routing](blocker-routing.md) |
| Which machine-readable fields describe an error | [API Error Details](error-details.md) |
| How `harness.close_task` produces method-specific blockers | [Close-Task Method](method-close-task.md) |

## Nearby Routes

- Method behavior: [API Methods](methods.md), then the linked method owner.
- Shared response and error envelope shapes: [API Schema Core](schema-core.md).
- State and blocker shapes: [API State Schemas](schema-state.md) and [API Value Sets](schema-value-sets.md).
- Core concepts that an error may reference: [Core Model](../core-model.md).
- Storage concerns: [Storage](../storage.md).
- Display text and rendered labels: [Template Bodies](../template-bodies.md).
