#!/usr/bin/env bash
# Build a standalone FAT APK for Android arm64.
#
# Usage:
#   scripts/build-android.sh release   # signed release APK
#   scripts/build-android.sh debug     # debug-signed APK
#
# Tool locations can be overridden with JAVA_HOME, ANDROID_HOME, and
# ANDROID_NDK_ROOT. Release signing uses CARGO_APK_RELEASE_KEYSTORE and
# CARGO_APK_RELEASE_KEYSTORE_PASSWORD, falling back to the development key at
# ~/.android/fat-release.keystore and its password file.
set -euo pipefail

ROOT_DIR=$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)
MODE=${1:-release}

case "$MODE" in
    release)
        CARGO_ARGS=(--release)
        PROFILE_DIR=release
        ;;
    debug)
        CARGO_ARGS=()
        PROFILE_DIR=debug
        ;;
    *)
        echo "usage: $0 [release|debug]" >&2
        exit 2
        ;;
esac

: "${JAVA_HOME:=$HOME/Android/jdk-17}"
: "${ANDROID_HOME:=$HOME/Android/sdk}"
: "${ANDROID_NDK_ROOT:=$ANDROID_HOME/ndk/27.2.12479018}"
export JAVA_HOME ANDROID_HOME ANDROID_NDK_ROOT
export PATH="$JAVA_HOME/bin:$ANDROID_HOME/platform-tools:$ANDROID_HOME/build-tools/35.0.0:$PATH"

if [ "$MODE" = release ]; then
    KEYSTORE=${CARGO_APK_RELEASE_KEYSTORE:-$HOME/.android/fat-release.keystore}
    PASSWORD_FILE=${CARGO_APK_RELEASE_KEYSTORE_PASSWORD_FILE:-$HOME/.android/fat-release.keystore.pass}
    if [ -z "${CARGO_APK_RELEASE_KEYSTORE_PASSWORD:-}" ] && [ -f "$PASSWORD_FILE" ]; then
        CARGO_APK_RELEASE_KEYSTORE_PASSWORD=$(cat "$PASSWORD_FILE")
    fi
    if [ ! -f "$KEYSTORE" ] || [ -z "${CARGO_APK_RELEASE_KEYSTORE_PASSWORD:-}" ]; then
        echo "Release signing key missing." >&2
        echo "Set CARGO_APK_RELEASE_KEYSTORE and CARGO_APK_RELEASE_KEYSTORE_PASSWORD," >&2
        echo "or create $KEYSTORE with its password in $PASSWORD_FILE." >&2
        exit 1
    fi
    export CARGO_APK_RELEASE_KEYSTORE=$KEYSTORE
    export CARGO_APK_RELEASE_KEYSTORE_PASSWORD
fi

cd "$ROOT_DIR"
cargo apk build --lib --target aarch64-linux-android "${CARGO_ARGS[@]}"

VERSION=$(sed -n 's/^version = "\(.*\)"/\1/p' Cargo.toml | head -1)
mkdir -p dist
cp "target/$PROFILE_DIR/apk/fat.apk" "dist/fat-$VERSION-arm64.apk"
echo "APK: $ROOT_DIR/dist/fat-$VERSION-arm64.apk"
