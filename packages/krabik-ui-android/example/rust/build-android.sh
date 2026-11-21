#!/usr/bin/env bash
#
# Builds the Rust shared library for the Android ABIs used by the example app and copies the
# resulting .so files into the directory the Android app packages as jniLibs.
#
# Usage:
#   ./example/rust/build-android.sh [output-dir]
#
# The NDK is discovered from ANDROID_NDK_HOME, ANDROID_NDK_ROOT, ANDROID_HOME,
# ANDROID_SDK_ROOT, or example/android/local.properties (sdk.dir), in that order.
#
# Set RUST_PROFILE=debug for an unoptimized library. The default is release.
# Set RUST_ANDROID_ABIS="arm64-v8a x86_64" to build only a subset.

set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
REPO_ROOT="$(cd "$SCRIPT_DIR/.." && pwd)"
OUT_DIR="${1:-$REPO_ROOT/android/app/build/rustJniLibs}"
PROFILE="${RUST_PROFILE:-release}"
API_LEVEL=24

find_ndk() {
    if [[ -n "${ANDROID_NDK_HOME:-}" && -d "${ANDROID_NDK_HOME}" ]]; then
        printf '%s\n' "$ANDROID_NDK_HOME"
        return 0
    fi
    if [[ -n "${ANDROID_NDK_ROOT:-}" && -d "${ANDROID_NDK_ROOT}" ]]; then
        printf '%s\n' "$ANDROID_NDK_ROOT"
        return 0
    fi

    local sdk="${ANDROID_HOME:-${ANDROID_SDK_ROOT:-}}"
    if [[ -z "$sdk" && -f "$REPO_ROOT/android/local.properties" ]]; then
        sdk="$(sed -n 's/^sdk\.dir=//p' "$REPO_ROOT/android/local.properties" | head -n 1)"
    fi
    if [[ -n "$sdk" && -d "$sdk/ndk" ]]; then
        find "$sdk/ndk" -mindepth 1 -maxdepth 1 -type d | sort -V | tail -n 1
        return 0
    fi

    return 1
}

case "$(uname -s)" in
    Darwin) HOST_TAG="darwin-x86_64" ;;
    Linux) HOST_TAG="linux-x86_64" ;;
    *)
        echo "error: unsupported host platform: $(uname -s)" >&2
        exit 1
        ;;
esac

if ! NDK="$(find_ndk)"; then
    echo "error: Android NDK not found." >&2
    echo "Set ANDROID_NDK_HOME or install an NDK under \$ANDROID_HOME/ndk." >&2
    exit 1
fi

TOOLCHAIN="$NDK/toolchains/llvm/prebuilt/$HOST_TAG/bin"
if [[ ! -d "$TOOLCHAIN" ]]; then
    echo "error: NDK toolchain not found at $TOOLCHAIN" >&2
    echo "Check the NDK installation at $NDK." >&2
    exit 1
fi

if [[ -n "${RUST_ANDROID_ABIS:-}" ]]; then
    read -r -a REQUESTED_ABIS <<< "$RUST_ANDROID_ABIS"
else
    REQUESTED_ABIS=(arm64-v8a armeabi-v7a x86_64 x86)
fi

is_requested() {
    local candidate="$1" abi
    for abi in "${REQUESTED_ABIS[@]}"; do
        [[ "$abi" == "$candidate" ]] && return 0
    done
    return 1
}

build_for() {
    local abi="$1" triple="$2" linker_name="$3"
    local linker="$TOOLCHAIN/$linker_name"
    local profile_dir="$PROFILE"
    local linker_var
    local cargo_args=(build --manifest-path "$SCRIPT_DIR/Cargo.toml" --target "$triple")

    if [[ "$PROFILE" == "release" ]]; then
        cargo_args+=(--release)
    elif [[ "$PROFILE" != "debug" ]]; then
        profile_dir="$PROFILE"
    fi

    if [[ ! -x "$linker" ]]; then
        echo "error: linker not found: $linker" >&2
        exit 1
    fi

    linker_var="CARGO_TARGET_$(printf '%s' "$triple" | tr '[:lower:]-' '[:upper:]_')_LINKER"

    echo "Building libkrabik_ui_example.so for $abi ($triple, $PROFILE)"
    env \
        "$linker_var=$linker" \
        "CARGO_TARGET_$(printf '%s' "$triple" | tr '[:lower:]-' '[:upper:]_')_AR=$TOOLCHAIN/llvm-ar" \
        cargo "${cargo_args[@]}"

    mkdir -p "$OUT_DIR/$abi"
    install -m 644 "$SCRIPT_DIR/target/$triple/$profile_dir/libkrabik_ui_example.so" "$OUT_DIR/$abi/libkrabik_ui_example.so"
}

if is_requested "arm64-v8a"; then
    build_for "arm64-v8a" "aarch64-linux-android" "aarch64-linux-android${API_LEVEL}-clang"
fi
if is_requested "armeabi-v7a"; then
    build_for "armeabi-v7a" "armv7-linux-androideabi" "armv7a-linux-androideabi${API_LEVEL}-clang"
fi
if is_requested "x86_64"; then
    build_for "x86_64" "x86_64-linux-android" "x86_64-linux-android${API_LEVEL}-clang"
fi
if is_requested "x86"; then
    build_for "x86" "i686-linux-android" "i686-linux-android${API_LEVEL}-clang"
fi

echo "Native libraries written to $OUT_DIR"
