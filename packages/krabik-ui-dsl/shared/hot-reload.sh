#!/bin/sh
# Hot reload for development: rebuilds an app's Rust library when its sources
# change and hands it to the running app, which continues with the new code
# and the state it had. See `rust/src/hot.rs` for how and for the limits.
#
#   shared/hot-reload.sh <app directory> android|ios [--once]
#
# The app must be built with the `hot-reload` feature of krabik-ui-dsl, invoke
# `krabik_ui_dsl::hot_reload!(App)`, and be running on an emulator or a
# simulator. A change to the app's state or message type, or to the framework,
# still needs a rebuild and a restart.
set -eu

app=$(cd "$1" && pwd)
platform=$2
once=${3:-}
crate="$app/rust"
package=$(sed -n 's/^name = "\(.*\)"/\1/p' "$crate/Cargo.toml" | head -1)
library=$(printf '%s' "$package" | tr - _)
adb=${ADB:-${ANDROID_HOME:+$ANDROID_HOME/platform-tools/}adb}

case "$platform" in
android)
  abi=$("$adb" shell getprop ro.product.cpu.abi | tr -d '\r')
  case "$abi" in
  arm64-v8a) triple=aarch64-linux-android ;;
  x86_64) triple=x86_64-linux-android ;;
  *) echo "unsupported ABI: $abi" >&2; exit 1 ;;
  esac
  application=$(sed -n 's/.*applicationId = "\(.*\)".*/\1/p' "$app/android/app/build.gradle.kts")
  min_sdk=$(sed -n 's/.*minSdk = \([0-9]*\).*/\1/p' "$app/android/app/build.gradle.kts")
  ndk=$(ls -d "$ANDROID_HOME"/ndk/* | sort | tail -1)
  host=$(ls "$ndk/toolchains/llvm/prebuilt" | head -1)
  variable=CARGO_TARGET_$(printf '%s' "$triple" | tr 'a-z-' 'A-Z_')_LINKER
  export "$variable=$ndk/toolchains/llvm/prebuilt/$host/bin/$triple$min_sdk-clang"
  built="$crate/target/$triple/debug/lib$library.so"
  extension=so
  ;;
ios)
  triple=aarch64-apple-ios-sim
  bundle=$(sed -n 's/.*PRODUCT_BUNDLE_IDENTIFIER: \(.*\)/\1/p' "$app/ios/project.yml" | head -1)
  built="$crate/target/$triple/debug/lib$library.dylib"
  extension=dylib
  ;;
*)
  echo "usage: $0 <app directory> android|ios [--once]" >&2
  exit 1
  ;;
esac

# Hands the library over under a new name: writing under another extension
# and renaming means the app never sees half a file.
deliver() {
  name="app-$(date +%s)-$$.$extension"
  if [ "$platform" = android ]; then
    "$adb" push "$built" "/data/local/tmp/$name" >/dev/null
    "$adb" shell "run-as $application sh -c 'mkdir -p code_cache/krabik-hot \
      && cp /data/local/tmp/$name code_cache/krabik-hot/$name.part \
      && mv code_cache/krabik-hot/$name.part code_cache/krabik-hot/$name'"
    "$adb" shell rm "/data/local/tmp/$name"
  else
    directory="$(xcrun simctl get_app_container booted "$bundle" data)/tmp/krabik-hot"
    mkdir -p "$directory"
    cp "$built" "$directory/$name.part"
    mv "$directory/$name.part" "$directory/$name"
  fi
}

reload() {
  started=$(date +%s)
  if cargo build --quiet --manifest-path "$crate/Cargo.toml" --target "$triple"; then
    deliver
    echo "reloaded in $(($(date +%s) - started))s"
  else
    echo "the build failed; the app keeps running the code it has"
  fi
}

stamp() {
  find "$crate/src" -type f -name '*.rs' -exec stat -f '%m %N' {} + 2>/dev/null | sort | cksum
}

reload
[ "$once" = "--once" ] && exit 0
echo "watching $crate/src"
last=$(stamp)
while sleep 1; do
  now=$(stamp)
  if [ "$now" != "$last" ]; then
    last=$now
    reload
  fi
done
