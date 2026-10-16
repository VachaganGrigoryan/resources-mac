#!/usr/bin/env bash
# Prints the Homebrew cask for a released version.
set -euo pipefail

version="${1:?usage: scripts/cask.sh <version> <sha256>}"
sha256="${2:?usage: scripts/cask.sh <version> <sha256>}"

cat <<CASK
cask "resources-mac" do
  version "$version"
  sha256 "$sha256"

  url "https://github.com/VachaganGrigoryan/resources-mac/releases/download/v#{version}/resources-mac-#{version}.dmg"
  name "Resources"
  desc "System monitor inspired by GNOME Resources"
  homepage "https://github.com/VachaganGrigoryan/resources-mac"

  depends_on macos: ">= :ventura"

  app "Resources.app"

  zap trash: "~/Library/Saved Application State/io.github.vachagangrigoryan.resources-mac.savedState"
end
CASK
