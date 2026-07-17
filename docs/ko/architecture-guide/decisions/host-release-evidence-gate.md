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

각 외부 `CodexReleaseEvidenceEntry`는 두 아티팩트 digest, 하나의
`PlatformEnvironment`, 완전한 첫 release `CodexCapability` 집합,
`integration_profile=record`, 정확한 target 및 runner 좌표, scenario 결과,
evidence digest를 결속합니다. 대응하는 `CodexSupportEntry`에는 Codex digest,
target triple, 플랫폼 및 릴리스 좌표, profile, 검증된 capability만 둡니다.

외부 증거 manifest는 목표 형태 placeholder가 아니라 사실에 맞는 보고입니다.
Entry를 0~6개 담을 수 있습니다. 적격 시도가 없는 필수 셀에는 entry가 없습니다.
통과 결과는 정확한 카탈로그 좌표와 Volicord digest의 릴리스 증거만 성립시키며 다른
셀이나 아티팩트로 전파되지 않습니다. 런타임 조회는 내장 카탈로그만 읽고 빈
카탈로그에서는 fail closed로 동작합니다.

review는 digest를 재계산하고 두 닫힌 형태를 검증합니다. production은 담당 문서가
정의한 정확한 지원 정책만 사용하며 `unsupported_host_artifact`에 대해 fail
closed로 동작합니다. 릴리스 증거는 사용하지 않습니다.

## 결과

- signing, stripping, packaging 또는 어떤 바이트 변경도 새 digest 검증을 요구합니다.
- 외부 증거의 Volicord digest는 내장 지원 카탈로그 identity를 바꾸지 않습니다.
- 외부 릴리스 증거는 내장 resource, 생성 Rust 상수, build script 입력이 되지
  않습니다.
- WSL2는 native Linux와 native Windows에서 독립적입니다.
- Linux 및 macOS architecture는 서로 독립적인 target identity입니다.
- mock과 parser fixture는 release evidence가 아닙니다.
- failed, unavailable, not-run 결과는 명시적으로 남고 passing으로 바꿀 수 없습니다.
- release 결과는 evidence이며 runtime identity나 사용자 인증이 아닙니다.

정확한 지원 카탈로그, 증거 manifest, 셀, digest 계약은
[Host Release Evidence](../../reference/host-release-evidence.md)가 소유합니다.
