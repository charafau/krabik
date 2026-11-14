#!/bin/sh
# Builds the app's Rust library for whatever Xcode is building: a device, an
# Apple-silicon simulator or an Intel simulator. Every app's Xcode project
# runs this as its first build phase, so the linked library is never stale.
#
# Xcode provides SRCROOT, CONFIGURATION, PLATFORM_NAME and ARCHS; the project
# sets RUST_LIB (the library's name without `lib` and `.a`).
set -eu

export PATH="$HOME/.cargo/bin:$PATH"
cd "$SRCROOT/../rust"

if [ "$CONFIGURATION" = "Release" ]; then
  profile=release
  flags=--release
else
  profile=debug
  flags=
fi

# The linker is pointed at this directory; see LIBRARY_SEARCH_PATHS.
out="target/xcode/$PLATFORM_NAME-$CONFIGURATION"
mkdir -p "$out"

libraries=
for arch in $ARCHS; do
  case "$PLATFORM_NAME-$arch" in
    iphoneos-arm64) target=aarch64-apple-ios ;;
    iphonesimulator-arm64) target=aarch64-apple-ios-sim ;;
    iphonesimulator-x86_64) target=x86_64-apple-ios ;;
    *)
      echo "error: no Rust target for $PLATFORM_NAME on $arch" >&2
      exit 1
      ;;
  esac
  # shellcheck disable=SC2086 # $flags is empty or one word
  cargo build --target "$target" $flags
  libraries="$libraries target/$target/$profile/lib$RUST_LIB.a"
done

# One file for the linker, whether Xcode asked for one architecture or two.
# shellcheck disable=SC2086 # the list is split on purpose
lipo -create $libraries -output "$out/lib$RUST_LIB.a"
