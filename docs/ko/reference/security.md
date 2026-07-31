# 보안

이 문서는 로컬 Codex workflow의 지원 보장과 명시적 비보장을 담당합니다.
메서드 schema, 저장 효과, Codex 구성 문법, 운영체제 정책은 정의하지 않습니다.

## 경계 요약

Volicord는 협력적 로컬 권한 기록입니다. 담당 문서가 정의한 workflow 상태를 검증하고
기록하지만 sandbox, 접근 통제 시스템, malware 방어, 네트워크 격리 계층, 변조 방지 감사
로그, 모델이 지침을 따랐다는 증명이 아닙니다.

## 지원 보장

담당 문서가 정의한 로컬 경계 안에서 Volicord는 다음을 보장합니다.

- Core 또는 Store commit 전 엄격한 typed validation
- 명시적인 Task, scope, Change Unit, 쓰기 티켓, evidence, UserAction, 닫기 상태 전이
- 담당 문서가 정의한 거부 분기의 무효과 동작
- Agent Connection 호출을 위한 현재 Connection, 프로젝트 membership, mode, 권위 있는
  managed-host session 검증
- Runtime Home과 Product Repository 분리
- Core가 요청 경로를 받아들이기 전에 수행하는 어휘 Product Repository 경로 검증과
  플랫폼 소유 canonical root, link, containment 관찰의 분리
- `resolved_by_actor_source=local_user`인 CLI 전용 UserAction 해결
- 네트워크 listener가 없는 관리 stdio MCP
- 허용적인 fallback 대신 기계 판독 실패 분류

이 보장은 Volicord 처리에만 적용됩니다. 처리 경계 밖의 Codex, shell, tool,
filesystem, 외부 시스템 동작을 통제하지 않습니다.

## 민감 동작과 사용자 판단

쓰기 티켓은 Core 권한 상태이며 파일시스템 권한이 아닙니다. 민감 승인, 최종 수락, 잔여
위험 수락, 취소, 그 밖의 사용자 소유 판단은 Agent Connection이 제출할 수 없습니다.
MCP 에이전트는 대기 요청을 만들 수 있지만 사용자는 로컬 CLI inbox로만 해결합니다.

Guard의 prompt 관련 관찰은 사용자 답이 되지 않습니다. 저장 resolution은 엄격한 typed
요청, 선택한 저장 option 또는 evidence 후보, CLI provenance, submission identity,
현재 근거가 모두 유효할 때만 권한 효력이 있습니다.

## 로컬 연결 가정

관리 Codex 프로세스와 `volicord`는 로컬 사용자의 운영체제 계정으로 실행됩니다.
Volicord는 그 OS 사용자를 인증하거나 프로세스 identity를 사람 identity로 바꾸지
않습니다. Agent 권한은 로컬에서 관찰한 협력적 runtime/project session 소유권, 현재
Connection Project membership, 현재 통합 revision, 현재 Connection mode가 허용한 범위만
증명합니다.

변경 불가능한 Store 소유 Connection 통합 instance ID와 integration generation은 로컬
Registry lifecycle revision을 구분합니다. 현재 소유자 입력과 함께 로컬 lifecycle 및
상관관계 좌표를 파생하며 호출자가 선택할 수 없습니다.

Runtime binding이 없는 Guard-only 프로젝트 session은 상관관계 이력이지 호출 권한이
아닙니다. Core 권한에는 현재 managed-host runtime과 정확한 Registry
runtime/project/host-session 예약이 정확한 현재 프로젝트 row에 attach된 상태도 필요합니다.
예약을 만들기 전에 프로젝트 소유권을 검증하므로 결정적인 Connection, 프로젝트, Guard
Installation, revision, native session, thread, attached runtime 충돌은 새 Registry 예약을
남기지 않습니다. Unbound 프로젝트 row는 권한이 아니며 프로젝트 attach 전 중단으로 남은
예약도 권한이 아닙니다. 정확한 replay는 소유자 상태가 바뀌지 않았을 때만 그 attach를
완료할 수 있습니다. Runtime row는 process-liveness 주장이 아닙니다. Crash 뒤 열린 것처럼
보이는 row는 이력이고 concurrent row 여러 개가 서로를 승인하거나 Guard event에 맞는
runtime으로 추측될 수 없습니다.

실행 파일 경로, process metadata, client name/version, host version, 환경 값, host
thread/turn metadata는 diagnostic 또는 상관관계 사실이며 actor나 human identity가
아닙니다. Thread와 turn metadata는 지원 workflow를 연결할 수 있지만 Connection이나
프로젝트 권한을 넓힐 수 없습니다. 내부 runtime ID와 revision 범위 프로젝트 session
ID도 비공개 로컬 상관관계 좌표입니다.

Package version과 구조화된 build provenance도 diagnostic 및 상관관계 사실입니다.
다른 필수 provenance를 알고 있으면 profile-class precision만 있다는 사실이 이 경계를
약화하지 않습니다. Dirty tree는 source 재현성을 명시적으로 제한하며, 완전해 보이는
build metadata도 실행 파일이 신뢰할 수 있거나 변조되지 않았거나 정확하거나 수정되지
않은 source에서 빌드됐다는 증명이 아닙니다.

지원 MCP 프로세스는 stdin/stdout을 사용하고 네트워크 전송 listener를 열지 않습니다.
이는 프로세스 topology 사실이며 네트워크 sandboxing이 아닙니다. Codex나 tool은
독립적으로 네트워크를 사용할 수 있습니다.

## 권한 경계

Product Repository 파일은 사용자 제품 데이터입니다. Runtime Home row는 Volicord
권한 기록입니다. 관리 Codex 구성은 프로세스를 시작하지만 권한, 승인, 쓰기 티켓,
Codex가 이를 읽었다는 증명이 아닙니다.

검증된 상대 Product Repository 경로는 파일시스템 권한이나 미래 containment의 증명이
아닙니다. 플랫폼 관찰은 Core가 얻은 시점의 한도 있는 로컬 사실입니다. 의미 서비스와
저장 권한 레코드는 canonical 절대 경로를 전달받거나 caller의 경로 text를 독립적으로
다시 열지 않습니다.

행동 기반 연결 관찰은 현재 관리 구성과 관찰한 protocol, 도구, 안전 호출, Guard 동작의
호환성을 성립시킵니다. Core 권한은 현재 활성 Connection, 프로젝트 membership, mode,
관리 runtime session, revision 범위 프로젝트 session, 정확한 Registry/프로젝트 binding을
별도로 검증합니다. 이 협력적 기록은 actor, client, 운영체제 사용자, human identity,
완전한 감시, 미래 host 동작을 성립시키지 않습니다.

<a id="historical-operation-result-access"></a>
## 과거 operation result 접근

`volicord.get_operation_result`는 담당 문서가 정의한 identity와 pagination으로 선택한
적격 불변 응답 bytes만 반환합니다. 호출자가 ID를 안다는 이유로 접근이 넓어지지 않습니다.
프로젝트·연결 불일치, 부적격, 손상, 사용할 수 없는 기록은 비공개 내용을 드러내지 않고
실패합니다.

<a id="generated-displays-and-text"></a>
## 생성 표시와 text

생성 guidance, CLI 산문, MCP text content, 상태 요약, template은 표시이며 별도 권한
기록이 아닙니다. 현재 typed 상태에서 도출하고 secret과 비공개 UserAction 내용을 빼며
구조화된 결과의 보장 경계를 보존해야 합니다. 오래된 표시가 작업을 승인할 수 없습니다.

### Workflow policy 검사

권위 있는 workflow policy 전체는 검사할 수 있는 로컬 구성이며 credential이 아닙니다.
`policy show --verbose`와 `policy show --json`은 정확한 MCP command, 인자 vector,
정적 `mcp.env` entry를 표시할 수 있습니다. 현재의 엄격한 정책 계약은 이 정적 환경
object에 문자열 값인 `VOLICORD_HOME`만 허용하며, 이 값은 로컬 Runtime Home
binding이지 secret이 아닙니다. 명령은 호출한 process 환경을 열거하거나 다른 환경
이름을 받아들이지 않습니다. 알 수 없거나 허용되지 않은 환경 member가 있는 정책은
일부만 표시하거나 조용히 redact하지 않고 엄격한 decoding에서 실패합니다.

## Guard와 기록되지 않은 변경

Guard와 조정 기록은 제한된 관찰입니다. 파일을 바꾼 actor, 악의, 완전한 감시, 예방을
증명하지 않습니다. 정확한 호출 범위 관찰 하나는 영속 baseline과 outcome 사이의 net
Product Repository transition만 기록합니다. Exact expected write는 그 완전한 delta
안에서 일치하는 경로만 포함할 수 있습니다. `unavailable` 관찰은 계속 표시해야 하며
완전한 빈 delta나 변경이 없었다는 증거로 취급할 수 없습니다. 호출 구간은 전이를
관찰할 뿐 actor identity나 단독 인과관계를 성립시키지 않습니다. 정확한 capture,
matching, finding 규칙은 [저장소 관찰](repository-observation.md)이 담당합니다.

## 명시적 비보장

Volicord는 다음을 보장하지 않습니다.

- filesystem, process, shell, command, network, credential, secret 격리
- Codex가 guidance, tool description, 관리 지침을 따름
- process, path, timing, prompt, observation 데이터에서 actor 귀속
- client name/version, host version, 환경 값, 로컬 session metadata에서 actor 귀속
- Product Repository 변경의 완전한 탐지 또는 예방
- 플랫폼 관찰이 끝난 뒤에도 경로가 계속 존재하거나 containment를 유지함
- 정확성, 테스트 충분성, QA, 배포 준비, 사람 검토
- 수락이 필요할 때 닫기 상태가 최종 사용자 수락을 대신함
- 구성이 있다고 활성 tool 노출이 증명됨
- 한 플랫폼 릴리스 결과가 다른 플랫폼에 적용됨
- 지원하지 않는 저장 또는 외부 계약 형식의 복구, decode, 자동 변환

## 관련 담당 문서

- [범위](scope.md)
- [Agent Connection](agent-connection.md)
- [MCP 전송](mcp-transport.md)
- [실패 모델](failure-model.md)
- [런타임 경계](runtime-boundaries.md)
- [API UserAction 스키마](api/schema-user-action.md)
