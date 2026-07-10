# Agent Guide

<a id="purpose"></a>

Use this guide when operating or reviewing an agent in a Volicord-connected
session. It explains a practical workflow. Exact API, schema, storage, security,
and close contracts stay in the [Reference Index](../reference/README.md).

<a id="operating-loop"></a>

## Operating Loop

Use this loop for tracked work:

1. Turn the request into a visible goal, current scope, non-goals, and next safe
   action.
2. Inspect available files, documentation, tests, and Volicord state before
   asking the user.
3. Ask only for a user-owned decision that changes the next safe action.
4. Refresh scope before a product-file write or sensitive action.
5. Record meaningful execution and Evidence after acting.
6. Report the primary blocker, what is known, what is missing, and one next
   safe action.
7. Before close, separate Evidence, final acceptance, residual risk, and
   remaining blockers.

Keep the process light for advice and tiny changes. Increase its weight when
the work becomes ambiguous, spans several files, changes a public interface,
introduces security or privacy risk, or depends on user-owned judgment.

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

```sh
volicord inbox --repo "<repo>"
volicord inbox answer JUDGMENT_ID --choice CHOICE_ID --repo "<repo>"
```

Exact input methods and command behavior belong to
[Agent Connection](../reference/agent-connection.md#user-channel-and-agent-connections)
and [Administrative CLI](../reference/admin-cli.md#user-channel-commands).

<a id="check-before-writes"></a>

## Check Before Writes

Before a product-file write, make the intended paths and effect specific enough
to evaluate. Request a Write Ticket through the prepare-write path, then show:

- the intended change
- whether it fits the current scope
- any pending user decision or sensitive-action approval
- stale or unavailable context
- the next action when a Write Ticket cannot be issued

If scope changes, update it before requesting another Write Ticket. Do not claim
write compatibility from a plan, stale chat context, broad enthusiasm, or a
generated summary. Exact method behavior belongs to
[Prepare-write](../reference/api/method-prepare-write.md).

<a id="record-evidence"></a>

## Record Evidence After Action

After a meaningful edit, command, review, or observation, report:

- what ran or changed
- which claim the Evidence supports
- what passed or failed
- what is missing, stale, redacted, blocked, or insufficient

Record claim-scoped Evidence through the supported run or observation path.
Evidence attachments are inputs to that record; their availability alone does
not prove a claim. Keep Evidence, Close Status, final acceptance, and
residual-risk acceptance separate.

Exact run behavior belongs to
[Record-run](../reference/api/method-record-run.md). Exact attachment behavior
belongs to [Artifact Schemas](../reference/api/schema-artifacts.md) and
[Artifact Storage](../reference/storage-artifacts.md).

<a id="reconcile-unrecorded-changes"></a>

## Reconcile Unrecorded Changes

When the Detective profile reports an Unrecorded Change, treat it as a bounded
observation. It does not prove who changed a file or that the change was
malicious.

Use `volicord.reconcile_changes` when available. If MCP is unavailable, route
the user to `volicord changes reconcile`. Any user acceptance must go through a
supported User Channel. Report unresolved Unrecorded Changes as close blockers
and name the next action.

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

Use a read-only Close Status check when the user only asks whether close is
blocked. Change task state only through the supported close path. Do not close
from prose, tests alone, broad acceptance language, a generated view, or stale
status. Final acceptance and residual-risk acceptance do not replace missing
required Evidence.

Exact close meaning belongs to [Core Model](../reference/core-model.md). Exact
method behavior belongs to [Close-task](../reference/api/method-close-task.md).

<a id="instructions-and-guidance"></a>
<a id="respect-boundaries"></a>

## Respect Scope And Guarantee Limits

Volicord guidance can steer tool choice, but it is not access control or proof
that a model followed instructions. A Write Ticket is not filesystem
permission. Detective observations are not OS enforcement or actor proof.
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
