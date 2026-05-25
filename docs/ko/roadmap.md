# 로드맵

## 이 문서가 도와주는 일

이 문서는 post-MVP 자동화 후보와 능력 확장 항목을 모아 둡니다. 독자가 나중에 다룰 수 있는 일을 볼 수 있게 하되, 그것을 첫 구현 작업으로 오해하지 않게 하는 것이 목적입니다.

이 문서는 다음을 확인할 때 사용합니다.

- 어떤 아이디어가 MVP 구현 계약 밖에 있는지
- 어떤 향후 능력이 승격되기 전에 정책, fixture, fallback 결정을 필요로 하는지
- 어떤 roadmap 항목이 owner가 명시적으로 scope를 부여하기 전까지 권한 없는 후보로 남아야 하는지

## 이 문서는 MVP 범위가 아닙니다

이 문서는 MVP 구현 계약의 일부가 아닙니다.

Kernel invariant, public MCP schema, MVP 구현 요구사항, MVP 필수 conformance는 이 문서가 소유하지 않습니다. MVP는 local kernel을 입증합니다. 즉 상태, gate, artifact, verification, projection, reconcile, 하나의 기준 agent 접점이 안정적으로 동작하는지를 먼저 증명합니다. 아래 항목들은 이 기본 요소가 안정된 뒤에 이어갈 수 있는 후속 후보입니다.

이 roadmap은 첫 증명과 최종 증명이 담당 문서에서 명확해진 뒤의 후보를 다룹니다. Kernel Smoke, Agency-Hardened MVP, Core state/`task_events`/artifact 경로를 우회하는 대체 경로가 아닙니다. Dashboard, Browser QA Capture, Context Index, native hook expansion, connector marketplace, orchestration은 나중에 Harness 동작을 수집하거나 보여 주거나 확장할 수 있지만, 첫 실행 가능한 권한 루프를 대체하지 않습니다.

```mermaid
flowchart LR
  MVP["MVP local kernel stable basics"] --> Basics["state, gates, artifacts, verification, projection, reconcile, one reference surface"]
  Basics --> Stable["Kernel Smoke와 Agency-Hardened MVP는 MVP owner-doc work"]
  Basics --> Later["post-MVP roadmap candidates"]
  Later --> Promote["future version은 owner 결정 이후에만 가능"]
```

Kernel Smoke와 Agency-Hardened MVP는 모두 MVP 전달 단계이지 roadmap 범위가 아닙니다. 이 roadmap은 MVP 담당 문서가 요구하는 kernel 권한, Decision Packet, 잔여 위험 표시, detached verification, Manual QA, recover/export, fixture conformance 동작을 흡수하면 안 됩니다.

## 승격 규칙

Roadmap 후보는 향후 owner 결정이 다음을 부여한 뒤에만 v1 또는 이후 범위의 작업이 될 수 있습니다.

- 명시적인 향후 버전 owner decision. MVP 구현 중 유용해 보인다는 이유만으로 승격되지 않습니다.
- 명확한 capability profile 요구사항
- redaction 및 secret/PII handling policy
- runtime 접점을 capture하는 경우 test environment와 artifact retention policy
- fixture 또는 conformance target
- 지원하지 않는 접점에 대한 fallback 동작
- projection을 기준 상태로 취급하는 의존성 없음

```mermaid
flowchart TD
  Candidate["later capability candidate"] --> Profile["clear capability profile requirement"]
  Profile --> Policy["redaction, test environment, retention policy"]
  Policy --> Fixture["fixture 또는 conformance target"]
  Fixture --> Fallback["unsupported surface fallback behavior"]
  Fallback --> Projection["projection-as-canonical dependency 없음"]
  Projection --> Promote["명시적 re-scope decision에만 eligible"]
  Candidate -- "criterion 누락" --> Later["post-MVP roadmap item으로 유지"]
```

## 로드맵 항목

### Dashboard

Dashboard는 active Task, gate, approval, 근거 coverage, projection 최신성, artifact 무결성, reconcile item을 시각화할 수 있습니다.

MVP는 dashboard가 보여 줄 record, projection, conformance fixture를 먼저 안정화해야 하므로 이 항목은 later입니다. 향후 첫 버전은 `state.sqlite`, artifact ref, projection job status 위의 읽기 전용 보기여야 합니다. Dashboard가 Task state, evidence, acceptance, close readiness의 source of truth가 되어서는 안 됩니다.

### Browser QA Capture

Browser QA Capture는 v1 우선 후보이지 MVP 요구사항이 아닙니다. 연결된 접점이 지원하는 경우 automatic 또는 assisted capture가 Manual QA record를 위해 screenshot, console log, network trace, accessibility snapshot, workflow recording을 수집할 수 있습니다.

승격에는 declared `T6 QA Capture` capability profile, redaction 및 secret/PII handling policy, test environment setup, artifact retention rules, fixture 또는 conformance target, 지원하지 않는 접점의 fallback 동작이 필요합니다.

캡처한 browser QA 자료는 artifact refs를 통해 Manual QA records에 연결되어야 합니다. 일반적으로 `qa_capture`, `screenshot`, `log`, 또는 캡처한 파일이 console log, network trace, accessibility snapshot, workflow recording인 경우 `other`를 사용할 수 있습니다. 이는 QA 근거를 개선할 수 있지만 final acceptance가 아니며, 사람의 취향이나 경험 판단이 필요한 경우 Manual QA judgment를 대체하지 않고, verification independence requirements도 충족하지 않는 한 detached verification을 대체하지 않습니다.

지원하지 않는 접점은 사람이 작성한 Manual QA notes와 수동 제공 artifacts를 대체 경로로 사용해야 합니다. MVP는 automated browser capture를 요구하지 않고 Manual QA record와 artifact refs를 지원합니다.

### Cross-Surface Verification

Cross-surface verification은 verification bundle을 다른 agent 접점 또는 evaluator environment로 보낼 수 있습니다.

MVP에는 하나의 기준 접점과 detached verification bundle/manual evaluator instruction이면 충분하므로 later입니다. Cross-surface verify는 connector conformance와 capability profile이 안정된 뒤에 다뤄야 합니다.

### Native Hook Expansion

Native hook은 이를 지원하는 접점에서 더 강한 pre-tool guard, command interception, file write blocking, richer artifact capture를 제공할 수 있습니다.

Hook API가 접점마다 다르므로 later입니다. MVP는 기준 접점이 실제로 지원할 때만 concrete hook을 사용할 수 있습니다. 그 외에는 native hook이 capability-dependent enhancement입니다. Hook은 `prepare_write`를 보조할 수 있지만 Core 권한 경로를 대체하거나 지원하지 않는 접점을 기본적으로 MVP 실패로 만들면 안 됩니다.

### Advanced Sidecar Watcher

Advanced sidecar watcher는 file write, command execution, generated-file drift, artifact capture opportunity, repo baseline drift를 거의 실시간으로 관찰할 수 있습니다.

MVP는 cooperative `prepare_write`, git diff check, artifact registration, detective validator로 시작할 수 있으므로 later입니다. Advanced watching이 Core 상태 모델의 동작에 필수여서는 안 됩니다.

### Parallel Orchestration

Parallel Change Unit orchestration은 work를 여러 active implementation lane으로 나누고, dependency DAG를 관리하고, baseline을 분리하고, 동시에 생긴 근거를 조정할 수 있습니다.

Parallel execution은 stable lock, baseline freshness, approval scope composition, artifact partitioning, close semantics에 의존하므로 later입니다.

### Context Index

Context Index는 읽기 전용 context provider입니다. Agent가 관련 projection, artifact ref, repo file, doc, user note를 찾도록 도울 수 있지만 인덱싱된 지식을 Harness 상태로 취급하지 않습니다.

인덱싱된 memory는 kernel과 기준 기록 경계가 안정되기 전에 도입하면 local 권한을 흐릴 수 있으므로 later입니다. 향후 Context Index는 context의 순위를 매기고, 요약하고, 가져올 수 있지만 인덱싱되었거나 가져온 context가 쓰기 권한, Decision Packet 해소, 승인 부여, gate 충족, 근거 생성, verification 수행 또는 기록, QA 기록, QA 또는 verification 면제, Residual Risk 수용, 결과 수용, assurance level 상승, projection 대기열 추가 또는 새로고침, projection 최신성 변경, 구현 준비 상태 선언, Task 닫기를 해서는 안 됩니다.

```mermaid
flowchart LR
  Projections["projections"] --> Index["Context Index<br/>read-only retrieval"]
  Artifacts["artifact refs"] --> Index
  Repo["repo files"] --> Index
  Docs["docs and notes"] --> Index
  Index --> Agent["agent context"]
  Index --> Boundary["non-authoritative context only"]
```

Context Index는 향후 결정에서 담당 문서, 최신성 및 오래됨 규칙, privacy/redaction 동작, connector capability 기대사항, fixture 범위, 가져온 context와 기준 상태를 구분하는 표시 규칙을 부여할 때만 v1 작업이 되어야 합니다.

### Local Derived Metrics

Local Derived Metrics는 `state.sqlite.task_events`, run, validator result, projection job, reconcile item에서 diagnostic rate, count, duration, guard-trigger summary를 파생할 수 있습니다.

Metric은 권한이 아니라 파생값이므로 later입니다. 사용자가 process bottleneck, 보고 공백, 반복되는 운영 패턴을 찾는 데 도움을 줄 수 있지만 diagnostic-only입니다. Metric 표시는 상태 변경, gate 충족, 쓰기 권한, 승인 부여, 근거 생성, projection 대기열 추가 또는 새로고침, projection 최신성 변경, close readiness 또는 구현 준비 상태 변경, verification 수행 또는 기록, QA 기록, QA 또는 verification 면제, Residual Risk 수용, 결과 수용, assurance level 상승, Task close를 하면 안 됩니다.

```mermaid
flowchart LR
  Events["state.sqlite.task_events"] --> Metrics["파생 metrics"]
  Runs["runs"] --> Metrics
  Validators["validator results"] --> Metrics
  Projections["projection jobs"] --> Metrics
  Reconcile["reconcile items"] --> Metrics
  Metrics --> Interpretation["future user-facing interpretation rule"]
  Metrics --> Boundary["non-authoritative diagnostics only"]
```

Legacy operations guide의 후보 파생 metric:

- `direct_to_work_escalation_rate`
- `approval_turnaround_time`
- `verify_latency`
- `reopen_within_7d`
- `evaluator_blocked_due_to_missing_evidence`
- `same_session_verify_guard_triggered`
- `surface_fallback_rate`
- `mcp_connection_failure_rate`
- `projection_stale_duration`
- `reconcile_pending_count`
- `shaping_unresolved_decision_count`
- `horizontal_exception_rate`
- `tdd_red_missing_rate`
- `manual_qa_pending_duration`
- `evidence_insufficiency_rate`
- `architecture_drift_warning_count`
- `domain_language_mismatch_count`
- `interface_review_required_count`

이 metric들은 향후 결정에서 담당 문서, fixture 범위, 보존 동작, 필요한 경우 privacy/redaction 동작, 사용자에게 보여 줄 해석 규칙을 부여할 때만 v1 작업이 되어야 합니다. 그 경우에도 metric value는 파생값으로 남으며 상태 변경은 여전히 일반 Core owner 경로를 거쳐야 합니다.

### Team Profile 내보내기와 가져오기

Team profile 내보내기/가져오기는 policy 기본값, connector 프로필, 접점 능력 가정, validator 프로필, 프로젝트 설정 템플릿을 team에 공유할 수 있습니다.

MVP는 local kernel이므로 later입니다. Team sharing은 runtime state에 영향을 주기 전에 versioning, privacy review, secret handling, conflict behavior가 필요합니다.

## 추가 이후 후보

다음 항목도 향후 batch가 fixture와 implementation ownership으로 승격하기 전까지 later입니다. 즉 아래 항목은 현재 MVP 요구사항이 아닙니다.

- deployment, canary, rollback, merge, production-monitoring automation. Release Handoff는 그런 권한을 external로 남기는 v1 보고서/export profile로만 더 일찍 존재할 수 있습니다.
- artifact dashboard
- worktree-based fresh verify automation
- advanced architecture drift validator
- advanced public interface validator
- semantic domain language consistency checks
- status/approval/acceptance/Manual QA card UX expansion
- multi-agent policy and scheduling
