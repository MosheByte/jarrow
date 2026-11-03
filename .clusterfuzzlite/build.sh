#!/bin/bash -eu
# ClusterFuzzLite build script: builds every cargo-fuzz target in fuzz/ and copies the
# binaries (and their dictionaries, when present) to $OUT.
# See https://google.github.io/clusterfuzzlite/build-integration/rust-lang/

cd "$SRC/jarrow"
# Release build with debug assertions and overflow checks, so arithmetic bugs crash too.
cargo fuzz build -O --debug-assertions

out_dir=fuzz/target/x86_64-unknown-linux-gnu/release
for target in $(cargo fuzz list); do
  cp "$out_dir/$target" "$OUT/"
  if [ -f "fuzz/dicts/$target.dict" ]; then
    cp "fuzz/dicts/$target.dict" "$OUT/$target.dict"
  fi
done
