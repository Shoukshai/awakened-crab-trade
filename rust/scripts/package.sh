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
    renderer/public/data/en/stats-matcher.index.bin renderer/public/data/en/stats-ref.index.bin \
    renderer/public/data/en/items-name.index.bin renderer/public/data/en/items-ref.index.bin \
    renderer/public/data/en/client_strings.js renderer/public/data/item-drop.json renderer/src/parser renderer/src/assets/data \
    renderer/src/web/price-check/filters \
    renderer/src/web/price-check/trade/common.ts \
    renderer/src/web/price-check/trade/pathofexile-trade.ts \
    renderer/src/web/background/Leagues.ts renderer/src/web/background/Prices.ts \
    renderer/src/web/price-check/unidentified-resolver renderer/src/web/price-check/trends renderer/src/web/price-check/related-items renderer/src/web/price-check/expected-value \
    renderer/src/web/overlay/WidgetDelveGrid.vue renderer/src/web/overlay/OverlayWindow.vue renderer/src/web/overlay/Widget.vue renderer/src/web/overlay/WidgetImageStrip.vue renderer/src/web/overlay/WidgetMenu.vue renderer/src/web/overlay/widget-registry.ts renderer/src/web/overlay/widgets.ts renderer/src/web/ui/Popover.vue renderer/src/web/ui/FullscreenImage.vue renderer/public/images/syndicate.jpg renderer/src/web/stopwatch renderer/tailwind.config.js \
    renderer/src/web/stash-search renderer/src/web/item-check/hotkeyable-actions.ts main/src/shortcuts/text-box.ts main/src/shortcuts/HostClipboard.ts main/src/shortcuts/Shortcuts.ts main/src/windowing/GameWindow.ts renderer/src/web/item-search renderer/src/web/map-check renderer/src/web/item-check/WidgetItemCheck.vue \
    renderer/src/web/item-check/ItemInfo.vue \
    renderer/package.json renderer/package-lock.json \
    ipc/KeyToCode.ts main/build/icons/128x128.png -C "$rust_dir/dist" license-bundle
apt_source_sha=$(sha256sum "rust/dist/native-source-${apt_pkg_version}.tar.gz" | cut -d ' ' -f 1)
sed "s/sha256sums=('SKIP')/sha256sums=('$apt_source_sha')/" rust/packaging/PKGBUILD > rust/dist/PKGBUILD
cd "$rust_dir/dist"
export CARGO_TARGET_DIR="$rust_dir/target"
makepkg --nodeps --force
sha256sum ./*.pkg.tar.zst native-source-${apt_pkg_version}.tar.gz > SHA256SUMS
