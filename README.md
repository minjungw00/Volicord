# Harness project / 하네스 프로젝트

## Repository status / 저장소 상태

This repository is documentation-only. It contains planning source material for a future local Harness Server. Based on the current repository contents, it does not contain the Harness runtime server implementation.

이 저장소는 문서 전용입니다. 향후 로컬 하네스 서버를 위한 계획 원천 자료를 담고 있습니다. 현재 저장소 내용 기준으로 하네스 런타임 서버 구현은 포함하지 않습니다.

For the canonical current scope, see [Active MVP scope](docs/en/reference/active-mvp-scope.md).

현재 범위의 canonical 설명은 [현재 MVP 범위 참조](docs/ko/reference/active-mvp-scope.md)를 확인하세요.

## What this repository contains / 이 저장소에 있는 것

- Active bilingual documentation under [`docs/`](docs/README.md).
- Reference owner documents for planning boundaries and technical contracts.
- Maintainer guidance in [`AGENTS.md`](AGENTS.md) and `docs/*/maintain/*`.

- [`docs/`](docs/README.md) 아래의 한영 문서 세트.
- 계획 경계와 기술 계약을 다루는 참조 담당 문서.
- [`AGENTS.md`](AGENTS.md)와 `docs/*/maintain/*`의 유지보수 지침.

## What this repository does not contain / 이 저장소에 없는 것

- A running Harness instance, runtime implementation, Harness Runtime Home, generated runtime state, operational artifacts, executable fixtures, or conformance runners.

- 실행 중인 하네스 인스턴스, 런타임 구현, Harness Runtime Home, 생성된 런타임 상태, 운영 아티팩트, 실행 가능한 fixture, 적합성 실행기.

## Start reading / 읽기 시작하기

| Need / 필요 | Link / 링크 |
|---|---|
| Documentation home / 문서 홈 | [docs/README.md](docs/README.md) |
| English start / 영어 시작 문서 | [docs/en/start.md](docs/en/start.md) |
| Korean start / 한국어 시작 문서 | [docs/ko/start.md](docs/ko/start.md) |
| English MVP scope owner / 영어 현재 MVP 범위 담당 문서 | [docs/en/reference/active-mvp-scope.md](docs/en/reference/active-mvp-scope.md) |
| Korean MVP scope owner / 한국어 현재 MVP 범위 담당 문서 | [docs/ko/reference/active-mvp-scope.md](docs/ko/reference/active-mvp-scope.md) |

## Documentation rules / 문서 규칙

README files are route documents. They do not define active MVP scope, API contracts, storage contracts, security guarantees, or runtime behavior.

README는 경로 안내 문서입니다. 현재 MVP 범위, API 계약, 저장소 계약, 보안 보장, 런타임 동작을 정의하지 않습니다.

English and Korean docs are both active. Keep semantic parity by meaning, not by line-by-line translation.

영어와 한국어 문서는 모두 활성 문서입니다. 줄 단위 번역이 아니라 의미 일치를 유지합니다.

## Contributor notes / 기여자 참고

Read [`AGENTS.md`](AGENTS.md) before editing. Keep work documentation-only, use small batches, and report changed files.

편집 전 [`AGENTS.md`](AGENTS.md)를 읽습니다. 작업은 문서 전용으로 유지하고, 작은 묶음으로 수정한 뒤 변경한 파일을 보고합니다.
