# Windows price checker 0.33.0

Native x64 build targeting Windows 11, for PoE 1 via Steam or standalone.
The [single portable EXE](windows-portable.md) prepares its runtime automatically.
For the ZIP, extract everything and run `awakened-crab-trade.exe`; keep its DLLs
and subfolders beside it.

Settings open on first launch. The notification icon opens Settings or quits.
Launching the EXE again opens Settings in the existing instance. Select English
or French and Save to change the language without restarting. The Style selector
previews ten native themes; Save keeps the style and Cancel restores it.
Style changes preserve the current filters and displayed offers. Existing profiles
are imported automatically; game item and mod names remain in English.

While hovering over an item in PoE:

- `Ctrl+Alt+D`: copy and open the interactive panel.
- `Ctrl+D`: open a passive panel; `Shift+Space` enables interaction.
- Escape, the close button or an outside click: close the panel.
- `Ctrl+Shift+D`: hide the panel.

Select filters, then click Search. No Trade search is sent when opening or
copying an item. poe.ninja reference quotes load automatically. Shortcuts are
configurable and scoped to PoE; copying sends only `Ctrl+Alt+C` to the active
game window. Linux and Windows use the same filters, offers and hover previews.
No rare-item price prediction engine is included.

Use windowed or borderless windowed PoE with matching privileges for both
programs, normally without administrator rights. Automated tests use Wine, a
fake game and local network responses; Windows hardware still needs real-game
validation.

Preferences: `%APPDATA%\awakened-crab-trade\config.json`.
Cache and `checker.log`: `%LOCALAPPDATA%\awakened-crab-trade`.

Quit the running checker from its notification icon before launching an updated EXE.

## Build from Linux

Requires stable Rust/rustup, Python 3.11+, HTTPS access, tar with zstd/xz,
pkg-config and glib-compile-schemas. Windows build inputs stay in `rust/dist`.

```sh
python3 rust/scripts/setup-windows.py
sh rust/scripts/cargo-windows.sh build --release --locked --bin awakened-crab-trade -j 3
python3 rust/scripts/package-windows.py
```

The 60 pinned MSYS2 inputs and SHA256 hashes are recorded in
`rust/packaging/windows/runtime-lock.json`. The final package includes only the
required DLL import closure, image loaders, GTK resources and licenses.
`runtime-imports.json` lists PE imports; `SHA256FILES.json` records file hashes.

`windows-sandbox.py` and `test-windows.py` use a private Wine prefix and River
desktop. The fake game is accepted only with its temporary guard token. Never
use a real Steam prefix or game window for automated input.

`setup-windows-test-layout.py` builds an LGPL
[ReactOS keyboard layout](https://github.com/reactos/reactos/tree/344ebfe594e4cf6a498bac08105e1996a03a2c61/dll/keyboard/kbdus)
for a keyboard component absent from Wine. It is installed only in the private
test prefix, never distributed. The app uses Windows keyboard layouts unchanged.
