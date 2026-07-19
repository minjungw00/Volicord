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
