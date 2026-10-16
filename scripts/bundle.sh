#!/usr/bin/env bash
# Builds dist/Resources.app (universal, ad-hoc signed) and dist/resources-mac-<version>.dmg.
# Expects release builds for both aarch64-apple-darwin and x86_64-apple-darwin.
set -euo pipefail

version="${1:?usage: scripts/bundle.sh <version>}"
root="$(cd "$(dirname "$0")/.." && pwd)"
dist="$root/dist"
app="$dist/Resources.app"
bin=resources-mac

rm -rf "$dist"
mkdir -p "$app/Contents/MacOS"

lipo -create \
  "$root/target/aarch64-apple-darwin/release/$bin" \
  "$root/target/x86_64-apple-darwin/release/$bin" \
  -output "$app/Contents/MacOS/$bin"
sed "s/__VERSION__/$version/g" "$root/macos/Info.plist" > "$app/Contents/Info.plist"
mkdir -p "$app/Contents/Resources"
cp "$root/macos/AppIcon.icns" "$app/Contents/Resources/"

# Ad-hoc signature: not notarized, but required for arm64 binaries to launch at all.
codesign --force --sign - "$app"

dmg="$dist/resources-mac-$version.dmg"
staging="$dist/dmg"
mkdir -p "$staging"
cp -R "$app" "$staging/"
ln -s /Applications "$staging/Applications"
hdiutil create -quiet -volname Resources -srcfolder "$staging" -ov -format UDZO "$dmg"
rm -rf "$staging"

shasum -a 256 "$dmg" | cut -d' ' -f1
