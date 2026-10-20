#!/usr/bin/env bash
# Builds dist/Resources.app and dist/Disk Sweep.app (universal, ad-hoc signed) and a dmg for
# each: dist/resources-mac-<version>.dmg and dist/disk-sweep-<version>.dmg.
# Prints "<sha256>  <dmg name>" per dmg.
# Expects release builds for both aarch64-apple-darwin and x86_64-apple-darwin.
set -euo pipefail

version="${1:?usage: scripts/bundle.sh <version>}"
root="$(cd "$(dirname "$0")/.." && pwd)"
dist="$root/dist"

rm -rf "$dist"
mkdir -p "$dist"
# Apache-2.0 and the crates' MIT/BSD licenses require their texts to ship with the binaries.
# One report for the workspace covers both apps.
licenses="$dist/THIRD_PARTY_LICENSES.html"
(cd "$root" && cargo about generate --fail about.hbs) > "$licenses"

# bundle <crate dir> <app name> <binary> <dmg slug>
bundle() {
  local crate="$root/crates/$1" name="$2" bin="$3" slug="$4"
  local app="$dist/$name.app"
  mkdir -p "$app/Contents/MacOS" "$app/Contents/Resources"
  lipo -create \
    "$root/target/aarch64-apple-darwin/release/$bin" \
    "$root/target/x86_64-apple-darwin/release/$bin" \
    -output "$app/Contents/MacOS/$bin"
  sed "s/__VERSION__/$version/g" "$crate/macos/Info.plist" > "$app/Contents/Info.plist"
  cp "$crate/macos/AppIcon.icns" "$root/LICENSE" "$root/NOTICE" "$licenses" "$app/Contents/Resources/"

  # Ad-hoc signature: not notarized, but required for arm64 binaries to launch at all.
  codesign --force --sign - "$app"

  local dmg="$slug-$version.dmg" staging="$dist/dmg"
  mkdir -p "$staging"
  cp -R "$app" "$staging/"
  ln -s /Applications "$staging/Applications"
  hdiutil create -quiet -volname "$name" -srcfolder "$staging" -ov -format UDZO "$dist/$dmg"
  rm -rf "$staging"
  echo "$(shasum -a 256 "$dist/$dmg" | cut -d' ' -f1)  $dmg"
}

bundle resources Resources resources-mac resources-mac
bundle disk-sweep "Disk Sweep" disk-sweep disk-sweep
rm "$licenses"
