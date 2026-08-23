#!/bin/sh
set -eu

prefix="${HOME}/.local"
runtime_dir="${XDG_DATA_HOME:-${HOME}/.local/share}/volicord"
uninstall=0

while [ "$#" -gt 0 ]; do
    case "$1" in
        --prefix) prefix=$2; shift 2 ;;
        --runtime-dir) runtime_dir=$2; shift 2 ;;
        --uninstall) uninstall=1; shift ;;
        *) echo "unknown option: $1" >&2; exit 2 ;;
    esac
done

case "$prefix" in /*) ;; *) echo "--prefix must be absolute" >&2; exit 2 ;; esac
case "$runtime_dir" in /*) ;; *) echo "--runtime-dir must be absolute" >&2; exit 2 ;; esac

bin_dir="$prefix/bin"
if [ "$uninstall" -eq 1 ]; then
    rm -f "$bin_dir/volicord" "$bin_dir/volicord-viewer" "$bin_dir/volicord-mcp"
    echo "Removed Volicord executables. Canonical data remains at $runtime_dir"
    exit 0
fi

script_dir=$(CDPATH= cd -- "$(dirname -- "$0")" && pwd)
cargo build --release --manifest-path "$script_dir/Cargo.toml" -p volicord-operations -p volicord-viewer -p volicord-host
mkdir -p "$bin_dir" "$runtime_dir"
install -m 755 "$script_dir/target/release/volicord" "$bin_dir/volicord"
install -m 755 "$script_dir/target/release/volicord-viewer" "$bin_dir/volicord-viewer"
install -m 755 "$script_dir/target/release/volicord-mcp" "$bin_dir/volicord-mcp"
VOLICORD_RUNTIME_DIR="$runtime_dir" "$bin_dir/volicord" doctor check >/dev/null

echo "Installed Volicord executables in $bin_dir"
echo "Runtime data: $runtime_dir"
case ":${PATH}:" in *":$bin_dir:"*) ;; *) echo "Add $bin_dir to PATH" ;; esac
