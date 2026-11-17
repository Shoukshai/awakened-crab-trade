# Awakened Crab Trade

<img src="rust/assets/branding/icon-256.png" alt="Awakened Crab Trade" width="128">

A native Path of Exile 1 price checker for Linux and Windows, inspired by
Awakened PoE Trade.

- Select item modifiers and adjust integer Min/Max filters.
- Search Trade offers and hover over a result to see its full mods and price.
- Check poe.ninja prices for currency, gems, uniques, maps and Viscera.
- Use a full-height overlay beside the inventory, with automatic league selection.
- Dark navy and cyan styling inspired by the crab icon.

## Screenshots

Cropped demo views from version 0.31.1. Click for full panels. Offers and prices are test data.

| Gem | Rare | Unique | Currency |
| --- | --- | --- | --- |
| [<img src="docs/screenshots/gem.png" width="200" alt="Gem price checker">](docs/screenshots/full/gem.png) | [<img src="docs/screenshots/rare.png" width="200" alt="Rare item stat filters">](docs/screenshots/full/rare.png) | [<img src="docs/screenshots/unique.png" width="200" alt="Unique item price checker">](docs/screenshots/full/unique.png) | [<img src="docs/screenshots/currency.png" width="200" alt="Currency prices">](docs/screenshots/full/currency.png) |

| Map | Viscera | Offer hover |
| --- | --- | --- |
| [<img src="docs/screenshots/map.png" width="200" alt="Map filters and prices">](docs/screenshots/full/map.png) | [<img src="docs/screenshots/viscera.png" width="200" alt="Viscera filters and prices">](docs/screenshots/full/viscera.png) | [<img src="docs/screenshots/offer-hover.png" width="200" alt="Offer preview with item mods">](docs/screenshots/full/offer-hover.png) |

## Usage

On Windows, run the portable EXE. In PoE, hover over an item and press
`Ctrl+Alt+D` to open the interactive checker. `Ctrl+D` opens a passive view.
Press `Esc` or click outside the panel to close it.

Linux currently targets River/Wayland with PoE running through XWayland.
Build and platform details: [native app](rust/README.md),
[Windows portable](rust/docs/windows-portable.md).

[Planned improvements](TODO.md).

## Credits

Based on [Awakened PoE Trade](https://github.com/SnosMe/awakened-poe-trade).
Original [MIT license](LICENSE) retained. Market data comes from GGG Trade and
poe.ninja; item references include PoEDB. Project icon supplied by Shoukshai.
