# Credential 전달을 위한 호스트 역량 검증

## 맥락

Local-web User Channel 전달값에는 bearer credential이 들어갑니다. 이전 어댑터는 listener
준비 상태와 클라이언트가 선언한 `model_invisible_user_surface=true` boolean을 결합했습니다.
일반 MCP 클라이언트도 같은 값을 선언할 수 있었고, 연결 설정 검증에는 정확한
호스트·클라이언트 버전, 어댑터 프로필, 실행 파일 다이제스트, 실제 증거, 만료 이력이
남지 않았습니다.

이 선언은 협상 입력으로 유용하지만 credential 전달 판단의 원천이 될 수 없습니다. 설정,
프로세스 marker, `clientInfo`, 호스트 생명주기 관찰도 host attestation이 아니라 협력적
로컬 사실입니다. 영속 검증 행 하나가 관찰된 증거 다이제스트와 그 다이제스트를 대조할
신뢰된 예상값을 동시에 제공해서는 안 됩니다.

## 결정

Volicord는 credential을 포함한 local-web `_meta` 전달값을 만들기 전에 중앙 호스트 역량
평가기 하나를 사용합니다. 평가기는 관리되는 generic이 아닌 stdio 경로, 준비된 listener
lease, 정확한 클라이언트 선언, 만료되지 않은 변경 불가능한 `outcome=passed` 검증을 가리키는
현재 영속 역량 상태를 요구합니다. 검증은 정확한 Agent Connection, 호스트·클라이언트 버전,
어댑터 프로필·버전, 관리 지문, Volicord 빌드, source revision, target, 실행 파일
다이제스트, 크기가 제한된 증거 아티팩트 다이제스트와 일치해야 합니다.

예상 `evidence_artifact_sha256`에는 [호스트 릴리스 증거](../../reference/host-release-evidence.md)가
정의한 외부 `volicord-host-release-manifest-v3`를 신뢰해 획득하는 운영 경로가 필요합니다.
그 manifest는 현재 행과 같은
역량, 호스트·클라이언트, 어댑터, Volicord 빌드, source revision, target, 실행 파일
다이제스트뿐 아니라 예상 증거 아티팩트 다이제스트에도 결속되어야 합니다. 평가기는
manifest를 검증하고 행의 `evidence_artifact_sha256`을 그 예상값과 정확히 일치시켜야
합니다. Manifest가 없거나, 알 수 없거나, 잘못됐거나, 검증되지 않았거나, 일치하지 않으면
닫힌 상태로 실패합니다. 행 자체의 다이제스트, 빌드 설명자, 복사한 manifest 값은 이
대조를 대신하지 못합니다. V3 클라이언트 좌표는 실제 셀에 사용한 관리 MCP initialize에서
관찰한 정확한 크기 제한 이름·버전이어야 하며 호스트 종류, probe, 설정, 프로토콜 버전,
상수, 다른 셀이 이를 제공할 수 없습니다. 과거 `volicord-host-release-manifest-v1`과
`volicord-host-release-manifest-v2` 입력은 거부하며 v3 규칙으로 재해석하지 않습니다.

내장 stdio 어댑터에서 통과 행은 독립된 두 런타임 버전이 아니라 관찰한 호스트 버전 하나를
나타냅니다. `host_version == client_version == clientInfo.version`이어야 하고 그 값은 실제
아티팩트의 설치 호스트 버전과 같아야 합니다. 통과하는 `source_revision`은 정확한 소문자
40자리 또는 64자리 16진수이며 `unknown`은 통과할 수 없습니다. 버전 같음이나 source
revision을 증명할 수 없으면 통과하지 않은 outcome으로 게시해야 합니다.
검토한 Codex 좌표에서 설치 호스트 probe의 정확한 원문 envelope는
`codex-cli 0.144.4`, 정규 행 좌표는 `0.144.4`, 정확한 MCP initialize 정체성은
`codex-mcp-client`/`0.144.4`입니다. 또한 정확한 call별 Codex 메타데이터가 훅과 같은
불투명 root 세션 및 변경 불가능한 프로세스 로컬 thread 다이제스트 하나에 stdio 세션을
결속하기 전까지 자격이 없습니다. 이 값은 정확한 일치 대조 및 상관관계 입력이지 host
attestation이 아닙니다.

검증은 정규 UTC 타임스탬프를 사용하며
`observed_at <= created_at`과
`observed_at < expires_at <= observed_at + 86,400 seconds`를 만족해야 하며 통과 행은
`created_at < expires_at`도 만족해야 합니다. 평가는 반개구간
`observed_at <= now < expires_at`을 사용합니다. 24시간은 기본 수명, 신원 증명,
attestation 기간이 아니라 최대 최신성 구간이며 게시자는 더 짧은 만료 시각을 선택할 수
있습니다.

레지스트리는 변경 불가능한 이력을 `host_capability_verifications`에 저장하고, 연결과 역량별
현재 포인터 하나를 `host_capability_state`에 저장합니다. 이후 failed, unavailable, revoked
관찰을 게시하면 포인터를 원자적으로 옮깁니다. 취소는 이전 행의 변경이 아니라 새로 만든
변경 불가능한 `outcome=revoked` 행입니다. 평가는 더 오래된 passed 행을 뒤로 찾아
사용하지 않습니다. 현재 행이 없거나 형식이 잘못되었거나, 만료되었거나, 통과하지 않았거나,
일치하지 않으면 token, `_meta`, 프로젝트 시간 효과 없이 CLI inbox로 닫힌 상태에서
fallback합니다.

같은 ID와 같은 내용의 정확한 중복 게시는 멱등이고 그사이에 더 새로운 행으로 전진한 현재
포인터를 옮기지 않습니다. 그 ID를 다른 내용으로 재사용하면 충돌합니다.

V1 검증 `metadata_json`은 엄격한 정규 `{}`만 허용합니다. 허용되는 모든 증거 좌표에는
전용 열이 있으므로 임의 구성원이 선언되지 않은 신뢰 입력이나 민감한 호스트 자료 보존
위치가 될 수 없습니다.

Generic 연결, 사용자가 관리하는 클라이언트, 수동 stdio, CLI 검증 probe, Local HTTP
transport, 유효하지 않거나 알 수 없는 관리 시작 marker는 범주적으로 자격이 없습니다.
정확한 클라이언트·프로세스 값은 계속 일치 대조 입력이며 신원 증명이 되지 않습니다.

실제 호스트 검증 bootstrap은 민감한 bearer 경로로 자기 자신을 증명하면 안 됩니다. 이후
검증 전용 경로를 추가한다면 별도 host-delivery-verification `_meta` namespace에 비밀이 아닌
challenge를 보내고, User Action이나 token을 만들지 않으며, 호스트 소유 표면과 모델 대상
표면 부재에 대한 크기 제한 사람 확인을 요구할 수 있습니다. 증거는 최종 실행 파일이 생긴
뒤에만 생성되므로 그 다이제스트를 실행 파일에 다시 내장하면 안 됩니다. 그렇게 다시
빌드하면 결속 대상 실행 파일 다이제스트가 바뀌어 재귀 결속이 생깁니다. 대신 신뢰된 내부
획득 경로가 위의 외부 manifest를 검증한 뒤에만 pass를 게시하거나 평가해야 합니다. 현재
어댑터에는 그런 신뢰된 획득 경로가 없습니다. 또한 검토한 Codex `0.144.4` 표는
`local_web_user_channel`을 `unsupported_by_host`로 분류하므로 이 정확한 좌표는 통과하는
local-web 행의 자격이 없습니다. `null` 또는 아직 검토하지 않은 Codex 좌표는 호스트 종류
구현됨 fallback을 유지하고 Claude Code도 구현됨 fallback을 유지하지만, 신뢰된 획득 경로가
없으므로 그런 경로는 구현되었지만 검증되지 않은 상태에 머뭅니다. 따라서 운영 local-web
자격은 닫힌 상태로 실패하며 CLI inbox를 사용합니다.

새 레지스트리 형태는 `baseline_sqlite_v6`입니다. v5 변환, relabel, 추론한 pass, 합성 이력은
없으며 호환되지 않는 Runtime Home은 다시 만들어야 합니다.

## 결과

- 클라이언트 선언만으로 bearer URL을 만들 수 없습니다.
- 호스트·클라이언트 업그레이드, 관리 설정 변경, 실행 파일 변경, 만료, 취소, 이후 실패한
  검증은 모두 자격을 없앱니다.
- 상태 보기, fallback 선택, 최종 token materialization은 같은 평가기를 사용하며,
  materialization은 listener 발급 lease를 보유한 채 현재 영속 상태를 다시 확인합니다.
- 빌드 메타데이터는 일치 대조 좌표일 뿐 증거 다이제스트의 신뢰 원천이 아닙니다. 정확한
  최종 아티팩트 릴리스 증거는 바이너리 밖에 둡니다.
- CLI inbox는 계속 지원되는 완전한 form fallback입니다.
- 저장된 각 검증은 크기가 제한된 운영 증거이며 이력은 추가만 가능합니다. 호스트 격리,
  현재 사용자 신원, 이후 외부 호스트 동작을 증명하지 않습니다.
- Agent Connection을 제거하면 그 연결의 역량 상태와 이력만 연쇄 삭제합니다.

## 비목표

- 공개 Core API 메서드를 추가하거나 관리 검증 명령을 공개 API 메서드로 만들지 않습니다.
- 암호학적 host attestation, OS 격리, 사용자 인증, 같은 로컬 principal에 대한 위조 방지를
  제공하지 않습니다.
- 정확한 실제 호스트 아티팩트가 생기기 전에 Codex 또는 Claude Code local-web 양성 결과를
  주장하지 않습니다.
- Bearer URL이나 token, prompt, transcript, screenshot, 원문 호스트 아티팩트, 비공개
  운영자 데이터, 임의 검증 메타데이터를 저장하지 않습니다.

## 거부한 대안

- Boolean, `clientInfo`, 환경 marker, 프로세스 인자를 신뢰하는 방안은 모두 호출자 또는
  호스트가 제어하는 협력적 입력이므로 거부했습니다.
- 연결 `complete`, `last_verification_report_json`,
  `guard_installations.host_capability_json`을 재사용하는 방안은 정확하고 만료되는 전달 증거와
  이력이 아니라 변경 가능한 설정 또는 훅 상태를 설명하므로 거부했습니다.
- 일반 클라이언트를 allowlist로 허용하는 방안은 복사한 이름이나 버전이 관리 내장 호스트
  경로를 확립하지 못하므로 거부했습니다.
- Fixture나 direct-wrapper 출력을 실제 호스트 증거로 보는 방안은 호스트 소유 전달이나
  모델 대상 표면 부재를 관찰하지 못하므로 거부했습니다.
- 실제 bearer URL로 bootstrap하는 방안은 자격 불변조건이 확립되기 전에 민감 경로를
  노출하므로 거부했습니다.
- 행 자체의 `evidence_artifact_sha256`을 예상값으로 쓰는 방안은 신뢰 점검이 자기 선언이
  되므로 거부했습니다.
- 릴리스 증거 다이제스트를 바이너리에 내장하는 방안은 증거가 최종 바이너리 생성 뒤에
  만들어지고, 이를 내장하려고 다시 빌드하면 결속할 실행 파일 다이제스트가 바뀌므로
  거부했습니다.
- 더 오래된 passed 행으로 fallback하는 방안은 취소, 만료, 이후 실패 관찰의 효력이 계속
  유지되어야 하므로 거부했습니다.

## 관련 구현 영역

- [`crates/volicord-store`](../../../../crates/volicord-store): 레지스트리 스키마, 변경 불가능한
  이력, 현재 포인터, 검증, 정확한 일치 평가.
- [`crates/volicord-mcp`](../../../../crates/volicord-mcp): initialize 입력 보존, 시작 프로필
  결속, 중앙 평가, listener lease, fallback 선택, token materialization.
- [`crates/volicord-cli`](../../../../crates/volicord-cli): 크기가 제한된 진단 상태 보기와 이후
  엄격한 검증 아티팩트 import.

## 관련 테스트와 참조 담당 문서

테스트는 누락·비통과 상태, 만료, 현재 포인터 교체, 선택된 결속 불일치, listener·예산
경쟁, replay 비발급, 현재 pass가 없는 generic 자기 선언, 정확한 관리 호스트 positive
fixture 하나를 다룹니다. 시작 원천 테스트는 수동 stdio와 CLI 검증을 별도로 분류하고,
Local HTTP에는 전송 중심 테스트가 있습니다. 그러나 그 밖의 모든 값이 정확한 현재 pass를
게시한 뒤 수동 stdio, CLI 검증, Local HTTP 각 경로에서 비발급을 증명하는 negative
회귀 테스트는 아직 없습니다. 그 테스트를 추가하기 전에는 이 경로들이 다뤄졌다고 주장하면
안 됩니다. 실제 호스트 검증은 별도 외부 셀이며 fixture로 대신할 수 없습니다.

참조 담당 문서:

- [호스트 릴리스 증거](../../reference/host-release-evidence.md)와
  [외부 게이트 결정](host-release-evidence-gate.md)
- [Agent Connection](../../reference/agent-connection.md)
- [MCP 전송](../../reference/mcp-transport.md)
- [관리 CLI](../../reference/admin-cli.md)
- [보안](../../reference/security.md)
- [저장소 기록](../../reference/storage-records.md)
- [저장소 DDL](../../reference/storage-ddl.md)
- [저장소 버전 관리](../../reference/storage-versioning.md)
