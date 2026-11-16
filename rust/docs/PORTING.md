# Migration native Rust

Source conservée intégralement, licence MIT et attributions dans `../LICENSE` et
`../README.md`. Base upstream : `3188208957422b21b69b12083a03edf3d1bac187`
(branche `rust-linux`). Les ajouts Rust sont sous la même licence.
Le dépôt local est `/home/shouk/awakened-poe-trade`. La connexion Git par credential
manager n'a pas répondu en mode non interactif (8 s) : fork GitHub non créé.

## Inventaire initial (lecture du code upstream)

« Testée » nécessite une preuve distincte de la compilation. Une case vide signifie
que le travail reste à faire, y compris pour les implémentations partielles.

| Fonction / source upstream | À porter | Implémentée Rust | Testée | Restante |
|---|---|---|---|---|
| Overlay, zones de clic, focus (`main/src/windowing`) | oui | layer-shell natif, suivi Wayland | panneau au-dessus de PoE, clic Fermer, 5 cycles, focus passif, changement d’app confirmé | réduction, plusieurs écrans/facteurs de sortie |
| Copie et restauration presse-papiers (`HostClipboard`) | oui | marqueur Proton, copie XWayland, lecture data-control | vraie Driftwood Wand copiée avant affichage | restauration optionnelle, autres jeux/langues/backends |
| Raccourcis (`Shortcuts`) | oui | grabs X11 limités à XWayland + IPC | Ctrl+D copie et prix ; Maj+Espace interaction | replay autres apps à vérifier, backend River/portail pour Wayland natif, configuration UI |
| Parsing multilingue, descriptions avancées (`renderer/src/parser`) | oui | anglais, catalogue bases, propriétés, mods simples non ambigus | fixtures, copie réelle avancée de wand | parité complète, mods complexes, autres langues et fixtures |
| Filtres et recherche Trade, bulk | oui | base, unique, ilvl, gemme, qualité, tier, liens, mods explicites ; Trade search/fetch | recherche réelle Covens, 10 offres visibles ; mock 429/JSON/annulation/cache | bulk, filtres avancés, parité de tous les types, UI complète |
| Prix poe.ninja, tendances, prix liés, poussière, valeur espérée | oui | non | non | données, caches, graphiques |
| Évaluation objets, cartes et mods dangereux | oui | non | non | règles, configuration, UI |
| Recherche objets, wiki et liens externes | oui | non | non | recherche et raccourcis |
| Recherche coffre et défilement | oui | non | non | intégration et configuration |
| Widgets minuteur, images, grille Delve, menu | oui | non | non | états et rendu natif |
| Réglages, raccourcis configurables, langues, échelle | oui | JSON, ligue/ilvl/en ligne dans le panneau, échelle au démarrage | ligue Covens persistée, échelle app 1,25 observée | éditeur complet, langues, sélection d’échelle dans l’UI |
| Chat / commandes / journal client | oui | non | non | port sans déclencher de commandes pendant les tests |
| OCR gemmes Heist (`vision`) | oui | non | non | upstream Windows uniquement, backend futur |
| Tray, mises à jour, debug, fichiers hôte | oui | logs stderr, status IPC, PKGBUILD et lanceur | status et journaux en direct, paquet construit/extrait/exécuté | installation propre, tray, mises à jour et fichiers hôte |

## Séparation

`overlay.rs` : machine d'états indépendante du bureau. `desktop/` : capacités,
fenêtres, entrées et presse-papiers. `ui.rs` : rendu GTK natif (aucun WebView).
Le métier et le réseau sont dans des modules séparés. X11 autonome et Windows
doivent avoir leurs propres implémentations : aucun n'est validé par les tests
Wayland/XWayland de cette machine.

États : Hidden → Preparing → Passive → Interactive (action explicite après premier
rendu) → Closing → Hidden. Une erreur révoque les entrées avant Error. Fermer ou
changer d'objet invalide la génération de requête. Une réponse ne peut ni ouvrir,
ni activer une fenêtre. Aucune activation exclusive et aucun appel pour forcer le
focus au jeu. River décide du focus lors d'un clic en mode OnDemand.

`item.rs`, `modifiers.rs`, `filters.rs` portent maintenant le métier ; `trade.rs`
le réseau ; `settings.rs` la persistance. `desktop/xwayland_shortcuts.rs` est une
intégration d'entrées X11 limitée aux fenêtres XWayland, pas un backend d'overlay
X11 autonome. Le prototype laisse les entrées passives traverser le panneau.
