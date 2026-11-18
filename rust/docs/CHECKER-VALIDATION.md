# Price checker validation

Run from the repository root:

```sh
cargo test --locked --manifest-path rust/Cargo.toml --bin awakened-crab-trade -j 3
cargo clippy --locked --manifest-path rust/Cargo.toml --all-targets -- -D warnings
cargo test --locked --manifest-path rust/portable/Cargo.toml
```

## Native GUI

Requires River, riverctl, wlr-randr, xdotool, wl-copy, wtype, grim and Python Pillow.
Build the release binary first, then run:

```sh
python3 rust/scripts/test-price-checker.py
APT_FAMILY_CASE=wand python3 rust/scripts/test-price-checker.py
APT_FAMILY_CASE=viscera-siren python3 rust/scripts/test-price-checker.py
APT_THEME_CYCLE=1 APT_FIXTURE_LANGUAGE=en python3 rust/scripts/test-price-checker.py
```

Other family cases: gem, currency, unique, map, viscera and influence. The test
creates its own compositor and fake game, serves local Trade/poe.ninja fixtures,
and blocks public market traffic. Reports go to `rust/dist/`. Captures and
failure logs stay in the private test directory or the explicitly selected path.

Coverage includes scoped copying, modifiers and Min/Max queries, integer slider
values and rendered cyan/grey colours, full-row hover, inside/outside clicks,
owned menus, Escape, focus changes, settings and clipboard restoration.
The full runner also checks live English/French switching and cancellation of
a language draft. Set `APT_FIXTURE_LANGUAGE=en` for English captures; the
default test language is French. All runners deliberately load a legacy
`automatic_search=true` profile and verify that copying sends no Trade request
until Search is clicked. Core tests cover profile import, language defaults and all ten persisted theme IDs.
Theme-cycle checks preserve a live query across all styles, validate Split/Focus/
Compact geometry and whole-row hover, and exercise Cancel/Escape/outside rollback.
Use `APT_FIXTURE_THEME=focus` (or another theme ID) for a family-specific layout check.

## Packaged Windows

Use [the Windows build instructions](windows.md) and build the portable EXE.
Run `windows-sandbox.py` in a separate terminal. With that guarded desktop active:

```sh
python3 rust/scripts/setup-windows-test-layout.py
APT_WINDOWS_THEME_CYCLE=1 python3 rust/scripts/test-windows-portable.py
```

The runner builds its Win32 fixture and launcher unit tests from the current
checkout. These helpers are excluded from the production packages. The sandbox has its own Wine prefix and keyboard layout;
never point it at a user's Steam prefix. Stop it by creating `stop` inside the
guarded directory recorded in `rust/dist/windows-sandbox.json`.

Automated offers and prices are synthetic. Wine checks do not replace testing
PoE under Windows hardware. The supported Linux setup is River with XWayland;
other compositors and monitor arrangements need separate validation.
