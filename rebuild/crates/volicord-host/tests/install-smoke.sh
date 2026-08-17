#!/bin/sh
set -eu

repository_root=$1
test_rustup_home=${RUSTUP_HOME:-$HOME/.rustup}
test_cargo_home=${CARGO_HOME:-$HOME/.cargo}
install_tmp=$(mktemp -d)
trap 'rm -rf "$install_tmp"' EXIT HUP INT TERM
test_home="$install_tmp/home"
test_prefix="$install_tmp/prefix"
test_runtime="$install_tmp/runtime"
mkdir -p "$test_home"

HOME="$test_home" RUSTUP_HOME="$test_rustup_home" CARGO_HOME="$test_cargo_home" \
    "$repository_root/rebuild/install.sh" \
    --prefix "$test_prefix" --runtime-dir "$test_runtime"

test -x "$test_prefix/bin/volicord"
test -x "$test_prefix/bin/volicord-viewer"
test -x "$test_prefix/bin/volicord-mcp"
test -f "$test_runtime/canonical.sqlite3"
test -f "$test_runtime/candidates.sqlite3"
test -f "$test_runtime/privacy.sqlite3"
test -f "$test_runtime/guarded.sqlite3"
test ! -e "$test_home/.codex/config.toml"

test_repository="$install_tmp/repository"
mkdir -p "$test_repository"
git -C "$test_repository" init --quiet
"$test_prefix/bin/volicord" --runtime "$test_runtime" codex enable "$test_repository" >/dev/null
test -f "$test_repository/.codex/config.toml"
grep -F "[mcp_servers.volicord]" "$test_repository/.codex/config.toml" >/dev/null
grep -F "[[hooks.SessionStart]]" "$test_repository/.codex/config.toml" >/dev/null
"$test_prefix/bin/volicord" codex disable "$test_repository" >/dev/null
test ! -e "$test_repository/.codex/config.toml"

"$test_prefix/bin/volicord" --runtime "$test_runtime" project init "Install smoke" >/dev/null
canonical_size=$(wc -c < "$test_runtime/canonical.sqlite3")
HOME="$test_home" RUSTUP_HOME="$test_rustup_home" CARGO_HOME="$test_cargo_home" \
    "$repository_root/rebuild/install.sh" \
    --prefix "$test_prefix" --runtime-dir "$test_runtime" --uninstall

test ! -e "$test_prefix/bin/volicord"
test -f "$test_runtime/canonical.sqlite3"
test "$(wc -c < "$test_runtime/canonical.sqlite3")" -eq "$canonical_size"

HOME="$test_home" RUSTUP_HOME="$test_rustup_home" CARGO_HOME="$test_cargo_home" \
    "$repository_root/rebuild/install.sh" \
    --prefix "$test_prefix" --runtime-dir "$test_runtime"
"$test_prefix/bin/volicord" --runtime "$test_runtime" health >/dev/null
