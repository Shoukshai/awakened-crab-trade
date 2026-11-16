#!/bin/sh
# A separate headless River/XWayland server; never sends test keys to the game.
set -eu
apt_test_dir=$(CDPATH='' cd -- "$(dirname -- "$0")/.." && pwd)
cd "$apt_test_dir"
command -v river >/dev/null
command -v xdotool >/dev/null
cargo build --locked --example shortcut-replay -j 3
apt_test_runtime=$(mktemp -d "${TMPDIR:-/tmp}/apt-replay-desktop-XXXXXXXX")
apt_test_pid=''
cleanup() {
    if [ -n "$apt_test_pid" ]; then kill "$apt_test_pid" 2>/dev/null || true; wait "$apt_test_pid" 2>/dev/null || true; fi
    rm -rf "${apt_test_runtime:?}"
}
trap cleanup EXIT HUP INT TERM
cat > "$apt_test_runtime/init.sh" <<'SH'
#!/bin/sh
printf '%s\n' "$DISPLAY" > "$XDG_RUNTIME_DIR/display"
printf '%s\n' "$WAYLAND_DISPLAY" > "$XDG_RUNTIME_DIR/wayland"
SH
chmod +x "$apt_test_runtime/init.sh"
XDG_RUNTIME_DIR="$apt_test_runtime" WLR_BACKENDS=headless WLR_HEADLESS_OUTPUTS=1 WLR_RENDERER=pixman river -c "$apt_test_runtime/init.sh" > "$apt_test_runtime/server.log" 2>&1 &
apt_test_pid=$!
apt_test_count=0
while [ ! -s "$apt_test_runtime/display" ] || [ ! -s "$apt_test_runtime/wayland" ]; do
    if ! kill -0 "$apt_test_pid" 2>/dev/null || [ "$apt_test_count" -gt 60 ]; then cat "$apt_test_runtime/server.log"; exit 1; fi
    apt_test_count=$((apt_test_count+1))
    sleep 0.05
done
apt_test_display=$(cat "$apt_test_runtime/display")
if [ "$apt_test_display" = "${DISPLAY:-}" ]; then printf '%s\n' 'Refusing to test on the current desktop display'; exit 1; fi
XDG_RUNTIME_DIR="$apt_test_runtime" DISPLAY="$apt_test_display" WAYLAND_DISPLAY=$(cat "$apt_test_runtime/wayland") "${CARGO_TARGET_DIR:-$apt_test_dir/target}/debug/examples/shortcut-replay" --isolated
