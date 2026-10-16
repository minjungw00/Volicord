#!/bin/sh
# Run only inside the disposable Linux container described in README.md.
set -eu
mode=${1:-all}
test -z "$(command -v node || true)"
test -z "$(command -v npm || true)"
cat /etc/os-release
if [ "$mode" != runtime ]; then
/toolchain/bin/rustc --version
cc --version | head -1
cd /inputs
sha256sum -c ASSETS.sha256
mkdir /tmp/source-rust /tmp/source-vue /tmp/runtime
cp reader.rs reader.css data.rs data.json app.js /tmp/source-vue/
cp reader.rs reader.css data.rs data.json /tmp/source-rust/
cp enhance.js /tmp/source-rust/app.js
for candidate in rust vue; do
    cd "/tmp/source-$candidate"
    started=$(date +%s%N)
    if [ "$candidate" = vue ]; then
        /toolchain/bin/rustc --edition=2021 --cfg vue -O reader.rs -o "/output/$candidate-reader"
    else
        /toolchain/bin/rustc --edition=2021 -O reader.rs -o "/output/$candidate-reader"
    fi
    ended=$(date +%s%N)
    echo "$candidate source_install_nanoseconds=$((ended-started))"
    "/output/$candidate-reader" "/output/$candidate-offline.html" click
    "/output/$candidate-reader" >"/tmp/$candidate-url" &
    server_pid=$!
    trap 'kill "$server_pid" 2>/dev/null || true' EXIT INT TERM
    while ! test -s "/tmp/$candidate-url"; do sleep 0.01; done
    url=$(cat "/tmp/$candidate-url")
    curl --fail --silent "${url}?work=click" >"/output/$candidate-live.html"
    curl --fail --silent "${url}app.js" >"/output/$candidate-app.js"
    if [ "$candidate" = vue ]; then curl --fail --silent "${url}data.json" >/output/vue-data.json; fi
    kill "$server_pid"
    wait "$server_pid" || true
    trap - EXIT INT TERM
done
echo 'Node-free prototype source installation and local HTTP: passed'
fi
if [ "$mode" = source ]; then exit 0; fi
# Prebuilt current Product ordinary CLI/MCP runtime, with no Node or Cargo.
/product/volicord --runtime /tmp/runtime --json doctor check >/output/product-doctor.json
printf '%s\n' '{"jsonrpc":"2.0","id":1,"method":"initialize","params":{"protocolVersion":"2024-11-05","capabilities":{},"clientInfo":{"name":"local-reader-measurement","version":"1"}}}' | VOLICORD_RUNTIME_DIR=/tmp/runtime /product/volicord-mcp >/output/product-mcp.json
echo 'Node-free source installation and prebuilt ordinary runtime: passed'
