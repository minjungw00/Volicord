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
        bin_dir=$(cd "$(dirname "$bin")" && pwd)
        command_path="$bin_dir/$(basename "$bin")"
        ;;
    *)
        command_path=$(command -v "$bin") || fail "$bin was not found on PATH"
        ;;
esac

tmp=${TMPDIR:-/tmp}
workdir=$(mktemp -d "$tmp/volicord-smoke.XXXXXX") || fail "failed to create temporary directory"
cleanup() {
    rm -rf "$workdir"
}
trap cleanup EXIT HUP INT TERM

repo="$workdir/product-repo"
home="$workdir/runtime-home"
bindir="$workdir/bin"
mkdir -p "$repo/.git" "$bindir"

if ! ln -s "$command_path" "$bindir/volicord" 2>/dev/null; then
    cp "$command_path" "$bindir/volicord" || fail "failed to stage volicord command"
    chmod 0755 "$bindir/volicord"
fi

cat > "$bindir/codex" <<'EOF'
#!/bin/sh
if [ "$1" = "--version" ]; then
    printf 'codex 1.2.3-test\n'
    exit 0
fi
printf 'unexpected codex invocation\n' >&2
exit 2
EOF
chmod 0755 "$bindir/codex"

smoke_path="$bindir:$PATH"

"$command_path" --help >/dev/null
"$command_path" mcp --help >/dev/null
"$command_path" status --help >/dev/null
"$command_path" connection --help >/dev/null
"$command_path" inbox --help >/dev/null

init_json="$workdir/init.json"
PATH="$smoke_path" VOLICORD_HOME="$home" "$command_path" init \
    --host codex \
    --repo "$repo" \
    --profile record \
    --json > "$init_json"

connection_id=$(
    sed -n 's/.*"connection_id": "\([^"]*\)".*/\1/p' "$init_json" | head -n 1
)
[ -n "$connection_id" ] || fail "init JSON did not include connection_id"

stdio_out="$workdir/stdio.out"
{
    printf '%s\n' '{"jsonrpc":"2.0","id":1,"method":"initialize","params":{"protocolVersion":"2025-11-25","capabilities":{},"clientInfo":{"name":"volicord-smoke","version":"0.0.0"}}}'
    printf '%s\n' '{"jsonrpc":"2.0","method":"notifications/initialized","params":{}}'
    printf '%s\n' '{"jsonrpc":"2.0","id":2,"method":"tools/list","params":{}}'
} | VOLICORD_HOME="$home" "$command_path" mcp --stdio --connection "$connection_id" > "$stdio_out"

grep -q '"protocolVersion":"2025-11-25"' "$stdio_out" || fail "MCP stdio did not negotiate the current protocol"
grep -q '"name":"volicord.status"' "$stdio_out" || fail "MCP stdio did not list status"
grep -q '"name":"volicord.close_task"' "$stdio_out" || fail "MCP stdio workflow tools did not list close_task"
grep -q '"name":"volicord.request_user_action"' "$stdio_out" || fail "MCP stdio did not list user-action request creation"
if grep -q '"name":"volicord.resolve_user_action"' "$stdio_out"; then
    fail "MCP stdio exposed user-only action resolution"
fi

printf 'volicord smoke test passed for %s\n' "$command_path"
