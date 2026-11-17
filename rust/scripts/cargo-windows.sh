#!/bin/sh
set -eu
apt_rust_dir=$(CDPATH='' cd -- "$(dirname -- "$0")/.." && pwd)
apt_win_root="$apt_rust_dir/dist/windows-tools"
apt_win_llvm="$apt_win_root/llvm-mingw-20261104-ucrt-ubuntu-22.04-x86_64"
export CARGO_TARGET_X86_64_PC_WINDOWS_GNULLVM_LINKER="$apt_win_llvm/bin/x86_64-w64-mingw32-clang"
export CC_x86_64_pc_windows_gnullvm="$apt_win_llvm/bin/x86_64-w64-mingw32-clang"
export AR_x86_64_pc_windows_gnullvm="$apt_win_llvm/bin/llvm-ar"
export APT_WINDOWS_WINDRES="$apt_win_llvm/bin/x86_64-w64-mingw32-windres"
export PKG_CONFIG_ALLOW_CROSS=1
export PKG_CONFIG_SYSROOT_DIR="$apt_win_root/sysroot"
export PKG_CONFIG_LIBDIR="$apt_win_root/sysroot/ucrt64/lib/pkgconfig:$apt_win_root/sysroot/ucrt64/share/pkgconfig"
apt_cargo_command=$1
shift
cargo "$apt_cargo_command" --manifest-path "$apt_rust_dir/Cargo.toml" --target x86_64-pc-windows-gnullvm "$@"
