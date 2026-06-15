# API errors

Use this error-family router only as the first hop to focused API error owners. For exact machine-readable owner routing, use [`docs/doc-index.yaml`](../../../doc-index.yaml).

This page is not a contract source.

It routes to owners for:

- Public `ErrorCode` meaning, error precedence, and API response branch routing.
- Close-readiness blocker/API boundaries and `ToolError.details`.
- Method-specific behavior, schema data shapes, storage effects, and display wording.

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
- Display wording only: [Template Bodies](../template-bodies.md).
