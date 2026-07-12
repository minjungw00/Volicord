FROM rust:1.85-bookworm AS builder

WORKDIR /usr/src/volicord
COPY Cargo.toml Cargo.lock ./
COPY crates ./crates
COPY tests ./tests
COPY xtask ./xtask

ARG VOLICORD_BUILD_GIT_COMMIT=
ARG VOLICORD_BUILD_GIT_DIRTY=
RUN set -eu; \
    if [ -z "$VOLICORD_BUILD_GIT_COMMIT" ] && [ -z "$VOLICORD_BUILD_GIT_DIRTY" ]; then \
        unset VOLICORD_BUILD_GIT_COMMIT VOLICORD_BUILD_GIT_DIRTY; \
    elif [ -n "$VOLICORD_BUILD_GIT_COMMIT" ] && [ -n "$VOLICORD_BUILD_GIT_DIRTY" ]; then \
        export VOLICORD_BUILD_GIT_COMMIT VOLICORD_BUILD_GIT_DIRTY; \
    else \
        echo "VOLICORD_BUILD_GIT_COMMIT and VOLICORD_BUILD_GIT_DIRTY must be provided together" >&2; \
        exit 2; \
    fi; \
    VOLICORD_BUILD_PROFILE=release \
    cargo build --locked --release -p volicord-cli --bin volicord

FROM debian:bookworm-slim AS runtime

RUN useradd --system --uid 10001 --create-home --home-dir /home/volicord volicord \
    && mkdir -p /var/lib/volicord /workspace \
    && chown -R volicord:volicord /var/lib/volicord /workspace

COPY --from=builder /usr/src/volicord/target/release/volicord /usr/local/bin/volicord

USER volicord
ENV VOLICORD_HOME=/var/lib/volicord
WORKDIR /workspace

ENTRYPOINT ["volicord"]
CMD ["--help"]
