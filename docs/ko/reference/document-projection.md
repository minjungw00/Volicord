# 문서 Projection 참조

## 이 문서로 할 수 있는 일

이 참조 문서는 Harness가 기준 상태 기록과 artifact 참조를 바탕으로 사람이 읽을 수 있는 Markdown projection을 어떻게 생성하는지 확인할 때 사용합니다.

Projection의 권한 경계, managed block 동작, 사람이 편집할 수 있는 영역, artifact 참조 표시 방식, template tier, projection 최신성 규칙을 정의합니다. 기준 kernel state, MCP request/response schema, SQLite DDL, 설계 품질 정책 요구사항, 전체 template 본문은 이 문서가 정의하지 않습니다. 전체 template 본문과 표시 카드 형태는 [Template 참조](templates/README.md)에 있습니다.

마이그레이션 참고 자료는 이 참조 문서에 반영되었습니다. 이제 이 파일이 활성 projection 계약입니다.

## 읽는 시점

- Markdown projection 동작을 구현하거나 리뷰할 때
- 보고서, status card, Journey Card가 기준 상태인지 확인할 때
- projected Markdown에 남긴 사람이 쓴 내용이 상태로 반영되는 경로를 판단할 때
- `TASK`, `APR`, `RUN-SUMMARY`, `EVIDENCE-MANIFEST`, `EVAL`, `DIRECT-RESULT`가 참조하는 기준 기록 목록을 확인할 때
- 최신이 아니거나, failed이거나, drifted된 projection을 진단할 때

## Projection을 쉽게 말하면

Harness projection은 이미 기준 상태나 artifact storage에 기록된 작업을 사람이 읽기 쉽게 보여주는 view입니다. Projector는 `state.sqlite` record, `state.sqlite.task_events`, 등록된 artifact 참조를 읽어 `TASK`, `APR`, `RUN-SUMMARY`, `EVIDENCE-MANIFEST`, `EVAL`, `DIRECT-RESULT` 같은 Markdown을 생성합니다.

Markdown은 사람이 작업을 이해하고, 맥락을 다시 잡고, evidence 검토와 수정 제안을 할 수 있게 돕습니다. 하지만 Markdown이 작업을 소유하지는 않습니다. 보고서는 gate를 요약하거나, evidence link를 제공하거나, Write Authorization 참조를 표시하거나, Decision Packet을 보여줄 수 있지만, 보고서 문장 자체가 gate, evidence, authorization, decision이 되지는 않습니다.

엄격한 경계는 다음과 같습니다.

| Item | What it is | Authority |
|---|---|---|
| Raw artifact | diff, log, screenshot, checkpoint, bundle, manifest file 같은 durable evidence file | artifact store |
| 상태 기록 | Task, Change Unit, Decision Packet, Journey Spine Entry, Residual Risk, Run, Approval, Write Authorization, Eval, Manual QA record, Evidence Manifest, Artifact record, Reconcile Item 같은 기준 structured record | `state.sqlite` |
| Markdown 보고서 | record 및 artifact ref에서 만든 사람이 읽을 수 있는 projection | projector output |

Markdown 보고서는 evidence link를 제공하고 상태를 요약할 수 있지만 raw artifact나 상태 기록은 아닙니다.

## 담당하는 참조 범위

이 문서는 다음을 담당합니다.

- projection principles
- document authority matrix
- managed block rules
- 사람이 편집할 수 있는 영역의 rules
- artifact 참조 표시 rules
- template tiers
- projection 기준 기록 rules
- projection 최신성 and failure rules
- projection rule 수준의 `source_state_version`과 `managed_hash` 해석

## 여기서 다루지 않는 것

이 문서는 다음을 담당하지 않습니다.

- 기준 kernel state와 transition rules. [Kernel Reference](kernel.md)를 봅니다.
- public MCP request/response schemas. [MCP API And Schemas](mcp-api-and-schemas.md)를 봅니다.
- SQLite DDL과 storage layout. [Storage And DDL](storage-and-ddl.md)를 봅니다.
- 설계 품질 정책 계약. [설계 품질 정책](design-quality-policies.md)을 봅니다.
- operator command 의미. [Operations And Conformance](operations-and-conformance.md)를 봅니다.
- conformance fixture assertion 의미. [Operations And Conformance](operations-and-conformance.md)를 봅니다.
- connector capability profile. [Agent 통합 참조](agent-integration.md)를 봅니다.
- surface recipe. [Surface Cookbook](surface-cookbook.md)을 봅니다.
- 전체 template 본문과 표시 카드 형태. [Template 참조](templates/README.md)를 봅니다.

## 작은 generated TASK 예시

일부러 아주 작게 보여주는 예시입니다. 전체 렌더링 형태는 [TASK template](templates/task.md)에 있습니다.

```md
---
doc_type: task
task_id: TASK-0001
display_state: executing
projection_version: 7
source_state_version: 42
updated_at: 2026-05-06T09:30:15+09:00
---

# TASK-0001 Add Import Preview

<!-- HARNESS:BEGIN managed -->
## Current Summary
- mode: work
- lifecycle phase: executing
- next action: record evidence for CU-01
- evidence gate: pending
- verification gate: pending
- Manual QA: pending
- active change unit: CU-01
- projection freshness: current

## Evidence And Reports
- Run Summary: RUN-20260506-093015-LEAD-01
- Diff: DIFF-0001 (`artifact_id=ART-0001`, sha256:abc123..., redaction:none)
<!-- HARNESS:END managed -->

## User Notes and Proposals
-
```

## 사람이 편집할 수 있는 것

사람은 다음과 같이 명시적으로 편집 가능하다고 표시된 영역을 편집할 수 있습니다.

```md
## User Notes and Proposals
-
```

사람이 편집할 수 있는 text는 입력입니다. Note, question, correction, proposal을 담을 수 있습니다. Reconcile은 편집 내용을 읽고 `reconcile_items` candidate를 만들 수 있습니다. Accepted proposal은 Core state-changing action과 추가된 `state.sqlite.task_events` row를 통해서만 상태가 됩니다. Rejected proposal은 note 또는 rejected reconcile item으로 남습니다.

사람이 편집한 proposal은 Task summary, acceptance criteria, Domain Language, Module Map, Interface Contract, Manual QA note, 기타 상태 기반 기록을 대상으로 삼을 수 있지만 proposal 자체가 target 기록은 아닙니다.

## 사람이 상태에 직접 반영할 수 없는 것

사람은 다음 projection text를 기준 상태에 직접 반영할 수 없습니다.

- managed block content
- `source_state_version` 같은 front matter field
- current gate value, lifecycle phase, result, close reason, assurance level
- approval, verification, Manual QA, acceptance, residual-risk status
- Decision Packet, Journey Card, Journey Spine, Autonomy Boundary, Write Authority Summary, Implementation Micro-Plan, Change Unit DAG, Residual Risk, Stewardship Impact, Review Stage, Write Authorization display text
- artifact 참조 identity, hash, redaction state, artifact availability
- status card, Journey Card, 기타 generated display 접점
- template body

Managed block을 직접 편집한 내용은 수용된 상태가 아니라 drift입니다. 권한처럼 보이는 문구를 직접 편집해도 write를 허가하거나, decision을 해결하거나, 필수 근거 조건을 충족하지 않습니다. 또한 verification 또는 Manual QA를 대체하거나, 잔여 위험을 수용하거나, assurance를 높이거나, 작업을 닫거나, owner 기록을 변경하지 않습니다.

## Projection principles

1. Projection은 기준 기록이 아닙니다.
2. 운영 상태의 기준 기록은 `state.sqlite` current record 및 `state.sqlite.task_events`입니다.
3. Raw evidence의 기준 위치는 artifact store입니다.
4. Markdown 보고서는 상태 기록 및 artifact 참조를 바탕으로 생성됩니다.
5. Markdown 보고서는 기본적으로 raw artifact가 아닙니다.
6. Front matter는 identity, projection version 또는 status, `source_state_version`, timestamp/freshness metadata만 가집니다.
7. Managed block은 projector가 생성하며 필요하면 다시 생성될 수 있습니다.
8. 사람이 편집할 수 있는 영역은 note와 proposal을 위한 입력 영역입니다.
9. 수용된 human edit만 reconcile 또는 Core state-changing action을 통해 상태가 됩니다.
10. Large log, diff, trace, screenshot, bundle, checkpoint는 embed하지 않고 artifact ref로 연결합니다.
11. Projection failure 또는 최신이 아님은 underlying task result를 절대 바꾸지 않습니다.
12. User-facing card는 friendly label을 사용할 수 있지만 기준 gate name은 kernel field로 남습니다.
13. Decision Packet, Journey Card, Journey Spine, Autonomy Boundary, Write Authority Summary, Implementation Micro-Plan, Change Unit DAG, Residual Risk, Stewardship Impact 표시는 owner 기록 및 artifact ref에서 만든 기준 기록이 아닌 projection입니다.

## Document authority matrix

| 사실 또는 접점 | 기준 source | Projection 또는 표시되는 보기 | Update path |
|---|---|---|---|
| Current Task state | `state.sqlite.tasks`, `task_gates`, `state.sqlite.task_events` | `TASK` Current Summary와 status card | Core transition, then projector |
| Task continuity | `state.sqlite` Task, Change Unit, Run, Evidence Manifest, Eval, Manual QA, Decision Packet, Approval, Residual Risk, `task_gates.acceptance_gate`, acceptance Decision Packet user-decision state, close events, artifact ref, 필요할 때 `journey_spine_entries`, `state.sqlite.task_events` | `TASK` Journey Spine | Core transition 또는 reconcile, Journey reconstruction, then projector |
| Decision Packet | `state.sqlite.decision_packets`, 관련 `decision_gate` state, decision event, 관련 approval 또는 reconcile record, artifact ref, 필요할 때 연결된 `state.sqlite.residual_risks` | `TASK` Pending Decisions, Journey Card decision line, status/next responses, judgment-context resources, decision-packet resources; standalone projection이 켜져 있을 때 optional `DEC` | `request_user_decision` / `record_user_decision`, then projector |
| Journey Spine | `state.sqlite` Task, Change Unit, Run, Decision Packet, Approval, Evidence Manifest, Eval, Manual QA, Residual Risk, `task_gates.acceptance_gate`, acceptance Decision Packet user-decision state, close events, artifact ref, 필요할 때 `journey_spine_entries`, `state.sqlite.task_events` | `TASK` Journey Spine section, resume view, Journey Spine-oriented card | Core transition 또는 reconcile, Journey reconstruction, then projector |
| Journey Card | 현재 `state.sqlite` Task state, gate, active Change Unit, Autonomy Boundary summary, active Decision Packet ref, residual-risk summary, latest evidence/eval/QA/보고서 ref, projection 최신성 | `JOURNEY-CARD`, status card, `harness.status` card text, `harness.next` 현재 위치 text, significant resume output | 현재 상태에서 read 또는 projection 새로고침; card를 직접 편집하지 않음 |
| Autonomy Boundary | active `state.sqlite.change_units` Autonomy Boundary field와 관련 Decision Packet 해소/event | `TASK` Autonomy Boundary, Change Unit block, Journey Card autonomy line, standalone projection이 켜져 있을 때 optional related `DEC` | shaping update 또는 user Decision Packet 해소, then projector |
| Write Authorization | `state.sqlite.write_authorizations`와 관련 Task, Change Unit, approval, Decision Packet, baseline, consumed Run ref | `TASK` Write Authority Summary, Journey Card Write Authority Summary line, `RUN-SUMMARY` relation | `prepare_write`가 생성함; idempotent replay는 already committed response를 반환함; `record_run`이 authorization을 consume한 뒤 projector |
| Implementation Micro-Plan | 현재 `state.sqlite` Task state와 gate, active Change Unit scope와 Autonomy Boundary, Change Unit dependency summary, selected feedback-loop records, TDD가 selected된 경우 TDD traces, expected evidence needs, Decision Packet blockers, latest 보고서 refs | `TASK` Implementation Micro-Plan managed section | Accepted reconcile outcome 또는 Core state-changing action이 owner 기록을 업데이트한 뒤 projector |
| Change Unit DAG | `state.sqlite.change_units`, `state.sqlite.change_unit_dependencies`, dependency 관련 event, active Task state | `TASK` Change Unit Dependencies / DAG summary | shaping update 또는 reconcile, then projector |
| Residual Risk | `state.sqlite.residual_risks`, accepted-risk metadata와 residual-risk refs, related Decision Packet, evidence/QA/eval ref, artifact ref | `TASK` Residual Risk, standalone projection이 켜져 있을 때 optional `DEC` accepted-risk context, Journey Card residual-risk line | decision, evidence, QA, Eval, reconcile 또는 close flow에서 Core transition, then projector |
| Stewardship Impact Summary | `domain_terms`, `module_map_items`, `interface_contracts`, `feedback_loops`, TDD가 selected된 경우 TDD records, `state.sqlite.residual_risks`, `state.sqlite.decision_packets`, policy validator 결과, related refs | `TASK` Stewardship Impact와 status/resume stewardship display | Owner 기록 업데이트, validator 결과, reconcile, close flow, then projector |
| User Notes | human-editable input -> `reconcile_items` -> accepted state event/record | `TASK` User Notes and Proposals | human edit, reconcile decision, Core event |
| Shared Design | shared design record 및 event | `TASK` summary, `DESIGN`, standalone projection이 켜져 있을 때 optional `DEC` | Core transition 또는 reconcile, then projector |
| Domain Language | `domain_terms` table | `DOMAIN-LANGUAGE` projection | Core transition 또는 reconcile, then projector |
| Module Map | `module_map_items` table | `MODULE-MAP` projection | Core transition 또는 reconcile, then projector |
| Interface Contract | `interface_contracts` table | `INTERFACE-CONTRACT` projection | Core transition 또는 reconcile, then projector |
| Feedback Loop | `feedback_loops` table plus runs, artifacts, TDD traces, Manual QA, evidence manifests refs | `TASK` Stewardship Impact와 Evidence Manifest 설계 품질 coverage; MVP에는 standalone Feedback Loop projection이 없음 | `record_run` shaping 또는 evidence update의 `FeedbackLoopUpdate`, `record_manual_qa`의 `feedback_loop_ref`, 또는 reconcile, then projector |
| Approval | `approvals`, approval-shaped Decision Packet, 구현이 유지하는 경우 optional decision request routing/replay record, event; `approval_request_candidate` alone은 제외 | `APR` projection과 Approval Card | `request_user_decision(decision_kind=approval)`이 pending Approval record를 만들고, `record_user_decision`이 approval decision을 업데이트한 뒤 projector |
| Run summary | `runs` table plus artifact refs | `RUN-SUMMARY` projection | `record_run`, then projector |
| Direct result | direct run record plus artifact refs | `DIRECT-RESULT` projection | `record_run` / `close_task`, then projector |
| Evidence coverage | `evidence_manifests` plus artifact refs | `EVIDENCE-MANIFEST` projection | evidence module update, then projector |
| Verification verdict | `evals` plus artifact refs | `EVAL` projection과 verification card | `record_eval`, then projector |
| TDD trace | `tdd_traces` plus artifact refs | `TDD-TRACE` projection | `record_run` 또는 reconcile, then projector |
| Manual QA | Aggregate QA 요구사항 상태에는 `qa_gate`; 기록이 있을 때 `manual_qa_records` plus artifact refs | `MANUAL-QA` projection과 QA card | `record_manual_qa`, then projector |
| Raw evidence | artifact store plus `artifacts` records | 보고서 안의 artifact 참조 | artifact registry |
| Projection freshness | `projection_jobs.source_state_version`, `projection_jobs.projection_version`, job status, managed hashes, artifact records | front matter mirror, status card, operations output | projector and recovery tools |

Required authority statements:

- User Notes: human-editable input -> `reconcile_items` -> accepted state event/record
- Domain Language: `domain_terms` table -> `DOMAIN-LANGUAGE` projection; 기준 term row에 대한 public ref는 `StateRecordRef.record_kind=domain_term`을 사용합니다.
- Module Map: `module_map_items` table -> `MODULE-MAP` projection; 기준 module row에 대한 public ref는 `StateRecordRef.record_kind=module_map_item`을 사용합니다.
- Interface Contract: `interface_contracts` table -> `INTERFACE-CONTRACT` projection; 기준 contract row에 대한 public ref는 `StateRecordRef.record_kind=interface_contract`를 사용합니다.
- Feedback Loop: `feedback_loops` table -> `TASK`와 Evidence Manifest display; 기준 feedback-loop row에 대한 public ref는 `StateRecordRef.record_kind=feedback_loop`를 사용합니다. TDD Trace refs는 separate execution evidence refs로 남습니다.
- Decision Packet: `state.sqlite.decision_packets`와 관련 ref -> `TASK` Pending Decisions, status/next responses, judgment-context resources, decision-packet resources; standalone projection이 켜져 있을 때 optional `DEC` projection
- Journey Spine: owner 기록, artifact ref, `journey_spine_entries` supplement, `state.sqlite.task_events`에서 재구성합니다. 자체 authority record는 아닙니다.
- Journey Card: 현재 상태와 ref에서 만든 파생 표시입니다. 절대 기준 상태가 아닙니다.
- Autonomy Boundary: active `state.sqlite.change_units` boundary field -> projection 접점. 판단 재량이지 scope authority가 아닙니다.
- Write Authority Summary: active scope, approval, Write Authorization, baseline, guarantee ref에서 만든 파생 표시입니다. 절대 기준 상태가 아니며 work를 허가할 수 없습니다.
- Write Authorization: `state.sqlite.write_authorizations`는 specific allowed write attempt를 기록합니다. Scope, approval, evidence, verification, QA, acceptance, Residual Risk 수용이 아닙니다.
- Implementation Micro-Plan: 현재 Task와 Change Unit의 owner 기록 및 관련 참조에서 생성되는 `TASK`의 managed execution-aid section. 기준 상태가 아니고, 새 `ProjectionKind`도 아니며, scope authority, approval, Write Authorization이 아닙니다.
- Approval: `approvals`와 approval-shaped Decision Packet -> Approval record 존재 또는 변경 뒤에만 `APR` projection을 만듭니다. `prepare_write`가 반환한 `approval_request_candidate`는 candidate 표시로 보여줄 수 있지만 `APR` source는 아닙니다.
- Change Unit DAG: `state.sqlite.change_unit_dependencies`와 Change Unit ref -> dependency projection. scheduler 또는 authorization 접점이 아닙니다.
- Residual Risk: accepted-risk metadata/refs를 포함한 `state.sqlite.residual_risks` -> residual-risk display
- Stewardship Impact Summary: owner 기록, validator 결과, 참조에서 파생됨 -> `StewardshipImpactSummary` display. 기준 record는 아닙니다.
- Review Stages: Task, Change Unit, gates, evidence, validator 결과, residual-risk refs, stewardship owner refs -> Spec Compliance Review와 Code Quality / Stewardship Review라는 managed `TASK` 또는 `RUN-SUMMARY` display sections. 기준 records는 아니고, 새 `ProjectionKind` values도 아니며, detached verification도 아닙니다.

## Managed block rules

Managed block은 projector가 덮어쓸 수 있는 유일한 Markdown 영역입니다.

```md
<!-- HARNESS:BEGIN managed -->
...
<!-- HARNESS:END managed -->
```

규칙:

- Managed block content는 committed 상태 기록 및 artifact ref에서 생성됩니다.
- Projector는 `projection_jobs.source_state_version`, projection version, 렌더링 timestamp, managed hash를 기록합니다. Front matter는 operator를 위해 recorded source state version을 그대로 비춥니다.
- Managed hash는 `HARNESS:BEGIN`과 `HARNESS:END` marker lines를 제외한 projector-owned managed block body에서 계산하며, line endings를 LF로 normalize하고 projector rules가 요구하는 meaningful whitespace를 보존합니다.
- 렌더링 전에 managed block hash가 last projected hash와 다르면 projector는 reconcile item을 만들거나 업데이트합니다.
- Managed hash는 drift detection에만 사용하며 렌더링된 Markdown을 기준 상태로 만들지 않습니다.
- Projector는 managed block 내부의 direct edit를 accepted state로 조용히 취급하지 않습니다.
- Managed block을 다시 렌더링할 때 관련 없는 사람이 편집할 수 있는 영역은 보존해야 합니다.
- 렌더링 실패는 projection 최신성을 `failed` 또는 `stale`로 표시하며 상태를 롤백하지 않습니다.

Front matter는 diagnostic 용도로 compact하게 유지합니다. 렌더링된 object를 식별하고, projection version 또는 status를 표시하며, `source_state_version`을 그대로 비추고, 렌더링 timestamp를 포함할 수 있습니다. Large state summary, evidence body, gate rollup, artifact inventory는 포함하면 안 됩니다.

`projection_version`은 projection/template/job version입니다. State clock이 아니며 source-state freshness basis로 사용하면 안 됩니다. `source_state_version`은 렌더링 source로 사용한 affected-scope state clock 값입니다. Projection이 task-scoped이면 Task State Version이고, 그렇지 않으면 Project State Version 또는 extension-defined owner state clock입니다.

기준 per-projection value는 성공한 렌더링 job의 `projection_jobs.source_state_version`입니다. Front matter `source_state_version`은 operator diagnosis를 위해 그 값을 그대로 비출 뿐입니다.

## Human-editable section rules

- 사람이 편집할 수 있는 text는 입력이지 기준 상태가 아닙니다.
- Reconcile은 edit를 읽고 상태 변경이 필요할 수 있으면 `reconcile_items` candidate를 만듭니다.
- Accepted proposal은 Core transition과 추가된 `state.sqlite.task_events` row를 통해서만 상태가 됩니다.
- Rejected proposal은 note 또는 rejected reconcile item으로 남습니다.
- Projector는 refresh 중 사람이 편집할 수 있는 content를 보존해야 합니다.
- 사람이 편집한 proposal은 Task summary, acceptance criteria, Domain Language, Module Map, Interface Contract, Manual QA note, 기타 상태 기반 기록을 대상으로 삼을 수 있지만 proposal 자체가 target 기록은 아닙니다.

## Artifact reference 렌더링

Markdown 보고서는 artifact reference를 compact하고 consistent하게 표시합니다. Payload shape는 MCP API document가 담당하며, projection은 presentation rule만 담당합니다.

권장 display:

```text
- Diff: DIFF-0001 (`artifact_id=ART-0001`, sha256:abc123..., redaction:none)
- Test log: LOG-0002 (`artifact_id=ART-0002`, sha256:def456..., redaction:redacted)
- Bundle: BUNDLE-0001 (`artifact_id=ART-0003`, sha256:789abc..., redaction:secret_omitted)
```

규칙:

- 모든 artifact ref는 artifact record로 해석되어야 합니다.
- 모든 raw artifact ref는 integrity metadata와 redaction 상태를 가져야 합니다.
- Large 또는 sensitive evidence는 Markdown에 붙여 넣지 않고 link만 둡니다.
- Missing 또는 hash-mismatched artifact는 related evidence 또는 projection 최신성을 `stale`로 표시합니다.
- State record ref는 record identity와 optional projection path를 사용합니다. `record_kind=projection`에서 identity는 `projection_jobs.projection_job_id`이고 path는 locator일 뿐입니다. Raw artifact ref로 표시하지 않습니다.
- `artifact_links.record_kind`는 existing same-Task state owner 또는 projection ref로 해석되어야 합니다. MVP artifact links는 Task-scoped입니다. `record_kind=projection`은 같은 `task_id`를 가진 completed `projection_jobs` row로 해석되며, link의 `record_id`는 `projection_jobs.projection_job_id`이고 path display는 `StateRecordRef.projection_path` 또는 `projection_jobs.output_path`를 사용합니다. Project-level owner rows와 project-level projection jobs는 future extension이 project-scoped artifact linking을 추가하지 않는 한 state 또는 projection job freshness/output metadata를 사용합니다. `EXPORT`는 `ProjectionKind`일 뿐입니다. Export snapshot과 component는 owner record 또는 `record_kind=projection`에 link되는 artifact로 남으며 `record_kind=export`에 link하지 않습니다.

## Template tiers

Projection template은 API `ProjectionKind` tier와 일치합니다.

| Tier | Templates | Rule | Template reference |
|---|---|---|---|
| MVP-required | `TASK`, `APR`, `RUN-SUMMARY`, `EVIDENCE-MANIFEST`, `EVAL`, `DIRECT-RESULT` | MVP projector는 이를 렌더링해야 합니다. | [TASK](templates/task.md), [APR](templates/approval.md), [RUN-SUMMARY](templates/run-summary.md), [EVIDENCE-MANIFEST](templates/evidence-manifest.md), [EVAL](templates/eval.md), [DIRECT-RESULT](templates/direct-result.md) |
| MVP-optional | `MANUAL-QA`, `TDD-TRACE`, `DOMAIN-LANGUAGE`, `MODULE-MAP`, `INTERFACE-CONTRACT` | Policy가 적용되거나, 관련 기록이 있거나, user/operator가 projection을 켰을 때 렌더링합니다. | [MANUAL-QA](templates/manual-qa.md), [TDD-TRACE](templates/tdd-trace.md), [DOMAIN-LANGUAGE](templates/domain-language.md), [MODULE-MAP](templates/module-map.md), [INTERFACE-CONTRACT](templates/interface-contract.md) |
| Extension / optional | `DEC`, `DESIGN`, `EXPORT`, `JOURNEY-CARD` | 해당 선택 projection이 켜져 있을 때만 렌더링합니다. | [DEC](templates/decision-packet.md), [DESIGN](templates/design.md), [EXPORT](templates/export.md), [JOURNEY-CARD](templates/journey-card.md) |

`ProjectionKind` tiering은 렌더러 지원 기대사항을 정하지만 projection을 기준 상태로 만들지 않습니다.

`EXPORT` template은 선택적 projection output입니다. Artifact links를 위한 `export` state record를 도입하지 않습니다.

Persisted `JOURNEY-CARD` Markdown은 선택 사항입니다. `harness.status`, `harness.next`, significant resume flow의 현재 위치 Journey Card 표시는 agency conformance에 필요합니다.

MVP Decision Packet visibility는 `TASK` projections, status/next responses, judgment-context resources, decision-packet resources를 통해 제공되어야 합니다. Standalone `DEC` Markdown은 standalone Decision Packet projection 기능이 켜진 경우에만 사용합니다.

Decision Packet record ID는 `DEC-*`를 사용합니다. `projection_kind`의 `DEC`는 projection kind label일 뿐입니다. Standalone projection에 별도 identity가 필요하면 `DEC-PROJ-0001` 같은 별도 `projection_id`를 사용합니다.

표시 카드 형태도 [Template 참조](templates/README.md)에 있습니다: [Compact Status Card](templates/compact-status-card.md), [Approval Card](templates/approval-card.md), [Verification Result Card](templates/verification-result-card.md), [Manual QA Card](templates/manual-qa-card.md).

## Freshness and failure rules

Projection 최신성은 current owner 또는 affected-scope state clock, 기준 `projection_jobs.source_state_version`, projection job state, managed hash, artifact availability, 알려진 `stale` trigger에서 계산됩니다. Front matter `source_state_version`은 마지막 successful 렌더링의 기준 값을 그대로 비추어, operator가 Markdown을 기준 상태로 취급하지 않고도 최신이 아닌 projection을 진단할 수 있게 합니다.

| Projection | Generated when | Stale when |
|---|---|---|
| `TASK` | Task가 created, resumed, changed, refreshed될 때 | `TASK` projection에서 current `tasks.state_version > projection_jobs.source_state_version`인 경우, managed block drift, 해소되지 않은 reconcile required, stewardship owner ref 또는 설계 품질 validator 결과 changed |
| `APR` | `request_user_decision(decision_kind=approval)`이 committed approval request를 create하거나, `record_user_decision`을 통해 approval decision이 changed될 때 | approval-shaped Decision Packet, linked Approval record status, scope, baseline, expiry, decision note가 changed |
| `RUN-SUMMARY` | `record_run`이 `runs.status=completed`, `interrupted`, `blocked`, `violation`을 포함한 Run을 commit할 때 | run relation changed, artifact ref missing, artifact integrity fails |
| `EVIDENCE-MANIFEST` | evidence coverage가 changed될 때 | baseline drift, changed files modified, required evidence missing/`stale`, approval expired |
| `EVAL` | verification result가 기록될 때 | Eval 후 baseline changes, evidence `stale` 상태, independence relation invalidated |
| `DIRECT-RESULT` | direct run이 closes 또는 escalates될 때 | changed file drift, escalation state changes, artifact ref missing |
| `EXPORT` | export/보고서 projection이 generated될 때. 켜진 경우 Release Handoff profile 포함 | 포함된 Task/gate/Change Unit/Decision Packet/Residual Risk/evidence/verification/Manual QA/artifact/projection/redaction/checklist source가 changed 또는 unavailable일 때 |
| `DEC` | standalone Decision Packet projection이 켜져 있고 Decision Packet이 created, requested, `resolved`, `deferred`, `rejected`, `blocked`, `superseded`될 때 | packet status, affected scope, current-state context, related approval/reconcile state, residual-risk ref, evidence 참조가 바뀔 때 |
| `JOURNEY-CARD` | card가 렌더링되거나 projection으로 persisted될 때. `harness.status`와 `harness.next`가 projection job 없이 ephemeral하게 반환할 수도 있음 | 표시된 Task/gate/Change Unit/Autonomy Boundary/Write Authorization/approval/baseline/guarantee/Decision Packet/Residual Risk/evidence/보고서/freshness source가 렌더링된 card보다 앞서 이동할 때 |
| `DOMAIN-LANGUAGE` | domain terms change | term conflict, accepted term record changes, related code representation moves |
| `MODULE-MAP` | module map records change | module path, public interface, dependency direction, internal complexity, 테스트 경계, owner 결정, watchpoint changes |
| `INTERFACE-CONTRACT` | interface contract records change | linked interface, caller, compatibility impact, boundary tests change |
| `TDD-TRACE` | trace recorded 또는 updated | red/green log missing, baseline drift, linked test file changes |
| `MANUAL-QA` | QA record created 또는 updated | linked UI/code changes, required capture missing, 해소되지 않은 finding |

Freshness state:

| State | Meaning |
|---|---|
| `current` | projected content가 기준 `projection_jobs.source_state_version`에 기록된 committed state version 및 managed hash와 match함 |
| `stale` | state 또는 referenced evidence가 projection보다 앞서 이동함 |
| `failed` | projector가 refresh를 attempted했고 failed함 |
| `unknown` | freshness를 compute할 수 없음. 보통 recovery 또는 migration 중 |

Projection `stale` 상태는 현재 readable context가 필요한 action을 block할 수 있지만, 그 자체로 lifecycle result, gate value, assurance를 바꾸지는 않습니다. Projection failure 또는 `stale` 상태는 underlying task result를 절대 바꾸지 않습니다.
