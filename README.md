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
judgment. It is a guarded local authority layer that helps an agent use those
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

Download or copy `scripts/install.sh` from the repository that publishes the
Volicord release assets, then install the release binary:

```sh
VOLICORD_REPO=OWNER/REPO sh ./scripts/install.sh
volicord --version
```

`OWNER/REPO` is the GitHub repository that hosts the Volicord release assets for
this checkout. The script detects supported Linux, WSL2, and macOS targets,
downloads the target-named tarball, verifies the `.sha256` file when available,
and installs only `volicord`. It does not edit shell startup files. This
checkout does not contain a Homebrew tap, Homebrew formula, Linux package, or
external package-registry install path.

Make sure the future agent host can run `volicord` through `PATH`, then
initialize the Product Repository where you want the agent to work:

```sh
volicord init --host codex --repo /path/to/your-product-repo
```

Use `--host claude-code` for Claude Code:

```sh
volicord init --host claude-code --repo /path/to/your-product-repo
```

`volicord init` is the primary first-run setup and connection command for
chat-first use. It initializes the Runtime Home if needed, records the
installation profile, registers or reuses the selected Product Repository,
creates the Agent Connection, writes project-scoped MCP configuration that
starts `volicord mcp --stdio`, writes Volicord-managed `AGENTS.md` guidance,
writes `.volicord/policy.json`, and writes supported host rule files when the
host has a supported project-local rule convention.

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
./target/debug/volicord init --host codex --repo /path/to/your-product-repo
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

## Guarded Mode

`volicord init` defaults to `--mode guarded`.

Guarded mode adds cooperative and detective guard surfaces around the MCP
workflow:

| Surface | What it contributes |
|---|---|
| MCP | Gives the host local `volicord.*` tools over `volicord mcp --stdio`, bound to the stored Agent Connection and allowed Product Repository. |
| `AGENTS.md` | Adds a Volicord-managed guidance block telling agents to check status, start tasks, prepare writes, request user judgment, check close, and report when Volicord tools are unavailable. |
| `.volicord/policy.json` | Records machine-readable guard command policy for supported lifecycle hooks: session start, pre-tool, post-tool, prompt capture, and stop. |
| Host hooks and rules | When the host supports them and loads the generated configuration, hooks can inject context, classify tool attempts, warn or deny some unsafe-looking operations, record observed unrecorded changes, capture strict chat judgment commands, and block stop when close blockers remain. Host rule files, such as Claude Code rules, point the host at the policy. |

Other modes are available:

- `--mode mcp-only` writes MCP configuration and guidance but disables guard
  commands in policy metadata.
- `--mode managed` currently uses the same setup surface as `guarded` while
  recording managed guard mode for integrations that distinguish it.

Guarded mode reduces bypass when the host actually runs the configured hooks
and respects the rules. It is still not OS-level enforcement. It does not
sandbox tools, monitor all files, block all commands, isolate the network, or
prove that the model followed instructions.

## User Judgment Capture

User judgment stays user-owned. An Agent Connection may request a judgment, but
it must not record authority-bearing user answers as if it were the user.

Supported capture paths:

| Path | When it is used |
|---|---|
| MCP elicitation | If the initialized MCP client declares `capabilities.elicitation`, Volicord can send an `elicitation/create` request for a focused pending judgment. A valid response is recorded through the local `User Channel` with user provenance. |
| Chat prompt capture | If elicitation is unavailable and guarded prompt capture is active, Volicord returns exact chat commands such as `Volicord: answer J-3 1`, `Volicord: answer J-3 reject`, `Volicord: answer J-3 defer`, or `Volicord: note J-3 "text"`. The prompt-capture hook records only strict valid commands. |
| CLI fallback | If chat capture is unavailable, disabled, or needs inspection, use `volicord user` from the Product Repository. |

CLI fallback example:

```sh
volicord user status
volicord user judgments
volicord user judgment show 1
volicord user judgment answer 1 1
```

There is no separate local web judgment UI documented in this checkout. The
experimental HTTP MCP serve mode also does not implement HTTP elicitation.

## What Volicord Does Not Guarantee

Volicord keeps work authority visible, but it is not a general security product
or correctness oracle. Do not rely on Volicord for:

- OS-level sandboxing or OS permission enforcement
- malware defense, malware scanning, or secret scanning
- network isolation, network monitoring, or network blocking
- prevention of all product-file writes
- universal pre-tool blocking or full filesystem monitoring
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
volicord serve --transport streamable-http
```

It is an explicit advanced mode for Docker and localhost MCP use, not the
default host setup path. It defaults to loopback, requires bearer
authentication, exposes `POST /mcp`, and does not implement server-sent event
streams, HTTP elicitation, or full MCP Streamable HTTP compatibility. Do not
treat it as an unauthenticated network service.

Use [Installation](docs/en/getting-started/installation.md) and
[MCP Transport](docs/en/reference/mcp-transport.md) for the detailed Docker and
HTTP boundaries.

## Troubleshooting

| Symptom | What to do |
|---|---|
| `volicord` is not found | Put the install directory on `PATH`, or install to a directory already on `PATH`, then rerun `volicord --version`. Future agent hosts must also be able to start `volicord`. |
| `init` reports `action_required` | Complete the named action, such as host restart or reload, project trust, MCP approval, OAuth, command-link repair, or setup repair, then rerun `volicord connection verify HOST --repo PATH`. |
| Host cannot start MCP | Confirm the host can run `volicord mcp --help` through the same command path. Run `volicord doctor` for installation-profile health. |
| Product Repository is not detected | Pass `--repo /path/to/your-product-repo` and make sure the path is an existing local repository separate from the Runtime Home. |
| A judgment is pending | Prefer the host's MCP elicitation or exact chat prompt-capture command when available. Use `volicord user judgments` and `volicord user judgment answer` as the CLI fallback. |
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
