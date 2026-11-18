# Awakened Crab Trade - native app

Native Rust/GTK3 price checker for Path of Exile 1. No Electron, Node.js or WebView is needed.

## Build and run

Linux dependencies: Rust, GTK3 development files, gtk-layer-shell, pkg-config,
wl-clipboard and xdotool. The current overlay targets River/Wayland with PoE
running through XWayland.

From the repository root:

```sh
cargo build --locked --release --manifest-path rust/Cargo.toml --bin awakened-crab-trade -j 3
GDK_BACKEND=wayland rust/target/release/awakened-crab-trade daemon
```

Windows build and usage: [Windows](docs/windows.md),
[single-file portable EXE](docs/windows-portable.md).

## Controls

- `Ctrl+D`: copy the hovered item and show a passive price panel.
- `Ctrl+Alt+D`: copy and open the interactive panel.
- `Shift+Space`: enable interaction in a passive panel.
- `Ctrl+Shift+D`, `Esc`, the close button or an outside click: hide the panel.

Choose modifiers, base, ilvl and other filters, then select **Rechercher**.
Sliders use integer thresholds, cyan below the minimum and dark grey above.
Hover over an offer to see the offered item's mods and price. Currency uses
poe.ninja quotes; other supported families also offer Trade searches.

The current league is selected automatically. Settings can override the league,
shortcuts, scale and default filters. Unknown historical preferences are preserved.
Choose English or French in Settings, then Save. New profiles start in English;
existing French profiles retain their language. Older profiles are imported once
without changing the originals. Current executable, config and IPC names use
`awakened-crab-trade`. Opening the checker never sends a Trade search: select
filters, then click Search. poe.ninja reference quotes still load on opening.

## Checks and packaging

```sh
cargo test --locked --manifest-path rust/Cargo.toml --bin awakened-crab-trade -j 3
cargo clippy --locked --manifest-path rust/Cargo.toml --all-targets -- -D warnings
python3 rust/scripts/test-price-checker.py
sh rust/scripts/package.sh
```

GUI checks require the local graphical test tools described in
[validation](docs/CHECKER-VALIDATION.md). They create guarded private desktops;
they never send test input to the real game. The four files in `examples/` are
helpers for those checks. Builds and reports stay in ignored `dist/` and `target/`.

`package.sh` builds an Arch/Artix package without installing it. Cargo dependencies
must be available locally (`cargo fetch --locked --manifest-path rust/Cargo.toml`
before an offline package build).

## Data and maintenance

All runtime data now lives in `src/data/`. The pinned English item/stat catalogues
and item-drop table retain their hashes and upstream revision in `sources.json`.
The other JSON rule tables preserve their upstream source hashes as provenance.
The original sources and migration tools remain available in Git history.

Refresh the official Trade supplement and current item/icon registry with:

```sh
python3 rust/scripts/refresh-current-data.py --download --league Covens
```

Use the desired league name. Public input snapshots are cached in
`dist/research/current/`; refresh provenance is stored in
[the current data report](docs/current-data.json). Existing upstream
disambiguation remains authoritative. [Game artwork credits](docs/GAME-ART.md).

The CLI `inspect FILE` prints parsed filters without querying Trade. `lookup FILE
[LEAGUE]` queries prices; `analyze FILE` retains the map-profile diagnostics.
