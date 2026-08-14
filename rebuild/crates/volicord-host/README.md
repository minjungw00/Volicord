# Volicord Codex host

`volicord-mcp` is a stdio MCP server exposing high-level Project, health,
Recall, repository understanding, Inquiry/Decision, Checkpoint, canonical and
Candidate lifecycle, privacy, document, analysis, and Guarded interaction
capabilities. It never exposes raw database operations or legacy methods.

`candidate_manage` accepts four explicit actions: `submit_question` stores an
agent-authored Question Candidate without canonical mutation, `promote_question`
invokes the maintained materiality/duplicate/source checks to create a
canonical Question, and `dismiss`/`delete` apply Candidate-local disposition or
content cleanup. `candidate_inspect` remains the read-only inspection surface;
an explicit current-host answer continues through `decision_record` and its
Question-revision/User-Source linkage.

Register an installed server with the current Codex CLI:

```text
codex mcp add volicord --env VOLICORD_RUNTIME_DIR=/absolute/runtime -- /absolute/bin/volicord-mcp
```

When the current host does not advertise elicitation, `guarded_interaction`
returns viewer and CLI fallback arguments for the same request identity,
revision, and fingerprint.
