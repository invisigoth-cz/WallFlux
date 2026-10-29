#!/bin/bash
# SPDX-License-Identifier: GPL-3.0-or-later
# Copyright (C) 2026 invisigoth
# Renders every scene wallpaper Steam has downloaded to a PNG and gathers what
# the renderer said about each into one log. Two sweeps compared picture by
# picture with `cmp` are how a change to the renderer is checked: whatever
# moved that was not meant to is a regression.
#
#   tools/sweep.sh <output-dir> [workshop-dir]
set -u
out=${1:?usage: tools/sweep.sh <output-dir> [workshop-dir]}
workshop=${2:-$HOME/.local/share/Steam/steamapps/workshop/content/431960}
cd "$(dirname "$0")/.." || exit 1
cargo build --offline -q -p wallflux --example shot || exit 1
mkdir -p "$out/vse"
# A copy of the binary, not `cargo run` and not the build's own file: a sweep
# takes minutes, and a source file saved and built during it would otherwise
# be what draws its second half.
cp target/debug/examples/shot "$out/shot" || exit 1
: > "$out/sweep.log"
for d in "$workshop"/*/; do
  id=$(basename "$d")
  [ -f "$d/scene.pkg" ] || continue
  echo "##### $id" >> "$out/sweep.log"
  timeout 240 "$out/shot" "$d" "$out/vse/$id.png" 640 360 6 >> "$out/sweep.log" 2>&1
  echo "##### exit=$?" >> "$out/sweep.log"
done
echo DONE >> "$out/sweep.log"
