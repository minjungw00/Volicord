#!/bin/sh
set -eu

fail() {
    printf 'volicord install: %s\n' "$1" >&2
    exit 1
}

download() {
    url=$1
    output=$2
    if command -v curl >/dev/null 2>&1; then
        curl -fsSL "$url" -o "$output"
    elif command -v wget >/dev/null 2>&1; then
        wget -q -O "$output" "$url"
    else
        fail "curl or wget is required to download release assets"
    fi
}

sha256_file() {
    file=$1
    if command -v sha256sum >/dev/null 2>&1; then
        sha256sum "$file" | awk '{print $1}'
    elif command -v shasum >/dev/null 2>&1; then
        shasum -a 256 "$file" | awk '{print $1}'
    else
        return 1
    fi
}

detect_target() {
    os=$(uname -s 2>/dev/null || printf unknown)
    arch=$(uname -m 2>/dev/null || printf unknown)

    case "$os" in
        Linux)
            os_part=unknown-linux-gnu
            ;;
        Darwin)
            os_part=apple-darwin
            ;;
        MINGW*|MSYS*|CYGWIN*|Windows_NT)
            fail "native Windows is not supported; use WSL2 with the Linux binary"
            ;;
        *)
            fail "unsupported operating system: $os"
            ;;
    esac

    case "$arch" in
        x86_64|amd64)
            arch_part=x86_64
            ;;
        arm64|aarch64)
            arch_part=aarch64
            ;;
        *)
            fail "unsupported CPU architecture: $arch"
            ;;
    esac

    printf '%s-%s\n' "$arch_part" "$os_part"
}

release_base_url() {
    if [ -n "${VOLICORD_RELEASE_BASE_URL:-}" ]; then
        printf '%s\n' "${VOLICORD_RELEASE_BASE_URL%/}"
        return
    fi

    if [ -z "${VOLICORD_REPO:-}" ]; then
        fail "set VOLICORD_REPO=OWNER/REPO or VOLICORD_RELEASE_BASE_URL before running this script"
    fi

    case "$VOLICORD_REPO" in
        */*) ;;
        *) fail "VOLICORD_REPO must use OWNER/REPO form" ;;
    esac

    version=${VOLICORD_VERSION:-latest}
    case "$version" in
        latest)
            printf 'https://github.com/%s/releases/latest/download\n' "$VOLICORD_REPO"
            ;;
        *)
            printf 'https://github.com/%s/releases/download/%s\n' "$VOLICORD_REPO" "$version"
            ;;
    esac
}

install_dir() {
    if [ -n "${VOLICORD_INSTALL_DIR:-}" ]; then
        printf '%s\n' "$VOLICORD_INSTALL_DIR"
        return
    fi

    if [ -z "${HOME:-}" ]; then
        fail "HOME is not set; set VOLICORD_INSTALL_DIR to choose an install directory"
    fi

    printf '%s/.local/bin\n' "$HOME"
}

target=$(detect_target)
base_url=$(release_base_url)
dest_dir=$(install_dir)
archive_name="volicord-$target.tar.gz"
archive_url="$base_url/$archive_name"
checksum_url="$archive_url.sha256"

tmp=${TMPDIR:-/tmp}
workdir=$(mktemp -d "$tmp/volicord-install.XXXXXX") || fail "failed to create temporary directory"
trap 'rm -rf "$workdir"' EXIT HUP INT TERM

archive="$workdir/$archive_name"
checksum="$workdir/$archive_name.sha256"
extract_dir="$workdir/extract"
mkdir -p "$extract_dir"

printf 'downloading %s\n' "$archive_url" >&2
download "$archive_url" "$archive"

verified=0
if download "$checksum_url" "$checksum"; then
    expected=$(awk 'NR == 1 {print $1}' "$checksum")
    case "$expected" in
        ""|*[!0123456789abcdefABCDEF]*)
            fail "checksum file does not contain a valid SHA-256 digest"
            ;;
    esac
    if [ "${#expected}" -ne 64 ]; then
        fail "checksum file does not contain a 64-character SHA-256 digest"
    fi
    actual=$(sha256_file "$archive") || fail "checksum file was downloaded but sha256sum or shasum is unavailable"
    expected=$(printf '%s' "$expected" | tr 'A-F' 'a-f')
    actual=$(printf '%s' "$actual" | tr 'A-F' 'a-f')
    if [ "$actual" != "$expected" ]; then
        fail "checksum mismatch for $archive_name"
    fi
    verified=1
else
    if [ "${VOLICORD_REQUIRE_CHECKSUM:-0}" = "1" ]; then
        fail "checksum file is unavailable and VOLICORD_REQUIRE_CHECKSUM=1"
    fi
    printf 'warning: checksum file unavailable; installing without checksum verification\n' >&2
fi

entry_count=$(tar -tzf "$archive" | wc -l | tr -d ' ')
first_entry=$(tar -tzf "$archive" | sed -n '1p')
if [ "$entry_count" != "1" ] || [ "$first_entry" != "volicord" ]; then
    fail "archive must contain only the volicord executable"
fi

tar -xzf "$archive" -C "$extract_dir"
[ -f "$extract_dir/volicord" ] || fail "archive did not extract a volicord executable"

mkdir -p "$dest_dir"
if command -v install >/dev/null 2>&1; then
    install -m 0755 "$extract_dir/volicord" "$dest_dir/volicord"
else
    cp "$extract_dir/volicord" "$dest_dir/volicord"
    chmod 0755 "$dest_dir/volicord"
fi

printf 'installed %s\n' "$dest_dir/volicord"
if [ "$verified" = "1" ]; then
    printf 'verified %s with SHA-256 checksum\n' "$archive_name"
fi

case ":${PATH:-}:" in
    *":$dest_dir:"*) ;;
    *)
        printf 'note: %s is not on PATH for this shell\n' "$dest_dir" >&2
        ;;
esac
