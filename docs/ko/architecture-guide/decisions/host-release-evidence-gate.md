# 외부 호스트 릴리스 증거 게이트

## 맥락

관리 호스트 기능 지원에는 정적 구현 사실과 실제 Codex 및 Claude Code 동작이 함께
필요합니다. 릴리스 바이너리가 생긴 뒤에 실제 호스트 증거를 만들 수 있으므로 그 결과
다이제스트를 바이너리에 내장하면 증명 대상 아티팩트가 달라집니다. 임의 CLI 출력,
fixture, 주장 상태, 서로 다른 호스트 버전에서 모은 결과는 정확한 릴리스 주장 하나를
확립할 수 없습니다.

과거 v1 셀, manifest, audit 평가기는 구현 disposition을 호스트 종류만으로 담당했습니다.
V2는 검토한 정확한 버전 표를 추가했지만 관리 initialize에서 실제로 관찰한 MCP 클라이언트
정체성에 셀을 결속하지 않았습니다. 따라서 복사하거나 추론한 버전이 실제 셀을 실행한
클라이언트를 증명하지 않고도 호스트 좌표를 차지할 수 있었습니다. 필수 클라이언트 좌표를
추가하면 셀, manifest, audit, 다이제스트 의미가 바뀌므로 v1과 v2 아티팩트 또는 셀 입력
다이제스트 도메인에 새 계약 의미를 부여할 수 없습니다.

Credential을 포함하는 local-web 경로는 더 민감합니다. 현재 운영 어댑터에는 신뢰할 수
있는 manifest 획득 경로가 없습니다. 따라서 릴리스 아티팩트는 런타임 신뢰 입력이 아니라
외부 검증 증거로 남아야 합니다.

## 결정

Volicord는 [호스트 릴리스 증거](../../reference/host-release-evidence.md)의 버전이 붙은
계약을 사용합니다. 깨끗한 소스 revision 하나에서 정확한 profile과 target으로 최종 후보
하나를 만들고 외부의 변경 불가능한 경로에 둡니다. 소스 좌표에는
`git_archive_tar_sha256_v1`에 따른 원본
`git archive --format=tar <source_revision>` 출력의 SHA-256을 넣습니다. 후보에는 자체
SHA-256과 정확한 빌드 환경을 넣습니다. 게이트는 후보가 제어하는 바이트를 실행하기 전에
유지한 일반 파일 핸들을 해시하고, 검증된 바이트를 비공개 create-new 실행 파일로 복사한
뒤 주변 환경을 비운 상태에서 그 복사본만 실행합니다. 이어서 유지한 바이트와 최종 경로의
파일 정체성이 변하지 않았는지 확인합니다. 실행 후 안정성 불일치는 실패 불변조건이며,
실행 전 설명자 또는 복사본 다이제스트 불일치는 manifest나 후보 실행 없이 중단합니다.
이 내장 빌드 좌표와 아카이브 검사는 비적대적
provenance 및 무결성을 제공하며, 재현 가능한 재빌드나 임의의 후보 바이트가 이름 붙인
소스에서 왔다는 attestation은 아닙니다.

게이트는 매번 고정 12개 셀 행렬을 평가합니다. `codex`와 `claude_code` 각각에 기능
식별자 여섯 개를 두며 호스트 종류마다 정확한 호스트 가용성 좌표 하나만 사용합니다.
호스트 가용성 세 필드는 독립적으로 모두 문자열이거나 모두 null입니다. 최상위 및
environment의 클라이언트 이름·버전 필드 네 개도 별도로 모두 문자열이거나 모두 null이고,
null이 아닌 클라이언트에는 null이 아닌 호스트가 필요합니다. 호스트를 사용할 수 없는 실제
행렬은 null 호스트·클라이언트 좌표를 사용합니다. 구현된 셀은 `ignored`이며 하향 조정으로
남습니다. 정적 미지원 셀은 `not_applicable`이고 disposition이 MCP initialize 전에 단락될
수 있으므로 호스트 가용성이 null이 아니어도 null 클라이언트 정체성을 사용할 수 있습니다.
구현된 셀의 검증됨 요청은 정체성이 없어도 게이트를 실패시킬 수 있으며, 명시적으로 제외할
때만 `requested_verified=false`를 사용합니다. v3 평가기는 각 정적 disposition을 호스트
버전을 인식하는 담당 표와 대조합니다. Codex의 정규 `host_version=0.144.4`에서는
`native_user_action`, `verified_tool_producer`, `registered_connection_observation`가
구현되었고, `local_web_user_channel`, `record_final_output`,
`detective_final_output`은 해당 호스트 버전에서 지원되지 않습니다. 정확한 원문 버전 probe
envelope는 `codex-cli 0.144.4`이고 셀에는 여기서 얻은 bare 정규 `0.144.4`를 저장합니다.
null이 아닌 모든 Codex 버전은 공유 bare parser를 통과해야 하며 `host_version`에 원문 probe
외피를 넣으면 구조적으로 유효하지 않습니다. null 또는 아직 검토하지 않은 Codex 버전에는
호스트 종류 fallback을 유지하여 앞의 네 기능은
구현됨, 두 final-output 기능은 미지원으로 둡니다. Claude Code는 여섯 기능 모두 구현된
호스트 종류 fallback을 유지합니다. 이는 최소 버전 주장이 아니라 검토한 정확한 버전
표입니다. null이 아닌 각 클라이언트 쌍은 그 셀에 사용한 성공한 관리 MCP `initialize`에서만
얻습니다. 호스트 종류, 실행 파일, probe 출력, 환경, 설정, 프로토콜 버전, 상수, 이후 도구
메타데이터, 다른 셀에서 추론하지 않습니다. 한 호스트의 null이 아닌 모든 셀은 정확한
클라이언트 쌍 하나를 사용합니다. 구현된 exact-live 셀은
`client_version == host_version`이어야 합니다. 정체성이 없으면
`client_identity_missing`, 버전 또는 예상 정체성이 불일치하면
`client_identity_mismatch`로 도출하며 어느 경우든 `implemented_unverified`입니다. 검토한
Codex `0.144.4`는 추가로 `codex-mcp-client`/`0.144.4`를 요구합니다. 기록기에는 크기가
제한된 이름·버전 쌍만 보존하고 원본 initialize 또는 프로토콜·세션·thread·turn payload를
릴리스 증거로 사용하지 않습니다. 기록기는 셀에 결속된 깨끗하고 폐기 가능한 Runtime
Home에서 크기가 제한된 전후 관찰을 대조하고, 인증된 셀 호스트 turn 동안 새로 생기거나
메타데이터가 바뀐 그 turn의 정확한 관리 기준선 행만 받아들입니다. 같은 연결의 변경되지
않은 과거 행은 클라이언트 provenance가 아니며 연결 전체에서 가장 최신이거나 유일한 값을
고르는 방식도 거부합니다. 정규
평가기는 좌표, 타임스탬프, 다이제스트를 검사하고 다시 계산하며 생산자가 주장한 상태를
신뢰하지 않고 지원 상태를 도출합니다. 어댑터 프로필은 기능에서 도출하며
`record_final_output`에서만 `record`, 그 밖에는 `detective`이고, 정적 미지원 셀을
포함해 어댑터 버전은 정확한 후보 `build_id`와 같아야 합니다. 최신성은
`started_at <= recorded_at <= evaluated_at < started_at + 24h`를 사용합니다. 서로 다른
호스트 버전의 결과를 합치지 않습니다.

구현된 셀은 완료되고, 최신이며, 좌표와 다이제스트가 정확히 일치하고 통과한 실행에서만
`verified`가 됩니다. 실제로 존재하는 구현 셀이 ignored, running, 오래됨, 실패,
불일치이면 `implemented_unverified`가 됩니다. 구조적 입력이 없거나 형식이 잘못되면
상태로 바꾸지 않고 manifest 생성을 막습니다. 정적 담당 사실인 `unsupported_by_host`
결과는 그대로 유지합니다. 요청한 검증됨 주장을 충족하지 못하면 게이트가 실패합니다.
요청한 주장은 모두 충족했지만 구현된 기능이 하향 조정된 행렬은 명시적인
`pass_with_downgrades`입니다. 명시적 `requested_verified=false` 제외는 셀 증거가
`verified`로 도출되어도 하향 조정으로 남습니다.

게이트는 크기가 제한된 외부 `volicord-host-release-manifest-v3` 파일을 덮어쓰지 않고
새로 만듭니다. 게이트 프로세스가 끝나면 별도 프로세스가 소스 후보, 원본 셀 파일 12개,
셀 증거, manifest를 독립적으로 다시 열고 SHA-256 값, 불변조건, 상태, finding,
exclusion, 판정을 다시 계산하며 원본 셀이 manifest에 내장된 원본 셀과 같은지 확인합니다.
그런 다음 크기가 제한된 외부 `volicord-host-release-audit-v3` 파일을
덮어쓰지 않고 새로 만듭니다. 셀 입력 집합 다이제스트는
`volicord-host-release-cell-inputs-v3` 도메인을 사용합니다. Audit은 manifest를 신뢰하는
표시 경로에 계산을 위임하면 안 됩니다. 관리 CLI 출력은 보조 수단일 뿐입니다.

관리 Codex 및 Claude Code 세션 상관관계에는 호스트 릴리스 증거 문서가 담당하는 domain
분리 SHA-256 매핑을 사용합니다. 관리 MCP 경로와 훅 경로는 같은 불투명 Volicord 세션
ID를 사용하고 원본 native session identifier는 영속 저장하지 않습니다. `mhs_`
namespace와 그 호스트·연결 좌표는 예약되고 변경할 수 없습니다. 잘못된 marker는 영속
진단 상태를 만들지 않으며 다른 native 상관관계 identifier도 영속화 전에 불투명 값으로
바꿉니다. 결속이 없거나 일치하지 않으면 Strong Evidence를 만들 수 없습니다.

검토한 Codex 버전에서 관리 stdio는 허용된 tool call이 정확한 MCP 클라이언트 정체성
`codex-mcp-client`/`0.144.4`와 내부적으로 일관된 call별 메타데이터를 제공할 때까지 세션
미결속 상태로 남습니다. 메타데이터에는 `_meta.threadId`와
`_meta["x-codex-turn-metadata"]` 아래의 `session_id`, `thread_id`, `turn_id`가 있어야
합니다. 평면 및 중첩 thread ID는 서로 같아야 하며, 예약 매핑의 입력은 두 thread ID가
아니라 native `session_id`입니다. 구체적인 thread는 별도의 domain 분리 프로세스 로컬
다이제스트로 줄입니다. 첫 유효 call이 stdio 프로세스를 두 좌표 모두에 한 번만 결속하고
이후 모든 call은 두 좌표와 모두 일치해야 하며 새 turn ID는 허용합니다. 메타데이터가
없거나 잘못됐거나 일치하지 않으면 tool dispatch 및 관리 영속 효과 전에 거부합니다.
주변 `CODEX_THREAD_ID`, 도착 순서,
타임스탬프, 가장 가까운 세션 선택은 결속 권한이 아닙니다. 기존 기능 assertion 집합이 이미
그 결과인 정확한 세션과 연결 범위를 요구하므로 이 transport 결속은 릴리스 assertion
식별자를 추가하지 않습니다.

검증 구현은 테스트 전용 `tests/release-validation` workspace 패키지로 격리합니다. 구현
담당 평가기를 재사용할 수 있지만 운영 crate는 이 패키지에 의존하지 않습니다. 유지하는
명령 경로는 호스트 릴리스 증거 및 Maintain 검증 문서가 담당합니다.

## 결과

- 릴리스 주장은 위의 비적대적 provenance 한계 안에서 선언된 깨끗한 revision 하나, 외부
  최종 실행 파일 하나, target 하나, 정확한 profile 하나, 정확한 호스트 가용성 좌표에
  무결성 결속됩니다.
- 생산자는 상태를 주장하거나 불리한 셀을 생략하여 지원 상태를 승격할 수 없습니다.
- 오래되거나 부분적인 결과는 다른 실행과 조용히 섞이지 않고 하향 조정으로 드러납니다.
- 없거나 추론했거나 일치하지 않는 관리 클라이언트 정체성으로 실제 셀을 검증됨으로 만들 수
  없습니다.
- Manifest와 별도 audit은 지속적인 릴리스 검토 입력이지만 Core 증거, 사용자 권한,
  host attestation, 런타임 신뢰를 만들지 않습니다.
- 운영 local-web manifest 획득은 계속 사용할 수 없어 닫힌 상태로 실패하며 CLI inbox가
  지원되는 fallback입니다.
- Native session identifier는 Volicord 저장소, 진단, 릴리스 증거에 들어가지 않습니다.

## 비목표

- 공개 API 메서드나 운영 import 명령을 추가하지 않습니다.
- Codex 또는 Claude Code 최소 버전을 정하지 않습니다.
- OS 격리, 호스트 신원, 사용자 신원, 이후 호스트 변경 부재를 증명하지 않습니다.
- 빌드 재현성을 증명하거나 악의적인 후보 생산자를 상대로 source-to-binary provenance를
  attestation하지 않습니다.
- 서로 다른 호스트 버전이나 후보의 결과를 합칠 수 있게 하지 않습니다.
- 외부 릴리스 아티팩트를 신뢰된 운영 입력으로 만들지 않습니다.

## 호환성과 마이그레이션

이 결정은 테스트 전용 셀, manifest, audit, 셀 입력 다이제스트 계약을 v3로 올리고 실제
관리 initialize 정체성에 실제 셀을 결속합니다. 공개 Core API 스키마, 공개 MCP 메서드,
SQLite DDL, 저장 profile 버전은 바꾸지 않습니다. V1 셀, manifest, audit, 셀 입력 도메인
및 v2 입력은 과거 자료로 남으며 import, migration, 재해석하지 않고 거부합니다. Preimage가
바뀌지 않았으므로 후보는 `volicord-release-candidate-v1`, 소스 아카이브 알고리즘은
`git_archive_tar_sha256_v1`을 유지합니다.

예약된 `mhs_` 규칙은 generic 경로에서 미리 심은 값, 다른 호스트나 연결의 값, 잘못된
관리 marker를 의도적으로 거부합니다. 이전 alias, fallback 매핑, 호환 decoder를 추가하지
않으며 호환되는 현재 관찰은 관리 어댑터를 통해 다시 만듭니다. 지원되는 공개 API 또는
배포 표면을 추가하거나 깨뜨리지 않으므로 이 변경 묶음은 현재 workspace SemVer 안에
남습니다. 외부에 저장하는 v3 아티팩트는 opt-in 릴리스 검증 출력입니다.

## 거부한 대안

- 실제 증거를 후보에 내장하는 방안은 다시 빌드하면 정확한 실행 파일 다이제스트가 바뀌고
  재귀 결속이 생기므로 거부했습니다.
- `claimed_status`, CLI 텍스트, fixture, 복사한 해시를 신뢰하는 방안은 정규 재계산을
  우회할 수 있어 거부했습니다.
- 희소하거나 끝이 열린 행렬은 생략으로 미지원 또는 미검증 기능을 숨길 수 있어
  거부했습니다.
- 24시간 경계의 같음을 최신성에 포함하는 방안은 계약이 정확한 반개구간을 사용하므로
  거부했습니다.
- 호스트 버전마다 최신 통과 셀을 모으는 방안은 결과 주장이 시험한 호스트 환경 하나를
  설명하지 못하므로 거부했습니다.
- Audit을 게이트 프로세스 안에서 실행하는 방안은 프로세스가 분리된 재개방 및 재계산을
  제공하지 못하므로 거부했습니다.
- 원본 호스트 세션 식별자를 저장하는 방안은 상관관계에 domain 분리 불투명 매핑만
  필요하므로 거부했습니다.
- V1 또는 v2 셀, manifest, audit, 셀 입력 다이제스트를 v3 의미로 다시 해석하는 방안은 과거
  다이제스트 하나가 의미 하나를 유지해야 하므로 거부했습니다.
- 호스트 종류, 버전 probe, 설정, 프로토콜 버전, 상수, 다른 셀에서 클라이언트 정체성을
  추론하는 방안은 어느 것도 해당 실제 실행에서 관찰한 클라이언트가 아니므로 거부했습니다.
- `CODEX_THREAD_ID`, 시각, 도착 순서, 가장 최근에 열린 세션, 근접성을 이용한 Codex 결속은
  동시 또는 재개된 세션이 구별할 수 없지만 뒤바뀐 짝을 만들 수 있으므로 거부했습니다.

## 관련 담당 문서와 예정된 검증 위치

- [호스트 릴리스 증거](../../reference/host-release-evidence.md)
- [관리 호스트 세션·thread 결속과 호출별 turn 검증](managed-host-session-turn-binding.md)
- [Agent Connection](../../reference/agent-connection.md)
- [시스템 요구사항](../../reference/system-requirements.md)
- [보안](../../reference/security.md)
- [검증](../../maintain/validation.md)
- `tests/release-validation`

위 패키지 경로는 예정된 테스트 전용 구현 위치입니다. 이 결정은 비공개 모듈 배치를
정의하지 않습니다.
