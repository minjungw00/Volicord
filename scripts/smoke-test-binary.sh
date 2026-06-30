#!/bin/sh
set -eu

fail() {
    printf 'volicord smoke test: %s\n' "$1" >&2
    exit 1
}

bin=${1:-volicord}

case "$bin" in
    */*)
        [ -x "$bin" ] || fail "$bin is not executable"
        ;;
    *)
        command -v "$bin" >/dev/null 2>&1 || fail "$bin was not found on PATH"
        ;;
esac

tmp=${TMPDIR:-/tmp}
workdir=$(mktemp -d "$tmp/volicord-smoke.XXXXXX") || fail "failed to create temporary directory"
trap 'rm -rf "$workdir"' EXIT HUP INT TERM

repo="$workdir/product-repo"
home="$workdir/runtime-home"
git init -q "$repo"

"$bin" --help >/dev/null
"$bin" mcp --help >/dev/null
VOLICORD_HOME="$home" "$bin" init --host codex --repo "$repo" --dry-run --json >/dev/null
"$bin" guard --help >/dev/null
"$bin" serve --help >/dev/null

printf 'volicord smoke test passed for %s\n' "$bin"
