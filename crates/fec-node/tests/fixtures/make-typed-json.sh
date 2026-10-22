#!/bin/sh
# Regenerate the *.typed.json.gz files the parity test compares against: the
# Python bindings' typed records for each fixture. Needs crates/fec-py built
# (`make develop` there). Commit the output.
set -eu
here=$(cd "$(dirname "$0")" && pwd)
py=$here/../../../fec-py
for f in "$py"/tests/fixtures/*.fec "$here"/edge.fec; do
  name=$(basename "$f" .fec)
  # gzip -n: no name or timestamp, so regenerating gives identical bytes.
  (cd "$py" && uv run --quiet python "$here/dump_typed.py" "$f") | gzip -9 -n > "$here/$name.typed.json.gz"
  echo "$name.typed.json.gz"
done
