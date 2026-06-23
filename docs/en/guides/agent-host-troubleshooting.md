# Agent host troubleshooting

Use this guide when a Codex, Claude Code, or generic MCP host integration does
not reach the state you expected after `harness agent install`, `harness agent
verify`, `harness agent status`, project membership changes, or uninstall.

For the normal setup path, use [Agent host setup](agent-host-setup.md). For one
user-scope integration serving multiple repositories, use
[Multi-repository agent setup](multi-repository-agent-setup.md).

This guide helps you identify the observed state, check likely causes without
making another change when possible, take a bounded recovery action, and verify
the result. It does not redefine CLI behavior, MCP process behavior, storage
effects, host adapter behavior, or security guarantees. Exact behavior remains
with [Administrative CLI](../reference/admin-cli.md),
[MCP Transport](../reference/mcp-transport.md),
[Runtime Boundaries](../reference/runtime-boundaries.md),
[Agent Integration](../reference/agent-integration.md), and the storage owners
routed from [Storage](../reference/storage.md).

## Before You Change Anything

Keep the same placeholder values you used during setup:

- `HARNESS_BIN` is the selected directory containing `harness` and
  `harness-mcp`.
- `HARNESS_HOME` or `--runtime-home` is the selected `Harness Runtime Home`.
- `<integration_id>`, `<project_id>`, `<repo_root>`, `<installation_id>`, and
  `<server_name>` are the actual values from your setup output.

Start with read-only or non-mutating checks when they are available:

```sh
"$HARNESS_BIN/harness" agent status \
  --integration-id <integration_id> \
  --runtime-home <runtime_home>

HARNESS_HOME=<runtime_home> \
"$HARNESS_BIN/harness-mcp" --check --integration <integration_id>
```

`harness agent status` reports registry and Host Installation inventory. It
does not prove that Codex or Claude Code loaded the MCP server. `harness-mcp
--check` validates startup for the MCP process only. Complete host verification
requires the administrative verification gates defined by
[Administrative CLI](../reference/admin-cli.md#agent-setup-result-states).

## Executable And Environment Problems

<a id="missing-harness-mcp"></a>
### `harness-mcp` is missing, not executable, or cannot be resolved

- **Observable symptom:** Setup, verification, or host startup reports that
  `harness-mcp` is missing, unavailable, not executable, or not found on
  `PATH`.
- **Most likely causes:** The selected executable directory does not contain
  both `harness` and `harness-mcp`; the file is not executable by the selected
  user; project-scoped host configuration stores `harness-mcp`, but the future
  host process does not receive a `PATH` that can resolve it.
- **Diagnostic check:** For an absolute command, run `test -x
  "$HARNESS_BIN/harness-mcp"` and `"$HARNESS_BIN/harness-mcp" --version`. For a
  project-scoped portable command, run `command -v harness-mcp` from the same
  shell, launcher, or service environment that will start the host.
- **Bounded recovery action:** Select or build one executable directory that
  contains both executables. For user, local, or generic export scope, rerun
  install or verify with an absolute `--mcp-command`. For project scope, keep
  the generated host entry portable and fix the host launch `PATH`.
- **Verification:** Rerun `harness-mcp --check --integration <integration_id>`
  with the intended `HARNESS_HOME`, then rerun `harness agent verify` for the
  affected integration or installation.
- **Durable effects that may already exist:** Runtime Home records, project
  membership, Host Installation inventory, host configuration, or guidance may
  already exist if the failure happened after durable setup began. Read
  `effects` and `residual_effects` before repeating installation.
- **State or files that should remain untouched:** Do not delete the Runtime
  Home, project state, Product Repository files, or unrelated host
  configuration just because the executable could not be resolved.
- **Owner links:** [System Requirements](../reference/system-requirements.md),
  [Administrative CLI](../reference/admin-cli.md), and
  [MCP Transport](../reference/mcp-transport.md#configuration-preflight).

<a id="wrong-absolute-mcp-command"></a>
### An absolute `--mcp-command` is wrong

- **Observable symptom:** The CLI rejects `--mcp-command`, or verification
  later reports that the configured command is missing, changed, unavailable,
  or cannot be launched.
- **Most likely causes:** The path is not absolute, points at a stale build
  output, points at `harness` instead of `harness-mcp`, or no longer exists
  after a rebuild or move.
- **Diagnostic check:** Run `test -x /absolute/path/to/harness-mcp` and
  `/absolute/path/to/harness-mcp --help` without changing host configuration.
- **Bounded recovery action:** Rerun `harness agent install` for the same
  `integration_id`, host, scope, and server name with the corrected absolute
  `--mcp-command`. If the existing managed entry was changed by the user, use
  replacement only when the managed fingerprint or ownership marker still shows
  that the content is Harness-managed.
- **Verification:** Rerun `harness agent verify --integration-id
  <integration_id> --runtime-home <runtime_home>`. Inspect `host` and
  `installation_verifications` in JSON output when you need the exact target.
- **Durable effects that may already exist:** The Host Installation inventory
  can still point to the earlier config target and managed fingerprint until
  the replacement succeeds.
- **State or files that should remain untouched:** Do not edit
  `registry.sqlite` or overwrite unrelated host entries by hand.
- **Owner links:** [Administrative CLI](../reference/admin-cli.md),
  [Agent Integration](../reference/agent-integration.md#host-installation), and
  [Runtime Boundaries](../reference/runtime-boundaries.md#runtime-location-server-installation).

<a id="portable-project-command-not-on-path"></a>
### A portable project-scoped command is not available on host `PATH`

- **Observable symptom:** Project-scoped Codex or Claude Code configuration
  contains `command = "harness-mcp"` or `"command": "harness-mcp"`, but a later
  host session cannot start Harness, or verification fails unless you add
  `PATH="$HARNESS_BIN:$PATH"` to the administrative command.
- **Most likely causes:** Project-scoped configuration intentionally omits
  personal build paths and personal `HARNESS_HOME`; the future host process was
  started from an environment that cannot resolve `harness-mcp`.
- **Diagnostic check:** From the host launch environment, run `command -v
  harness-mcp`. If the intended Runtime Home is not the default, confirm the
  same launch environment supplies `HARNESS_HOME`.
- **Bounded recovery action:** Change the host launch environment, shell
  startup, service configuration, or equivalent host-owned path so it can
  resolve `harness-mcp`. Keep the project-scoped host file portable.
- **Verification:** Start or reload the host from that environment, then run
  `harness agent verify` with the selected directory on the administrative
  command `PATH`.
- **Durable effects that may already exist:** Project-scoped `.codex/config.toml`
  or `.mcp.json`, Runtime Home records, and Host Installation inventory may
  already be present and correct.
- **State or files that should remain untouched:** Do not replace the
  project-scoped `harness-mcp` command with a personal absolute build path in a
  shared Product Repository file.
- **Owner links:** [Administrative CLI](../reference/admin-cli.md),
  [System Requirements](../reference/system-requirements.md),
  and [Runtime Boundaries](../reference/runtime-boundaries.md#explicit-integration-files-in-product-repositories).

## Location And File Problems

<a id="runtime-home-product-repository-overlap"></a>
### Runtime Home and Product Repository are incorrectly placed or overlap

- **Observable symptom:** Project registration, setup reuse, MCP startup,
  project listing, or public tool routing rejects a project with a path
  separation or registration invariant error, such as a same-path or
  ancestor-descendant relationship.
- **Most likely causes:** The selected `Harness Runtime Home` is the Product
  Repository, is inside it, or contains it. A stored registration may also point
  at a project state path that no longer matches the registered project home.
- **Diagnostic check:** Compare the resolved Runtime Home path and repository
  root before changing anything. Use `harness agent status` or `harness project
  list` to observe the registered state; invalid operational rows are rejected
  instead of being returned as normal projects.
- **Bounded recovery action:** Choose a separate Runtime Home and Product
  Repository. Register or install against the corrected paths through the
  administrative CLI. If an old invalid row exists, treat it as data to
  diagnose; do not repair it by editing SQLite.
- **Verification:** Rerun `harness agent install --dry-run` or `harness project
  list`, then run `harness-mcp --check --integration <integration_id>` after
  any corrected setup has been applied.
- **Durable effects that may already exist:** Raw registry content may remain
  inspectable even when operational lookup rejects it. Runtime Home and project
  state locations may already exist.
- **State or files that should remain untouched:** Do not move, delete, or
  rewrite Product Repository contents or Runtime Home databases manually to
  make the paths fit.
- **Owner links:** [Runtime Boundaries](../reference/runtime-boundaries.md#runtime-home-product-repository-separation),
  [Storage Records](../reference/storage-records.md), and
  [Administrative CLI](../reference/admin-cli.md).

<a id="host-config-read-write-failure"></a>
### A host configuration file cannot be read or written

- **Observable symptom:** Install, verify, guidance, or uninstall reports that
  a configuration target is a directory, not UTF-8 text, malformed JSON or
  TOML, unsupported filesystem type, changed since planning, or cannot be
  read, created, written, moved, or removed.
- **Most likely causes:** The host target path is not a normal file, the parent
  directory is unavailable, permissions block the selected user, the file
  changed between planning and writing, or the existing host format is
  malformed.
- **Diagnostic check:** Use `harness agent install --dry-run --output json` or
  `harness agent uninstall --dry-run --output json` to preview the exact target
  path. Inspect that path with ordinary filesystem tools without editing it.
- **Bounded recovery action:** Fix the host-owned file or directory condition,
  then rerun the same administrative command. If the content changed, preserve
  unrelated entries and replace or remove only Harness-managed content whose
  marker or fingerprint still matches.
- **Verification:** Rerun `harness agent status` to inspect inventory and then
  `harness agent verify` when the host target is readable again.
- **Durable effects that may already exist:** Runtime Home state, Host
  Installation inventory, host configuration, or guidance may have been applied
  before the read or write failure. Use `effects` and `residual_effects` to
  identify the exact target.
- **State or files that should remain untouched:** Do not delete the whole host
  configuration file or Product Repository. Preserve unrelated host entries,
  user edits, and unmanaged guidance.
- **Owner links:** [Administrative CLI](../reference/admin-cli.md#setup-output),
  [Runtime Boundaries](../reference/runtime-boundaries.md),
  and [Agent Integration](../reference/agent-integration.md#host-installation).

<a id="managed-fingerprint-conflict"></a>
### Managed configuration fingerprint conflict or user-modified content

- **Observable symptom:** Install, verify, guidance, or uninstall reports a
  changed managed entry, fingerprint mismatch, unrelated entry, or conflict for
  the same server name.
- **Most likely causes:** A user or host changed a Harness-managed block or
  MCP entry after Harness last recorded its fingerprint, or the same
  `<server_name>` is already used by unmanaged host configuration.
- **Diagnostic check:** Run `harness agent status --output json` and inspect
  the reported host target, `managed_fingerprint`, `fingerprint_state`, or
  warning text. Compare only the named host entry or managed block.
- **Bounded recovery action:** If the current content is still Harness-managed
  and you intend to replace it, rerun install or guidance apply with
  `--replace-managed`. If you intend to remove managed content, use uninstall
  or guidance remove with `--remove-managed` only when ownership checks permit
  it. Otherwise choose a different `--server-name` or preserve the user-owned
  entry.
- **Verification:** Rerun `harness agent verify` after replacement, or
  `harness agent status` after preserving or removing the managed entry.
- **Durable effects that may already exist:** Host Installation inventory can
  retain the prior fingerprint, and verification can record `failed` for the
  changed installation.
- **State or files that should remain untouched:** Do not overwrite or remove
  unrelated host configuration, unmanaged host entries, or user-edited guidance
  to satisfy a Harness fingerprint.
- **Owner links:** [Administrative CLI](../reference/admin-cli.md#noninteractive-approval-behavior),
  [Agent Integration](../reference/agent-integration.md#host-installation), and
  [Runtime Boundaries](../reference/runtime-boundaries.md#explicit-integration-files-in-product-repositories).

## Host-Owned Follow-Up

<a id="codex-availability-or-trust"></a>
### Codex executable availability or project trust is incomplete

- **Observable symptom:** Result status is `action_required`; verification
  shows the Codex executable is unavailable, `codex --version` failed, or
  project trust was not confirmed.
- **Most likely causes:** `codex` is not on the administrative process `PATH`,
  cannot run `codex --version`, or Codex has not trusted the project that owns
  a project-scoped `.codex/config.toml`.
- **Diagnostic check:** Run `command -v codex` and `codex --version` from the
  environment used for administrative verification. Use `harness agent status`
  to confirm the managed configuration target.
- **Bounded recovery action:** Install or repair Codex availability for the
  selected user, then complete the Codex project trust step in Codex when
  project scope is used.
- **Verification:** Rerun `harness agent verify --integration-id
  <integration_id> --runtime-home <runtime_home>`.
- **Durable effects that may already exist:** Runtime Home records and Codex
  configuration may already be installed; `last_verified_status` may be
  `action_required`.
- **State or files that should remain untouched:** Do not edit Harness storage
  to force `complete`, and do not remove Codex configuration that still matches
  the managed fingerprint.
- **Owner links:** [System Requirements](../reference/system-requirements.md),
  [Administrative CLI](../reference/admin-cli.md#agent-setup-result-states), and
  [Agent Integration](../reference/agent-integration.md#host-installation).

<a id="claude-project-approval"></a>
### Claude Code project MCP approval remains incomplete

- **Observable symptom:** Project-scoped Claude Code install or verify reports
  `status: action_required`, or `claude mcp get <server_name>` reports pending
  approval.
- **Most likely causes:** The project `.mcp.json` entry exists, but Claude Code
  has not yet accepted the project-scoped MCP server.
- **Diagnostic check:** From the Product Repository, run `claude mcp get
  <server_name>` without editing `.mcp.json`.
- **Bounded recovery action:** Approve the project MCP server through Claude
  Code's host-owned approval flow, then reload or restart the host if Claude
  Code requires it.
- **Verification:** Rerun `harness agent verify`. A pending approval can still
  allow a diagnostic MCP handshake, but the final status remains
  `action_required` until the host approval gate is satisfied.
- **Durable effects that may already exist:** `.mcp.json`, Runtime Home
  records, project membership, and Host Installation inventory may already be
  present.
- **State or files that should remain untouched:** Do not remove or rewrite
  `.mcp.json` only to bypass approval. Keep unrelated Claude Code MCP entries
  intact.
- **Owner links:** [Administrative CLI](../reference/admin-cli.md#agent-setup-result-states),
  [Agent Integration](../reference/agent-integration.md#host-installation), and
  [Runtime Boundaries](../reference/runtime-boundaries.md).

<a id="already-running-process-stale"></a>
### An already-running MCP process does not reflect newly changed integration state

- **Observable symptom:** You changed integration membership, default project,
  host configuration, or server command, but an open Codex or Claude Code
  session behaves as if the old state is still in effect.
- **Most likely causes:** A running `harness-mcp` process is bound to one
  `integration_id` for its lifetime. Registry membership changes can be
  observed by a running process, but a changed integration binding, changed
  command, changed host configuration, reload, or restart is host-owned.
- **Diagnostic check:** Use `harness agent status` to inspect stored inventory.
  If the existing MCP session can still call tools, call `harness.list_projects`
  to see which allowed projects that running process observes.
- **Bounded recovery action:** For membership-only changes, retry the tool call
  with an explicit `project_id` after `harness.list_projects` reflects the new
  list. For changed host configuration, command path, server name, or
  integration binding, reload or restart the host so it starts a new MCP
  process.
- **Verification:** After restart, run `harness agent verify` and, inside the
  host, use `harness.list_projects` before project-routed calls when selection
  is unclear.
- **Durable effects that may already exist:** Runtime Home registry changes
  and host configuration can already be committed even while an old process is
  still running.
- **State or files that should remain untouched:** Do not duplicate Host
  Installation records or add another server entry just to refresh a running
  process.
- **Owner links:** [MCP Transport](../reference/mcp-transport.md) and
  [Agent Integration](../reference/agent-integration.md).

## Result Statuses

<a id="status-action_required"></a>
### `status: action_required`

- **Observable symptom:** `harness agent install` or `harness agent verify`
  exits successfully and reports `status: action_required`.
- **Most likely causes:** Durable integration state and host configuration are
  present, but a host-owned trust, approval, OAuth, reload, restart, or
  comparable action remains.
- **Diagnostic check:** Inspect the `action_required` array, `verification`
  details, host gate fields, and Host Installation inventory in `--output json`.
- **Bounded recovery action:** Complete only the named host-owned action:
  trust the project, approve the MCP server, reload or restart the host, or
  repair the host executable availability.
- **Verification:** Rerun `harness agent verify` for the integration or a
  specific `--installation-id`.
- **Durable effects that may already exist:** The Runtime Home records, managed
  host configuration, Host Installation inventory, and optional guidance are
  expected to exist.
- **State or files that should remain untouched:** Do not roll back or delete
  the integration just because `action_required` appeared; it is not a failure
  result by itself.
- **Owner links:** [Administrative CLI](../reference/admin-cli.md#agent-setup-result-states)
  and [Agent Integration](../reference/agent-integration.md#host-installation).

<a id="status-partial_failure"></a>
### `status: partial_failure`

- **Observable symptom:** An agent command exits with status `1` and reports
  `status: partial_failure`.
- **Most likely causes:** Some durable administrative action succeeded, but a
  later install, verify, host target, rollback, cleanup, or persistence step
  failed.
- **Diagnostic check:** Read `effects`, `residual_effects`, warnings, and
  `installation_verifications` in JSON output. Each residual effect names the
  component, target, current state, reason, and recommended action.
- **Bounded recovery action:** Fix the reported cause, then address only the
  named residual targets. Use uninstall, guidance remove, project default, or
  project membership commands instead of broad file or database deletion.
- **Verification:** Rerun the command that failed, or run `harness agent verify`
  after a setup or host-state fix. Confirm residual effects are gone or
  intentionally preserved by owner-supported commands.
- **Durable effects that may already exist:** Host configuration, guidance,
  Host Installation inventory, Runtime Home creation or migration, project
  registration, surface registration, integration records, default project, or
  membership rows may remain when reported as residual effects.
- **State or files that should remain untouched:** Do not delete the entire
  Runtime Home, Product Repository, artifact storage, Core records, unrelated
  host entries, or user-edited guidance.
- **Owner links:** [Administrative CLI](../reference/admin-cli.md#setup-output),
  [Runtime Boundaries](../reference/runtime-boundaries.md), and
  [Storage Records](../reference/storage-records.md).

<a id="status-failed"></a>
### `status: failed`

- **Observable symptom:** Install or verify exits with status `1` and reports
  `status: failed`.
- **Most likely causes:** The requested operation did not establish usable
  durable integration state or host configuration, or verification selected a
  Host Installation that is now missing, changed, rejected, unavailable,
  unknown, or failed.
- **Diagnostic check:** Read `verification.details`, `warnings`, `effects`,
  and `residual_effects`. When `residual_effects` is empty, the command does
  not know about remaining applied effects.
- **Bounded recovery action:** Fix the reported root cause, run a dry-run when
  the next command could write files, then retry install or verify. If verify
  says no Host Installation exists, run install for the intended host first.
- **Verification:** Rerun `harness agent status`, then `harness agent verify`
  after the root cause is corrected.
- **Durable effects that may already exist:** Pre-existing Runtime Home,
  project state, or host configuration can remain. New effects from the failed
  operation should be identified in `effects` and `residual_effects` if they
  remain known.
- **State or files that should remain untouched:** Do not infer that failure
  authorizes deleting user data, Product Repository contents, or unrelated
  configuration.
- **Owner links:** [Administrative CLI](../reference/admin-cli.md#agent-setup-result-states)
  and [Administrative CLI](../reference/admin-cli.md#setup-output).

## Project Selection And Membership Problems

<a id="no-allowed-project"></a>
### No explicitly allowed project exists

- **Observable symptom:** Status warns that the integration has no allowed
  projects, `harness-mcp --check` fails startup validation, verification fails,
  or `harness.list_projects` returns an empty list from a process that started
  before the last project was removed.
- **Most likely causes:** The final integration project membership was removed,
  or no membership was successfully created for the Agent Integration Profile.
- **Diagnostic check:** Run `harness agent status --integration-id
  <integration_id> --runtime-home <runtime_home>`. If an MCP process is already
  running, call `harness.list_projects` to observe whether it now sees an empty
  allowlist.
- **Bounded recovery action:** Add or restore one explicit project:
  `harness agent project add --integration-id <integration_id> --project-id
  <project_id> --repo-root <repo_root> --runtime-home <runtime_home>`. Set a
  default only if that convenience default should be used.
- **Verification:** Run `harness-mcp --check --integration <integration_id>`,
  then `harness agent verify`.
- **Durable effects that may already exist:** Agent Integration Profile, Host
  Installation inventory, host configuration, and guidance can remain while no
  project is allowed.
- **State or files that should remain untouched:** Do not reinstall the host
  entry or remove host configuration solely because the allowlist is empty.
- **Owner links:** [Agent Integration](../reference/agent-integration.md),
  [MCP Transport](../reference/mcp-transport.md), and
  [Administrative CLI](../reference/admin-cli.md).

<a id="ambiguous-project-selection"></a>
### More than one allowed project exists without a usable selector or default

- **Observable symptom:** A public MCP tool call is rejected before Core
  execution with actionable text such as `project selection is ambiguous; call
  harness.list_projects and retry with envelope.project_id`.
- **Most likely causes:** Multiple projects are available, the call omitted
  `envelope.project_id`, and no valid explicit `default_project_id` can be
  used.
- **Diagnostic check:** Call the read-only adapter utility
  `harness.list_projects` and inspect the returned project ids,
  availability, and default status.
- **Bounded recovery action:** Retry the public tool call with an explicit
  `envelope.project_id`. If omitted selection is truly desired, set a default
  with `harness agent project default set --integration-id <integration_id>
  --project-id <project_id>`.
- **Verification:** Retry the rejected tool call and confirm it reaches the
  intended project.
- **Durable effects that may already exist:** None from the rejected public
  call; ambiguous selection is rejected before Core execution.
- **State or files that should remain untouched:** Do not guess from folder
  names, current working directory, MCP roots, host labels, or memory. Do not
  remove projects to make selection easier.
- **Owner links:** [Agent Integration](../reference/agent-integration.md#current-surface-context)
  and [MCP Transport](../reference/mcp-transport.md).

<a id="default-project-invalid-or-blocking-removal"></a>
### A default project is invalid or must be cleared before removal

- **Observable symptom:** Removing a project fails because it is still
  `default_project_id`, or startup/selection cannot use the stored default
  because the project is no longer allowed, active, available, or executable.
- **Most likely causes:** The default still names the project you are trying to
  remove, or the default is stale relative to current membership or project
  availability.
- **Diagnostic check:** Run `harness agent status --output json` and inspect
  `default_project_id`, `allowed_projects`, and any project availability
  warnings.
- **Bounded recovery action:** If another allowed project should remain the
  convenience default, run `harness agent project default set`. If the final
  project is being removed, run `harness agent project default clear` first,
  then remove the membership.
- **Verification:** Rerun `harness agent status`. For new startup eligibility,
  run `harness-mcp --check --integration <integration_id>` after at least one
  project is allowed again.
- **Durable effects that may already exist:** The membership remains until the
  default is changed or cleared and removal succeeds. Host configuration is not
  rewritten by default changes or membership removal.
- **State or files that should remain untouched:** Do not edit host
  configuration or registry rows manually to bypass the default-project rule.
- **Owner links:** [Administrative CLI](../reference/admin-cli.md)
  and [Agent Integration](../reference/agent-integration.md).

<a id="storage-version-unsupported"></a>
### Registry or project-state storage version is unsupported

- **Observable symptom:** Dry-run, install, status, verify, project listing, or
  MCP startup reports that a registry or project state schema version, migration
  row, or storage profile is unsupported.
- **Most likely causes:** The Runtime Home or project state database was
  created by an unsupported profile, a newer build, a corrupt or partial
  migration, or the exact old `baseline_sqlite` profile.
- **Diagnostic check:** Prefer `harness agent install --dry-run --output json`
  or `harness agent status --output json` so no migration or repair is
  attempted by the diagnostic command.
- **Bounded recovery action:** Stop using that Runtime Home with this checkout.
  Reinitialize an explicit new Runtime Home when you want a fresh baseline, or
  restore from a compatible backup if you need existing records.
- **Verification:** Rerun the same dry-run or status command against the
  selected compatible Runtime Home. Then run normal setup or `harness-mcp
  --check`.
- **Durable effects that may already exist:** Unsupported existing SQLite files
  remain where they are; the baseline path does not convert, delete, rewrite,
  or silently migrate them.
- **State or files that should remain untouched:** Do not edit migration rows,
  storage profile values, registry tables, or project `state.sqlite` by hand.
- **Owner links:** [Storage Versioning](../reference/storage-versioning.md),
  [Storage Records](../reference/storage-records.md), and
  [Administrative CLI](../reference/admin-cli.md#dry-run).

## Removal And Cleanup Problems

<a id="partial-removal"></a>
### Removal completed only partially

- **Observable symptom:** `harness agent uninstall` exits with status `1` and
  reports `status: partial_failure`, often with warnings such as residual
  guidance preserved.
- **Most likely causes:** Managed host configuration was removed, but managed
  repository guidance could not be safely removed; a host entry or guidance
  block changed after planning; or a file operation failed during cleanup.
- **Diagnostic check:** Run `harness agent uninstall --dry-run --output json`
  with the same `--integration-id`, `--installation-id` if used,
  `--allow-repository-write`, and `--remove-managed` flags to preview the
  exact remaining targets.
- **Bounded recovery action:** Resolve only the named residual guidance or host
  target. Rerun uninstall or guidance remove with `--remove-managed` when
  ownership markers still match.
- **Verification:** Run `harness agent status` to confirm remaining Host
  Installation inventory and guidance status. If host configuration remains,
  run `harness agent verify` before relying on it.
- **Durable effects that may already exist:** Some Host Installation inventory
  may already be removed, the Agent Integration Profile may be disabled when no
  installations remain, and some guidance or host files may remain as reported.
- **State or files that should remain untouched:** Do not delete the Product
  Repository, project registration, Core records, Runtime Home location,
  artifact storage, or unrelated host entries.
- **Owner links:** [Administrative CLI](../reference/admin-cli.md),
  [Runtime Boundaries](../reference/runtime-boundaries.md#explicit-integration-files-in-product-repositories),
  and [Agent Integration](../reference/agent-integration.md#host-installation).

<a id="host-config-remains-zero-projects"></a>
### Host configuration remains while no project is currently allowed

- **Observable symptom:** `harness agent status` lists Host Installation
  inventory or host configuration, but `allowed_project_count: 0` or a warning
  says the integration is not executable until one is added.
- **Most likely causes:** The last allowed project was intentionally removed.
  Host configuration and inventory can remain, but they are not startup
  eligibility.
- **Diagnostic check:** Run `harness agent status --integration-id
  <integration_id>` and, if a previous MCP process is still alive, call
  `harness.list_projects` to see whether it returns an empty list.
- **Bounded recovery action:** If the integration should be usable again, add a
  project with `harness agent project add`. If the integration should be fully
  removed, run `harness agent uninstall --remove-managed` with the required
  repository-write flag when guidance may be removed.
- **Verification:** For reuse, run `harness-mcp --check` and `harness agent
  verify` after adding a project. For removal, rerun `harness agent status` and
  inspect remaining installations and guidance.
- **Durable effects that may already exist:** Agent Integration Profile, Host
  Installation inventory, host configuration, and guidance can remain after
  the allowlist becomes empty.
- **State or files that should remain untouched:** Do not treat the remaining
  host file as proof that new startup can succeed, and do not delete unrelated
  host entries.
- **Owner links:** [Agent Integration](../reference/agent-integration.md),
  [MCP Transport](../reference/mcp-transport.md), and
  [Administrative CLI](../reference/admin-cli.md).
