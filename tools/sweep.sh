#!/bin/bash
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
: > "$out/sweep.log"
for d in "$workshop"/*/; do
  id=$(basename "$d")
  [ -f "$d/scene.pkg" ] || continue
  echo "##### $id" >> "$out/sweep.log"
  timeout 240 cargo run --offline -q -p wallflux --example shot -- "$d" "$out/vse/$id.png" 640 360 6 >> "$out/sweep.log" 2>&1
  echo "##### exit=$?" >> "$out/sweep.log"
done
echo DONE >> "$out/sweep.log"
