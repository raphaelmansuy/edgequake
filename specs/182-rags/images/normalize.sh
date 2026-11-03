#!/usr/bin/env bash
# Re-apply the exact geometry and encoding SPEC-182 pins for every picture.
#
# WHY a script and not a note: a replacement image dragged in from anywhere would
# otherwise arrive at 4000 px and 4 MB, and the ingest of twenty documents would slow
# from a minute to ten while the screenshots gained nothing.
#
# Idempotent: running it on an already-normalised file changes nothing but the mtime.
set -euo pipefail
cd "$(dirname "$0")"

for file in *.jpg; do
  magick "$file" \
    -auto-orient \
    -resize '1400x1400>' \
    -strip \
    -quality 86 \
    -sampling-factor 4:2:0 \
    "normalized-$file"
  mv "normalized-$file" "$file"
done

magick identify -format '%f %wx%h %b\n' ./*.jpg
