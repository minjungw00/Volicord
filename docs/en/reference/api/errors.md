# API errors

Shaping methods reject stale Task, scope, baseline, checkpoint, Change Unit, and UserAction resolution coordinates through the existing validation, conflict, and replay branches. Aggregate failures have no partial checkpoint, request, event, replay, or state-version effect.

Use this page to find the focused API error Reference page for a question. It
is a router, not a contract source; exact error contracts live in the linked
owners.

It routes to owners for:

- Public `ErrorCode` meaning, error precedence, and API response branch routing.
- Product-wide failure-category meaning and public `FailureCategory` values.
- Close-readiness blocker/API boundaries and `ToolError.details`.
- Method-specific behavior, schema data shapes, storage effects, and display wording.

## Error Routes

| Question | Owner |
|---|---|
| What a failure category means | [Failure Model](../failure-model.md) |
| Which exact `FailureCategory` identifiers the API accepts | [API Value Sets](schema-value-sets.md#failure-category-values) |
| What a public `ErrorCode` means | [API Error Codes](error-codes.md) |
| Which typed current facts accompany a workflow rejection | [API Error Details](error-details.md#workflow-rejection-detail-fields) |
| Which public error is selected | [API Error Precedence](error-precedence.md) |
| Which API response branch is used | [API Error Routing](error-routing.md) |
| Where close-readiness blockers meet API responses | [API Blocker Routing](blocker-routing.md) |
| Which machine-readable fields describe an error | [API Error Details](error-details.md) |
| Which discriminator-first, descriptor-derived issue rejects an MCP argument before Core | [MCP Transport](../mcp-transport.md#public-argument-projection) |
| Where a successful descriptor validation that disagrees with exact MCP request decoding routes | [MCP Transport](../mcp-transport.md#public-argument-projection) |
| How `volicord.close_task` produces method-specific blockers | [Close-Task Method](method-close-task.md) |

## Nearby Routes

- Method behavior: [API Methods](methods.md), then the linked method owner.
- Shared response and required `ToolError.category` envelope shape: [API Schema Core](schema-core.md).
- State and blocker shapes: [API State Schemas](schema-state.md) and [API Value Sets](schema-value-sets.md).
- Core concepts that an error may reference: [Core Model](../core-model.md).
- Storage concerns: [Storage](../storage.md).
- Display wording only: [Template Bodies](../template-bodies.md).
