# 정확한 Codex release evidence gate

## 맥락

지원 주장은 최종 Codex 실행 파일 바이트와 필수 첫 release 동작을 관찰한 플랫폼을
지정해야 합니다. fixture, version label, 인접 build, 다른 플랫폼의 결과로는 충분하지
않습니다.

## 결정

검증 전에 Codex candidate build와 최종화를 마치고 정확한 SHA-256 digest를 계산한 뒤
폐쇄 release scenario catalog를 다음 환경에서 독립적으로 실행합니다.

```text
linux
macos
native_windows
wsl2
```

각 `CodexReleaseCell`은 artifact digest, 하나의 `PlatformEnvironment`, 완전한 첫
release `CodexCapability` 집합, `integration_profile=record`, 정확한 runner 좌표,
scenario 결과, evidence digest를 결속합니다.

체크인한 manifest는 목표 형태 placeholder가 아니라 사실에 맞는 보고입니다. 0개부터
4개 셀이 있을 수 있습니다. 적격 시도가 없는 플랫폼에는 셀이 없습니다. evidence
status가 `passed`인 셀만 정확한 자기 좌표를 지원하며 결과는 셀이나 아티팩트 사이에
전파되지 않습니다.

review는 digest를 재계산하고 폐쇄 형태를 검증합니다. production은 owner-defined exact
passing evidence만 소비하며 `unsupported_host_artifact`에 대해 닫힌 상태로
실패합니다.

## 결과

- signing, stripping, packaging 또는 어떤 바이트 변경도 새 digest 검증을 요구합니다.
- WSL2는 native Linux와 native Windows에서 독립적입니다.
- mock과 parser fixture는 release evidence가 아닙니다.
- failed, unavailable, not-run 결과는 명시적으로 남고 passing으로 바꿀 수 없습니다.
- release 결과는 evidence이며 runtime identity나 사용자 인증이 아닙니다.

정확한 셀 schema, catalog, digest 알고리즘은
[Host Release Evidence](../../reference/host-release-evidence.md)가 소유합니다.
