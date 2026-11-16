#!/bin/sh
set -eu
rust_dir=$(CDPATH='' cd -- "$(dirname -- "$0")/.." && pwd)
repo_dir=$(dirname -- "$rust_dir")
mkdir -p "$rust_dir/dist"
cd "$repo_dir"
# Only the native program and licensed build-time inputs; no local profiles/logs.
tar -czf "$rust_dir/dist/native-source-0.1.0.tar.gz" LICENSE README.md \
    rust/Cargo.toml rust/Cargo.lock rust/src rust/examples rust/scripts \
    rust/packaging rust/README.md rust/docs \
    renderer/public/data/en/items.ndjson renderer/public/data/en/stats.ndjson \
    main/build/icons/128x128.png
cp rust/packaging/PKGBUILD rust/dist/PKGBUILD
cd "$rust_dir/dist"
export CARGO_TARGET_DIR="$rust_dir/target"
makepkg --nodeps --force
sha256sum ./*.pkg.tar.zst native-source-0.1.0.tar.gz > SHA256SUMS
