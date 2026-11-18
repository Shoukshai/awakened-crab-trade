# Windows price checker 0.34.0

Native x64 build for PoE 1 via Steam or standalone. The cloud OCR path targets
Windows 10/11 and is experimental until tested with the real Boosteroid client.
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

## Cloud gaming OCR (experimental)

In Settings > Capture, select OCR and set the executable that owns the cloud
stream window. Boosteroid.exe is the default; a different .exe basename or full
path can be entered. Clipboard mode remains the default for local PoE.

Hover an item, hold Alt until its advanced tooltip is visible, then press Ctrl+D
(with Alt still held). Release Alt after capture. The panel shows editable item
text and any unrecognized lines. Verify the base, item level, values and modifiers,
then choose Use these stats. Search remains manual. English game text is required.
An absent item level stays absent; required character/gem levels are never used.

Capture and OCR run on a bounded background worker. Window/focus changes and
Hide cancel stale work. The image is acquired before the overlay opens, without
injecting copy commands. Windows.Media.Ocr via windows-rs is tried first. Because
Microsoft only supports that API for apps with package identity, this portable
build also includes Tesseract and pinned English language data. Images are not
uploaded to a recognition service. The local fallback exchanges pixels and text through memory pipes; captures are
not written to disk during normal use. Its child process is bounded and stopped
by Windows when the checker exits.

Capture uses DXGI Desktop Duplication, with a client-window GDI fallback. An
explicit open reasserts topmost placement without a recurring focus-stealing loop.
Use windowed or borderless cloud streaming; protected video and exclusive fullscreen
can prevent capture or normal desktop overlays. Blank captures fail explicitly.

The automatic area detector handles yellow rare-item headers. For other layouts,
or if surrounding UI text interferes, set OCR area to x, y, width, height as
percentages of the client area (for example 58, 0, 38, 65). A configured area is
used exactly. Spelling is not guessed; unknown stats require manual correction.
OCR does not reconstruct unseen socket links, icon-only influence or hidden mods.

The private Wine suite checks real pixel recognition with the bundled fallback,
Alt-only metadata, responsive GTK, cancellation, process scope and a topmost fake
cloud window. It cannot validate Microsoft's OCR engine, hardware video capture,
or Boosteroid's real fullscreen behavior; those need a Windows 10/11 test.

API references: [windows-rs](https://github.com/microsoft/windows-rs),
[Windows OCR requirements](https://learn.microsoft.com/en-us/uwp/api/windows.media.ocr),
[Desktop Duplication](https://learn.microsoft.com/en-us/windows/win32/direct3ddxgi/desktop-dup-api).

## Build from Linux

Requires stable Rust/rustup, Python 3.11+, HTTPS access, tar with zstd/xz,
pkg-config and glib-compile-schemas. Windows build inputs stay in `rust/dist`.

```sh
python3 rust/scripts/setup-windows.py
sh rust/scripts/cargo-windows.sh build --release --locked --bin awakened-crab-trade -j 3
python3 rust/scripts/package-windows.py
```

The pinned MSYS2 inputs and SHA256 hashes are recorded in
`rust/packaging/windows/runtime-lock.json`. The final package includes only the
required DLL import closure, image loaders, GTK resources, local OCR and licenses.
`ocr-data.json` pins the English model and its license to upstream hashes.
`runtime-imports.json` lists PE imports; `SHA256FILES.json` records file hashes.

`windows-sandbox.py` and `test-windows.py` use a private Wine prefix and River
desktop. The fake game is accepted only with its temporary guard token. Never
use a real Steam prefix or game window for automated input.

`setup-windows-test-layout.py` builds an LGPL
[ReactOS keyboard layout](https://github.com/reactos/reactos/tree/344ebfe594e4cf6a498bac08105e1996a03a2c61/dll/keyboard/kbdus)
for a keyboard component absent from Wine. It is installed only in the private
test prefix, never distributed. The app uses Windows keyboard layouts unchanged.
