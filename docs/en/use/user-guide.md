# User Guide

## What this document helps you do

Use Harness through an ordinary conversation: start a task, understand what the agent should show you, know when your judgment is needed, and ask for the right close information without turning the work into a management ritual.

## Read this when

Read this when Harness is connected and you are starting, resuming, unblocking, or closing an AI-assisted task. It is especially useful when product files may change, scope may drift, a human decision is needed, completion claims need support, or sensitive actions may be involved.

## Before you read

No startup phrase or internal Harness label knowledge is required. [Harness in One Task](../learn/harness-in-one-task.md) is helpful background, but it is not required.

## Main idea

Speak normally. Describe the work you want and any boundary you already know; the agent should infer from the task shape whether Harness applies. Tiny questions or clearly read-only advice can stay light. Larger, riskier, multi-file, or unclear work should be shaped before product files change.

Harness should keep the work understandable, not replace the conversation or the engineering process around it. Source control still records product-file history, tests still check executable behavior, code review still reviews the change, and user-owned product or material technical judgment still belongs to the user.

The agent should translate your request into the right Harness steps. You should not need to operate internal records by hand. Use ordinary language first and exact Harness labels second, only when they explain a real stop, boundary, or close condition.

For small direct work, the ceremony budget is intentionally small: a compact scope, a minimal active Change Unit when product files may change, a write-authority check before the exact write, and a concise result with what changed, what was checked, whether it escalated, what remains risky, and what decision is needed if anything blocks close. Direct means fewer user-facing steps, not bypassed scope or write authority.

If you want to be explicit, you can still say:

```text
Run this work under the harness.
```

## The four display groups

Harness has exact internal gates, but user-facing status should usually group them into four readable lines. These groups explain why work is stopped and what must happen next. They are display groups only; they do not replace kernel gates, create schema fields, change recompute rules, or define close eligibility. Exact gate values, recompute behavior, and close semantics stay in [Kernel Reference](../reference/kernel.md#gates) and [`close_task`](../reference/kernel.md#close_task).

| Display group | Plain question | What it usually includes |
|---|---|---|
| Scope | What may change? | Task scope, out of bounds, active Change Unit, Autonomy Boundary limits, and write authority compatibility. |
| Judgment | What must the user decide? | Decision Packets, sensitive-action Approval, product/UX choices, material technical choices, QA or waiver choices, acceptance, residual-risk, or scope/autonomy decisions. |
| Evidence | What supports completion claims? | Evidence Manifest coverage, Run and artifact refs, self-checks, missing evidence, stale evidence, or redaction/omission state. |
| Close Readiness | What still prevents close? | Verification, Manual QA, final acceptance, residual-risk visibility or acceptance, close blockers, and close reason. |

Status may still show exact Harness terms or source refs when they help, but it should lead with the display group and then point to the owner record instead of making the user read the full gate taxonomy.

## First-read path

### 1. Say what you want

Start with the work and any boundary you already know:

```text
Add email login flow. Keep password reset and account creation out of scope.
```

The agent should decide whether the request is read-only advice, small direct work, or tracked work. When tracking is useful, it should answer four plain questions before it gets deep into the work:

- Scope: what is in bounds, and what is out of bounds?
- Judgment: what user-owned decision is needed now, if any?
- Evidence: what support or checks already exist, and what is still missing?
- Close Readiness: what would still block verification, Manual QA, acceptance, residual-risk handling, or close?

If the task is small, the agent may handle it as direct work. Tiny edits such as a typo, one docs sentence, or an obvious rename can use the tiny direct profile under `direct`: the agent should keep the display to the trivial scope, changed path or no-file result, and self-check. If the task is larger, risky, multi-file, unclear, evidence-heavy, or touches user-owned judgment or sensitive boundaries, it should shape the work before changing product files.

When you want stronger clarification before planning, ask for Discovery:

```text
Start Discovery. Ask targeted questions until the first safe Change Unit is clear. Separate product decisions from technical decisions. Show options, recommendation, and uncertainty. Only ask me what the codebase cannot answer.
```

Discovery is for requirement clarification before write authority. A good Discovery pass separates goal, user value, non-goals, acceptance criteria, assumptions, product decisions, technical decisions, security choices, QA or verification expectations, operational concerns, and scope boundaries. It may ask multiple targeted questions, but it should stop once the first safe Change Unit candidate is clear enough to propose. Discovery output should feed Shared Design, Decision Packet candidates, and Change Unit shaping; it is not approval, sensitive-action Approval, Write Authorization, evidence, verification, QA, acceptance, residual-risk acceptance, close, scope authority, or a new authority path.

### 2. Expect a compact start

At the start, or before significant resume, the agent should show a short status or Journey Card. It should fit a quick scan and show only what affects the next decision or safe action:

- task and mode
- scope and out of bounds
- next safe action
- the four display groups: Scope, Judgment, Evidence, and Close Readiness
- what is blocked, what has been checked, what remains, and what you are deciding
- decision or blocker, including who owns the next move
- smallest unblocker
- write permission, evidence, verification, Manual QA, residual risk, and acceptance status when they matter
- capability or readable-status freshness only when it affects whether you can rely on the display

A good first status can look like this:

```text
Task: TASK-123 Add email login flow
Mode: tracked work (`work`)
Next safe action: decide failed-login UX before wiring final UI behavior
Scope: login form, login API call, session storage; out of bounds: password reset, account creation; write authority not requested yet
Judgment: failed-login message in DEC-014 is user-owned; smallest unblocker is choosing one option
Evidence: current Core state v42 read; no implementation evidence or Evidence Manifest ref yet
Close Readiness: final copy/layout Manual QA, residual-risk visibility, acceptance, and close decision remain later in the flow
Capability/status: cooperative surface; readable status current from source_state_version v42
```

Look first for the next safe action and the smallest unblocker. A blocker should say who owns the next move: user-owned when it needs your product, material technical, Approval, QA, risk, or acceptance judgment; agent-resolvable when the agent can refresh state, collect evidence, rerun a check, retry `prepare_write`, or narrow scope without changing your decision.

If the status looks stale or wrong, say:

```text
Show the current status and next action again from state.
```

The agent should resume from current Harness state, not from stale chat memory. Older conversation can help find refs to inspect, but it cannot authorize writes, accept results, accept residual risk, close a task, or replace current owner records.

When the agent needs your judgment, status alone is not enough. It should add a focused prompt with a decision title, display judgment type, the exact choice, options, a recommendation, uncertainty, affected gates or acceptance criteria when relevant, what can continue if you defer, and source refs and evidence, risk, or design refs when available or relevant.

### 3. When blocked, ask for the one unblocker

When the task is blocked, ask:

```text
What is blocking this task now, and what one decision or check would unblock it?
```

### 4. Before close, ask what remains

Near close, ask:

```text
Show close-relevant residual risk before I accept.
```

Ask for the close checklist if you want the full close basis:

```text
Show the close checklist.
```

The agent should keep Approval, Decision Packet outcomes, Write Authorization, evidence, verification, Manual QA, acceptance, and residual risk separate. One of them should not be used as a substitute for another.

Authority claims should come with refs. "Write allowed" should point to a Write Authorization ref; "evidence sufficient" to an Evidence Manifest ref; "detached verified" to an Eval ref; "Manual QA passed" to a Manual QA record; "accepted" to an Acceptance Decision Packet; and "residual risk handled" to Residual Risk refs or an explicit `ResidualRiskSummary.status=none`.

Residual-risk wording should also be precise. `status=none` means there is no known close-relevant residual risk for this requested action. `not_visible` means known close-relevant risk exists but has not yet been shown well enough for acceptance or close. Treat `not_visible` as something to surface, not as "no risk."

A casual "go ahead" is only usable when the agent has already named the exact thing you are deciding. It is not enough for product trade-offs, architecture choices, QA or verification waivers, final acceptance, or residual-risk acceptance unless the prompt shows the options, consequences, relevant refs, what the agent may still decide without you, and the specific route being recorded.

## The four everyday groups

### Scope

Scope answers: "What work are we doing, and what are we not doing?"

Good scope is narrow enough that the agent can avoid accidental expansion. It should name affected areas, important exclusions, and any path or behavior boundaries that matter.

Once scope is clear, the agent may decide routine implementation details inside it without asking every tiny question. Examples include using an existing helper, splitting a private function, adding focused tests, or choosing the conservative internal approach that best fits the agreed result.

The agent should stop and ask when the choice changes what users or other code can rely on: public API or module contracts, security or privacy trade-offs, UX or product behavior, material dependency or migration direction, scope expansion, or accepting known residual risk.

A useful split is: Change Unit scope says what work surface may change; Autonomy Boundary says what judgment the agent may exercise inside that surface. Neither one authorizes a write by itself.

Harness may use several related labels for this:

| Label | Plain meaning |
|---|---|
| Change Unit scope | The work area that is in bounds. It does not authorize writes by itself. |
| Autonomy Boundary | The judgment the agent may exercise alone inside that scope. It is not write authority and does not grant paths, tools, commands, network, secrets, or sensitive categories. |
| Approval | Permission for a sensitive step. It is not acceptance, correctness, or user-owned judgment. |
| Decision Packet | The recorded path for user-owned product, material technical, waiver, acceptance, residual-risk, or reconcile judgment. It is not sensitive-action permission unless it is approval-shaped and linked to Approval. |
| Acceptance | Your final judgment that the result is acceptable when the task path requires it. It does not replace evidence, QA, verification, Approval, or residual-risk acceptance. |
| Residual-risk acceptance | Your judgment that known remaining risk is acceptable for this close. It is not a normal no-risk close and does not upgrade assurance. |
| Write Authorization | A one-attempt write allowance from `prepare_write`. It does not expand the scope or Autonomy Boundary. |

For a small direct task, the agent can usually generate the minimal Change Unit from the request instead of asking you to fill in fields. These examples are explanatory, not a schema:

- Tiny docs sentence: purpose "fix this sentence only"; out of bounds "no meaning, contract, or link behavior change"; paths "the named doc only"; stop if "evidence beyond changed path and self-check is needed."
- Docs typo: purpose "fix spelling in one paragraph"; out of bounds "no meaning change"; paths "the named doc only"; stop if "the edit changes the contract."
- Copy-only UI change: purpose "rename this label"; out of bounds "behavior, layout, localization strategy"; paths "the target component and direct copy test."
- Focused test change: purpose "add a regression test for the reported case"; out of bounds "implementation changes"; paths "the named test file or nearby test."

If the agent asks you to approve something, the prompt should label the actual authority or recorded decision. The user may be approving a sensitive action, confirming scope, resolving a Decision Packet, accepting residual risk, accepting the final result, or checking Write Authorization status. "Approved" should not be a catch-all label or blank check.

Useful phrases:

```text
Start with the scope and questions.
That scope works. Do not expand beyond what we just agreed.
If scope needs to grow, show me the options and impact first.
What exact decision or sensitive action am I being asked to record?
```

Harness may describe those boundaries as the active Change Unit, and it may use a Decision Packet when a scope change needs your judgment. You do not need to lead with those labels.

### Judgment

Judgment answers: "What do I need to decide before the work can safely continue or close?"

Most judgment is one of these:

- choose a Product / UX direction or trade-off you own
- choose a Technical architecture direction whose cost, compatibility, security, migration, interface, or maintenance impact you own
- choose a Security / privacy rule or trade-off, such as redaction, audit trail, retention, or PII handling
- grant sensitive-action Approval
- make a QA / acceptance judgment, including whether Manual QA is needed, whether a waiver is acceptable, or whether the result is acceptable
- make a Residual risk judgment, including accepting a named remaining risk for this Task
- make a Scope / autonomy judgment, including whether to expand scope or let the agent decide more inside the active Change Unit

When user-owned product, technical, security/privacy, QA/acceptance, residual-risk, or scope/autonomy judgment blocks progress, the agent should show a Decision Packet. It should not flatten that into a vague "approve everything?" question.

A good Decision Packet should feel like decision support, not a permission slip. It should name the real choice, compare realistic paths, recommend one, and say what can safely continue if you defer, or why nothing should continue until you decide.

User-facing Decision Packet displays should have this shape:

- Decision title
- Display judgment type: Product / UX, Technical architecture, Security / privacy, QA / acceptance, Residual risk, or Scope / autonomy
- Why this is needed now
- What the user is deciding / exact choice
- Options
- Trade-offs
- Recommendation
- Uncertainty
- Deferral consequence
- Residual risk when relevant
- Affected gates and acceptance criteria when relevant
- Source refs and evidence, risk, or design refs when available or relevant
- What the agent may decide without the user

The display judgment type helps readers scan what kind of decision they are being asked to make. Use it as the primary display category. If a decision is cross-cutting, show secondary considerations in trade-offs, affected gates, risk, evidence, or follow-up instead of pretending the category is exclusive. It is display guidance, not a new schema field, gate, owner record, validator input, or authority path. Exact public fields are owned by [`harness.request_user_decision`](../reference/mcp-api-and-schemas.md#harnessrequest_user_decision); canonical authority is owned by [Decision Packet](../reference/kernel.md#decision-packet) and [Decision Gate](../reference/kernel.md#decision-gate).

Examples:

- Product / UX: failed-login feedback could be an inline layer, a toast, or a modal. The packet should compare user flow, interruption, accessibility, and copy risk, then recommend a path.
- Product / UX: failed-login wording could be generic, specific, or hybrid. The packet should compare account enumeration risk, clarity, support burden, recovery usefulness, and product tone.
- QA / acceptance: a polished interaction may need Manual QA for layout, accessibility interpretation, and feel; a simpler conservative behavior may be easier to verify. The packet should show the trade-off and what can continue if QA is deferred, or why nothing should continue until the decision is made.
- Technical architecture: auth approach can compare local session cookie, bearer token/JWT, OAuth/OIDC sign-in, or social-login provider integration. OAuth/OIDC may still produce a local session or token strategy, so the packet should separate identity-provider choice from session/storage model when both matter. It should also explain revocation, CSRF/XSS exposure, client compatibility, operational complexity, migration impact, and implementation cost.
- Technical architecture: dependency additions can involve both sensitive-action Approval and a Decision Packet. Granting Approval for an install or dependency-file edit is not the same as deciding the dependency is the architecture direction.
- Technical architecture: schema migrations should show whether the path is additive, compatibility-shimmed, or breaking. The packet should name migration evidence, rollback risk, data-backfill risk, test boundary, and future maintenance impact.
- Technical architecture: public API or module-boundary changes can need a compatibility or breaking-change Decision Packet. Passing tests does not settle caller impact, documentation promises, migration path, or release risk.
- Scope / autonomy: expanding from a copy fix into account behavior, or from a private helper change into a public module boundary, needs a decision that names the new surface, what remains out of bounds, and whether a smaller Change Unit can continue.
- Security / privacy: sensitive-action Approval to access a secret, change permissions, or export data only answers whether that sensitive step may proceed. It does not decide which data is exported, who may export it, what gets redacted, what is omitted from artifacts, or what audit trail is acceptable.
- Security / privacy: a PII logging policy might compare "do not log PII," "log redacted or tokenized identifiers," and "log limited fields for debugging." The packet should show privacy risk, debugging value, retention, redaction, audit trail, and whether existing evidence can prove the policy is followed.
- QA / acceptance: a QA waiver prompt should name the skipped check or surface, the residual risk you would accept, the residual-risk follow-up, relevant refs, and whether close would become residual-risk accepted. "Go ahead" is not enough.
- Residual risk: a residual-risk accepted close should show the remaining limitation, evidence that does exist, missing or waived QA/verification, the accepted residual risk, and residual-risk follow-up. It is not a detached-verified close.
- QA / acceptance and Residual risk: final acceptance means the result is acceptable when required; residual-risk acceptance means the named remaining risk is acceptable for close. The agent should ask for these separately, after showing evidence, verification, QA, and residual-risk visibility.

### Evidence

Evidence answers: "What supports the claim that this work is done?"

Evidence is not just "the agent says it changed the thing." It can include changed paths, test output, logs, screenshots, QA notes, verification results, or other artifacts that support the acceptance criteria.

Enough evidence means the stated acceptance criteria or completion conditions are covered, not that many files or artifacts exist. A tiny docs-only fix may need only a changed path, one-line diff or patch summary, and self-check that no meaning changed. If durable evidence coverage, link/render proof, or more than a trivial docs edit is needed, the agent should treat it as ordinary Direct or Work. A code fix usually needs the diff plus a focused test, command, log, or a recorded reason no automated check applies. A feature should map each acceptance criterion to Run and artifact refs. UI, UX, and copy work may need visual evidence and Manual QA when human judgment matters. Sensitive work keeps sensitive-action Approval and redaction refs visible, but Approval is not proof of correctness. Verification-required work needs an Eval that says which evidence it reviewed.

For large evidence, the agent should show refs and short outcomes first. Logs, screenshots, diffs, traces, Run details, Eval details, Manual QA notes, and artifacts should not be pasted into the default context unless you or the next reviewer need to inspect them. The artifact store is not a loose file dump: useful evidence should appear as registered artifact refs with hash or size details when relevant, redaction state, retention or availability, and the owner record they support.

Markdown reports are useful views over that evidence, not the evidence or state record itself. Report prose and chat text can explain the evidence story, but they are not enough to prove evidence sufficiency unless the relevant criteria point to compatible owner records and artifact refs. If you edit a report, use the human notes or proposal area; edits inside generated or managed report text should be treated as drift or reconcile input, not as a gate change.

Secret values should not be stored as artifacts. If secret-related evidence is needed, the useful display is a redacted artifact, a secret handle or omission note, or an operator note that passed the relevant validator. When an artifact is redacted, omitted, blocked, expired, or unavailable, the agent should say that visibly instead of implying the raw bytes were reviewed.

Evidence can go stale even after it once looked sufficient. Common causes are baseline drift, changed files after the supporting run or eval, approval drift or expiry, a missing artifact, an artifact hash or size mismatch, an expired or unavailable artifact, or a relevant design record change.

Useful phrase:

```text
Show which acceptance criteria are missing evidence, and suggest what additional checks would be enough.
```

### Close Readiness

Close Readiness answers: "What still has to be true before this Task can close?"

This group pulls together the close-facing parts of the work: verification, Manual QA, acceptance, residual-risk visibility or acceptance, and close blockers. It is a user-facing summary layer, not a new gate. If the exact kernel gate state matters, the agent should link the relevant source refs and the [Kernel Reference](../reference/kernel.md#close_task) instead of redefining the values in chat.

A useful Close Readiness line says whether the result is blocked, ready to request acceptance, ready to attempt close, or waiting on a specific check or judgment. It should name the smallest unblocker and keep residual-risk accepted close visibly different from a normal self-checked or detached-verified close.

### Close Readiness blocker matrix

This matrix is a scan aid for the user-facing blocker summary. Notice that every row points back to existing owner records and gates; Close Readiness itself is not a new gate, schema field, or authority path.

```mermaid
flowchart TB
  subgraph Matrix["Close Readiness blocker matrix"]
    direction LR
    State["State and scope<br/>active Run clear<br/>Change Unit compatible when needed<br/>scope passed for writes"]
    Judgment["Judgment<br/>Decision Packets resolved or<br/>compatibly deferred<br/>Approval granted when sensitive<br/>acceptance handled when required"]
    Checks["Evidence and checks<br/>evidence sufficient when required<br/>verification passed, not required,<br/>or waived with risk<br/>Manual QA passed or waived when required"]
    Risk["Risk and result<br/>residual risk visible or none<br/>accepted when risk-close<br/>close reason stays honest"]
  end
  State --> Outcome{"all required parts<br/>compatible?"}
  Judgment --> Outcome
  Checks --> Outcome
  Risk --> Outcome
  Outcome -->|yes| Attempt["attempt close_task"]
  Outcome -->|no| Blocker["show close blocker<br/>and smallest unblocker"]
```

Exact close behavior is owned by [`close_task`](../reference/kernel.md#close_task), close-result wording by [Close result semantics](../reference/kernel.md#close-result-semantics), projection freshness by [Document Projection Reference](../reference/document-projection.md#freshness-and-failure-rules), and public error selection by [MCP API And Schemas](../reference/mcp-api-and-schemas.md#primary-error-code-precedence). The matrix only explains what the user should expect to see.

Useful phrases:

```text
Show Close Readiness in plain language.
What one check or decision still blocks close?
Show close-relevant residual risk before I accept.
```

## Phrase reference

Everyday work starts as a conversation, not as a command language. Use ordinary language first. Harness terms are there so the agent can explain a real stop, boundary, or close condition when precision helps.

| You can say | Harness term the agent may use |
|---|---|
| Add email login flow. Keep password reset out of scope. | Tracked Harness work, if the task shape calls for it. |
| Show me the status. | Journey Card or current Task status. |
| Continue this work. Check harness state first. | Resume from Harness state. |
| Show me the Journey Card before resuming. | Resume status before more work. |
| If this is small, just handle it; if it grows, use the tracked flow. | `direct` or `work` classification. |
| Start Discovery. Ask targeted questions until the first safe Change Unit is clear. Separate product decisions from technical decisions. Show options, recommendation, and uncertainty. Only ask me what the codebase cannot answer. | Discovery feeding Shared Design, Decision Packet candidates, and Change Unit shaping. |
| Start with the scope and questions. | Task scope; active Change Unit when product writes may happen. |
| Do not expand beyond the scope we just agreed. | Change Unit boundary. |
| If scope needs to grow, show me the options and impact first. | Decision Packet for scope or user-owned judgment. |
| Show what you can actually block and what you can only detect later. | guarantee level or surface capability. |
| Check your work independently if possible. | detached verification. |
| Decide whether Manual QA is needed. | Manual QA requirement or waiver. |
| Show the remaining risks before I accept. | residual risk and close-relevant risk status. |
| If final acceptance is required, ask me for it before close. | final acceptance before task close. |
| No separate final acceptance is needed here; close once the relevant blockers are clear. | final acceptance not required for this task path. |
| Accepted. Close this task. | task close, when blockers are clear. |

You may also say "Run this work under the harness" when you want to be explicit, but it is not required.

For review help, stay plain unless the label is useful:

```text
Look at the product or technical trade-offs before choosing.
Check this from engineering, design, security, QA, or release-handoff perspective.
```

Power-user labels for those review requests include product-review, eng-review, design-review, security-review, qa-review, and release-handoff. They focus the review; they are not new gates.

A useful final review often separates two questions:

```text
Spec Compliance Review: Did we build the requested thing under the current scope and authority?
Code Quality / Stewardship Review: Is the result maintainable and coherent in the codebase?
```

For cautious work:

```text
Do not expand beyond the scope we just agreed.
If scope needs to grow, show me the options and impact first.
Pause writes until I answer the open decision.
Show what you can actually block and what you can only detect later.
Use careful mode for this change: narrow scope, show write authority before writes, and ask before user-owned product or material technical trade-offs.
If I step away, continue only inside the active scope and stop before public commitments or new user-owned judgment.
```

Power-user equivalents for the same requests include Change Unit, Decision Packet, guarantee level, detached verification, residual risk, `prepare_write`, and Write Authorization. They are useful labels for explaining blocks and close conditions; they are not words you must memorize before using Harness.

## Basic flow

The normal path should feel like a short conversation. Users should see the current position, the next safe action, and any decision that genuinely needs them. Implementers who want the same path from the runtime side can read [Runtime Walkthrough](../build/runtime-walkthrough.md).

```mermaid
flowchart LR
  Request["request"] --> Classify["classify task shape"]
  Classify --> Discovery["Discovery when clarification is needed"]
  Classify --> Scope["scope"]
  Discovery --> Brief["Discovery Brief"]
  Brief --> Decisions["Decision Packet candidates when user-owned"]
  Brief --> Scope["First Safe Change Unit Candidate"]
  Decisions --> Scope
  Scope --> Work["do allowed work"]
  Work --> Evidence["Evidence: supporting refs"]
  Evidence --> Readiness["Close Readiness: verify / QA / risk / acceptance"]
  Readiness --> Close["close or ask"]
```

Typical flow:

1. The agent checks status or starts intake.
2. The agent classifies the request as `advisor`, `direct`, or `work`.
3. If the request is ambiguous, feature-shaped, auth/security-sensitive, UX/workflow-heavy, public-interface-facing, or likely to become `work`, and clarification is needed, the agent can use Discovery and produce a Discovery Brief.
4. The agent routes user-owned product, technical, security, QA, operational, or scope judgments under Judgment to Decision Packet candidates or existing decision paths.
5. The agent proposes the First Safe Change Unit Candidate, then confirms Scope and the active Change Unit when product writes may happen.
6. Before product writes, the agent checks write authority.
7. After changes or advice, the agent records the relevant result and Evidence when evidence applies.
8. When needed, Close Readiness covers verification, Manual QA, residual risk, acceptance, and close blockers before close.

Many small direct tasks skip some later checks. Bigger work should not hide those checks; it should show them only when they matter. In every case, useful user-facing output favors the same plain questions: what changed, what was checked, what remains risky, and what decision is needed now.

A tiny direct result can be as small as:

```text
Done as tiny direct.
Scope: fixed one typo in `docs/help.md`; no meaning or contract change.
Evidence: changed path plus self-check; no escalation.
```

A direct task result should stay compact and low-ceremony: what was requested, what stayed in scope, what changed, what was checked, whether it escalated, and any close-relevant risk or follow-up. It should not restate every gate when those gates did not affect the result.

Compact direct result:

```text
Done as direct.
Scope: settings label only; account behavior stayed out of bounds; Write Authorization WA-031 was consumed; no escalation.
Judgment: no user-owned decision was needed.
Evidence: changed `src/settings/Profile.tsx`; checked RUN-031 and diff ART-DIFF-031; Evidence Manifest EM-031 covers the claim.
Close Readiness: no close-relevant blocker remains; residual risk is none for this close (`ResidualRiskSummary.status=none`).
```

Fuller work close summary:

```text
Close summary:
Scope: changed scope stayed inside login form, login API call, and session storage.
Judgment: residual risk accepted in DEC-022; final acceptance recorded in DEC-023.
Evidence: Evidence Manifest EM-009 covers AC-01 and AC-02, supported by RUN-018 and ART-TEST-018.
Close Readiness: verification is self-checked in RUN-018; no detached Eval was required for this path. Manual QA passed in MQA-006. Residual Risk RISK-004 covers untested mobile Safari behavior with follow-up TASK-144. Close reason: completed with accepted residual risk.
```

Direct work should escalate to `work` when the target is no longer obvious, changed paths cross the active Change Unit, more than a local product area is affected, a public API or module contract may change, sensitive or risky behavior appears, Manual QA or detached verification becomes important, or a user-owned product or material technical trade-off is needed.

If an out-of-scope changed path is observed after action, the agent should not describe it as authorized work. It should show the mismatch, hold further product writes, and route to the smallest repair path: revert or isolate the extra change, ask for a scope decision, or escalate the same Task to `work` if the wider change is now the real task.

## Advanced status details

Most users can continue with the quick path: scope, next safe action, blocker, smallest unblocker, and close-relevant risk. The details below matter when a status view looks stale, a Harness/Core capability is unavailable, the agent mentions guard or freeze behavior, or you ask for a specialized review lens.

### readable status and MCP availability

Projection freshness is the freshness of the readable view, not the task result. `current` means the card or report matches the state version it names. `stale`, `failed`, or `unknown` means the readable view may need refresh or reconcile before you rely on it.

That is different from stale state, stale baseline, or stale evidence. Those mean the underlying work inputs moved, became outdated, or no longer prove the claim; they may block writes or close even when the status card itself is current.

It is also different from MCP unavailable. If the agent cannot reach the required Harness/Core capability, it should say that directly and avoid claiming an authoritative state change, Approval, result acceptance, residual-risk acceptance, gate update, projection repair, or close until the connection or capability is restored.

Typical recovery readings:

- Projection stale but Core state current: refresh or reconcile the readable view, then continue from Core state. Do not treat the stale Markdown report as authority.
- MCP unavailable: hold product writes and gate updates; do not claim Approval, result acceptance, residual-risk acceptance, or close until the required Harness/Core capability is available or the work moves to a capable surface.
- Managed block edited by hand: treat the edit as drift or a proposal, then route it through Reconcile before it becomes state.
- Stale chat or cached recommendations: use them only to find current refs to inspect. They do not authorize writes, satisfy gates, accept results, accept residual risk, close the task, or replace current state.

### guarantee level and careful mode

If the agent uses words like guard, freeze, or careful mode, it should explain them in ordinary terms: what can actually be blocked before execution, and what can only be detected later. A freeze on a cooperative surface means a scope hold or stricter next-action posture by instruction. On a detective surface it may also include after-action checks. It is hard prevention only when the connected profile has proven pre-tool blocking for that exact kind of operation.

The exact label may be guarantee level or surface capability. It is display and risk context, not Approval, verification, QA, acceptance, residual-risk acceptance, close, or a kernel gate. The useful question is still plain: "Can this surface prevent the action before it happens, or only detect a problem afterward?"

AFK or "continue while I am away" instructions do not expand authority. Careful mode also does not create a new authority tier; it just asks the agent to use a stricter posture. The agent may continue only inside the active Change Unit, Autonomy Boundary, granted sensitive-action Approvals, and compatible write authority. It should stop before scope expansion, public commitments such as API/module contracts or release promises, residual-risk acceptance, final acceptance, QA or verification waivers, or any new user-owned product or material technical judgment. On cooperative or detective surfaces, that stop is a held instruction or later detection path, not a claim of hard pre-execution blocking.

### Role Lens requests

Product-review, eng-review, design-review, security-review, qa-review, and release-handoff labels are Role Lens, playbook, or display requests. Status/next recommendations are the same kind of read-only guidance. They help focus what to inspect; they are not new gates and do not by themselves create Approval, Write Authorization, evidence, QA, verification, acceptance, risk, or close effects.

If a lens or recommendation finds an issue, route it through the existing path: Decision Packet, evidence, Eval or verification need, Manual QA, residual risk, Approval, Change Unit update recommendation, or close blocker. Same-session review can be a helpful self-check or stewardship signal, but it is not detached verification. The exact Role Lens boundary lives in [Agent Integration](../reference/agent-integration.md#role-lens-behavior).

## When the task is blocked

A block should be explained as a concrete reason the task cannot safely continue or close. The display should lead with the primary blocker, name the smallest unblocker, and keep secondary blockers visible only when they will still matter. It should also distinguish user-owned blockers from agent-resolvable blockers.

Good blocked status:

```text
Blocked:
- Judgment (user-owned): empty-state behavior is not chosen.
- Evidence: AC-02 support can be collected after the behavior is chosen.
- Close Readiness: updated onboarding copy still needs user-owned Manual QA before close.
- Smallest unblocker: choose the empty-state behavior from DEC-021.

Next safe action: answer DEC-021, or ask the agent to propose a smaller Change Unit that avoids the empty state.
```

Useful phrases:

```text
What is blocking this task now?
What one decision or check would unblock it?
Show the smallest safe next step.
Defer that decision and propose a smaller Change Unit.
```

## Decisions, approvals, QA, acceptance, and residual risk

These words answer different questions. Keep them separate near close, even when the same artifact or conversation mentions more than one of them.

| Item | Plain job | Do not substitute it with |
|---|---|---|
| Evidence | Supports the claim that a criterion or result was met. | The agent saying "done", a report sentence, or final acceptance. |
| Verification | Checks correctness from an appropriate review boundary. Detached verification needs independence. | Same-session self-review, passing tests alone, or Manual QA. |
| Manual QA | Records human inspection where human judgment matters, commonly UI/UX, copy, accessibility interpretation, workflow, product taste, or visual output. | Automated tests, browser smoke, Browser QA artifacts, verification, or acceptance. |
| Acceptance | Records the user's judgment that the result is acceptable when the task requires it. | Correctness proof, QA, verification, or sensitive-action Approval. |
| Residual Risk | Names known remaining uncertainty, limitation, unchecked condition, or trade-off. | Evidence, verification, QA, or acceptance. |
| Decision | Records the user-owned product direction, material technical direction, waiver, or close-relevant choice. | Broad approval or chat agreement that does not answer the actual trade-off. |
| Approval | Allows a named sensitive action to proceed. | Acceptance, correctness, evidence, verification, QA, or residual-risk acceptance. |

Approval is not acceptance. Tests passing do not mean Manual QA happened. Same-session self-review can be a useful self-check, but it is not detached verification. Accepting a result does not prove it is correct. Accepting residual risk is not proof either; it means the known uncertainty was visible and accepted for this Task. Final acceptance, when required, should come after close-relevant residual risk has been shown or reported as no known close-relevant risk.

Manual QA judgment is separate from Browser QA artifacts. Screenshots, browser smoke, console logs, network traces, accessibility snapshots, and workflow recordings can support evidence when they are registered through the existing artifact path, but they do not become Manual QA, final acceptance, or detached verification. Browser QA Capture remains a v1+ Expansion candidate unless owner docs explicitly promote it. If a surface cannot support browser capture, use human Manual QA notes and manually supplied artifacts instead.

Useful verification wording:

| Phrase | What it means to you |
|---|---|
| Self-checked | The agent checked its own work and recorded what it checked. This is useful, but not independent. |
| Detached candidate | A separate verifier, session, worktree, sandbox, or bundle may be independent enough, but Harness has not yet recorded a passing detached verification. |
| Detached verified | A qualifying independent Eval passed, and its reviewed evidence and baseline were still current. |
| Waived with accepted residual risk | You chose to close despite a missing or waived check after seeing and accepting the remaining residual risk. This is not verified close. |

Examples that may need sensitive-action Approval include dependency additions, auth or permission changes, data model changes, public API changes, destructive writes, secret access, and production configuration changes. Approval only answers whether a sensitive step may proceed; a separate Decision Packet may still be needed for the dependency, migration, interface, module-boundary, product, material technical, QA, or risk choice itself.

When a sensitive category appears, the useful prompt should use ordinary language first: what side effect will happen, which path, system, service, secret, or data is involved, whether Harness can prevent it or only detect issues after action, what evidence will be recorded, and what will be redacted or omitted. The category label can follow in parentheses, such as `secret_access` or `data_export`. Exact category examples live in [MCP API And Schemas](../reference/mcp-api-and-schemas.md#sensitive-categories), and exact write authority behavior lives in [Kernel Reference](../reference/kernel.md#prepare_write).

Common "approved" mix-ups:

- Granting sensitive-action Approval for a dependency install is not the same as choosing that dependency as the architecture direction.
- Granting sensitive-action Approval for secret access is not permission to reveal secret values in artifacts, projections, exports, logs, screenshots, or summaries.
- Granting sensitive-action Approval for auth or system-file access is not choosing the identity-provider or session/storage model, such as local session cookie, bearer token/JWT, OAuth/OIDC sign-in, or social-login provider integration; it also does not decide role design, lockout behavior, or user notice.
- Deciding a public API change is not permission to deploy, merge, or make additional writes.
- Final acceptance means you accept the result when that task path requires it; it is not Write Authorization for more edits.

If the agent asks for a QA or verification waiver, it should name the existing recording path it will use and link the owner refs. QA waiver effects are owned by the Manual QA / QA policy path; when product/user risk or policy-required judgment is involved, the prompt should reference a QA waiver Decision Packet. Verification waiver effects are owned by the kernel verification-waiver path; when the waiver needs user-owned judgment, the prompt should reference the relevant Decision Packet and accepted Residual Risk refs. The prompt should say what is not being checked, what residual risk you would accept, what residual-risk follow-up remains, which refs matter, and how close is affected. A casual chat statement should not be treated as a close-relevant waiver when residual-risk acceptance is involved. If the agent asks to close with residual risk, it should show the remaining limitation first, then ask whether you accept that residual risk for this Task. Verification waiver can close only as residual-risk accepted; it should not be presented as detached verified. Exact gate effects stay in [Kernel Reference](../reference/kernel.md#waiver-semantics) and [Design Quality Policies](../reference/design-quality-policies.md#waiver-rules).

Applied examples:

- Direct docs or copy fix: a changed path, diff or patch summary, and self-check can support the claim. It should not be described as detached verification, and it does not need Manual QA unless the changed surface needs human inspection.
- UI/UX, workflow, copy, accessibility, product-taste, or visual-output work: tests, browser smoke, and Browser QA artifacts can support rendering or behavior claims. Manual QA is still the human check for layout, interaction feel, copy, accessibility interpretation, workflow quality, and product taste. When automated browser capture is unavailable, human notes and manually supplied artifacts are the fallback. A QA waiver should name the skipped surface, accepted residual risk, residual-risk follow-up, relevant refs, and close impact.
- Auth or security work: sensitive-action Approval may allow secret access, permission changes, or auth-file writes. The security or product choice still needs a Decision Packet when roles, redaction, audit trail, session model, lockout behavior, or user notice are being decided.
- Public API work: passing tests support behavior, but compatibility, caller impact, migration path, and documentation promises may need a Decision Packet and independent verification.
- Residual-risk accepted close: the agent should show the evidence that exists, the verification or QA that is missing or waived, the remaining limitation, and the residual-risk follow-up. Closing with accepted residual risk is not the same as closing as detached verified.

## Close checklist

Before close, the agent should make these points clear in everyday language:

For a `work` task, the close summary should show the changed scope, evidence, verification, Manual QA, residual risk, acceptance, and close reason when those apply. If a check was waived, pending, or not required, say that directly. Do not bury close-relevant residual risk inside a general "done" statement.

- The result matches the agreed scope.
- Required evidence is present, or evidence is not required for this task shape.
- Verification is either not expected for this task path, completed, or explicitly waived with recorded risk.
- Manual QA is either not needed, completed, or validly waived.
- Known close-relevant residual risk has been shown, or the agent reports that there is no known close-relevant residual risk.
- Final acceptance is requested separately from sensitive-action Approval when final acceptance is required.

Useful close phrases:

```text
Show the close checklist.
I accept the residual risk shown here. Close with risk accepted.
Accepted. Close this task.
I do not accept it. Rework the UX before close.
```

"Accepted. Close this task." is normal close wording only when no residual-risk accepted close is being requested. When known residual risk is part of the close basis, use residual-risk accepted wording and expect the close reason to remain visibly different from verified or self-checked close.

## Where to go next

For the agent-side procedure, read [Agent Session Flow](agent-session-flow.md).

For deeper concepts before using Harness, read [Harness in One Task](../learn/harness-in-one-task.md) and [Concepts](../learn/concepts.md).

Detailed connector contracts and capability profiles belong in [Agent Integration Reference](../reference/agent-integration.md). Surface-specific setup belongs in [Surface Cookbook](../reference/surface-cookbook.md).
