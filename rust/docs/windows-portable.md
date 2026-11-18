# Windows portable EXE

Run `awakened-crab-trade-0.34.0-windows-x64-portable.exe` directly. No manual
extraction, installer or administrator privileges are needed.

On first launch, GTK, the checker and their licenses are extracted to
`%LOCALAPPDATA%\awakened-crab-trade\portable`. Later launches reuse that cache;
nothing is written beside the portable EXE.

Preferences live in `%APPDATA%\awakened-crab-trade\config.json`. Older profiles
are imported automatically without changing the originals. Choose English or
French in Settings, then Save. Choose a Style for an immediate preview; Save
keeps it and Cancel restores the previous style. Current filters and offers
are preserved when saving a style change. Item names and modifiers remain in English.

In PoE, hover over an item and press `Ctrl+Alt+D`. Select your filters, then
click Search. Opening the panel never starts a Trade search; poe.ninja reference
quotes still load automatically. Escape or a click outside closes the panel.

Run with `--extract-only` from a terminal to print the extracted folder without
opening the checker. The extracted checker can also run directly, and its
shared libraries can be replaced. A conventional ZIP is available too.

The launcher checks each file during extraction and publishes the complete
folder atomically. Concurrent launches share preparation; an interrupted
extraction is rebuilt on the next launch.

Quit the running checker from its notification icon before launching an updated EXE.

## Build

After building the [Windows ZIP](windows.md):

```sh
python3 rust/scripts/package-windows-portable.py
```

The launcher uses compression and SHA256 verification and imports only Windows
system DLLs. Its dependencies and licenses are included in the extracted files.
Wine checks use a private desktop and do not replace testing PoE on Windows.
