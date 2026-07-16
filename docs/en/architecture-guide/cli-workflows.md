# CLI workflows

This guide owns architecture-level execution-flow boundaries for local
`volicord` administrative workflows. It explains how CLI orchestration combines
Runtime Home setup, installation profile preparation, Agent Connection records,
host adapters, guard integration, final-output authority disclosure,
verification, diagnostics, and rendering.

This page does not define command syntax, flags, stdout or stderr contracts,
exit codes, JSON output schemas, public API behavior, storage effects, security
guarantees, Core authority semantics, or product contracts. Use the
[Source Map](source-map.md) for exact source paths and module responsibilities.
Use [Administrative CLI](../reference/admin-cli.md) for exact command syntax,
flags, result states, output boundaries, and hidden hook command contracts. Use
[Runtime Boundaries](../reference/runtime-boundaries.md),
[Agent Connection](../reference/agent-connection.md),
[MCP Transport](../reference/mcp-transport.md), and
[Security](../reference/security.md) when exact runtime, connection, transport,
or non-guarantee wording matters.

The implementation source names setup helpers separately from connection
provisioning. Public command ownership remains with Administrative CLI; this
page uses setup workflow to mean installation-profile preparation and local CLI
orchestration, not a separate public command family.

## Workflow ownership map

| Workflow | Architecture-level owner on this page | Exact owner route |
|---|---|---|
| Setup workflow | Runtime Home resolution, installation profile preparation, command discovery, optional interactive choice, link installation, shell startup file update, and report rendering boundaries. | [Administrative CLI](../reference/admin-cli.md#runtime-home-selection) and [Runtime Boundaries](../reference/runtime-boundaries.md). |
| Connection init/add | Project registration, Agent Connection registration, host plan construction, guard integration planning or application, verification, and rendering boundaries. | [Administrative CLI](../reference/admin-cli.md#volicord-agent-install), [Agent Connection](../reference/agent-connection.md), and [MCP Transport](../reference/mcp-transport.md). |
| Connection status/verify | Stored connection facts, current host diagnostics, CLI MCP preflight, optional stdio handshake, guard audit facts, final-output disclosure capability diagnostics, and rendering boundaries. | [Administrative CLI](../reference/admin-cli.md#agent-connection-result-states), [Agent Connection](../reference/agent-connection.md), and [MCP Transport](../reference/mcp-transport.md). |
| Guard hook lifecycle | Hidden internal hook command orchestration across session-start, pre-tool, post-tool, prompt capture, and stop phases. | [Administrative CLI](../reference/admin-cli.md#guard-hook-commands), [Agent Connection](../reference/agent-connection.md), and [Security](../reference/security.md). |
| Final-output authority disclosure | Fresh read-only status refresh, shared typed receipt validation, profile-independent disclosure planning, host-native fixed UI rendering, and bounded fallback boundaries separate from Stop enforcement. | [Projection and Templates](../reference/projection-and-templates.md), [Administrative CLI](../reference/admin-cli.md), [Agent Connection](../reference/agent-connection.md), and [Security](../reference/security.md). |
| Doctor diagnostics | Read-only inspection of setup, profile, connection, host, guard, and privacy-footprint facts, then diagnostic rendering. | [Administrative CLI](../reference/admin-cli.md#runtime-home-selection), [Runtime Boundaries](../reference/runtime-boundaries.md), and [Security](../reference/security.md). |
| Host integration | Host adapter planning, apply, verify, and remove responsibilities that the CLI orchestrates. | [Administrative CLI](../reference/admin-cli.md#external-host-configuration) and [Agent Connection](../reference/agent-connection.md). |
| Guard integration | Generated file planning and application for the profile-independent final-output handler subset and fuller Detective lifecycle, plus capability metadata and factual audit helpers used by init, status, verification, and doctor. | [Administrative CLI](../reference/admin-cli.md#guard-hook-commands) and [Security](../reference/security.md). |

## Setup workflow

The setup workflow prepares local CLI execution facts before later connection
and MCP startup flows depend on them.

1. Resolve the selected Runtime Home from parsed CLI input, environment, or the
   platform default, then initialize or reuse the Runtime Home registry.
2. Discover the running `volicord` command and the MCP launch command that the
   installation profile should record. Discovery failures produce setup checks
   and named required actions instead of partially writing a profile.
3. In human text mode, the workflow may ask an interactive command-availability
   question when command paths are not ready. JSON mode stays noninteractive.
4. When a command-link directory is selected, the workflow prepares that
   directory, installs the managed command link, checks whether the directory is
   on `PATH`, and may write a managed shell startup block when the selected
   interactive choice asks for it. A shell startup file update still reports the
   need for a new shell or host restart because the running parent shell is not
   mutated.
5. The workflow writes the installation profile with command paths, selected
   binary directory, default connection mode, setup metadata, and timestamps.
6. Rendering turns checks, performed actions, optional actions, required
   actions, and profile facts into text or JSON output. `action_required`
   indicates a named local follow-up, not a public API failure or security
   finding.

The setup workflow does not register user-owned judgments, issue write tickets,
prove host trust, or define public command syntax.

## Connection init and add

Connection provisioning is local administrative orchestration. It is separate
from public Core method execution.

The planning stage parses the selected host, connection intent, profile, mode,
and repository root. It resolves or prepares Runtime Home and installation
profile facts, derives or reuses the Agent Connection identity, builds the host
configuration plan, rejects host-plan conflicts, and, for init, builds the guard
integration plan that matches the selected profile.

Dry-run provisioning stops after planning and rendering. It reports what would
be written or checked without creating Runtime Home state, registering projects
or connections, applying host configuration, applying guard integration files,
running MCP preflight, or performing tool discovery.

Non-dry-run provisioning initializes or reuses Runtime Home state, registers or
reuses the selected Product Repository project, creates or updates the Agent
Connection record, enforces the selected project membership boundary, and adds
or confirms Connection Projects membership. When init is moving to a different
host or intent, the requested project membership stays inactive while a prior
connection that was already eligible remains eligible. An explicitly disabled
prior remains disabled. A new requested Agent Connection is staged
disabled; an existing enabled connection can keep serving its other projects.
The CLI then applies the host plan through the selected host adapter. Because
that apply can create or change directories shared with guard targets, init
rebuilds the guard integration plan against the resulting filesystem state
before applying it. It derives guard installation metadata, then uses the
Store's immediate-transaction helper to record that metadata, add the
requested membership, retire superseded memberships from connections that have
other projects, and activate the requested connection together. For a
superseded connection's last project,
the helper disables it but retains the membership as durable pending-cleanup
inventory. A second immediate transaction revalidates that disabled inventory,
then releases the Registry lock before host cleanup. A final immediate
transaction revalidates the Store-owned marker and removes the retained
membership. Generic registration, enable/disable, membership mutation, and
staged-target activation reject invariant-changing operations on marked rows;
mode and verification-report updates do not change the marker.
Fresh migrations rebase older valid cleanup markers to their new replacement
and preserve unrelated disabled alternatives.
The staging upsert preserves an existing enabled bit, and a transactional
classification distinguishes inactive staging from an exact cleanup resume.
No stale planning snapshot removes an active requested membership.

Verification runs after host and guard application. It asks the host adapter for
observable host facts, runs CLI MCP preflight using the resolved Runtime Home
and Agent Connection binding, and performs direct stdio initialization and
`tools/list` discovery only when the host gate and preflight allow it. The CLI
stores the resulting last-known verification status and renders the connection
result with any user-controlled next actions.

Provisioning is not a single transaction across Runtime Home registry state,
Product Repository files, external host configuration, guard files, and MCP
process checks. If a later boundary reports a failure after earlier durable
effects were applied, init renders an explicit partial-application result and
later status, verify, project, doctor, or remove workflows can observe those
earlier effects. The narrow Registry activation transition is transactional;
the wider workflow remains a convergent multi-surface operation.

## Connection status and verify

Connection status is read-oriented. It selects one Agent Connection, reads its
connected project membership and stored verification facts, reconstructs the
managed host plan where possible, attaches current host diagnostics when the
adapter can report them, gathers guard state, and renders the stored or derived
status, including final-output disclosure capability and configuration facts.
It does not launch the host, rewrite host configuration, or refresh MCP
preflight.

Connection verify is an active diagnostic workflow. It selects one Agent
Connection, reconstructs the host plan, runs host verification, runs CLI MCP
preflight, optionally performs a direct stdio handshake and tool discovery, and
updates the connection's last-known verification report. Verification output can
combine stored connection facts, current host diagnostics, MCP command and
preflight facts, managed host lifecycle observations, final-output disclosure
capability diagnostics, and guard audit facts.

These workflows report observable facts and next actions. They do not prove
that an external host loaded, trusted, approved, initialized, or exposed a
configuration unless the relevant Reference owner defines that exact meaning.
They also do not prove OS enforcement, user approval, actor identity, product
correctness, test sufficiency, or Close Status.

## Guard hook lifecycle

Generated host wrapper files invoke the hidden internal hook namespace for
supported lifecycle phases. The CLI hook workflow resolves Runtime Home and the
registered project, normalizes the host event into a guard envelope, ensures or
records the session when required, observes guard installation activation when
the event matches recorded capability and policy facts, and dispatches to the
phase handler.

Phase handlers have distinct architecture responsibilities:

- `session-start` records or reuses an Agent Session and renders context for
  host session injection.
- `pre-tool` classifies tool attempts, checks current task and write-ticket
  compatibility where applicable, and may persist expected-write correlation
  facts.
- `post-tool` records observed tool results, correlates them with expected
  writes or current write-ticket facts, and can record unresolved observed
  Product Repository changes.
- `prompt-capture` handles prompt metadata and strict chat command handling for
  User Channel action resolutions when prompt capture is available.
- `stop` checks close-related facts through the shared typed status/receipt
  validation boundary and renders the host-native allow or deny result for
  session completion. Stop enforcement does not own the general final-output
  disclosure surface.

The event timestamp remains observation metadata for guard recording and
correlation. Current Task, write-ticket, pending UserAction, and prompt-command
eligibility reads use the project/Core current clock rather than host-reported
time, so delayed or clock-skewed events cannot rewrite current authority.

After phase handling, the CLI attaches the cooperative disclosure, persists the
guard event when it has not already been recorded, persists expected-write facts
when the phase produced them, and renders either Volicord JSON, text, or
host-native output.

Guard hook decisions are cooperative host decisions and observations. They are
not public Core methods, user-owned judgments by themselves, write tickets,
host trust, shell approval, OS sandboxing, full write prevention, actor
attribution proof, correctness proof, test sufficiency proof, or human review
replacement.

## Final-output authority disclosure

Final-output disclosure is a profile-independent host-adapter workflow, not the
Detective Stop enforcement decision. When a configured built-in adapter reports a
final-output event, the CLI requests a fresh read-only Core status for the
selected project and Task.
The shared Core-owned typed validator compares the status and candidate
`AuthorityReceipt` under the owner-defined relationships. The CLI then builds
one in-memory disclosure plan and asks the selected host adapter to render it
on a fixed host-native UI surface.

For a selected Task, the plan preserves the complete canonical receipt or a
bounded Task-specific `volicord status` fallback; it does not truncate receipt
JSON. Missing Task state, a failed refresh, or malformed or mismatched status
becomes the explicit fallback or diagnostic path owned by the applicable
Reference documents. Every final-output event repeats the read-only refresh,
including an event that follows replay. A previous mutation response, Stop
result, or model-authored answer is not cached as current authority.

Record and Detective profiles share this disclosure workflow. Detective Stop
uses the same validation facts for its separate completion-claim decision while
always allowing session termination; Record disclosure remains non-blocking. Generic, user-managed,
unsupported, inactive, or degraded host paths report only their supported
fallback and diagnostic facts rather than claiming a fixed host UI surface.

Renderer and generated-configuration fixtures verify adapter bytes and routing.
They do not establish that an actual host loaded or displayed the surface. Live
Codex and Claude Code observations belong in opt-in host integration validation.
The implementation rationale is recorded in
[Final-output authority disclosure](decisions/final-output-authority-disclosure.md);
exact behavior remains in the focused Reference owners.

## Doctor diagnostics

Doctor is a read-oriented diagnostic workflow. It resolves the Runtime Home,
inspects Runtime Home access and registry shape, reads installation profile
facts, checks stored command paths and `PATH` availability, reports registry
counts, inspects guard installation records, audits generated guard files and
capability metadata, reads available session-watch observation summaries, and
can render a privacy-footprint view.

Doctor maps factual inspection results to diagnostic checks and suggested
actions. It does not create projects, install or remove host configuration,
change Agent Connection mode, run active host verification, answer User Channel
judgments, repair guard files, or prove security, correctness, review, QA,
final acceptance, residual-risk acceptance, or Close Status.

## Host integration boundary

Host adapters own host-specific planning, application, verification, removal,
capability declarations, and conflict detection. CLI workflows choose the host,
intent, mode, profile, Runtime Home, project context, and Agent Connection
facts, then call the adapter at the plan, apply, verify, or remove boundary.

Profile-independent final-output capability contracts and validation also live
at this boundary. Guard integration applies the host-specific generated handler
plan, but the full Detective lifecycle remains distinct from the
final-output-only subset.

The CLI treats host configuration as an external integration surface. A
successful host configuration write is distinct from host trust, host approval,
host reload, active tool exposure, and model behavior. Generic external MCP host
configuration remains user-managed; the CLI can report guidance for an enabled
Agent Connection, but the resulting process must still pass MCP startup
validation and the CLI does not write arbitrary external host configuration.

## Guard integration boundary

Guard integration plans generated files, policy JSON, host event commands,
capability metadata, and factual audit inputs. The built-in Record and Detective
setup paths share the final-output handler subset; only Detective
adds the remaining lifecycle handlers and prompt-capture observations.
Application writes only the planned managed files or managed blocks. Managed-file
application pins the Product Repository parent path, compares the planned
target snapshot before commit, stages a sibling entry, uses the platform
filesystem facade when a native namespace operation is needed, and verifies the
participating entries after the operation. The CLI
caller owns cleanup, recovery inspection, and diagnostic construction; the
platform facade does not make those decisions. Audit reads recorded metadata
and generated files to classify missing, stale, broken, unsafe, or unobserved
facts for status, verification, and doctor.

Guard integration facts can support diagnostics and workflow routing. They do
not imply security guarantees, host approval, user approval, correctness proof,
complete filesystem monitoring, or that a model followed Product Repository
guidance.
