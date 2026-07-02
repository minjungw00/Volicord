# Volicord

**AI moves. Judgment stays yours.**

**[English](README.md)** | [한국어](README.ko.md)

## Overview

Volicord is a local work-authority system for AI-assisted product work. It
gives an agent host, such as Codex or Claude Code, a local record of work facts
that should not live only in chat: what task is active, what writes are
compatible with the current scope, what evidence exists, what judgment still
belongs to the user, and what blocks an honest close.

Volicord is not a replacement for your editor, shell, tests, code review, or
judgment. It is a local authority-record layer that helps an agent use those
things without hiding scope, evidence, user decisions, or close blockers inside
a polished summary.

Core is the local authority record for Volicord state. Chat messages, generated
Markdown, status summaries, and projections can describe Core state, but they
do not replace it.

## Why Volicord Exists

Volicord helps keep these questions explicit during agent-assisted product
work:

- What is the agent trying to do?
- What is in scope and out of scope?
- What evidence supports the current claim?
- Is a write ready under the current scope?
- What did the agent run or record?
- Which user-owned decision is still needed?
- What still blocks an honest close?

AI agents can inspect files, run tools, edit code, and summarize results faster
than a human can keep every boundary in working memory.

That speed is useful, but it can blur boundaries if the durable record lives
only in chat. Scope can drift. Acceptance can sound implied. Residual risk can
disappear from the conversation. A product decision can be hidden inside an
implementation step.

Volicord exists so scope, evidence, write readiness, user
judgment, run records, and close readiness stay visible as separate workflow
facts.

## Mental Model

Use this short model when reading the rest of the README:

| Concept | First-user meaning |
|---|---|
| `Task` | The user-value unit being shaped, worked, blocked, or closed. It carries the current goal, scope, non-goals, and current work boundary. |
| Write | A product-file change should be compatible with the current `Task` and current scope. `Write Check` is a narrow Volicord compatibility record for one proposed write, not OS permission or final approval. |
| Evidence | Recorded support for a specific claim, such as a run, observation, or artifact reference. Evidence supports claims, but it does not become user judgment or proof of correctness. |
| User Judgment | A decision that belongs to the user: product direction, material technical direction, scope, sensitive action, final acceptance, residual-risk acceptance, cancellation, or similar authority-bearing choices. |
| Close | A check that the current `Task` can finish honestly without hiding unresolved owner-defined requirements. Close readiness is decision support, not proof that the product result is correct. |

## Install And Initialize

The normal user path is one installed `volicord` executable. Release binary
installation is the primary path when your system matches a supported target.
Source builds are for development.

On Linux, WSL2, or macOS, download or copy `scripts/install.sh` from the
repository that publishes the Volicord release assets, then install the release
binary:

```sh
VOLICORD_REPO=OWNER/REPO sh ./scripts/install.sh
volicord --version
```

On native Windows x86_64, download or copy `scripts/install.ps1` and run it in
PowerShell:

```powershell
.\scripts\install.ps1 -Repo OWNER/REPO
volicord --version
```

`OWNER/REPO` is the GitHub repository that hosts the Volicord release assets for
this checkout. The POSIX script detects supported Linux, WSL2, and macOS
targets and downloads the target-named tarball. The PowerShell script installs
the `x86_64-pc-windows-msvc` zip artifact under a user-local directory by
default. Both scripts verify the `.sha256` file when available and install only
the `volicord` executable for that platform. They do not edit shell startup
files implicitly. This checkout does not contain a Homebrew tap, Homebrew
formula, Linux package, Windows package-manager package, or external
package-registry install path.

Make sure the future agent host can run `volicord` through `PATH`, then
initialize the Product Repository where you want the agent to work:

```sh
volicord init --host codex --repo /path/to/your-product-repo --profile record
```

Use `--host claude-code` for Claude Code:

```sh
volicord init --host claude-code --repo /path/to/your-product-repo --profile record
```

`volicord init` is the primary first-run setup and connection command for
chat-first use. It initializes the Runtime Home if needed, records the
installation profile, registers or reuses the selected Product Repository,
creates the Agent Connection, writes project-scoped MCP configuration that
starts `volicord mcp --stdio`, writes Volicord-managed guidance and policy
metadata, and records integration status. `--profile record` does not require
host lifecycle hook installation or a session watcher and is the supported
native Windows profile. `--profile observe` requires supported host hook and
session watcher capabilities and is not supported on native Windows; when
required hook support is missing on supported observe hosts, observe setup must
be explicitly selected with `--allow-degraded`.

If the command reports `action_required`, follow the named host-controlled or
local action, such as restarting or reloading the host, approving project MCP
configuration, trusting the project, or repairing command availability. Then
verify:

```sh
volicord connection verify codex --repo /path/to/your-product-repo
```

Exact command behavior lives in the
[Administrative CLI Reference](docs/en/reference/admin-cli.md). Environment
support lives in [System Requirements](docs/en/reference/system-requirements.md).

## Source Build For Development

Use the source build path when you are developing Volicord itself or need a
local development binary:

```sh
cargo build --workspace --bins
./target/debug/volicord --version
./target/debug/volicord init --host codex --repo /path/to/your-product-repo --profile record
```

This path requires the Rust toolchain named in
[System Requirements](docs/en/reference/system-requirements.md#toolchain-requirements).
It is not the primary first-user install path.

## Normal Use Is Chat

After initialization, work normally through the agent host in the Product
Repository. You do not need to drive the workflow from the terminal.

For example, ask in chat:

```text
Add idempotency-key support for payment creation, update the tests, and tell me what still blocks close.
```

The host remains your chat/editor agent. Volicord provides local MCP tools the
host can call when durable workflow state matters:

- create or update a `Task`
- show current scope, blockers, evidence, and pending judgment
- prepare a proposed product-file write
- stage artifacts and record runs or observations
- request a focused user judgment
- check close readiness before the agent claims completion

Agents should use Volicord state when it is available and say explicitly when
it is unavailable. Volicord tools, MCP server instructions, host rules, and
`AGENTS.md` guidance help steer the agent, but they do not absolutely force
model behavior.

## Integration Profiles

`volicord init` defaults to `--profile record`.

Volicord reports a control-surface summary for the selected connection or
session. The summary is not a security proof; it states which cooperative and
detective surfaces are active.

| Profile | What it means now |
|---|---|
| `record` | MCP tools, local authority records, setup guidance, and policy metadata are available. Host lifecycle hooks and session watcher observation are not required. |
| `observe` | Authority records are combined with supported host hooks and session watcher observation. Host hooks may return cooperative pre-tool warnings or denials, and the watcher may detect unrecorded Product Repository changes after coverage starts. |

The control-surface summary reports `selected_profile`,
`host_hooks_active`, `session_watcher_active`,
`cooperative_pre_tool_warning_available`,
`cooperative_pre_tool_denial_available`,
`unrecorded_changes_detectable`, `actor_identity_provable`, and
`os_enforced`. Current Volicord output reports `actor_identity_provable=false`
and `os_enforced=false`; observe profile does not prevent all writes, identify
the actor who changed a file, isolate the network, or provide a sandbox.

Observe profile adds cooperative and detective guard surfaces around the MCP
workflow:

| Surface | What it contributes |
|---|---|
| MCP | Gives the host local `volicord.*` tools over `volicord mcp --stdio`, bound to the stored Agent Connection and allowed Product Repository. |
| `AGENTS.md` | Adds a Volicord-managed guidance block telling agents to check status, start tasks, prepare writes, request user judgment, check close, and report when Volicord tools are unavailable. |
| `.volicord/policy.json` | Records machine-readable hook command policy for supported lifecycle hooks: session start, pre-tool, post-tool, prompt capture, and stop. |
| Host hooks and rules | When the host supports them and loads the generated configuration, hooks can inject context, classify tool attempts, warn or deny some unsafe-looking operations, record observed unrecorded changes, capture strict chat judgment commands, and block stop when close blockers remain. Host rule files point the host at the policy. |

Observe profile reduces bypass when the host actually runs the configured hooks
and respects the rules. It is still not OS-level enforcement. It does not
sandbox tools, monitor all files, block all commands, isolate the network, or
prove that the model followed instructions.

Guard installation has separate file and activation phases. `volicord init`
installs or updates the host configuration, Volicord-managed `AGENTS.md`
guidance, `.volicord/policy.json`, host hook or rule files, and guard state.
For Codex observe setup, generated files include project MCP config,
Volicord-managed POSIX `sh` wrapper scripts under `.codex/hooks/`,
`.codex/hooks.json`, and `.codex/rules/*.rules`. Its pre-tool and post-tool
matchers cover `Bash`, `apply_patch`, `Edit`, `Write`, and
`mcp__.*__(write|edit|create|update|delete|remove|move|patch).*` tool names.
For Claude Code observe setup, generated files include `.mcp.json`,
Volicord-managed POSIX `sh` wrapper scripts under `.claude/hooks/`,
`.claude/settings.json`, and `.claude/rules/*.md`. Its pre-tool and post-tool
matchers cover `Bash`, `Edit`, `Write`, `MultiEdit`, and
`mcp__.*__(write|edit|create|update|delete|remove|move|patch).*` tool names.
Generated hook configs call the wrapper scripts with `--host-output codex` or
`--host-output claude-code`, so hook stdout is host-native JSON/context or empty
output, not Volicord wrapper JSON. Generated hook commands are also
cwd-independent for subdirectory host sessions: Codex resolves the Git
work-tree root at hook runtime and dispatches to the Volicord-managed wrapper
under that root, while Claude Code uses `${CLAUDE_PROJECT_DIR}`-rooted wrapper
commands. Do not replace generated commands with bare `.codex/hooks/...` or
`.claude/hooks/...` relative paths. Verification reports those as
`relative_path_unsafe`, and unsafe hook paths keep observe host hooks inactive
until `volicord init --host HOST --repo PATH --profile observe` regenerates safe
hook commands and the host reloads or trusts them when required. Codex may still
require project trust, hook trust, restart, or reload before rules and hooks
run. For Claude Code, Volicord merges managed settings without owning unrelated
settings, and the host may still require project MCP approval, workspace trust,
or settings reload. The first matching observed guard hook event activates the
installation. `volicord connection verify` and `volicord doctor` report file
health, required host action, observed activation, and control-surface facts
separately; installed files, `AGENTS.md`, and `.volicord/policy.json` alone do
not prove that hooks are active.

## Unrecorded Changes And Close Blockers

Guarded hooks and an active session watcher can report unrecorded Product
Repository changes when a product file changes without a matching expected
write. Session watcher findings come from bounded product-file metadata
comparison for the selected session. They detect changed paths; they do not
store full file contents, prove who changed a file, prove intent, or prevent
writes. Those findings remain guard findings until reconciled, and unresolved
findings block close.

Reconciliation can resolve deterministic cases, such as a finding already
covered by a compatible `Write Check` or recorded run. If acceptance is needed,
Volicord creates a focused user-owned judgment. The user answers through MCP
elicitation, a strict chat command, or CLI recovery. Agents cannot silently
dismiss Product Repository bypass findings or mark them accepted for the user.

In chat, ask the agent to show `volicord.reconcile_changes` results and next
actions. CLI recovery is available through `volicord changes reconcile`.

## User Judgment Capture

User judgment stays user-owned. An Agent Connection may request a judgment, but
it must not record authority-bearing user answers as if it were the user.

Supported capture paths:

| Path | When it is used |
|---|---|
| MCP elicitation | If the initialized MCP client declares `capabilities.elicitation`, Volicord can send an `elicitation/create` request for a focused pending judgment. A valid response is recorded through the local `User Channel` with user provenance. |
| Chat prompt capture | If elicitation is unavailable and prompt-capture availability is `configured`, `observed`, or `active`, Volicord returns exact chat commands such as `Volicord: answer J-3 1 #AB7K`, `Volicord: answer J-3 reject #AB7K`, `Volicord: answer J-3 defer #AB7K`, or `Volicord: note J-3 "text" #AB7K`. The prompt-capture hook records only strict valid commands with the current verification code. |
| Local web consent | If elicitation and prompt capture are unavailable and the adapter can safely expose the fallback, Volicord returns a loopback-only consent URL. The URL uses a short-lived one-time token tied to the project, connection, and pending judgment; a valid answer is recorded through the `User Channel` with local user provenance. |
| CLI fallback | If elicitation, chat capture, and local web consent are unavailable, disabled, degraded, or need inspection, use `volicord inbox` from the Product Repository. |

CLI fallback example:

```sh
volicord inbox
volicord inbox answer JUDGMENT_ID --choice CHOICE_ID
```

Local web consent is separate from MCP elicitation. The local HTTP MCP serve
mode still does not implement HTTP elicitation, and local web consent is
available only on loopback endpoints with a valid consent token.

## What Volicord Does Not Guarantee

Volicord keeps work authority visible, but it is not a general security product
or correctness oracle. Do not rely on Volicord for:

- OS-level sandboxing or OS permission enforcement
- malware defense, malware scanning, or secret scanning
- network isolation, network monitoring, or network blocking
- prevention of all product-file writes
- universal pre-tool blocking or full filesystem monitoring
- tamper-proof audit logging
- proof that code is correct
- proof that tests are sufficient
- replacement for human review, QA, release judgment, or risk judgment
- proof that an external host trusted, approved, loaded, initialized, or exposed
  `volicord mcp --stdio`
- proof that `AGENTS.md`, host rules, or MCP instructions forced model behavior

Guarded mode may return `warn` or `deny` decisions through configured hooks, and
close/write checks may expose blockers. Those are cooperative local controls,
not kernel-level enforcement or a guarantee that tools cannot write files
outside Volicord-aware paths.

See the [Security Reference](docs/en/reference/security.md) for exact guarantee
wording and explicit non-guarantees.

## Docker And Local HTTP MCP

Docker support exists through the checked-in `Dockerfile` for local container
layouts:

```sh
docker build -t volicord:local .
```

The local HTTP MCP mode is implemented as:

```sh
volicord serve --transport local-http
```

It is an explicit advanced mode for Docker and localhost MCP use, not the
default host setup path. It accepts only loopback listen addresses, requires
bearer authentication for the MCP local HTTP endpoint, exposes `POST /mcp`, and
does not implement server-sent event streams, HTTP elicitation, or full MCP
Streamable HTTP compatibility. Do not treat it as a general network service.

Use [Installation](docs/en/getting-started/installation.md) and
[MCP Transport](docs/en/reference/mcp-transport.md) for the detailed Docker and
HTTP boundaries.

## Troubleshooting

| Symptom | What to do |
|---|---|
| `volicord` is not found | Put the install directory on `PATH`, or install to a directory already on `PATH`, then rerun `volicord --version`. Future agent hosts must also be able to start `volicord`. |
| `init` reports `action_required` | Complete the named action, such as host restart or reload, project trust, MCP approval, OAuth, command-link repair, or installation-profile repair, then rerun `volicord connection verify HOST --repo PATH`. |
| Guarded setup reports unsafe hook paths | Rerun `volicord init --host HOST --repo PATH` to regenerate cwd-independent managed hook commands, then complete any host trust, restart, or reload action and rerun `volicord connection verify HOST --repo PATH`. |
| Host cannot start MCP | Confirm the host can run `volicord mcp --help` through the same command path. Run `volicord doctor` for installation-profile health. |
| Product Repository is not detected | Pass `--repo /path/to/your-product-repo` and make sure the path is an existing local repository separate from the Runtime Home. |
| A judgment is pending | Prefer the host's MCP elicitation or exact chat prompt-capture command when available. Use `volicord inbox` and `volicord inbox answer` as the CLI fallback. |
| Close is blocked | Ask the agent to show `volicord.check_close` results, pending user judgments, missing evidence, unresolved unrecorded changes, and residual risks. Address the named blocker instead of closing from a summary. |

## Deeper Docs

| Need | Read |
|---|---|
| Install details and Docker examples | [Installation](docs/en/getting-started/installation.md) |
| Supported environments | [System Requirements](docs/en/reference/system-requirements.md) |
| User workflow and judgment boundaries | [User Guide](docs/en/guides/user-workflow.md) |
| Host setup and repair | [Agent Host Setup](docs/en/guides/agent-host-setup.md) and [Agent Host Troubleshooting](docs/en/guides/agent-host-troubleshooting.md) |
| Exact CLI behavior | [Administrative CLI Reference](docs/en/reference/admin-cli.md) |
| MCP stdio and HTTP transport | [MCP Transport](docs/en/reference/mcp-transport.md) |
| Agent Connection and User Channel boundaries | [Agent Connection Reference](docs/en/reference/agent-connection.md) |
| Core authority concepts | [Core Model](docs/en/reference/core-model.md) |
| Security wording and non-guarantees | [Security Reference](docs/en/reference/security.md) |
| Public API methods and schemas | [Reference Index](docs/en/reference/README.md) |

Volicord commands are local administrative commands, not public Volicord API
methods. Exact public API behavior is owned by the Reference docs.
