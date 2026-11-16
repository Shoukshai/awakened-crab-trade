# Configuration observée — 16 novembre 2026

- Artix Linux, x86_64 ; Rust/Cargo 1.99.0 ; GTK 3.24.52, gtk-layer-shell 0.10.1.
- Bureau River (PID 1128), session Wayland `wayland-1`. Serveur XWayland rootless
  PID 1152, DISPLAY `:0`. Pas de processus Gamescope observé.
- Steam app 238960, DWProton 11.0-14, processus jeu PID 22408 (`Main`), lancé avec
  `PathOfExileSteam.exe --nopatch`. Fenêtre X11 `0x4600001`, classe
  `steam_app_238960`, titre `Path of Exile`, état fullscreen et focused.
- Un écran eDP-1, 2560×1600, 240 Hz, facteur 1, transform normal (wlr-randr).
  xrandr ne décrit que la vue XWayland et n'est pas la preuve du bureau natif.
- Registre Wayland inspecté via `WAYLAND_DEBUG=1 wlr-randr` : layer-shell v4,
  foreign-toplevel v3, data-control v2, virtual-keyboard v1, virtual-pointer v2,
  screencopy v3, fractional-scale, xdg-activation. Accès effectivement annoncé,
  pas encore une preuve de tous les comportements applicatifs.
- Introspection D-Bus du portail : GlobalShortcuts et RemoteDesktop absents.
  Portails gtk et wlr installés. Pas de raccourci global natif supposé par XWayland.
- `wl-paste --list-types` fonctionne (texte). `grim` capture bien PoE plein écran.
- River configuré en focus-follows-cursor always : ce facteur doit être inclus
  dans les tests de clic et focus ; aucune modification de cette configuration.
- Aucun Awakened PoE Trade lancé au début. Le symptôme original n'est donc pas
  déclaré reproduit. Le code upstream peut activer immédiatement après envoi IPC
  de l'objet, sans accusé du premier rendu (`Shortcuts.ts`, `OverlayWindow.ts`).
  C'est une piste statique, pas une cause démontrée sur cette machine.

PoE reste ouvert. Aucun réglage session, Steam/Proton, système ou écran modifié.
Les captures de validation restent locales et ne sont pas destinées au dépôt.
