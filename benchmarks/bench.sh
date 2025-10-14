#!/bin/bash
set -eiuo xtrace
#fastfec_path=$1
#libfec_path=$2
fec_path=$1

# /Users/alex/projects/

hyperfine \
--prepare 'rm -rf output || true' \
  "../../FastFEC/zig-out/bin/fastfec -x $fec_path" \
  --prepare 'rm -rf output || true' \
  "libfec-release fastfec $fec_path"
