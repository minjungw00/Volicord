# Agent Guide

<a id="purpose"></a>

Use this guide when operating or reviewing an agent in a Volicord-connected
session. It explains a practical workflow. Exact API, schema, storage, security,
and close contracts stay in the [Reference Index](../reference/README.md).

<a id="operating-loop"></a>

## Operating Loop

Do not memorize one universal method sequence. Start from the latest
authoritative `next_action` and follow its `owner_method` when it names one. If
there is no current Volicord result, the Task is unknown, or an authoritative
refresh is required, obtain current state first. Do not poll status between
steps merely for reassurance.

For each returned handoff:

1. Confirm that it belongs to the intended Product Repository and `Task`.
2. Inspect files, documentation, and tests that can resolve uncertainty without
   changing authority state.
3. Ask only for a user-owned decision that changes the next safe action.
4. Act only inside the current scope and compatible write or sensitive-action
   boundary shown by Volicord.
5. Record meaningful execution and Evidence after acting when the returned
   handoff calls for it.
6. Report the primary blocker, what is known, what is missing, the next actor,
   and one next safe action.
7. Keep Evidence, final acceptance, residual risk, and Task completion
   separate.

The returned handoff can legitimately say to inspect, write, record, ask the
user, wait, reconcile, or close. A later response can choose a different route
when the recorded facts change.

## Keep Agent Work And User Judgment Separate

| Moment | Agent responsibility | User responsibility |
|---|---|---|
| Shape the work | Inspect context, propose a bounded scope, and name the next safe action. | Set the goal, non-goals, and limits in ordinary language. |
| Request judgment | Show one focused question, the available options, consequences, and any bounded recommendation. | Answer, reject, defer, narrow the work, or ask for more evidence. |
| Record judgment | Route the user to a supported User Channel and avoid depending on an unrecorded answer. | Record one shown option when the answer must become Volicord state. |
| Continue or close | Refresh state, prepare writes, record Evidence, and surface blockers. | Decide final acceptance, residual-risk acceptance, cancellation, supersession, or the next blocker to address. |

An Agent Connection must not record the user's decision for them. Chat text,
generated Markdown, guidance, and status views can display a decision need, but
they are not the recorded user answer. Exact authority meaning belongs to
[Core Model](../reference/core-model.md), and exact connection boundaries belong
to [Agent Connection](../reference/agent-connection.md).

<a id="infer-use"></a>

## Infer Procedure Weight From The Work

Users do not need to say “Volicord” or name an API method before work begins.
Choose the smallest workflow that preserves the relevant boundaries:

- **Advice or inspection:** inspect available sources, state uncertainty, and
  avoid write or close ceremony.
- **Small change:** confirm narrow scope, edit inside it, run a focused check,
  and report briefly.
- **Tracked work:** clarify scope, preserve user judgment, check writes, record
  Evidence, and report Close Status.

Escalate a small change to tracked work when you find scope drift, a new public
interface, a dependency or migration choice, destructive risk, security or
privacy impact, an Evidence limit, final-acceptance need, residual risk, or
another user-owned decision.

Representative flows are intentionally not exact API sequences:

| Work shape | Follow the returned handoff |
|---|---|
| Advice or read-only investigation | Inspect the available sources, state uncertainty, and stop without creating write or close ceremony that the work does not need. |
| Narrow product-file change | Establish the Task only when needed, obtain a compatible current write authorization before editing, run focused verification, record the meaningful result, and follow the resulting close or continuation action. |
| Multi-file or long-running work | Keep scope, the current Change Unit, Evidence, and user-owned decisions visible; resume from the persisted next action rather than reconstructing a sequence from chat. |
| Waiting on the user or another blocker | Report the blocker and next actor. The session may end, but do not claim that the Task is complete. |
| Sensitive or newly expanded work | Stop before the affected action and follow the projected policy, scope, and User Channel handoff. Do not self-approve or silently keep a lighter path. |

## Keep Connection Setup And Task Control Separate

The Codex `record` profile selects connection setup; it is not a Task-risk
grade. Each Task has a separate requested and effective control level with an
owner-provided reason. Treat the effective level and project-owned policy as
authoritative, and follow returned escalation actions when scope, sensitivity,
or external effects change.

Exact values and derivation rules belong to [Core Model](../reference/core-model.md),
[Intake](../reference/api/method-intake.md), and the public schema owners.

<a id="project-selection"></a>

## Select The Project Deliberately

An Agent Connection can have more than one explicitly connected Product
Repository. Never choose a project from memory, a folder label, or the current
working directory alone.

If the target is unclear, call `volicord.list_projects`. Use the returned
`project_selector` when the workflow tool exposes that argument. If a call is
rejected because project selection is ambiguous, list the connected projects,
select the intended one, and retry.

Exact selection and omission rules belong to
[MCP Transport](../reference/mcp-transport.md) and
[Agent Connection](../reference/agent-connection.md). For operator setup, see
[Multi-Repository Agent Setup](multi-repository-agent-setup.md).

<a id="keep-context-small"></a>

## Keep Context Small

Carry only what the next action needs:

- current `Task`, scope, non-goals, and relevant paths
- current Agent Connection capability limits
- pending user-owned decisions or approvals
- Evidence summaries and gaps that affect the next claim
- current blockers, stale-state warnings, and visible residual risk
- one next safe action

Load exact Reference sections when the next action needs them. Do not inject
full schemas, DDL, templates, logs, Evidence attachment bodies, unrelated
contracts, or both language versions into every prompt.

<a id="clarify-focused"></a>
<a id="request-judgment-narrowly"></a>

## Clarify With Focused Questions

Inspect first. Ask a question only when its answer changes the next safe action
or resolves a user-owned decision. Prefer one blocking question at a time.

A useful question states:

- what was inspected and what remains uncertain
- the current goal, scope, and non-goals
- the options and their consequences
- a bounded recommendation when current facts support one
- what the answer will and will not settle
- what can safely continue if the user defers

Do not ask the user to solve something the agent can safely inspect, refresh,
retry, narrow, or record.

<a id="preserve-user-judgment"></a>
<a id="route-user-interaction"></a>

## Preserve User-Owned Judgment

The user decides product-visible behavior, material technical direction, scope
changes, new dependencies or services, security and privacy choices,
compatibility breaks, costly-to-reverse choices, sensitive actions, final
acceptance, residual-risk acceptance, cancellation, and supersession.

The agent may usually choose local implementation details that stay within
accepted scope and preserve the accepted behavior. Escalate when a detail
becomes product-visible, changes scope or verification criteria, introduces a
dependency, affects security or privacy, breaks compatibility, or becomes hard
to reverse.

Do not interpret “approved,” “looks good,” or “continue” as every pending
decision. Keep product direction, technical direction, scope, sensitive-action
approval, final acceptance, and residual-risk acceptance separate.

When a decision must become Volicord state, show the user the supported User
Channel path. The stable CLI fallback is:

```sh cli-example
volicord inbox --repo "<repo>"
volicord inbox resolve USER_ACTION_REQUEST_ID --choice CHOICE_ID --repo "<repo>"
```

The supported CLI-inbox delivery boundary belongs to
[Agent Connection](../reference/agent-connection.md#supported-surface), and
exact command behavior belongs to
[Administrative CLI](../reference/admin-cli.md#user-channel-commands).

<a id="check-before-writes"></a>

## Check Before Writes

Before a product-file write, make the intended paths and effect specific enough
to evaluate. When the returned next action routes to write preparation, obtain
or reuse a compatible current Write Ticket, then show:

- the intended change
- whether it fits the current scope
- any pending user decision or sensitive-action approval
- stale or unavailable context
- the next action when a Write Ticket cannot be issued

Compatibility includes an exact non-null binding to the current normalized
project write authority. A policy change can invalidate an earlier ticket or
make it stale even when the Task control level does not rise. Request fresh
write preparation so the proposed write is reevaluated under current policy; it
may move to `sensitive` control or require new sensitive-action approval. Final
acceptance after the write cannot replace approval that was required before it.

If scope changes, follow the returned scope handoff before another write. Do not
claim write compatibility from a plan, stale chat context, broad enthusiasm,
elapsed time alone, or a generated summary. Exact method behavior belongs to
[Prepare-write](../reference/api/method-prepare-write.md).

<a id="record-evidence"></a>

## Record Evidence After Action

After a meaningful edit, command, review, or observation, report:

- what ran or changed
- which acceptance-criterion or supplemental-claim target the Evidence supports
- what passed or failed
- what is missing, stale, redacted, blocked, or insufficient

Record target-scoped Evidence through the supported run or observation path.
Evidence attachments are inputs to that record; their availability alone does
not prove a claim. Keep Evidence, Close Status, final acceptance, and
residual-risk acceptance separate.

Exact run behavior belongs to
[Record-run](../reference/api/method-record-run.md). Exact attachment behavior
belongs to [Artifact Schemas](../reference/api/schema-artifacts.md) and
[Artifact Storage](../reference/storage-artifacts.md).

<a id="reconcile-unrecorded-changes"></a>

## Reconcile Unrecorded Changes

When Volicord reports an Unrecorded Change, treat it as a bounded
observation. It does not prove who changed a file or that the change was
malicious.

Use `volicord.reconcile_changes` when available. If MCP is unavailable, route
the user to `volicord changes reconcile`. Any user acceptance must go through a
supported User Channel. Preserve the reported confidence: a suspected change
calls for verification, not an agent-authored promotion to confirmed state.
Report the Close Status and next action projected by the owner rather than
turning every uncertain observation into a blocker.

<a id="report-status"></a>
<a id="handle-close"></a>

## Report Status And Handle Close

Lead with the primary blocker and the action that would remove it. A compact
status report includes the current work boundary, current scope, freshest
relevant facts, pending decision or approval, Evidence gap, close blocker, and
one next safe action.

Before close, show the visible close facts:

- scope and result
- checks and Evidence
- required user decisions
- visible residual risk
- remaining blockers
- the next close-unblocking action

Use a read-only Close Status check when the user asks whether close is blocked
or the current close facts need an authoritative refresh. Do not insert a
separate check merely because a memorized ritual says that completion is near
when the current response already provides a fresh close handoff. Change `Task`
state only through the supported close path. Do not close from prose, tests
alone, broad acceptance language, a generated view, or stale status. Final
acceptance and residual-risk acceptance do not replace missing required
Evidence.

If authority refresh is unavailable, disclose that Volicord state was not
verified rather than inventing a terminal result.

Exact close meaning belongs to [Core Model](../reference/core-model.md). Exact
method behavior belongs to [Close-task](../reference/api/method-close-task.md).

<a id="instructions-and-guidance"></a>
<a id="respect-boundaries"></a>

## Respect Scope And Guarantee Limits

Volicord guidance can steer tool choice, but it is not access control or proof
that a model followed instructions. A Write Ticket is not filesystem
permission. Unrecorded-change observations are not OS enforcement or actor proof.
Evidence and Close Status are not correctness, QA, deployment, or human-review
proof.

Use [Scope](../reference/scope.md) for supported and unsupported capabilities,
and [Security](../reference/security.md) for exact guarantees and
non-guarantees. Do not invent a new quality gate or waiver path in this guide.

<a id="language-context"></a>

## Language Context

Use the language needed for the current user and task. Preserve exact API names,
commands, fields, enum values, paths, and error codes. In Korean-facing work,
write ordinary concepts in natural Korean instead of carrying unnecessary
English noun chains into the prompt.

<a id="where-next"></a>

## Next Paths

- [Agent Host Setup](agent-host-setup.md) for connection setup and removal
- [Multi-Repository Agent Setup](multi-repository-agent-setup.md) for one
  connection serving several explicitly connected repositories
- [User Workflow](user-workflow.md) for the user's collaboration loop
- [Reference Index](../reference/README.md) when the next action requires an
  exact contract
