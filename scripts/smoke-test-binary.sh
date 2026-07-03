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
serve_pid=""
cleanup() {
    if [ -n "$serve_pid" ] && kill -0 "$serve_pid" >/dev/null 2>&1; then
        kill "$serve_pid" >/dev/null 2>&1 || true
        wait "$serve_pid" >/dev/null 2>&1 || true
    fi
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
"$command_path" serve --help >/dev/null

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
if grep -q '"name":"volicord.record_user_judgment"' "$stdio_out"; then
    fail "MCP stdio exposed user-only judgment recording"
fi

if command -v curl >/dev/null 2>&1; then
    serve_stderr="$workdir/serve.stderr"
    serve_stdout="$workdir/serve.stdout"
    token="volicord-smoke-token"
    token_file="$workdir/serve.token"
    printf '%s\n' "$token" >"$token_file"
    VOLICORD_HOME="$home" "$command_path" serve \
        --transport local-http \
        --listen 127.0.0.1:0 \
        --connection "$connection_id" \
        --token-file "$token_file" >"$serve_stdout" 2>"$serve_stderr" &
    serve_pid=$!

    listen_url=""
    attempts=0
    while [ "$attempts" -lt 100 ]; do
        listen_url=$(
            sed -n 's#.*\(http://[^ ]*/mcp\).*#\1#p' "$serve_stderr" | tail -n 1
        )
        if [ -n "$listen_url" ]; then
            break
        fi
        if ! kill -0 "$serve_pid" >/dev/null 2>&1; then
            if grep -q 'Operation not permitted' "$serve_stderr"; then
                printf 'volicord smoke test skipped Local HTTP TCP checks: local bind is unavailable\n' >&2
                serve_pid=""
                break
            fi
            fail "Local HTTP server exited before startup: $(cat "$serve_stderr")"
        fi
        attempts=$((attempts + 1))
        sleep 0.1
    done

    if [ -n "$listen_url" ]; then
        health_url="${listen_url%/mcp}/healthz"
        unauth_code=$(curl -sS -o "$workdir/unauth.json" -w '%{http_code}' "$health_url") \
            || fail "Local HTTP unauthenticated health request failed"
        [ "$unauth_code" = "401" ] || fail "Local HTTP health without token returned $unauth_code"
        grep -q 'AUTH_REQUIRED' "$workdir/unauth.json" || fail "Local HTTP unauthenticated health did not return AUTH_REQUIRED"

        auth_code=$(
            curl -sS -o "$workdir/health.json" -w '%{http_code}' \
                -H "Authorization: Bearer $token" \
                "$health_url"
        ) || fail "Local HTTP authenticated health request failed"
        [ "$auth_code" = "200" ] || fail "Local HTTP health with token returned $auth_code"

        init_payload='{"jsonrpc":"2.0","id":1,"method":"initialize","params":{"protocolVersion":"2025-11-25","capabilities":{},"clientInfo":{"name":"volicord-smoke","version":"0.0.0"}}}'
        origin_code=$(
            curl -sS -o "$workdir/origin.json" -w '%{http_code}' \
                -X POST "$listen_url" \
                -H "Authorization: Bearer $token" \
                -H "Accept: application/json, text/event-stream" \
                -H "Content-Type: application/json" \
                -H "Origin: https://example.invalid" \
                --data "$init_payload"
        ) || fail "Local HTTP Origin check request failed"
        [ "$origin_code" = "403" ] || fail "Local HTTP invalid Origin returned $origin_code"
        grep -q 'ORIGIN_NOT_ALLOWED' "$workdir/origin.json" || fail "Local HTTP invalid Origin did not return ORIGIN_NOT_ALLOWED"

        headers="$workdir/init.headers"
        init_code=$(
            curl -sS -D "$headers" -o "$workdir/init-http.json" -w '%{http_code}' \
                -X POST "$listen_url" \
                -H "Authorization: Bearer $token" \
                -H "Accept: application/json, text/event-stream" \
                -H "Content-Type: application/json" \
                --data "$init_payload"
        ) || fail "Local HTTP initialize request failed"
        [ "$init_code" = "200" ] || fail "Local HTTP initialize returned $init_code"
        grep -qi '^Mcp-Session-Id:' "$headers" || fail "Local HTTP initialize did not return Mcp-Session-Id"

        kill "$serve_pid" >/dev/null 2>&1 || true
        wait "$serve_pid" >/dev/null 2>&1 || true
        serve_pid=""
    fi
else
    printf 'volicord smoke test skipped Local HTTP checks: curl is unavailable\n' >&2
fi

printf 'volicord smoke test passed for %s\n' "$command_path"
