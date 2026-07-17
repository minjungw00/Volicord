#!/bin/sh
set -eu

if [ "$#" -ne 4 ]; then
    echo "usage: package-release-artifacts.sh BUILD_ROOT DIST_DIR RUN_ID RUN_ATTEMPT" >&2
    exit 2
fi

build_root=$1
dist=$2
run_id=$3
run_attempt=$4

case "$run_id:$run_attempt" in
    *[!0-9:]* | :* | *:) echo "run ID and attempt must contain only digits" >&2; exit 2 ;;
esac

test -d "$build_root"
if [ -e "$dist" ]; then
    test -d "$dist"
    test -z "$(find "$dist" -mindepth 1 -maxdepth 1 -print -quit)"
else
    mkdir -p "$dist"
fi

scratch=$(mktemp -d "${TMPDIR:-/tmp}/volicord-release-packaging.XXXXXX")
cleanup() {
    test -n "$scratch"
    test "$scratch" != "/"
    rm -rf -- "$scratch"
}
trap cleanup EXIT HUP INT TERM

sha256_file() {
    if command -v sha256sum >/dev/null 2>&1; then
        sha256sum "$1" | awk '{print $1}'
    else
        shasum -a 256 "$1" | awk '{print $1}'
    fi
}

for target in \
    x86_64-unknown-linux-gnu \
    aarch64-unknown-linux-gnu \
    aarch64-apple-darwin \
    x86_64-apple-darwin \
    x86_64-pc-windows-msvc
do
    artifact="$build_root/volicord-build-$target-$run_id-$run_attempt"
    test -d "$artifact"
    if [ "$target" = x86_64-pc-windows-msvc ]; then
        binary_name=volicord.exe
        extension=zip
    else
        binary_name=volicord
        extension=tar.gz
    fi
    source_binary="$artifact/$binary_name"
    expected=$(awk 'NR == 1 { print $1 }' "$artifact/volicord.sha256")
    test "${#expected}" -eq 64
    test "$(sha256_file "$source_binary")" = "$expected"

    package="volicord-$target"
    staging="$scratch/$package"
    verification="$scratch/$package-verified"
    mkdir -p "$staging" "$verification"
    cp "$source_binary" "$staging/$binary_name"
    chmod 0755 "$staging/$binary_name"
    test "$(sha256_file "$staging/$binary_name")" = "$expected"

    archive="$dist/$package.$extension"
    if [ "$extension" = zip ]; then
        python3 -c 'import pathlib, sys, zipfile; archive, source, name = sys.argv[1:]; handle = zipfile.ZipFile(archive, "w", zipfile.ZIP_DEFLATED); handle.write(source, name); handle.close()' "$archive" "$staging/$binary_name" "$binary_name"
        test "$(python3 -c 'import sys, zipfile; print("\n".join(zipfile.ZipFile(sys.argv[1]).namelist()))' "$archive")" = "$binary_name"
        python3 -c 'import sys, zipfile; zipfile.ZipFile(sys.argv[1]).extractall(sys.argv[2])' "$archive" "$verification"
    else
        tar -C "$staging" -czf "$archive" "$binary_name"
        test "$(tar -tzf "$archive")" = "$binary_name"
        tar -C "$verification" -xzf "$archive"
    fi
    test "$(sha256_file "$verification/$binary_name")" = "$expected"
    test "$(sha256_file "$source_binary")" = "$expected"

    archive_digest=$(sha256_file "$archive")
    printf '%s  %s\n' "$archive_digest" "$(basename "$archive")" > "$archive.sha256"
done
