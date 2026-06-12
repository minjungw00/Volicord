# Harness project / 하네스 프로젝트

## Repository Role / 저장소 역할

This repository contains the bilingual Harness documentation set and owner-route metadata for the local Harness Server.

이 저장소는 로컬 하네스 서버를 위한 한영 문서 세트와 담당 문서 경로 메타데이터를 담고 있습니다.

For the canonical current scope, see [Scope](docs/en/reference/scope.md).

현재 범위의 기준 설명은 [범위](docs/ko/reference/scope.md)를 확인하세요.

## What this repository contains / 이 저장소에 있는 것

- Active bilingual documentation under [`docs/`](docs/README.md).
- Reference owner documents for planning boundaries and technical contracts.
- Maintainer guidance in [`AGENTS.md`](AGENTS.md) and `docs/*/maintain/*`.

- [`docs/`](docs/README.md) 아래의 한영 문서 세트.
- 계획 경계와 기술 계약을 다루는 참조 담당 문서.
- [`AGENTS.md`](AGENTS.md)와 `docs/*/maintain/*`의 유지보수 지침.

## Runtime Boundaries / 런타임 경계

Runtime state, generated artifacts, operational records, executable fixtures, conformance outputs, and product implementation files belong outside this documentation tree.

런타임 상태, 생성된 아티팩트, 운영 기록, 실행 가능한 픽스처, 적합성 출력, 제품 구현 파일은 이 문서 트리 밖에 둡니다.

## Start reading

### English

- New user: [`docs/en/start.md`](docs/en/start.md)
- Working user: [`docs/en/use/user-guide.md`](docs/en/use/user-guide.md)
- Agent behavior: [`docs/en/use/agent-guide.md`](docs/en/use/agent-guide.md)
- Technical contract: [`docs/en/reference/README.md`](docs/en/reference/README.md)
- Current scope: [`docs/en/reference/scope.md`](docs/en/reference/scope.md)

### 한국어

- 처음 읽는 사용자: [`docs/ko/start.md`](docs/ko/start.md)
- 작업 중인 사용자: [`docs/ko/use/user-guide.md`](docs/ko/use/user-guide.md)
- 에이전트 동작: [`docs/ko/use/agent-guide.md`](docs/ko/use/agent-guide.md)
- 기술 계약: [`docs/ko/reference/README.md`](docs/ko/reference/README.md)
- 현재 범위: [`docs/ko/reference/scope.md`](docs/ko/reference/scope.md)

## Documentation rules / 문서 규칙

README files are route documents. They do not define active MVP scope, API contracts, storage contracts, security guarantees, or runtime behavior.

README는 경로 안내 문서입니다. 현재 MVP 범위, API 계약, 저장소 계약, 보안 보장, 런타임 동작을 정의하지 않습니다.

English and Korean docs are both active. Keep semantic parity by meaning, not by line-by-line translation.

영어와 한국어 문서는 모두 활성 문서입니다. 줄 단위 번역이 아니라 의미 일치를 유지합니다.

## Contributor notes / 기여자 참고

Read [`AGENTS.md`](AGENTS.md) before editing. Use small documentation batches and report changed files.

편집 전 [`AGENTS.md`](AGENTS.md)를 읽습니다. 작은 문서 묶음으로 수정한 뒤 변경한 파일을 보고합니다.
