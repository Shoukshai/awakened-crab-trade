# Awakened Classic

Select Settings > Style > Awakened Classic. Preview, Save and Cancel work as with
other styles. Existing profiles keep their selected theme.

[Native preview](../../docs/screenshots/styles/awakened.png) (synthetic item and offers).

This eleventh native theme adapts Awakened PoE Trade's familiar price-checker
hierarchy: a 460 px full-height panel, dark league bar, separate item-name strip,
slate palette, wrapping filter chips, compact modifier rows with min/max on the
right, a slider underneath, small Search button and alternating price-first offers.
The same filter widgets and values are reparented when the style changes; switching
styles does not start a Trade request or reset the current item or results.
OCR, explicit Search, outside-click closing and offer previews keep their behavior.

Reference: SnosMe/awakened-poe-trade, MIT, commit
`2498730beecc1a46d39214ac437ffbe9f5f542d8`. Palette and layout were checked against
[PriceCheckWindow.vue](https://github.com/SnosMe/awakened-poe-trade/blob/2498730beecc1a46d39214ac437ffbe9f5f542d8/renderer/src/web/price-check/PriceCheckWindow.vue),
FiltersBlock.vue, FilterModifier.vue, TradeListing.vue and tailwind.config.js.
The upstream MIT notice remains in the project's LICENSE.

This is a native GTK implementation, not the original Electron renderer;
platform font metrics and controls can differ. Fontin SmallCaps or Fontin is used
if already installed, with Georgia/Liberation Serif as a fallback. Font files are
not bundled: the foundry lists app embedding/distribution under
[extended licensing](https://www.exljbris.com/extended_license.html).
