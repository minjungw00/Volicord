# 변경 가이드

이 가이드는 구현 작업의 경로를 정합니다. 제품 동작은 현재 코드나 이 가이드가 아니라
집중된 Reference 소유자가 정의합니다.

## 편집 전

1. 저장소와 가장 가까운 범위의 `AGENTS.md`를 읽습니다.
2. [`docs/doc-index.yaml`](../../doc-index.yaml)에서 집중된 영문/국문 소유자 쌍을
   찾습니다.
3. 지속되는 Rust 구조를 바꾸기 전에 [아키텍처](architecture.md)와 이 가이드를 읽습니다.
4. worktree를 확인하고 관련 없는 사용자 변경을 보존합니다.
5. 필요한 동작을 소유자가 정의하지 않으면 소유자를 먼저 갱신하거나 빈틈을 보고합니다.

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
| 릴리스 빌드 또는 패키지 무결성 | `tests/release-integrity`, 릴리스 workflow | 검증 |
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

Rust 변경은 작업 범위가 더 좁은 crate 명령을 명확히 정당화하지 않는 한 workspace에서
다음을 실행합니다.

```sh
cargo fmt
cargo run -p xtask -- architecture-check
cargo clippy --all-targets --all-features
cargo test --all-targets --all-features
```

문서 변경은 [검증](../maintain/validation.md)의 검사를 실행하며 다음을 포함합니다.

```sh
cargo run -p xtask -- docs-check
git diff --check
```

릴리스 변경은 일반 release-integrity 패키지와 변경에 해당하는 빌드, 패키지,
checksum, 플랫폼, workflow 점검도 요구합니다. 실제 Codex smoke 실행은 현재 구성과 환경의
선택적인 운영 관찰입니다. Version이 바뀌면 관찰을 갱신하며 관리 호출 권한은 계속 session
binding으로 판단합니다.

## 인계

변경 파일, 검증과 결과, 사유가 있는 생략 검사, 남은 위험 또는 범위 밖 발견을
보고합니다. 작업 로그나 검증 출력을 유지 문서에 쓰지 않습니다.
