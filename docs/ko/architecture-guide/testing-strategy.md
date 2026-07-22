# 테스트 전략

테스트는 소유자가 정의한 현재 동작을 보호합니다. 테스트가 제품 계약을 만들거나 삭제된
표면을 보존하거나 더 넓은 지원 주장을 정당화하지 않습니다.

## 가장 좁은 계층 선택

| 테스트 계층 | 용도 |
|---|---|
| Unit test | 순수 파싱, 정규 인코딩, 폐쇄 값, 정책 결정. |
| Crate integration test | 어댑터 경계, Store 읽기/쓰기, 프로세스 동작, 엄격한 저장 레코드 거부. |
| Conformance test | 공개 교차 메서드 결과, 오류 범주, replay, 효과, projection. |
| Release-integrity 테스트 | Volicord target, 버전, 패키지, checksum, workflow invariant. |
| 문서 검사 | 소유자 경로, 링크, 용어, 언어 동등성, 예시, 생성 소스 drift. |

일회용 Runtime Home과 Product Repository를 사용합니다. fixture는 최소이며 typed여야
합니다. fixture는 parser 또는 구현 동작만 증명하며 실제 Codex 설치의 행동이나 플랫폼
지원을 증명하지 않습니다.

## 고정된 MCP 명세 입력

`tests/conformance/mcp-spec/`은 결정론적인 MCP 적합성 작업에 필요한 최소한의 버전별
upstream schema와 라이선스 저작자 표시를 담당합니다. 이 경로의 manifest는 확정된
초기화 기반 revision과 pre-release 전용 입력을 분리하고, 전체 upstream commit을
고정하며, handshake family와 release 분류를 기록하고, 모든 로컬 artifact의 checksum을
관리하며, `volicord_conformance_covered`를 기록합니다. 이 필드는 해당 revision이
Volicord 저장소가 소유하는 오프라인 런타임 적합성 매트릭스에 포함된다는 뜻일 뿐이며
외부 MCP 인증을 뜻하지 않습니다. 프로덕션 지원에는 릴리스되었고 pre-release 전용이
아닌 항목, 고정 schema, 프로덕션 protocol profile,
`volicord_conformance_covered=true`가 모두 필요합니다. 추적 중인 pre-release 항목은
프로덕션 지원 밖에 있으며 `volicord_conformance_covered`는 `false`입니다.

`cargo run -p xtask -- mcp-spec-check`는 오프라인 무결성 gate입니다. 네트워크 접근 없이
manifest를 parsing하고, 분류와 변경 불가능한 참조를 검증하며, schema 존재 여부,
schema family, 저작자 표시, checksum뿐 아니라 manifest의 프로덕션 지원 집합, 컴파일된
프로덕션 protocol profile 집합, 어댑터 소유 적합성 revision 선언의 정확한 집합 일치를
확인합니다. 보고서는 전체 고정 revision, 프로덕션 지원 revision,
`volicord_conformance_covered=true`인 revision, 추적 중인 pre-release revision 수를
결정론적으로 제공합니다.
`cargo run -p xtask -- mcp-spec-sync`는 명시적으로 실행하는 유지보수 작업입니다. 기록된
release가 고정 commit으로 해석되는지 확인하고 임시 디렉터리에 내려받으면서 검토된 지원
및 `volicord_conformance_covered` metadata를 보존한 다음, 후보 전체를 검증한 뒤에만
fixture를 교체합니다.
일반 build와 test는 네트워크를 사용하는 sync 경로를 실행하지 않습니다.

## 필수 경계 coverage

해당하는 지속 테스트는 다음을 다룹니다.

- 알 수 없는 멤버, 중복 키, 잘못된 폐쇄 값, 손상된 저장 owner record
- 정책, replay, ticket 무효화, mutation 전에 일어나는 구조적 거부
- authority event나 `state_version` 증가가 없는 read-only branch
- 하나의 원자적 성공 mutation과 정확한 replay 동작
- owner-defined corrupt-data failure로 라우팅되는 current-contract 불일치
- 행 없음 또는 비적격 operation result에서 유지되는
  `OPERATION_RESULT_UNAVAILABLE`
- 숨은 context의 MCP 거부와 CLI-only UserAction resolution
- 권위 있는 MCP runtime-session source 분리, milestone ordering, 현재 revision,
  프로젝트 binding, diagnostics 비권위성
- `production_supported=true`인 manifest 항목, 프로덕션 protocol profile,
  `volicord_conformance_covered=true`인 항목, 어댑터 소유 적합성 case 사이의 정확한
  revision 집합 일치 및 프로덕션 지원에서 추적 중인 pre-release generation 제외
- `volicord_conformance_covered=true`인 모든 revision의 독립된 `initialize`, initialized notification,
  `tools/list`, 고정 schema 검증, 필수 도구, 지정 왕복 도구, revision별 도구 projection과
  작업 단계 batching, 잘못된 lifecycle 동작, 초기화 batch 거절, EOF/종료
- exact-match와 counter-offer 협상, profile별 initialize capability, batching,
  `tools/list`, `tools/call` wire projection
- 프로덕션 protocol registry에서 파생하지 않고 독립적으로 고정하며 revision 적합성을
  대신하지 않는 Codex host fixture, CLI conformance evidence와 실제 `managed_host`
  관찰의 분리
- typed diagnostic code와 한도 및 민감정보 제거가 적용된 fact, finding 및 cause의
  transaction 영속화, 결정론적 root, dependency에 따른 `Blocked` check, 보고서 하나를
  사용하는 동등한 concise·verbose·lossless JSON projection
- Guard manifest의 exact shape와 owner binding, hash가 없는 policy command와 hash에
  결속된 runtime command의 구분, wrapper/file drift, 플랫폼 독립적인 script executable
  기대값, 현재 소유권의 hook 관찰, 이전 event 제외
- 안정적인 identity를 유지하는 반복 Guard 초기화와 관련 없는 repository content 보존
- Guard 관찰과 미기록 변경 suppression 결과
- Codex 구성 drift와 행동 probe 실패 보고

운영 상호운용성 coverage는 제한 안의 임의 version 문자열을 받고, initialize와 도구 목록
milestone을 실행하며, 필수 도구와 안전한 읽기 전용 호출, Guard artifact와 필수 phase 관찰,
session 소유권 및 integration revision 격리를 점검합니다.

## 릴리스 무결성과 선택적 호스트 smoke

오래 유지되는 릴리스 테스트 패키지는 `tests/release-integrity`입니다. 게시하는
Volicord target 다섯 개, 버전 일치, 기준 텍스트 바이트, 패키지와 archive 형태,
패키징한 binary identity, checksum 출력, 릴리스 workflow의 일반 빌드와 패키지
구조를 검증합니다.

일반 릴리스 무결성 테스트는 Volicord 플랫폼 빌드와 패키지 artifact를 다룹니다. 운영
Codex 상호운용성 테스트는 [Agent Connection](../reference/agent-connection.md)이 정의한
관리 구성, MCP 초기화, 필수 도구, 안전한 도구 왕복, Guard 관찰, session 소유권,
revision 격리를 별도로 다룹니다.

실제 Codex 실행은 선택적인 운영 smoke입니다. 제한된 host version을 진단으로
보고할 수 있고 version이 바뀌면 관찰을 다시 수행할 수 있습니다. 결과는 해당 구성과
환경에서 관찰한 행동에만 적용되며 미래 host 동작, human identity, 런타임 권한을
성립시키지 않습니다. smoke 인프라 부재는 일반 Volicord 릴리스 점검을 차단하지 않습니다.

## 문서 검증

의미가 바뀐 문서 쌍은 영문/국문 의미 동등성을 요구합니다. 생성 계약 projection은
소스와 일치해야 합니다. 다음을 실행합니다.

```sh
cargo run -p xtask -- docs-check
git diff --check
```

그다음 변경에 맞는 구식 표면 targeted scan을 실행하고 diff에서 owner routing, 정확한
식별자, 경로, anchor, 저장소 위생을 확인합니다.

## Rust 검증

일반 workspace gate는 다음과 같습니다.

```sh
cargo fmt
cargo clippy --all-targets --all-features
cargo test --all-targets --all-features
```

더 좁은 명령이 필요하면 이유와 실행하지 않은 workspace 검사를 인계에 기록합니다.
