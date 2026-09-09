#!/usr/bin/env bash
# Xcode pre-build step: compiles crates/cl-app as a static library for the platform being built
# and places it where project.yml's LIBRARY_SEARCH_PATHS expects it.
set -euo pipefail
here="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
root="$(cd "$here/../.." && pwd)"
export PATH="$HOME/.cargo/bin:$PATH"

case "${PLATFORM_NAME:-iphoneos}" in
  iphonesimulator) target="aarch64-apple-ios-sim" ;;
  *) target="aarch64-apple-ios" ;;
esac
case "${CONFIGURATION:-Debug}" in
  Release) profile="release"; flag="--release" ;;
  *) profile="debug"; flag="" ;;
esac

cd "$root"
# shellcheck disable=SC2086
cargo build -p cl-app --lib --target "$target" $flag
out="$here/rust-lib/${PLATFORM_NAME:-iphoneos}"
mkdir -p "$out"
cp "target/$target/$profile/libcl_app.a" "$out/libcl_app.a"
echo "rust: $target ($profile) -> $out/libcl_app.a"
