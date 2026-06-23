# Multi-repository agent setup

Use this guide when one user-scope integration should serve multiple explicitly allowed `Product Repository` registrations.

The baseline topology is:

```mermaid
flowchart LR
  host[Codex user MCP entry]
  process["harness-mcp --integration int-codex-team"]
  allowlist[explicit integration project allowlist]
  a["project_id: acme-api<br/>/work/acme-api"]
  b["project_id: billing-api<br/>/work/billing-api"]

  host --> process
  process --> allowlist
  allowlist --> a
  allowlist --> b
```

There is one host MCP entry, one `harness-mcp --integration <integration_id>` process, one explicit allowlist, and multiple repositories selected per tool call. Adding a project does not grant every Runtime Home project. Removing access takes effect through registry state without requiring the host entry to be rewritten.

Project and local host scopes remain single-repository scopes. Use user scope for this topology.

## Executable Convention

The command examples assume you have selected one absolute directory containing both `harness` and `harness-mcp`, then exported it in the current shell:

```sh
export HARNESS_BIN="/absolute/path/to/selected/bin"
```

When building from the Harness Server source repository root, a debug build can use:

```sh
export HARNESS_BIN="$(pwd)/target/debug"
```

Replace `/absolute/path/to/selected/bin` with your real selected directory; do not copy it literally. `HARNESS_BIN` is only a shell convenience variable for these examples. Harness does not read it as runtime or host configuration. For release builds and installed-directory choices, see [Installation](../getting-started/installation.md) and [Agent host setup](agent-host-setup.md).

Administrative commands use `"$HARNESS_BIN/harness"`. The user-scope Codex install passes `--mcp-command "$HARNESS_BIN/harness-mcp"` so generated configuration stores the resolved absolute executable path, not the literal `HARNESS_BIN` variable.

## Install Product Repository A

```sh
"$HARNESS_BIN/harness" agent install \
  --host codex \
  --scope user \
  --server-name harness-main \
  --integration-id int-codex-team \
  --project-id acme-api \
  --repo-root /work/acme-api \
  --default-project-id acme-api \
  --runtime-home /Users/alex/.harness \
  --mcp-command "$HARNESS_BIN/harness-mcp"
```

This example pins `--server-name harness-main` so the host entry has a short predictable key. The option is not required; omitting it derives a stable name from `integration_id`.

The host config has one server entry:

```toml
[mcp_servers.harness-main]
command = "/absolute/path/to/selected/bin/harness-mcp"
args = ["--integration", "int-codex-team"]

[mcp_servers.harness-main.env]
HARNESS_HOME = "/Users/alex/.harness"
```

The actual generated `command` value is the resolved absolute path selected through `HARNESS_BIN`; generated TOML does not contain `HARNESS_BIN`.

## Add Product Repository B

```sh
"$HARNESS_BIN/harness" agent project add \
  --integration-id int-codex-team \
  --project-id billing-api \
  --repo-root /work/billing-api \
  --runtime-home /Users/alex/.harness
```

`harness agent project add` reuses `billing-api` if that project is already registered in the selected Runtime Home. If it is not registered, this command can register it because the required `--repo-root /work/billing-api` value is supplied, then add the integration membership. The command does not rewrite host configuration; the detailed command contract stays in [Administrative CLI](../reference/admin-cli.md).

Expected result:

```text
status: complete
allowed_projects:
  acme-api
  billing-api
verification_detail: project-specific startup preflight passed
```

Confirm the host still has one MCP server entry. The Codex config should still contain only `mcp_servers.harness-main` for this integration; it should not gain one server entry per project.

```sh
"$HARNESS_BIN/harness" agent status \
  --integration-id int-codex-team \
  --runtime-home /Users/alex/.harness
```

Status should list both `acme-api` and `billing-api` under `allowed_projects`.

## What The Agent Should Do

When a user asks which repositories are available, the agent calls the adapter utility:

```json
{"name":"harness.list_projects","arguments":{}}
```

The MCP result contains text with a JSON object like:

```json
{
  "integration_id": "int-codex-team",
  "default_project_id": "acme-api",
  "projects": [
    {
      "project_id": "acme-api",
      "repo_root": "/work/acme-api",
      "available": true,
      "is_default": true
    },
    {
      "project_id": "billing-api",
      "repo_root": "/work/billing-api",
      "available": true,
      "is_default": false
    }
  ]
}
```

For Product Repository A, the agent supplies `project_id: "acme-api"` in the public method envelope:

```json
{
  "name": "harness.status",
  "arguments": {
    "envelope": {
      "project_id": "acme-api",
      "actor_kind": "agent",
      "request_id": "req_status_acme",
      "idempotency_key": null,
      "expected_state_version": null,
      "dry_run": false,
      "locale": "en-US",
      "task_id": null
    },
    "include": {
      "task": true,
      "pending_user_judgments": true,
      "write_authority": false,
      "evidence": false,
      "close": true,
      "guarantees": true
    }
  }
}
```

For Product Repository B, the later call changes only the explicit project selector and request id:

```json
{
  "name": "harness.status",
  "arguments": {
    "envelope": {
      "project_id": "billing-api",
      "actor_kind": "agent",
      "request_id": "req_status_billing",
      "idempotency_key": null,
      "expected_state_version": null,
      "dry_run": false,
      "locale": "en-US",
      "task_id": null
    },
    "include": {
      "task": true,
      "pending_user_judgments": true,
      "write_authority": false,
      "evidence": false,
      "close": true,
      "guarantees": true
    }
  }
}
```

The agent must not guess a project ID from folder names, current working directory, MCP roots, host labels, or memory. If multiple projects are available and no explicit project or valid default is supplied, the adapter rejects the call before Core execution with actionable text like:

```text
project selection is ambiguous; call harness.list_projects and retry with envelope.project_id
```

## Defaults And Ambiguity

A valid explicit `default_project_id` lets the adapter route an omitted `project_id` to that default. Defaults are convenience, not authority. They must name an allowed project and can become unavailable if that project is inactive or execution-ineligible.

When the user's request names a repository, the agent should still use the matching `project_id` explicitly. Explicit project selection is clearest in multi-repository work and prevents accidental work against the default project.

Set or change the default without rewriting host configuration:

```sh
"$HARNESS_BIN/harness" agent project default set \
  --integration-id int-codex-team \
  --project-id billing-api \
  --runtime-home /Users/alex/.harness
```

Expected result:

```text
status: complete
prior_default_project_id: acme-api
resulting_default_project_id: billing-api
```

If the default is cleared while multiple projects remain available, omitted `project_id` calls become ambiguous. The agent should call `harness.list_projects` and retry with an explicit `envelope.project_id`.

## Remove Projects And Re-Add Later

After the default has moved to `billing-api`, Product Repository A is only a formerly default project. Remove it while retaining the integration and host MCP entry:

```sh
"$HARNESS_BIN/harness" agent project remove \
  --integration-id int-codex-team \
  --project-id acme-api \
  --runtime-home /Users/alex/.harness
```

Expected result:

```text
status: complete
allowed_projects:
  billing-api
verification_detail: project membership removed; host configuration was not rewritten
```

To remove the final remaining project, clear the default first if it still names that project, then remove the membership:

```sh
"$HARNESS_BIN/harness" agent project default clear \
  --integration-id int-codex-team \
  --runtime-home /Users/alex/.harness

"$HARNESS_BIN/harness" agent project remove \
  --integration-id int-codex-team \
  --project-id billing-api \
  --runtime-home /Users/alex/.harness
```

Expected result:

```text
status: complete
allowed_project_count: 0
not executable until one is added
```

After removal, Host Installation inventory and host configuration can remain, but that stored state is not proof of new startup eligibility. A `harness-mcp` process that was already running can refresh registry state, so `harness.list_projects` may return an empty list for `int-codex-team`; project-routed public tools cannot proceed because no allowed project remains. A newly started `harness-mcp` process, `harness-mcp --check`, and verification paths that need new MCP startup fail until a project is added again and normal configuration checks pass.

Observe the zero-project state:

```sh
"$HARNESS_BIN/harness" agent status \
  --integration-id int-codex-team \
  --runtime-home /Users/alex/.harness
```

Expected status includes:

```text
allowed_project_count: 0
not executable
```

Add a project again without reinstalling the host entry. This restores eligibility for new startup, subject to normal configuration checks:

```sh
"$HARNESS_BIN/harness" agent project add \
  --integration-id int-codex-team \
  --project-id billing-api \
  --repo-root /work/billing-api \
  --runtime-home /Users/alex/.harness
```

If the re-added project should be the convenience default again, set it after adding it:

```sh
"$HARNESS_BIN/harness" agent project default set \
  --integration-id int-codex-team \
  --project-id billing-api \
  --runtime-home /Users/alex/.harness
```

## Full Uninstall

Remove managed host configuration and managed guidance for the integration:

```sh
"$HARNESS_BIN/harness" agent uninstall \
  --integration-id int-codex-team \
  --runtime-home /Users/alex/.harness \
  --allow-repository-write \
  --remove-managed
```

Uninstall removes selected Harness-managed host configuration when ownership and safety checks allow it. With `--remove-managed`, it also removes managed repository guidance only when selected and safely owned. A successful managed uninstall removes the corresponding Host Installation inventory; if no Host Installations remain for the Agent Integration Profile, the profile can be disabled, which is not deletion. Product Repositories, project registration and project state, Core task, evidence, decision, run, and artifact-related records, artifact storage, and unrelated host entries are preserved according to their owners.

## Reference Links

- Exact host/scope and command behavior: [Administrative CLI](../reference/admin-cli.md)
- Exact Agent Integration Profile and project selection behavior: [Agent Integration](../reference/agent-integration.md)
- Exact `harness.list_projects` transport behavior: [MCP Transport](../reference/mcp-transport.md)
- Exact Product Repository write boundaries: [Runtime Boundaries](../reference/runtime-boundaries.md#explicit-integration-files-in-product-repositories)
