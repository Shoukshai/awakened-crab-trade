# Awakened Crab Trade

<img src="rust/assets/branding/icon-256.png" alt="Awakened Crab Trade" width="128">

A native Path of Exile 1 price checker for Linux and Windows, inspired by
Awakened PoE Trade.

- Select item modifiers and adjust integer Min/Max filters.
- Search Trade offers and hover over a result to see its full mods and price.
- Check poe.ninja prices for currency, gems, uniques, maps and Viscera.
- Use a full-height overlay beside the inventory, with automatic league selection.

## Usage

On Windows, run the portable EXE. In PoE, hover over an item and press
`Ctrl+Alt+D` to open the interactive checker. `Ctrl+D` opens a passive view.
Press `Esc` or click outside the panel to close it.

Linux currently targets River/Wayland with PoE running through XWayland.
Build and platform details: [native app](rust/README.md),
[Windows portable](rust/docs/windows-portable.md).

## Credits

Based on [Awakened PoE Trade](https://github.com/SnosMe/awakened-poe-trade).
Original [MIT license](LICENSE) retained. Market data comes from GGG Trade and
poe.ninja; item references include PoEDB. Project icon supplied by Shoukshai.
