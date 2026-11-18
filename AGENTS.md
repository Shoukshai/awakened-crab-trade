# Project instructions

- Product name: Awakened Crab Trade. The native app lives in `rust/`.
- The repository is public. Change its visibility only when explicitly requested.
- The user wants completed changes committed and pushed to `origin/main` from
  now on. Run relevant checks first, then push, unless the user asks otherwise.
- Keep the root README short, in English, with ASCII characters only and no emoji.
- Preserve the upstream MIT license and credit Awakened PoE Trade.
- Use `rust/assets/branding/` for app, notification and executable icons.
- Standard edition uses `awakened-crab-trade` for executable, settings and IPC.
- Windows OCR edition uses `awakened-crab-trade-ocr` for its profile, cache and
  control endpoint. It is compiled with `cloud-ocr`; standard builds omit OCR.
- Keep capture methods fixed per edition and default shortcut scopes separate.
  Local PoE OCR testing is an explicit opt-in using Ctrl+Alt+O, never clipboard copy.
- OCR must locate the tooltip automatically; do not require manual coordinates.
- Import older profiles through the compatibility module; preserve user settings.
- Never send test input to the real game or use the user's Steam/Wine prefix.
  Use the guarded private test desktops under `rust/scripts/`.
- Keep generated builds, local profiles, caches and credentials out of Git.
