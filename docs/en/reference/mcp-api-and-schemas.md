# MCP API And Schemas

This page is now a routing page for the split API reference. It describes future Harness Server behavior for planning and review; it does not mean an MCP server or runtime implementation exists in this repository today.

Use the split documents instead of loading one large schema file:

| Need | Owner |
|---|---|
| MVP-1 active public API | [MVP API](api/mvp-api.md) |
| Shared core schemas, read-only resources, artifact refs, staged value sets | [API Schema Core](api/schema-core.md) |
| Later/profile-gated methods and future schema material | [API Schema Later](api/schema-later.md) |
| Error taxonomy, primary precedence, idempotency, state conflict behavior | [API Errors](api/errors.md) |

## MVP-1 shortcut

The MVP-1 public tool surface is intentionally small:

- `harness.intake`
- `harness.status`, including `status.next_actions`
- `harness.prepare_write`
- `harness.record_run`
- `harness.request_user_judgment`
- `harness.record_user_judgment`
- `harness.close_task`

`harness.next` is not a separate MVP-1 method. It remains later/compatibility material in [Schema Later](api/schema-later.md#harnessnext).

## Legacy anchor map

Older links to this page should be updated to the split owners above. The anchors below remain only to route readers during the documentation split.

<a id="public-tools"></a>
Public tools: [MVP API](api/mvp-api.md).

<a id="harnessstatus"></a>
`harness.status`: [MVP API: `harness.status`](api/mvp-api.md#harnessstatus).

<a id="harnessintake"></a>
`harness.intake`: [MVP API: `harness.intake`](api/mvp-api.md#harnessintake).

<a id="harnessnext"></a>
`harness.next`: [Schema Later: `harness.next`](api/schema-later.md#harnessnext).

<a id="harnessprepare_write"></a>
`harness.prepare_write`: [MVP API: `harness.prepare_write`](api/mvp-api.md#harnessprepare_write).

<a id="harnessrecord_run"></a>
`harness.record_run`: [MVP API: `harness.record_run`](api/mvp-api.md#harnessrecord_run).

<a id="harnessrequest_user_judgment"></a>
<a id="harnessrequest_user_decision"></a>
`harness.request_user_judgment`: [MVP API: `harness.request_user_judgment`](api/mvp-api.md#harnessrequest_user_judgment).

<a id="harnessrecord_user_judgment"></a>
<a id="harnessrecord_user_decision"></a>
`harness.record_user_judgment`: [MVP API: `harness.record_user_judgment`](api/mvp-api.md#harnessrecord_user_judgment).

<a id="harnessclose_task"></a>
`harness.close_task`: [MVP API: `harness.close_task`](api/mvp-api.md#harnessclose_task).

<a id="read-only-resources"></a>
Read-only resources: [Schema Core: Read-only resources](api/schema-core.md#read-only-resources).

<a id="schema-notation-convention"></a>
Schema notation convention: [Schema Core: Schema notation convention](api/schema-core.md#schema-notation-convention).

<a id="stage-profile-manifest"></a>
Stage Profile Manifest: [Schema Core: Stage Profile Manifest](api/schema-core.md#stage-profile-manifest).

<a id="stage-specific-active-value-sets"></a>
Stage-specific active value sets: [Schema Core: Stage-Specific Active Value Sets](api/schema-core.md#stage-specific-active-value-sets).

<a id="tool-envelope"></a>
Tool envelope: [Schema Core: Tool envelope](api/schema-core.md#tool-envelope).

<a id="mcp-boundary-and-caller-trust"></a>
MCP boundary and caller trust: [Schema Core: MCP boundary and caller trust](api/schema-core.md#mcp-boundary-and-caller-trust).

<a id="common-response"></a>
Common response: [Schema Core: Common response](api/schema-core.md#common-response).

<a id="shared-schemas"></a>
Shared schemas: [Schema Core: Shared schemas](api/schema-core.md#shared-schemas).

<a id="sensitive-categories"></a>
Sensitive Categories: [Schema Core: Sensitive Categories](api/schema-core.md#sensitive-categories).

<a id="artifactref"></a>
ArtifactRef: [Schema Core: ArtifactRef](api/schema-core.md#artifactref).

<a id="validatorresult"></a>
ValidatorResult: [Schema Core: ValidatorResult](api/schema-core.md#validatorresult).

<a id="error-taxonomy"></a>
Error taxonomy: [API Errors: Error taxonomy](api/errors.md#error-taxonomy).

<a id="primary-error-code-precedence"></a>
Primary Error Code Precedence: [API Errors: Primary Error Code Precedence](api/errors.md#primary-error-code-precedence).

<a id="harnessclose_task-close-blockers"></a>
`harness.close_task` Close Blockers: [API Errors: `harness.close_task` Close Blockers](api/errors.md#harnessclose_task-close-blockers).

<a id="idempotency"></a>
Idempotency: [API Errors: Idempotency](api/errors.md#idempotency).

<a id="state-conflict-behavior"></a>
State conflict behavior: [API Errors: State conflict behavior](api/errors.md#state-conflict-behavior).

<a id="harnesslaunch_verify"></a>
`harness.launch_verify`: [Schema Later: `harness.launch_verify`](api/schema-later.md#harnesslaunch_verify).

<a id="harnessrecord_eval"></a>
`harness.record_eval`: [Schema Later: `harness.record_eval`](api/schema-later.md#harnessrecord_eval).

<a id="harnessrecord_manual_qa"></a>
`harness.record_manual_qa`: [Schema Later: `harness.record_manual_qa`](api/schema-later.md#harnessrecord_manual_qa).
