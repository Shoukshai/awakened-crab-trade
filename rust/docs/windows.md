# Windows price checker 0.36.0

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

Preferences: `%APPDATA%\awakened-crab-trade\config.json` (standard) or
`%APPDATA%\awakened-crab-trade-ocr\config.json` (OCR).
Cache and `checker.log`: the matching edition directory under `%LOCALAPPDATA%`.

Quit the running checker from its notification icon before launching an updated EXE.

## Separate Windows editions

- `awakened-crab-trade-0.36.0-windows-x64-portable.exe`: standard clipboard edition
  for local PoE. It does not include the OCR engines or language model.
- `awakened-crab-trade-ocr-0.36.0-windows-x64-portable.exe`: cloud OCR edition,
  enabled immediately. Boosteroid.exe is the default client; change the executable
  basename or full path in Settings > Capture if needed.

The OCR package includes the English PP-OCRv5 recognition model and native CPU
runtime. No Python interpreter, recognition server, GPU or model download at
first launch is required. The standard package omits these assets.

The capture method is fixed at build time. Renaming an EXE or loading an older
profile cannot switch it to the other edition. The standard edition retains the
existing profile. OCR has its own `awakened-crab-trade-ocr` configuration, cache,
logs, extraction cache and control endpoint; the notification tooltip includes OCR.
Both may run together: standard shortcuts target local PoE, OCR shortcuts target
the chosen cloud client. Quit the old mixed 0.34 build before using these editions.

## Cloud gaming OCR (experimental)

Hover an item, hold Alt until its advanced tooltip is visible, then press Ctrl+D
(with Alt still held). Release Alt after capture. The panel shows editable item
text and any unrecognized lines. Verify the base, item level, values and modifiers,
then choose Use these stats. Search remains manual. English game text is required.
An absent item level stays absent; required character/gem levels are never used.

Capture and OCR run on a bounded background worker. Window/focus changes and
Hide cancel stale work. The image is acquired before the overlay opens, without
injecting copy commands. The existing Windows/Tesseract scout locates the tooltip;
the small English PP-OCRv5 model then reads its lines through native ONNX Runtime
called directly from Rust. The model is loaded once on first use and reused.
Its CPU session uses two compute threads, sequential execution and no idle
spinning. Rust handles line segmentation, resizing, CTC decoding and validation.
There is no Python process or GPU inference. Recognition is bounded and checked
for cancellation between lines. See [hardware targets and measurements](ocr-benchmark.md).

Windows.Media.Ocr via windows-rs and bundled Tesseract remain local fallback
readings for missing or uncertain text. Microsoft only supports its OCR API for
apps with package identity; the included engines also work without a Windows OCR
language pack. Images are not uploaded to a recognition service.
The Tesseract fallback exchanges pixels and text through memory pipes; captures are
not written to disk during normal use. Its child process is bounded and stopped
by Windows when the checker exits.

Capture tries Windows Graphics Capture (Windows 10 1903+), Desktop Duplication,
desktop GDI cropped to the client area, then client-window GDI. Each method's
pixels are checked: a successful API call returning black no longer stops the
fallback chain. WGC and DXGI wait for a readable frame with bounded,
cancellable polling; mouse-only DXGI updates are skipped. Errors report each
attempt in the panel and the OCR edition's checker.log. No PrintWindow is used.
An explicit open reasserts topmost placement without a recurring focus-stealing loop.
Use windowed or borderless cloud streaming; protected video and exclusive fullscreen
can prevent capture or normal desktop overlays. Blank captures fail explicitly.

The area is automatic. A scout pass reads the client frame and matches visible
item names/bases against the catalogue; a second pass reads the isolated tooltip.
Rare headers can provide an initial hint, while other rarities use the full frame.
The detector handles gem/currency titles, unique name + base, magic base names and
map names. There is no coordinate field or rectangle-selection step. If the frame
is ambiguous, retry while hovering one item with its advanced tooltip visible.
Spelling is not guessed, and OCR does not reconstruct unseen socket links,
icon-only influence or hidden mods. Always review values before searching.

Advanced tooltips keep observed roll ranges and modifier groups. A separate
color reading removes neutral map labels behind translucent tooltips and recovers
obscured stats. Fractured and crafted text colors supplement faded source labels.
Conflicting stat readings stay in review instead of becoming search filters.
Overlapping readings with different glyph heights count as one physical line.
A conflicting flat defence roll can be resolved only when one observed candidate
fits a separately observed total confirmed by two passes, the catalogue base range
and all recognized modifiers. Missing values are never calculated this way.

Windows OCR can split property labels from their values. A value is reattached
only to its explicit label on the same baseline; a detached requirement level
cannot fill the item-level field or become a modifier. An enlarged grayscale
retry supplements uncertain captures. Conflicting stat values need two distinct
passes to agree, while adding a roll range to the same value is not a conflict.

Alt shows unmodified base defences. These remain labelled `Base Energy Shield`,
`Base Armour`, etc. A final defence marked `(calculated)` is added only when
the catalogue verifies the base roll and local modifiers are recognized. It is
recomputed after review edits; uncertain modifiers prevent that property filter.
Contradictions between displayed defences, the base range and local modifiers
also stay in review, preventing compressed digits from becoming trusted filters.
The calculation includes local flat/increased defences, quality and integer
rounding, following [Path of Building's item calculations](https://github.com/PathOfBuildingCommunity/PathOfBuilding/blob/dev/src/Classes/Item.lua).

The private Wine suite checks real pixel recognition with the bundled fallback,
Alt-only metadata, responsive GTK, cancellation, process scope and a topmost fake
cloud window. It cannot validate Microsoft's OCR engine, hardware video capture,
or Boosteroid's real fullscreen behavior; those need a Windows 10/11 test.

API references: [windows-rs](https://github.com/microsoft/windows-rs),
[Windows OCR requirements](https://learn.microsoft.com/en-us/uwp/api/windows.media.ocr),
[HWND graphics capture](https://learn.microsoft.com/en-us/windows/win32/api/windows.graphics.capture.interop/nf-windows-graphics-capture-interop-igraphicscaptureiteminterop-createforwindow),
[Desktop Duplication](https://learn.microsoft.com/en-us/windows/win32/direct3ddxgi/desktop-dup-api).

## Build from Linux

Requires stable Rust/rustup, Python 3.11+, 7-Zip, HTTPS access, tar with zstd/xz,
pkg-config and glib-compile-schemas. Windows build inputs stay in `rust/dist`.

```sh
python3 rust/scripts/setup-windows.py
cargo fetch --locked --manifest-path rust/Cargo.toml --target x86_64-pc-windows-gnullvm
cargo fetch --locked --manifest-path rust/portable/Cargo.toml --target x86_64-pc-windows-gnullvm
python3 rust/scripts/build-windows-editions.py
```

The pinned MSYS2 inputs and SHA256 hashes are recorded in
`rust/packaging/windows/runtime-lock.json`. The final package includes only the
required DLL import closure, image loaders, GTK resources, local OCR and licenses.
`ocr-data.json` pins the English model and its license to upstream hashes.
`neural-ocr.json` pins PP-OCRv5, native ONNX Runtime and app-local Visual C++ DLLs.
`setup-neural-ocr.py` is a build helper only: it extracts native libraries and
license notices from checksum-verified upstream containers, never Python modules.
`runtime-imports.json` lists PE imports; `SHA256FILES.json` records file hashes.

`windows-sandbox.py` and `test-windows.py` use a private Wine prefix and River
desktop. The fake game is accepted only with its temporary guard token. Never
use a real Steam prefix or game window for automated input.

`setup-windows-test-layout.py` builds an LGPL
[ReactOS keyboard layout](https://github.com/reactos/reactos/tree/344ebfe594e4cf6a498bac08105e1996a03a2c61/dll/keyboard/kbdus)
for a keyboard component absent from Wine. It is installed only in the private
test prefix, never distributed. The app uses Windows keyboard layouts unchanged.
