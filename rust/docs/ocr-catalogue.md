# Local item and modifier library - 0.36.7

Both editions prepare a versioned offline catalogue from bundled English item
and modifier definitions. It includes the curated PoEDB Viscera/Wickerman
supplement. The cache stores `items.json`, `mods.json` and an `active.json`
manifest beneath the edition's cache directory:

- Windows: `%LOCALAPPDATA%\awakened-crab-trade[-ocr]\catalog\v1\0.36.7`.
- Linux: `$XDG_CACHE_HOME/awakened-crab-trade/catalog/v1/0.36.7`, falling back
  to `~/.cache` when XDG_CACHE_HOME is unset.

Startup prepares and loads these files on a dedicated worker. With automatic
catalogue updates enabled in General settings, it checks the official English
Trade item and stat lists at most once per day after a successful refresh.
Failed attempts have a one-hour retry cooldown. No update request is made per
capture or per OCR line. There is no item image, screenshot or log upload.

The bundled definitions work without a network connection. Downloads have
timeouts and size limits. Both responses must pass validation before publication.
Files are checksummed and a single manifest rename publishes the generation;
interrupted or corrupt caches fall back to bundled references. Curated parsing,
category and duplicate-ID rules take precedence over new raw Trade definitions.
Missing item categories, gem levels and Viscera slots are not guessed.

Parsed indexes live in memory and rebuild once when a new generation is
published. Existing readers retain their immutable generation. Item recognition,
the native parser and Trade modifier-ID lookup share this reference library.

OCR first tries exact matching. A short, unambiguous spelling repair can then
recover an item name or modifier wording. Modifier candidates must keep the
observed number count, signs, decimals, percent symbols and roll ranges. Fixed
numbers in a template cannot replace observed numbers. Clearly read elements,
polarity and restrictions are protected; close competing candidates are rejected.
The existing range and defence consistency checks still apply. No missing ilvl
is supplied. A matched wording is not proof that OCR read the digits correctly.

Rare names are generated; the catalogue identifies their base instead. Trade
pseudo filters are not treated as extra physical item modifiers. Gem descriptions
and arbitrary flavor text are not forced into modifier matches. Unknown or
ambiguous readings remain in the editable OCR review. Successful word repairs
and catalogue refresh errors appear in the local session log.

Diagnostics without opening the overlay:

```sh
awakened-crab-trade catalogue --offline
awakened-crab-trade catalogue
```

These commands print the local directory and record counts; the second allows
a refresh under the same freshness/cooldown policy. The feature uses Rust and
the existing HTTP stack; no Python runtime or model fine-tuning is introduced.

Visual wraps are reassembled only when the complete nearby text matches a
modifier. Source headers, properties, separate columns and conflicting readings
remain boundaries. Numeric rolls and conditions stay attached to the complete
modifier. Cluster enchantments retain their enchant Trade IDs in basic and
advanced tooltips; jewel placement instructions are excluded from modifiers.
