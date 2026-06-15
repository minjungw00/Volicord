# Start

This is the first-read overview for Harness. It introduces the concept model in ordinary language and routes exact contract questions to the reference owners.

## What Harness is

Harness is the local work-authority product/system for AI-assisted product work. It helps users and agents keep the basis of a task visible while the work moves: scope, user-owned judgment, evidence, verification criteria, final acceptance, residual-risk acceptance, and close readiness.

Harness itself is not a local authority record. Core is the local authority record for Harness state. Core records the authority state that Harness uses; Harness is the broader product/system around that record, its server/runtime components, connected surfaces, and reader workflows.

Users can still speak normally:

```text
Make this plan concrete enough to implement.
Tell me if the scope is getting bigger.
Show what I need to decide and what you can verify.
Before you say it is done, show the evidence and residual risk.
```

Harness gives an agent a place to keep those boundaries explicit instead of letting chat momentum turn hidden choices into product direction.

## The core concepts

These concepts are related, but they are not interchangeable:

| Concept | First-read meaning | Exact owner |
|---|---|---|
| Harness | The local work-authority product/system for AI-assisted product work. | [Scope](reference/scope.md) |
| Core | The local authority record for Harness state. | [Core Model](reference/core-model.md) |
| `Harness Server` | A serving/runtime component of Harness, not a synonym for Harness as a whole. | [Runtime Boundaries](reference/runtime-boundaries.md) |
| `Harness Runtime Home` | The local runtime data space for Harness operational data as storage/runtime owners define it. | [Runtime Boundaries](reference/runtime-boundaries.md) |
| `Product Repository` | The user's project workspace and product files. | [Runtime Boundaries](reference/runtime-boundaries.md) |
| `Projection` | A read-only state view or display of owner records. | [Projection Authority Reference](reference/projection-and-templates.md) |
| `Write Authorization` | The exact Harness product label for one compatible product-file write attempt under current Harness state. | [Core Model](reference/core-model.md) |

Do not use one of these terms as a shortcut for another. A readable `Projection`, a file in the `Product Repository`, or a directory selected as `Harness Runtime Home` is not Core. `Harness Server` names a component; it does not rename the whole product/system.

## The problem Harness solves

AI-assisted work can move faster than the record around it:

- A small request grows into a larger product change.
- A product choice gets buried in implementation.
- A passing test starts sounding like proof of the whole user experience.
- A user says "looks good" and the agent treats every unresolved judgment as settled.

Harness exists to make those substitutions visible. The useful question is not only "did the agent do something?" It is also "what was in scope, what did the user decide, what evidence supports the claim, what verification criteria were checked, what risks remain, and can the work honestly close?"

## One ordinary task

A user might ask:

```text
Add remember-me behavior to login, but clarify the plan before changing files.
```

A useful agent response does not choose hidden product or security behavior first. It names the narrow goal, likely non-goals, facts it can inspect, and judgments the user still owns:

```text
Goal I heard:
Add remember-me behavior to login without redesigning authentication.

Out of scope unless you decide otherwise:
Password reset, signup, social login, and unrelated session maintenance.

I can inspect:
The login form, session settings, and focused tests.

You likely need to decide:
Whether "remember me" means a longer session, remembered email, or both, and what session risk is acceptable.

Safe next step:
Inspect and return a narrow plan. No product-file writes yet.
```

The user did not need to name an internal mode. The agent clarified because the request touches product behavior, security implications, verification criteria, and user-owned judgment.

## Authority concepts stay separate

Harness documentation uses these concepts separately:

- User-owned judgment is a decision or assessment the user owns. An agent may explain options, but it must not invent the judgment.
- Sensitive-action approval is approval for a named sensitive step. It is not final acceptance and not `Write Authorization`.
- `Write Authorization` is the exact product label for Core authority around one compatible product-file write attempt. It must not be collapsed into ordinary write approval.
- Evidence is material support for a specific claim, such as a diff, test output, screenshot, log, source citation, review note, or artifact reference. Evidence supports a claim; it is not the user's judgment.
- Verification criteria are user-visible criteria for checking work. They guide what should be checked; they are not themselves evidence, final acceptance, residual-risk acceptance, or close readiness.
- Final acceptance is a user-owned judgment about the visible close basis.
- Residual-risk acceptance is a user-owned judgment about a named visible residual risk.
- Close readiness is the reference concept for whether a task can honestly close from its current state. In user-facing language, "ready to close" is a friendly alias only when it points back to close readiness.

For exact authority rules and non-substitution boundaries, use [Core Model](reference/core-model.md).

## What Harness is not

Harness is not a prompt pack, chat script, API wrapper, workflow engine, report generator, dashboard, hosted agent platform, `Product Repository`, or `Harness Runtime Home`.

Harness also does not turn a polished chat answer, generated summary, readable status card, or `Projection` into the authority record. Exact display boundaries belong to [Projection Authority Reference](reference/projection-and-templates.md), runtime and location boundaries belong to [Runtime Boundaries](reference/runtime-boundaries.md), and security wording belongs to [Security](reference/security.md).

## Baseline scope

The baseline scope is intentionally narrow. Use [Scope](reference/scope.md) for baseline, profile-gated, and out-of-scope boundaries.

## Where to go next

| Reader | Path |
|---|---|
| New user | [User Guide](use/user-guide.md) |
| Working user | [User Guide](use/user-guide.md) -> [Judgment Examples](use/judgment-examples.md) -> [Scope](reference/scope.md) |
| Agent author or operator | [Agent Guide](use/agent-guide.md) -> [Agent Integration Reference](reference/agent-integration.md) |
| Implementer | [Reference Index](reference/README.md) -> baseline scope -> API methods -> schema owners -> storage effects |
| Documentation maintainer | [Authoring Guide](maintain/authoring-guide.md) -> [Translation Guide](maintain/translation-guide.md) -> [Checks](maintain/checks.md) |

Use the [Reference Index](reference/README.md) when you need exact owner documents. New users should not need API schemas to understand the product.
