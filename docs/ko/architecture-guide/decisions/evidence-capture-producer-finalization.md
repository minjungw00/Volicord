# Evidence capture intent와 producer 최종화

## 배경

Evidence 스키마에는 verified command execution, verified tool invocation,
registered connection observation producer 분류가 예약돼 있었지만, 기준 구현에는 이를
만드는 authority-owned record나 전이가 없었습니다. 따라서 `record_run`은 직접
제출된 외부 tool 또는 connection 주장을 모두 협력적 보고로 강등해야 했습니다.
Artifact integrity, 설명용 tool 필드, `SourceRef`, raw guard payload, session 시각은
빠진 producer 권한을 안전하게 대신할 수 없습니다. Target relevance는 별도의 권한
축으로 유지됩니다.

Agent 입력에서 producer를 직접 만들면 주장자가 실행, 출력, 관찰자, relevance를
스스로 선택하게 됩니다. Host callback에서 즉시 producer를 만들면 callback
transaction과 이후 Run commit 사이로 권한이 분리되고 Run에 속하지 않는 두 번째
영속 artifact 생명주기도 필요합니다.

## 결정

세 producer kind 모두 하나의 3단계 invariant를 사용합니다.

`EvidenceCaptureIntent -> EvidenceCaptureReceipt -> record_run finalization`.

1. 새 안정적 workflow 메서드 `volicord.prepare_evidence_capture`는 불변 15분
   intent만 만듭니다. 현재 Task, Change Unit, scope revision, baseline, target,
   workspace, 요청 connection과 actor, 정확한 canonical command/tool input digest 또는
   Core가 파생한 connection source-selector digest, 예상 결과를 결합합니다.
2. 등록된 source가 intent를 충족하고 불변 영속 source-fact receipt와 크기가 제한되고
   가려진 receipt-artifact staging, 각 원천 사실에 대한 배타적 정규화 claim을
   원자적으로 만듭니다. Agent API에는 receipt나 producer 생성 메서드가 없습니다.
3. `volicord.record_run`이 전체 체인을 다시 검증하고 기존 원자적 Core commit 안에서
   receipt artifact를 승격하고, 불변 producer와 그 1:1 EvidenceObservation을
   삽입하고, record를 연결하고, event와 replay row를 추가하고, state를 한 번
   진행시킵니다.

권한 invariant는 공유하지만 source adapter는 의도적으로 다릅니다.

- Volicord 소유의 관리용 command runner가 digest에 결합된 정확한 UTF-8 인자 벡터를
  실행하고 exit status와 output digest를 기록합니다. 명령을 허가, 승인,
  sandbox하지 않습니다.
- 등록 tool adapter는 같은 connection, session, host invocation ID, tool,
  canonical input을 가진 정확한 pre/post 쌍을 요구합니다. session/시간 fallback을
  사용하지 않습니다. 등록 hook은 협력적 host consistency를 제공할 뿐 암호학적
  host attestation이나 같은 로컬 주체로부터의 보호를 제공하지 않습니다.
- 등록 connection 관찰은 폐쇄형 event kind가 intent 이전 selector와 일치하는
  정확한 등록 guard 사실 또는 intent에 결합된 connection과 session의 유일한 현재
  `active` baseline에서 나온 완전하고 degraded되지 않은 session-watcher snapshot을
  요구합니다. Event/observation identity, observation time, raw-event 또는 snapshot
  digest는 source 소유 receipt에서만 확정됩니다.

Store는 호출자가 고른 claim을 받지 않고 엄격한 receipt와 불변 capture spec에서 claim
집합을 파생합니다. Command capture는 정규화한 host invocation을 claim합니다. Tool
capture는 정규화한 host invocation과 서로 다른 guard event 두 개를 모두 claim합니다.
Guard connection capture는 guard event 하나를 claim하고 watcher capture는 observation
하나를 claim합니다. Capture class에 필요한 좌표가 누락되거나, 추가되거나, 모호하면
거부하며 receipt, staging body, 모든 claim은 함께 commit되거나 rollback됩니다. Host
invocation claim identity에는 connection, session, installation, host-local invocation
좌표가 포함되므로 서로 무관한 host-local namespace는 충돌하지 않습니다.

Capture intent, receipt, producer, 승격된 receipt artifact 체인은 원시 명령, 환경값,
stdout, stderr, tool input, tool response, 크기 제한 없는 host payload를 저장하지
않습니다. 기존 guard-event subject 저장은 별도 경로입니다. 이 경로는 현재 guard
redaction 규칙이 허용하는 tool 필드를 포함한 redacted `raw_event`를 보존할 수
있습니다. 그 guard record는 capture receipt나 producer가 아니며 어느 쪽도 대신할
수 없습니다. Receipt에는 크기가 제한된 안전한 identity, digest, 관찰 결과, source
ref, completeness, limitation을 저장합니다. 불완전하거나 잘린 source는 적격
producer를 만들지 않습니다. 완전한 producer도 별도의 supported relevance 평가
없이는 Strong Evidence가 되지 않습니다.

저장된 expectation과 일치하는 완전한 outcome은 강한 producer provenance를 만들고 relevance는
`unassessed`로 남습니다. 선택한 target에 대한 뒷받침을 스스로 권한화하지 않습니다.
저장된 expectation과 완전히 불일치하는 outcome은 `contradicted` relevance가 됩니다. 명시적으로 참조한
intent가 없거나 오래됐거나 손상됐거나 맥락이 다르거나 이미 소비됐다면 조용히
강등하지 않고 거부합니다. Intent가 없는 입력은 기존 협력적 강등 동작을 유지합니다.
배타적 source claim은 사실 하나를 여러 분류로 capture하지 못하게 하므로 producer
분류는 불변 intent의 정확한 capture kind를 따릅니다. Producer 재사용은 기존 evidence
reuse chain을 사용합니다.

## 저장소와 호환성

이 모델은 `evidence_capture_intents`, `evidence_capture_receipts`,
`evidence_capture_source_claims`, `evidence_producers`와 `evidence_producer`
artifact-link owner kind를 추가합니다. Intent, claim, producer record는
insert-only입니다. Receipt, staged bytes, 모든 source claim은 Store 경계에서 함께
만듭니다. 프로젝트 범위 source-claim key는 정확한 원천 invocation, event, watcher
observation이 둘 이상의 intent나 producer class를 충족하지 못하게 합니다. 복합 receipt
외래 키는 producer의 intent/receipt 교차 결합을 막습니다. Producer의 intent와
observation identity는 unique이므로 완료한 intent를 두 번 소비할 수 없습니다.

초기 producer-finalization 모델은 `baseline_sqlite_v4` / `0.7.0`의 비호환 canonical
SQLite 형태 변경이었습니다. 현재 baseline은 이 record family를 `baseline_sqlite_v5`에
유지합니다. 후속 connection-selector 보정은 호출자가 제공하던 미래 observation digest를
공개 및 영속 capture-spec 형태에서 제거하지만 table, column, index, constraint를 추가하지
않습니다. 따라서 별도 storage-profile 또는 package-version 전이를 만들지 않고 현재
pre-major `baseline_sqlite_v5` / `0.8.0` 계약 batch 안에서 완료합니다. 제거된 형태에는
legacy alias나 fallback decoder가 없습니다. 호환되지 않는 v3과 v4 Runtime Home은
계속 compatibility 검사에 실패하며 다시 만들어야 합니다.

## 결과

- Agent는 의도한 capture와 target을 선언할 수 있지만 이후 인용할 실행 receipt,
  producer 권한, supported relevance를 만들 수 없습니다.
- Connection 호출자는 intent 이전 source selector만 결합할 수 있습니다. Host가 생성할
  event/observation ID, source timestamp, snapshot digest, redacted raw-event digest를 미리
  예측하도록 요구받지 않습니다.
- Source output, target identity, Run 권한을 내용과 맥락에 결합하면서 transient
  fulfillment를 별도 Core commit으로 만들지 않으며 target relevance는 독립적으로
  평가합니다.
- 실패한 `record_run`은 producer, persistent artifact, observation, event, replay
  row, state 증가를 남기지 않습니다. 남을 수 있는 것은 만료되는 안전한 staged
  receipt뿐입니다.
- Producer record가 canonical 실행/관찰 receipt입니다. 승격된 artifact는 크기가
  제한된 output receipt이며 두 번째 권한 본문이 아닙니다.
- 체크인된 Codex와 Claude Code fixture는 parser compatibility만 세웁니다. 실제 host
  지원 주장은 invocation ID, output completeness, retry, resume, 병렬 호출에 대한
  opt-in live validation이 필요합니다.

## 거부한 대안

- 호출자 생성 producer record는 주장한 provenance를 스스로 권한화하므로
  거부했습니다.
- Artifact integrity, `SourceRef`, tool metadata, raw guard event를 producer로
  취급하는 방법은 어느 것도 source, 현재 근거, target relevance, 정확한 output을
  독립적으로 모두 결합하지 못하므로 거부했습니다.
- 일치하는 source outcome을 supported relevance로 취급하는 방법은 성공한 실행이나
  관찰이 임의의 기준을 스스로 승인할 수 있게 하므로 거부했습니다.
- Hook에서 persistent producer를 즉시 만드는 방법은 producer 권한을 Run
  finalization과 분리하고 Run 없는 persistent artifact transaction을 추가하므로
  거부했습니다.
- Session과 시각으로 tool event를 결합하는 방법은 동시 실행, retry, resume에서
  충돌할 수 있으므로 Strong Evidence에는 사용하지 않습니다.
- 미래 host-generated event 또는 watcher observation의 digest에 connection intent를
  결합하는 방법은 intent 이후 source identity, timestamp, snapshot/raw-event digest가
  호출자가 알 수 있는 intent 사실이 아니므로 거부했습니다.
- 원시 command 또는 tool output 저장은 권한을 개선하지 않으면서 secret, privacy,
  retention, response-budget 위험을 넓히므로 거부했습니다.
- 새 canonical 형태에 v3 profile을 재사용하면 compatibility 진단이 부정확해지므로
  거부했습니다.

## 관련 구현과 담당 문서

- `crates/volicord-core/src/methods`: intent 생성과 Run 최종화
- `crates/volicord-store/src`: source receipt staging과 insert-only record
- `crates/volicord-cli/src`: 관리용 command 및 등록 source adapter
- `crates/volicord-mcp/src`: intent-only Agent Connection tool projection
- [`volicord.prepare_evidence_capture`](../../reference/api/method-prepare-evidence-capture.md)
- [Core 모델](../../reference/core-model.md#9-evidence-and-run-authority)
- [저장 버전 관리](../../reference/storage-versioning.md)
