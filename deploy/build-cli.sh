#!/bin/bash
#
# build-cli.sh — Build Skill Garden CLI binaries and copy to data/cli/ for distribution.
#
# Usage:
#   ./deploy/build-cli.sh                           # native targets only
#   ./deploy/build-cli.sh linux-x86_64,macos-aarch64 # specific targets
#   DATA_DIR=./custom/output ./deploy/build-cli.sh   # custom output dir
#
set -euo pipefail

# Read version from Cargo.toml
VERSION=$(grep -m1 '^version' Cargo.toml | sed 's/.*"\(.*\)"/\1/')
echo "=== Building Skill Garden CLI v${VERSION} ==="

TARGETS="${1:-}"
DATA_DIR="${DATA_DIR:-cli-dist}"

# Target definitions (function-based to support bash 3.x on macOS)
get_triple() {
    case "$1" in
        windows-x86_64)  echo "x86_64-pc-windows-msvc" ;;
        windows-aarch64) echo "aarch64-pc-windows-msvc" ;;
        linux-x86_64)    echo "x86_64-unknown-linux-gnu" ;;
        linux-aarch64)   echo "aarch64-unknown-linux-gnu" ;;
        macos-x86_64)    echo "x86_64-apple-darwin" ;;
        macos-aarch64)   echo "aarch64-apple-darwin" ;;
        *)               echo "" ;;
    esac
}

# Binary name per OS
bin_name() {
    case "$1" in
        windows-*) echo "skill-garden.exe" ;;
        *)         echo "skill-garden" ;;
    esac
}

# Determine build targets
if [ -n "$TARGETS" ]; then
    IFS=',' read -ra BUILD_TARGETS <<< "$TARGETS"
else
    BUILD_TARGETS=()
    case "$(uname -s)" in
        Linux)
            BUILD_TARGETS+=("linux-x86_64")
            rustup target list --installed | grep -q "aarch64-unknown-linux-gnu" && BUILD_TARGETS+=("linux-aarch64") || true
            ;;
        Darwin)
            BUILD_TARGETS+=("macos-x86_64")
            rustup target list --installed | grep -q "aarch64-apple-darwin" && BUILD_TARGETS+=("macos-aarch64") || true
            ;;
        MINGW*|MSYS*|CYGWIN*)
            BUILD_TARGETS+=("windows-x86_64")
            rustup target list --installed | grep -q "aarch64-pc-windows-msvc" && BUILD_TARGETS+=("windows-aarch64") || true
            ;;
    esac
fi

echo "Targets: ${BUILD_TARGETS[*]}"
echo ""

SUCCESS=()
FAILED=()
SKIPPED=()

for name in "${BUILD_TARGETS[@]}"; do
    triple="$(get_triple "$name")"
    if [ -z "$triple" ]; then
        echo "  SKIP: Unknown target '$name'"
        SKIPPED+=("$name")
        continue
    fi

    bin=$(bin_name "$name")

    echo "--- Building: $name ($triple) ---"

    # Install rust target if needed
    if ! rustup target list --installed | grep -qF "$triple"; then
        echo "  Installing target: $triple..."
        rustup target add "$triple" || {
            echo "  FAIL: Could not install target $triple"
            FAILED+=("$name")
            continue
        }
    fi

    # Build (--no-default-features to skip server deps: sqlx, tantivy, axum, etc.)
    echo "  Compiling..."
    if cargo build --release --no-default-features --features cli --target "$triple" 2>&1; then
        # Copy to cli-dist/
        src="target/${triple}/release/${bin}"
        dest_dir="${DATA_DIR}/${VERSION}/${name}"
        mkdir -p "$dest_dir"
        cp "$src" "$dest_dir/$bin"
        chmod +x "$dest_dir/$bin"

        size=$(du -h "$dest_dir/$bin" | cut -f1)
        echo "  OK -> ${dest_dir}/${bin} (${size})"
        SUCCESS+=("$name")
    else
        echo "  FAIL: Build error for $name"
        FAILED+=("$name")
    fi
done

echo ""
echo "=== Build Summary ==="
echo "  Success : ${#SUCCESS[@]} [${SUCCESS[*]:-none}]"
if [ ${#FAILED[@]} -gt 0 ]; then
    echo "  Failed  : ${#FAILED[@]} [${FAILED[*]}]"
fi
if [ ${#SKIPPED[@]} -gt 0 ]; then
    echo "  Skipped : ${#SKIPPED[@]} [${SKIPPED[*]}]"
fi

if [ ${#SUCCESS[@]} -eq 0 ]; then
    echo ""
    echo "No binaries built. Add cross-compilation targets:"
    echo "  rustup target add x86_64-unknown-linux-gnu"
    echo "  rustup target add x86_64-apple-darwin"
    exit 1
fi

echo ""
echo "CLI binaries ready in: ${DATA_DIR}/${VERSION}/"
find "${DATA_DIR}/${VERSION}/" -type f -exec ls -lh {} \;
