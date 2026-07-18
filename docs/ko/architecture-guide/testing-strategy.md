# 테스트 전략

테스트는 소유자가 정의한 현재 동작을 보호합니다. 테스트가 제품 계약을 만들거나 삭제된
표면을 보존하거나 더 넓은 지원 주장을 정당화하지 않습니다.

## 가장 좁은 계층 선택

| 테스트 계층 | 용도 |
|---|---|
| Unit test | 순수 파싱, 정규 인코딩, 폐쇄 값, 정책 결정. |
| Crate integration test | 어댑터 경계, Store 읽기/쓰기, 프로세스 동작, 엄격한 저장 레코드 거부. |
| Conformance test | 공개 교차 메서드 결과, 오류 범주, replay, 효과, projection. |
| Release-validation 셀 | 한 플랫폼 환경에서 정확한 최종 Codex 아티팩트 동작. |
| 문서 검사 | 소유자 경로, 링크, 용어, 언어 동등성, 예시, 생성 소스 drift. |

일회용 Runtime Home과 Product Repository를 사용합니다. fixture는 최소이며 typed여야
합니다. fixture는 parser 또는 구현 동작만 증명하며 실제 Codex 아티팩트나 플랫폼 지원을
증명하지 않습니다.

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
- 지원되지 않는 Codex 아티팩트와 구성 drift 동작

## Codex Release 검증

release 지원은 모든 게시 binary target을 포괄하는 독립된 target/environment 셀
여섯 개입니다.

```text
x86_64-unknown-linux-gnu / linux
aarch64-unknown-linux-gnu / linux
aarch64-apple-darwin / macos
x86_64-apple-darwin / macos
x86_64-pc-windows-msvc / native_windows
x86_64-unknown-linux-gnu / wsl2
```

각 셀은 정확히 최종 확정된 Codex와 Volicord 실행 파일 digest를 자기의 정확한
환경에서 사용해 닫힌 scenario catalog를 실행합니다. 어느 플랫폼 결과도 다른
플랫폼을 대신하지 않습니다. 릴리스 검증 테스트는 유지 중인
`CodexSupportCatalog`와 외부 `CodexReleaseEvidenceManifest`의 결정론적 parsing과 카탈로그 교차 대조를
검사합니다. 증거 manifest에는 사실대로 entry를 0~6개 둘 수 있으며 실제 시도만
보고해야 합니다. `passed` 결과는 정확한 카탈로그 좌표와 Volicord digest의 릴리스
증거만 성립시키며 런타임 권한은 어느 계약도 사용하지 않습니다.

저장소의 workflow 테스트는 `.github/workflows/release.yml`을 parse하고 target 다섯
개와 셀 여섯 개 계약에 맞는지 교차 대조합니다. Raw 빌드 matrix 하나, 각 셀의 정확한
빌드 아티팩트 다운로드, native Linux와 WSL2의 공통 Linux x86-64 출처, 게시 단계의
모든 필수 셀 의존성, 게시 단계의 Volicord 재빌드 금지, digest에 결속된 완전한 증거,
패키징 직전의 최종 verifier, 외부 verified index 준비, archive 구성원 재hash를
요구합니다. 합성 전체 bundle 테스트는 실제 임시 Codex와 Volicord 파일을 만들고
그 digest를 계산합니다. 결정론적 카탈로그 생성과 verified index, 비어 있는 운영
카탈로그, 불완전한 증거와 `not_run`, 중복 증거, source revision 및 아티팩트 digest
불일치, 누락된 target 또는 환경 셀을 다룹니다. 릴리스 무결성 게이트는 변경된 raw
binary, 빌드 metadata 불일치, 사용되지 않거나 모호한 카탈로그 entry, 불완전한 셀
증거, 검증되지 않은 게시 입력을 별도로 거부합니다.

mock, fixture, 재빌드, 선택된 또는 인접 아티팩트는 최종 바이트를 대신할 수 없습니다.
failed, unavailable, not-run scenario는
[Host Release Evidence](../reference/host-release-evidence.md)가 소유한 evidence 규칙에
명시적으로 남습니다.

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
