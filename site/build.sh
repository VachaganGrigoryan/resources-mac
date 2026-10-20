#!/usr/bin/env sh
# Assembles the static site into public/: the pages plus the screenshots, icons and demo video
# from docs/. Vercel runs it (vercel.json buildCommand, which is capped at 256 characters).
set -eu
rm -rf public
mkdir -p public/screenrecord
cp -R site/static/. public/
cp -R docs/screenshots public/screenshots
cp docs/icon.png docs/disk-sweep-icon.png public/
cp docs/screenrecord/resources-mac-demo.mp4 public/screenrecord/
