#!/usr/bin/env bash
# Prints the Homebrew cask for a released app: scripts/cask.sh <resources-mac|disk-sweep> <version> <sha256>
set -euo pipefail

usage="usage: scripts/cask.sh <resources-mac|disk-sweep> <version> <sha256>"
token="${1:?$usage}"
version="${2:?$usage}"
sha256="${3:?$usage}"

case "$token" in
  resources-mac) name="Resources"; desc="System monitor inspired by GNOME Resources" ;;
  disk-sweep) name="Disk Sweep"; desc="Finds caches, build output and Docker data to clean up" ;;
  *) echo "$usage" >&2; exit 1 ;;
esac

cat <<CASK
cask "$token" do
  version "$version"
  sha256 "$sha256"

  url "https://github.com/VachaganGrigoryan/resources-mac/releases/download/v#{version}/$token-#{version}.dmg"
  name "$name"
  desc "$desc"
  homepage "https://github.com/VachaganGrigoryan/resources-mac"

  depends_on macos: :ventura

  app "$name.app"

  zap trash: "~/Library/Saved Application State/io.github.vachagangrigoryan.$token.savedState"
end
CASK
