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
codex_log="$install_tmp/codex.log"
mkdir -p "$test_home"

PATH="$repository_root/rebuild/crates/volicord-host/tests/fixtures:$PATH" \
HOME="$test_home" RUSTUP_HOME="$test_rustup_home" CARGO_HOME="$test_cargo_home" CODEX_TEST_LOG="$codex_log" \
    "$repository_root/rebuild/install.sh" \
    --prefix "$test_prefix" --runtime-dir "$test_runtime" --setup-codex

test -x "$test_prefix/bin/volicord"
test -x "$test_prefix/bin/volicord-viewer"
test -x "$test_prefix/bin/volicord-mcp"
test -f "$test_runtime/canonical.sqlite3"
test -f "$test_runtime/candidates.sqlite3"
test -f "$test_runtime/privacy.sqlite3"
test -f "$test_runtime/guarded.sqlite3"
rg -F "mcp add volicord --env VOLICORD_RUNTIME_DIR=$test_runtime -- $test_prefix/bin/volicord-mcp" "$codex_log"

"$test_prefix/bin/volicord" --runtime "$test_runtime" project init "Install smoke" >/dev/null
canonical_size=$(wc -c < "$test_runtime/canonical.sqlite3")
PATH="$repository_root/rebuild/crates/volicord-host/tests/fixtures:$PATH" \
HOME="$test_home" RUSTUP_HOME="$test_rustup_home" CARGO_HOME="$test_cargo_home" CODEX_TEST_LOG="$codex_log" \
    "$repository_root/rebuild/install.sh" \
    --prefix "$test_prefix" --runtime-dir "$test_runtime" --uninstall

test ! -e "$test_prefix/bin/volicord"
test -f "$test_runtime/canonical.sqlite3"
test "$(wc -c < "$test_runtime/canonical.sqlite3")" -eq "$canonical_size"

PATH="$repository_root/rebuild/crates/volicord-host/tests/fixtures:$PATH" \
HOME="$test_home" RUSTUP_HOME="$test_rustup_home" CARGO_HOME="$test_cargo_home" CODEX_TEST_LOG="$codex_log" \
    "$repository_root/rebuild/install.sh" \
    --prefix "$test_prefix" --runtime-dir "$test_runtime"
"$test_prefix/bin/volicord" --runtime "$test_runtime" health >/dev/null
