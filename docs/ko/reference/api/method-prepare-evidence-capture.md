<a id="volicordprepare_evidence_capture"></a>

# `volicord.prepare_evidence_capture` 참조

## 이 문서가 담당하는 내용

이 문서는 `volicord.prepare_evidence_capture`의 기준 동작을 담당합니다.

- 만료되는 불변 `EvidenceCaptureIntent` 하나 생성
- 메서드 요청, 결과, 기본값, 재실행, 무효과 분기
- Agent Connection 요청과 source-owned fulfillment 사이의 경계

Evidence와 Run 권한은 [Core 모델](../core-model.md#9-evidence-and-run-authority)이
담당합니다. 공통 intent, receipt, producer 형태는
[API 상태 스키마](schema-state.md)가, 저장 형태와 최종화 효과는 저장소 참조
문서가 담당합니다.

## 목적과 권한 경계

Agent Connection은 미래의 명령, 호스트 도구 호출, 등록 connection 관찰을 현재
Evidence 대상 하나에 정확히 결합하도록 요청할 수 있습니다. 이 메서드는 intent만
생성합니다. source가 실행됐다고 기록하지 않고 `EvidenceProducer`를 만들지 않으며
Strong Evidence를 부여하지 않습니다.

등록된 source만 intent를 충족할 수 있습니다.

- `verified_command_execution`: 관리용
  `volicord evidence capture-command` runner가 digest에 결합된 정확한 인자 벡터를
  실행하고 결과를 캡처합니다. 이 로컬 fulfillment runner는 Linux와 macOS에서
  지원되며 다른 platform에서는 실행 전에 거부합니다.
- `verified_tool_invocation`: 활성 등록 guard adapter가 같은 connection, session,
  host invocation ID, tool 이름, canonical input digest, 완전한 result digest를 가진
  정확한 pre/post 쌍을 결합합니다. session/시간 fallback은 적격하지 않습니다.
- `registered_connection_observation`: 활성 등록 guard 관찰 또는 baseline과 현재
  scan이 모두 완전하고 degraded되지 않은 session-watcher 관찰이 intent에
  결합된 source selector와 일치해야 합니다. 구체적인 event 또는 watcher
  observation identity와 observation digest는 fulfillment에서만 생성되는
  source 소유 사실로 남습니다.

Fulfillment는 불변 영속 `EvidenceCaptureReceipt` source-fact record와 크기가 제한되고
가려진 transient receipt artifact staging handle을 만듭니다. Core 상태는 진행시키지 않습니다.
`volicord.record_run`만 intent와 receipt를 다시 검증하고, receipt artifact를
승격하고, 불변 `EvidenceProducer`와 그 1:1 `EvidenceObservation`을 Core 커밋 하나로
만들 수 있습니다.

## 요청

```yaml
PrepareEvidenceCaptureRequest:
  envelope: ToolEnvelope
  task_id: string
  change_unit_id: string
  baseline_ref: string
  target: EvidenceTarget
  capture: EvidenceCaptureSpec

EvidenceCaptureSpec:
  # 정확히 하나의 tagged branch
  capture_kind: verified_command_execution
  command_sha256: 소문자 64자 SHA-256
  command_label: 정규화된 비어 있지 않은 안전한 표시 텍스트, 최대 256 UTF-8 bytes
  expected_exit_code: integer | null

  capture_kind: verified_tool_invocation
  tool_name: 앞뒤 공백을 제거한 비어 있지 않은 string, 최대 256 UTF-8 bytes
  tool_input_sha256: 소문자 64자 SHA-256
  expected_success: boolean | null

  capture_kind: registered_connection_observation
  source_selector:
    source_kind: guard_event
    event_kind: pre_tool | post_tool | prompt_capture | stop
  # 또는
  source_selector:
    source_kind: session_watcher
  expected_complete: boolean | null
```

`command_sha256`는 실행 파일을 0번째 요소로 포함하는 전체 UTF-8 인자 벡터의
canonical JSON에 대한 접두사 없는 소문자 SHA-256입니다. `tool_input_sha256`는
선택한 canonical tool input 객체에 대한 같은 형태의 digest입니다. Connection
observation 호출자는 observation digest, event ID, watcher-observation ID, source
timestamp, snapshot digest, raw-event digest를 제공하지 않습니다. Core는 대신
canonical `source_selector` JSON의 접두사 없는 소문자 SHA-256을
`input_sha256`로 파생합니다. Guard selector는 폐쇄형 `event_kind` 하나를
요구합니다. Session-watcher selector는 검증된 현재 connection과 정확한 session이
intent의 별도 좌표로 이미 결합되므로 추가 호출자 필드가 없습니다.
`session_start`는 선택할 수 없습니다. 정확한 session이 intent 생성 전에 이미
존재하므로 그 start event는 intent 이후 observation window를 만족할 수 없습니다. 이 digest에는
[API Core 스키마](schema-core.md)의 canonical JSON hash 규칙을 사용합니다. 원시
인자, 환경값, 명령 출력, tool input/output은 공개 요청 필드가 아니며 intent에
저장하지 않습니다.

`command_label`에는 공통 display-text 정규화를 적용하고, `tool_name`은 내부 identifier
텍스트를 바꾸지 않은 채 앞뒤 공백만 제거합니다. 그 뒤 비어 있지 않은지와 256
UTF-8-byte 상한을 검사하고 결과 값을 불변 intent에 저장합니다.

MCP 생략 기본값은 `expected_exit_code=0`, `expected_success=true`,
`expected_complete=true`입니다. 명시적 `null`도 같은 뜻입니다. 안전한 label이
비어 있거나 digest가 잘못됐거나 source selector가 지원되지 않거나 형태가
잘못됐거나 대상이 현재 Task에
속하지 않거나 Change Unit이 현재 상태가 아니거나 baseline이 호환되지 않거나
검증된 Agent Connection 맥락이 없으면 커밋 전에 거부합니다. Tool 및 registered
connection capture는 호출 맥락의 정확한 verified Agent Session도 요구하지만 command
capture는 요구하지 않습니다.

Intent는 선택한 project, Task, 현재 Change Unit, 현재 scope revision, 호환되는
baseline, 정확한 target, 현재 Git workspace identity, 요청 connection과 actor,
capture kind, canonical input digest 또는 source-selector digest, 예상 결과, 생성 시각,
고정 15분 만료에
결합됩니다. 이후 관련 없는 state-version 증가만으로 intent가 만료되지는 않지만,
결합된 근거가 바뀌면 오래된 상태가 됩니다.

## 결과

```yaml
PrepareEvidenceCaptureResult:
  base: ToolResultBase
  capture_intent_ref: StateRecordRef
  capture_intent: EvidenceCaptureIntent
  expires_at: UtcTimestamp
```

커밋 결과는 `effect_kind=core_committed`를 사용하고 authority event 하나와 intent
하나, replay row 하나를 삽입하며 `project_state.state_version`을 한 번
증가시킵니다. 반환 ref의 `record_kind`는 `evidence_capture_intent`입니다. 정확한
멱등 재실행은 원래 응답을 반환하며 두 번째 intent를 만들지 않습니다. dry run은
영속 ID, intent, event, replay row, receipt, artifact, producer, state-version 변화를
만들지 않습니다.

## Fulfillment와 receipt 규칙

충족 source는 project와 Task identity, 현재 Change Unit, scope revision, baseline,
target source selector, workspace identity, 요청 connection, 만료, Core가 파생한 정확한 input digest를
다시 확인합니다. 비활성 connection, 정확한 pre가 없는 post, invocation ID나
digest 불일치, 잘리거나 불완전한 output, degraded watcher scan, 이미 사용한
intent는 적격 receipt를 만들 수 없습니다. Guard event는 selector의 정확한
`event_kind`를 가져야 합니다. Watcher observation은 intent에 결합된 connection과
session의 유일한 현재 `active` baseline에 속해야 하며, 현재 baseline이 없거나
모호하면 거부합니다.

선택한 source observation은 반개구간
`intent.created_at <= observed_at < intent.expires_at`을 만족해야 합니다. Receipt는
`observed_at <= receipt.created_at < intent.expires_at`을 만족해야 하고 staging
handle은 정확히 `intent.expires_at`에 만료됩니다. Intent 이전 observation, expiry
시각의 observation, observation보다 이른 receipt timestamp, expiry 시각 이후에
생성한 receipt는 거부합니다.
Core finalization은 저장된 intent expiry가 더 늦더라도 현재 Core clock보다 미래인
observation 또는 receipt timestamp를 거부합니다.

Fulfillment는 불변 intent와 엄격한 receipt 형태에서 배타적인 source claim을
도출하며 호출자가 claim을 선택하지 않습니다. command는 정규화한 host invocation을,
tool은 정규화한 host invocation과 서로 다른 guard event 두 개를, guard observation은
해당 guard event를, watcher observation은 정확한 observation을 claim합니다. 각 기반
source 사실은 한 project에서 intent와 producer class 하나만 충족할 수 있습니다.
Source 좌표가 누락되거나 불필요하게 추가되거나 모호하거나 이미 claim되어 있으면
receipt 및 staging 생성을 포함한 fulfillment 전체를 거부합니다.

Connection fulfillment은 후보 source 사실 중 하나를 자동 선택하지 않습니다.
등록 source 명령은 guard event 하나 또는 watcher observation 하나를 정확히 지정해야
합니다. Source 좌표가 없거나, 두 source family를 모두 주거나, receipt에 불필요한
좌표가 있거나, 선택한 family에 좌표가 둘 이상이면 receipt 없이 거부합니다.
선택한 source의 정확한 ID, observation timestamp, raw-event 또는 snapshot/selection
digest, source별 안전 요약은 source 소유 receipt에 확정됩니다. 이 값을 나중에
불변 intent에 추가하거나 호출자가 미리 아는 필드로 취급하지 않습니다.

안전한 receipt JSON은 24 KiB로 제한되며 schema version, capture kind, intent ID,
input/result digest, 예상/관찰 결과, success/status 또는 exit code, 등록된
connection/session/guard 또는 watcher 참조, 관찰 시각, completeness, limitations,
`redaction_state=redacted`가 들어갑니다. 원시 명령, 환경값, stdout, stderr, tool
input, tool response, 비밀값, 크기 제한이 없는 host payload는 들어가지 않습니다.
Receipt record는 1회용이며 staging bytes에 내용으로 결합됩니다.

`result_sha256`는 완전한 `observed_outcome`의 canonical JSON에 대한 접두사 없는
소문자 SHA-256입니다. command outcome은 raw output 대신 exit code와 stdout/stderr
digest 및 byte count를 보존합니다. Command runner는 합산 최대 16 MiB를 streaming
처리하고 intent expiry 전에 끝나야 하며 어느 경계든 넘으면 receipt를 만들지
않습니다. tool outcome은 success, 선택적 exit code, 완전한
tool-result digest와 byte count를 보존합니다. registered connection outcome은
완전성, 선택한 observation digest, source별 safe summary를 보존합니다. staged
receipt 자체는 항상 `redaction_state=redacted`입니다.

등록 hook은 협력적 로컬 integration입니다. host 서명, actor attribution 증명,
OS 격리, 같은 로컬 주체에 대한 위조 방지 경계가 아닙니다. Command runner는 실행을
기록할 뿐 명령을 승인하거나 권한을 부여하거나 sandbox를 만들거나 테스트 충분성이나
넓은 정확성을 증명하지 않습니다.

## `record_run` 소비

Evidence input은 `input_refs`에 현재 `evidence_capture_intent` ref 정확히 하나를
넣어 이 경로를 요청합니다. Core는 intent와 receipt를 직접 읽습니다. 호출자가
제공한 tool 필드, actor 필드, output ref, receipt handle, 결과 metadata는 저장된
source 사실을 대체할 수 없습니다.

완전한 관찰 결과가 저장된 예상을 만족하면 Core는 강한 producer provenance를
기록하지만 관찰 relevance는 `unassessed`로 둡니다. 등록 source는 무엇이 실행되거나
관찰됐는지를 세우며, 그 결과가 선택한 대상을 뒷받침하는지는 판단하지 않습니다.
완전하지만 저장된 expectation과 불일치하는 결과는 `contradicted`로 보존합니다. 따라서
capture-backed 관찰만으로 필요한 기준을 충분하게 만들 수 없습니다. `supported`에는
별도의 담당 문서가 정의한 relevance 권한이 필요합니다. Capture intent ref는 assessing
actor 없이 `relevance_assessment.assessment_ref`의 분류 근거로 남으며, 별도 relevance
판정이나 support 권한이 아닙니다. 참조된 intent가
없거나 만료됐거나 이미 소비됐거나 손상됐거나 project/Task/Change Unit/connection이
다르거나 scope/baseline/workspace/target 기준이 오래됐거나 receipt bytes와
불일치하면 Core 커밋 없이 거부하며 조용히 강등하지 않습니다. Intent가 없는 입력은
기존 협력적 강등 규칙을 유지합니다.

Receipt 하나는 producer를 최대 하나만 만들고 producer 하나는 observation 하나에만
속합니다. Source-claim 배타성 때문에 사실 하나를 여러 분류로 capture할 수 없으며,
분류 선택은 fallback match가 아니라 intent의 정확한 capture kind를 따릅니다. 이후
재사용은 intent를 다시 소비하지 않고 기존 `reused_evidence` 체인을 사용합니다.

## 호환성과 외부 검증

이 clean pre-major selector 보정은 호출자에게 미래 observation digest를 요구하던
이전 형태를 제거합니다. Table, column, index, constraint를 추가하지 않고
`baseline_sqlite_v5` / `0.8.0` 계약 batch에서 도입된 보정이므로 그 자체로 별도
storage-profile 또는 package-version 전이를 만들지 않았습니다. 같은 보정 메서드 형태는
`baseline_sqlite_v7` / `0.9.1`에서도 현재 형태로 유지되며, v6의 호스트 역량 Registry
추가와 v7의 workflow 권한 추가는 이 메서드와 독립적입니다. 제거된
request/capture-spec 형태를 위한 legacy alias나 fallback decoder는 없습니다.
호환되지 않는 v5 또는 이전 Runtime Home을
변환하는 경로는 없으며, 호환되지 않는 상태는 명확히 실패하고 다시 만들어야
합니다. 현재 권장 release version은 `0.9.1`입니다.

체크인된 host fixture는 adapter만 검증합니다. 실제 Codex 또는 Claude Code의
invocation identity, output completeness, retry, resume, 병렬 호출 동작에 대한
지원 주장은 해당 opt-in live-host test가 필요합니다.

## 관련 담당 문서

- [실행 기록 메서드](method-record-run.md)
- [Core 모델](../core-model.md#9-evidence-and-run-authority)
- [API 상태 스키마](schema-state.md)
- [API 값 집합](schema-value-sets.md)
- [Agent Connection](../agent-connection.md)
- [보안](../security.md)
- [저장 레코드](../storage-records.md)
- [저장 DDL](../storage-ddl.md)
- [저장 효과](../storage-effects.md)
- [저장 버전 관리](../storage-versioning.md)
