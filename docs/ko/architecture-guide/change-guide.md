# 변경 가이드

이 가이드는 구현 작업의 경로를 정합니다. 제품 동작은 현재 코드나 이 가이드가 아니라
집중된 Reference 소유자가 정의합니다.

## 편집 전

1. 저장소와 가장 가까운 범위의 `AGENTS.md`를 읽습니다.
2. `cargo run -p xtask -- owner-route --changed`를 실행해 한정된 지침, 변경
   패키지, 유지 문서, 직접 담당 문서, 검증 분류 경로를 얻습니다. commit
   series에는 `--base <revision>`을 사용합니다.
3. [`docs/doc-index.yaml`](../../doc-index.yaml)에서 반환된 집중 한영 담당 문서
   쌍을 확인합니다.
4. 오래 유지되는 Rust 구조를 바꾸기 전에 [아키텍처](architecture.md)와 이
   가이드를 읽습니다.
5. working tree를 살펴보고 관련 없는 사용자 변경을 보존합니다.
6. 필요한 동작을 담당 문서가 정의하지 않으면 먼저 담당 문서를 갱신하거나 그
   공백을 보고합니다.

## 변경 종류별 경로

| 변경 | 구현 시작점 | 필수 소유자 검토 |
|---|---|---|
| 공개 메서드, 요청, 응답, 오류, 값 | `volicord-types`, 그다음 Core | API 메서드/schema와 Failure Model |
| 계획, 정책, replay, 권한 | `volicord-core` | 집중된 API, Core Model, Storage Effects |
| 재사용 가능한 UserAction 검증, 구성, authority, lifecycle, 영속화 매핑, resolution, continuity, fact projection | `volicord-user-action-service` | User Action API/schema, Core Model, Storage Records와 Effects |
| DDL, 엄격한 저장 레코드, transaction 효과 | `volicord-store` | Storage DDL, Records, Effects, Versioning |
| MCP 생명주기, 디코딩, 도구 목록, projection | `volicord-mcp` | MCP Transport와 API 소유자 |
| 관리 MCP 시작 또는 runtime source | 숨겨진 CLI launcher, MCP bootstrap, 그다음 Store session | Agent Connection, MCP Transport, Storage Records와 DDL |
| 관리 명령 문법, 인수, 가시성, introspection | `volicord-command-model` | Administrative CLI 소유자 |
| 관리 명령 실행 또는 CLI 받은 편지함 | `volicord-cli` | Administrative CLI와 User Action 소유자 |
| Codex 설정 또는 검증 | Codex 어댑터와 connection 명령 | Agent Connection, Security, System Requirements |
| 릴리스 빌드, 소스 번들, 패키지 무결성 | `xtask`, `tests/release-integrity`, 릴리스 workflow | 검증 |
| 문서 경로 또는 용어 | `docs/doc-index.yaml`, 문서 쌍 | 문서와 번역 정책 |

현재 어댑터 표면은 `personal`, `shared` 관리 stdio 연결을 사용하는 Codex Record
profile입니다. 명시적인 소유자 변경 없이 다른 어댑터, profile, transport, user-action
resolution channel을 추가하지 않습니다.

## 경계 보존

- `volicord-command-model`은 Clap에만 의존하며 명령 실행, Core, Store, MCP,
  렌더링, Runtime Home, application service 동작을 소유하지 않습니다.
- CLI와 MCP 어댑터는 Core-facing interface를 호출할 수 있지만 Core는 어댑터 내부에
  의존하지 않습니다.
- `volicord-user-action-service`는 Store와 공유 타입에 의존할 수 있지만 Core,
  어댑터, presentation, 메서드 결과 인프라에는 의존할 수 없습니다.
- Store는 저장된 엄격한 owner record를 사용 전에 검증하고 owner-defined effect를
  원자적으로 적용합니다.
- 공개 어댑터는 숨은 invocation context를 거부하며 서버 소유 맥락은 로컬에서
  파생합니다.
- MCP는 UserAction 요청을 생성하거나 재개할 수 있습니다. CLI 받은 편지함만 해결합니다.
- Guard prompt capture는 관찰일 뿐입니다.
- 생성 문서는 소스와 생성기를 통해 변경합니다.

## 검증

첫 series commit의 상위 commit을 기록합니다. 중간 변경 상태나 commit마다 저장소
소유 집중 profile을 실행합니다.

```sh
cargo run -p xtask -- validate focused --base <revision>
```

계획한 모든 commit이 준비된 뒤 최종 검증 session 하나를 시작합니다.

```sh
cargo run -p xtask -- validate final --base <revision>
```

집중 profile은 담당 경로 결과를 사용하며 정확한 workspace aggregate를 실행하지
않습니다. 최종 profile이 완전한 workspace 정책과 한도가 있는 aggregate 처리를
담당합니다. 중간 commit에서 최종 profile이나 그 넓은 명령을 반복하지 않습니다.
기계 판독 출력에는 `--json`을 사용하며 완전한 명령 결과는 보고된 무시 경로
`target/volicord-validation/<run-id>/` 아래에 남습니다.

패키지 아키텍처 metadata를 변경했다면 구현 중에 `docs-sync`를 실행하여 생성되는
영어·한국어 책임 및 의존 표를 현재 상태로 유지합니다. Profile은 생성 내용 drift를
검증합니다. 실제 Codex smoke 실행은 현재 구성과 환경의 선택적인 운영 관찰로
남으며 일반 최종 결과에 포함되지 않습니다.

## 인계

변경 파일, 검증 run ID와 summary 경로, 통과, 실패, 분해, 생략 결과, 남은 위험
또는 범위 밖 발견을 보고합니다. 작업 로그나 검증 출력을 유지 문서에 쓰지
않습니다.
