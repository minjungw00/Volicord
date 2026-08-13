# Volicord Codex host

`volicord-mcp` is a stdio MCP server exposing high-level Project, health,
Recall, repository understanding, Inquiry/Decision, Checkpoint, canonical and
Candidate inspection, privacy, document, analysis, and Guarded interaction
capabilities. It never exposes raw database operations or legacy methods.

Register an installed server with the current Codex CLI:

```text
codex mcp add volicord --env VOLICORD_RUNTIME_DIR=/absolute/runtime -- /absolute/bin/volicord-mcp
```

When the current host does not advertise elicitation, `guarded_interaction`
returns viewer and CLI fallback arguments for the same request identity,
revision, and fingerprint.
