#!/bin/sh
set -eu
rust_dir=$(CDPATH='' cd -- "$(dirname -- "$0")/.." && pwd)
repo_dir=$(dirname -- "$rust_dir")
apt_pkg_version=$(sed -n 's/^version = "\(.*\)"/\1/p' "$rust_dir/Cargo.toml" | head -n 1)
mkdir -p "$rust_dir/dist"
python3 "$rust_dir/scripts/collect-licenses.py" "$rust_dir/dist/license-bundle"
cd "$repo_dir"
# Only the native program and licensed build-time inputs; no local profiles/logs.
tar --exclude='__pycache__' --exclude='*.pyc' --exclude='rust/portable/target' -czf "$rust_dir/dist/native-source-${apt_pkg_version}.tar.gz" LICENSE README.md AGENTS.md docs/screenshots \
    rust/Cargo.toml rust/Cargo.lock rust/build.rs rust/src rust/examples rust/scripts \
    rust/packaging rust/portable rust/README.md rust/docs rust/assets \
    -C "$rust_dir/dist" license-bundle
apt_source_sha=$(sha256sum "rust/dist/native-source-${apt_pkg_version}.tar.gz" | cut -d ' ' -f 1)
if [ "${1-}" = "--source-only" ]; then
    printf '%s  %s\n' "$apt_source_sha" "native-source-${apt_pkg_version}.tar.gz" > "$rust_dir/dist/native-source-${apt_pkg_version}.tar.gz.sha256"
    exit 0
fi
sed "s/sha256sums=('SKIP')/sha256sums=('$apt_source_sha')/" rust/packaging/PKGBUILD > rust/dist/PKGBUILD
cd "$rust_dir/dist"
export CARGO_TARGET_DIR="$rust_dir/target"
makepkg --cleanbuild --nodeps --force
sha256sum ./*.pkg.tar.zst native-source-${apt_pkg_version}.tar.gz > SHA256SUMS
