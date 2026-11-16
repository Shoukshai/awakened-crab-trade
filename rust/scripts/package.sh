#!/bin/sh
set -eu
rust_dir=$(CDPATH='' cd -- "$(dirname -- "$0")/.." && pwd)
repo_dir=$(dirname -- "$rust_dir")
apt_pkg_version=$(sed -n 's/^version = "\(.*\)"/\1/p' "$rust_dir/Cargo.toml" | head -n 1)
mkdir -p "$rust_dir/dist"
python3 "$rust_dir/scripts/collect-licenses.py" "$rust_dir/dist/license-bundle"
cd "$repo_dir"
# Only the native program and licensed build-time inputs; no local profiles/logs.
tar --exclude='__pycache__' --exclude='*.pyc' -czf "$rust_dir/dist/native-source-${apt_pkg_version}.tar.gz" LICENSE README.md \
    rust/Cargo.toml rust/Cargo.lock rust/src rust/examples rust/scripts \
    rust/packaging rust/README.md rust/docs \
    renderer/public/data/en/items.ndjson renderer/public/data/en/stats.ndjson \
    main/build/icons/128x128.png -C "$rust_dir/dist" license-bundle
apt_source_sha=$(sha256sum "rust/dist/native-source-${apt_pkg_version}.tar.gz" | cut -d ' ' -f 1)
sed "s/sha256sums=('SKIP')/sha256sums=('$apt_source_sha')/" rust/packaging/PKGBUILD > rust/dist/PKGBUILD
cd "$rust_dir/dist"
export CARGO_TARGET_DIR="$rust_dir/target"
makepkg --nodeps --force
sha256sum ./*.pkg.tar.zst native-source-${apt_pkg_version}.tar.gz > SHA256SUMS
