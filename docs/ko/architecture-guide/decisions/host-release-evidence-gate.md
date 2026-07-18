# 정확한 Codex release evidence gate

## 맥락

지원 주장은 최종 Codex 실행 파일 바이트와 필수 첫 release 동작을 관찰한 플랫폼을
지정해야 합니다. fixture, version label, 인접 build, 다른 플랫폼의 결과로는 충분하지
않습니다.
Volicord 실행 파일은 자신의 최종 byte digest를 포함하는 증거를 내장할 수 없습니다.
그렇게 하면 identity가 자기 자신을 참조합니다.

## 결정

정확한 Codex 정책 좌표만 담는 엄격한 `CodexSupportCatalog`를 내장합니다. 실제
실행한 Volicord digest와 셀 결과를 포함하는 `CodexReleaseEvidenceManifest`는 모든
Volicord 실행 파일 외부에 둡니다. 릴리스 검증은 외부 증거를 읽고 각 entry를 내장
카탈로그와 교차 대조합니다.

검증 전에 Codex 및 Volicord candidate build와 최종화를 마치고 정확한 SHA-256
digest를 계산한 뒤 폐쇄 release scenario catalog를 다음 환경에서 독립적으로
실행합니다.

```text
x86_64-unknown-linux-gnu / linux
aarch64-unknown-linux-gnu / linux
aarch64-apple-darwin / macos
x86_64-apple-darwin / macos
x86_64-pc-windows-msvc / native_windows
x86_64-unknown-linux-gnu / wsl2
```

게시하는 각 Volicord target을 한 번만 빌드합니다. 빌드 job은 target, source revision,
실행 파일 이름, raw 실행 파일 SHA-256을 기록하고 raw byte를 변경 불가능한 workflow
아티팩트로 업로드합니다. 일치하는 모든 셀은 그 아티팩트를 다운로드합니다. WSL2는
같은 Linux x86-64 byte를 ext4로 옮기고 그 안에서 digest를 검증합니다. 각 셀은 한
번 검증하고 그 digest를 결속한 새 외부 증거를 생성합니다. 게시 단계는 통과한 셀
여섯 개를 모두 요구하고 같은 빌드 아티팩트 다섯 개를 다운로드하며, 다시 빌드하지
않고 패키징한 뒤 archive에서 추출한 각 실행 파일을 검증한 raw digest와 대조합니다.

각 외부 `CodexReleaseEvidenceEntry`는 두 아티팩트 digest, 하나의
`PlatformEnvironment`, 완전한 첫 release `CodexCapability` 집합,
`integration_profile=record`, 정확한 target 및 runner 좌표, scenario 결과,
evidence digest를 결속합니다. 대응하는 `CodexSupportEntry`에는 Codex digest,
target triple, 플랫폼 및 릴리스 좌표, profile, 검증된 capability만 둡니다.

외부 증거 manifest는 목표 형태 placeholder가 아니라 사실에 맞는 보고입니다.
Entry를 0~6개 담을 수 있습니다. 적격 시도가 없는 필수 셀에는 entry가 없습니다.
통과 결과는 정확한 카탈로그 좌표와 Volicord digest의 릴리스 증거만 성립시키며 다른
셀이나 아티팩트로 전파되지 않습니다. 운영 런타임 권한 경로는 어느 릴리스 계약도
읽지 않습니다.

Review는 digest를 재계산하고 두 닫힌 형태를 검증합니다. 릴리스 게시는
`unsupported_host_artifact`에 대해 fail closed로 동작합니다. 운영 MCP, CLI, Core,
Store 권한은 카탈로그나 릴리스 증거를 사용하지 않습니다.

## 결과

- signing, stripping 또는 실행 파일 byte 변경은 새 빌드 아티팩트와 새 digest 검증을
  요구합니다. 패키징은 실행 파일을 둘러싼 archive와 metadata만 바꿀 수 있습니다.
- 외부 증거의 Volicord digest는 내장 지원 카탈로그 identity를 바꾸지 않습니다.
- 외부 릴리스 증거는 내장 resource, 생성 Rust 상수, build script 입력이 되지
  않습니다.
- WSL2는 native Linux와 native Windows에서 독립적입니다.
- Native Linux와 WSL2는 같은 Linux x86-64 빌드 아티팩트에 대해 서로 다른 증거를
  생성합니다.
- 빌드, runner, Codex 아티팩트, 증거 entry, WSL2 실행 중 하나라도 없으면 게시를
  차단합니다.
- Linux 및 macOS architecture는 서로 독립적인 target identity입니다.
- mock과 parser fixture는 release evidence가 아닙니다.
- failed, unavailable, not-run 결과는 명시적으로 남고 passing으로 바꿀 수 없습니다.
- release 결과는 evidence이며 runtime identity나 사용자 인증이 아닙니다.

정확한 지원 카탈로그, 증거 manifest, 셀, digest 계약은
[Host Release Evidence](../../reference/host-release-evidence.md)가 소유합니다.
