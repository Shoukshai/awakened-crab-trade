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
| Raccourcis (`Shortcuts`) | oui | grabs X11 limités à XWayland + IPC ; vérification du focus réel | Ctrl+D copie et prix ; Maj+Espace interaction ; replay et conflit sur serveur isolé | backend River/portail pour Wayland natif, configuration UI |
| Parsing multilingue, descriptions avancées (`renderer/src/parser`) | oui | anglais, catalogue typé, propriétés, identifiants Trade/bulk, piles, influences ; traduction des plages, flags, multilignes, variantes par catégorie ; cumul et effet des mods | copies réelles wand/Wisdom ; 55 961 comparaisons traduction/effet avec le TypeScript inchangé ; tests des blocs | parité du parser d’objet entier et enrichissements spécialisés restants, autres langues |
| Filtres et recherche Trade, bulk | oui | base, unique, ilvl, gemme, qualité, tier, liens, bornes Min/Max, options, alternatives OR ; 35 règles pseudo générales ; propriétés équipement/Q20, bornes suggérées et IDs internes ; Trade et bulk | recherches réelles Covens ; 1 651 comparaisons objet→calculs/bornes avec TypeScript ; DPS de la vraie wand et suggestions d'une fixture dans l'UI | presets cartes/charts/flacons/Heist portés au jalon 0.6 (voir validation) ; restent presets équipement/base, ajustements finaux, clusters/Timeless/anointments/Atzoatl/etc. ; politique des filtres d’objet portée au jalon 0.7, éditeur encore partiel |
| Prix poe.ninja, tendances, prix liés, valeur espérée (poussière locale portée dans l’évaluation) | oui | feed dense, cache 31 min, variantes simples, prix unitaire/pile, graphe Cairo, lien source | feed réel Covens et pile de 3 Wisdom ; tests variantes/cache/erreurs | uniques à variantes complexes, prix liés, valeur espérée (poussière locale portée dans l’évaluation) |
| Évaluation objets, cartes et mods dangereux | oui | calcul local DPS/poussière ; trois profils persistés, couleurs/vu/nouveaux ; catalogue 340 mods, inconnus et obsolètes | 3 144 cas parseur/calculs et 1 875 décisions comparés au TypeScript upstream ; tests natifs documentés dans VALIDATION | images des cartes, autres objets réels et langues |
| Recherche objets, wiki et liens externes | oui | liens Wiki/PoEDB/CoE de l’objet copié, fermeture avant ouverture explicite | génération/encodage et conditions CoE testés ; ouverture réelle navigateur non testée | recherche du catalogue, raccourcis dédiés, validation navigateur |
| Recherche coffre et défilement | oui | non | non | intégration et configuration |
| Widgets minuteur, images, grille Delve, menu | oui | non | non | états et rendu natif |
| Réglages, raccourcis configurables, langues, échelle | oui | JSON compatible 0.1, ligue/en ligne/devise/plage et préférences marchands/regroupement ; filtres d’objet transitoires dans le panneau, échelle au démarrage | ligue Covens saisie/persistée, devise/stock manipulés, plage dans les requêtes ; échelle app 1,25 observée en 0.1 ; migration JSON | éditeur complet, langues, sélection d’échelle dans l’UI |
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
et `economy.rs` les services réseau, chacun avec son worker borné ; `settings.rs`
la persistance. Les réponses obsolètes sont supprimées par génération ; une
requête HTTP bloquante déjà partie reste bornée par son timeout de 15 s.
Les dépendances graphiques sont ciblées Linux dans Cargo ; aucun build Windows
n'a été effectué. `desktop/xwayland_shortcuts.rs` est une
intégration d'entrées X11 limitée aux fenêtres XWayland, pas un backend d'overlay
X11 autonome. Le prototype laisse les entrées passives traverser le panneau.

`calculated.rs`, `pseudo.rs`, `properties.rs` préparent les filtres sans dépendre
du bureau. La table pseudo exportée depuis le TypeScript conserve sa provenance
et son hash dans `src/data/pseudo-rules.json` ; son vérificateur reste hors ligne.

`inspection.rs` porte l’évaluation locale et les décisions des cartes ;
`ui/inspection_panel.rs` rend les profils et un TreeView natif pour le catalogue.
Les valeurs par défaut sont exportées avec hash depuis WidgetItemCheck upstream.
La copie dispose de son propre worker borné : une recherche HTTP en cours ne
retarde plus la capture. Changer de mode invalide les anciennes réponses réseau.

`presets.rs` porte les presets spécialisés cartes/charts/flacons/Heist ; le choix
est natif et les exclusions génèrent un groupe NOT, y compris les IDs alternatifs.
La résolution par référence sert aussi aux stats absentes de l’objet. Le parseur
conserve les tiers, les propriétés de zone et les métadonnées Heist. Les autres
presets continuent de passer par le moteur de filtres calculés existant.

`item_filters.rs` porte le modèle et la politique des filtres d’objet upstream,
séparés des presets de mods ; `ui/item_filters_panel.rs` fournit l’éditeur natif.
Le modèle et le corps Trade complet sans presets sont comparés sur 2 312 cas
(2 299 reconnus, 13 rejets obsolètes attendus). Le parseur conserve les métadonnées
gemmes/uniques/voilés nécessaires. Une édition invalide la génération ; un signal
GTK déjà en file ne peut pas annuler une recherche plus récente. Les minima par
copie ne sont jamais sérialisés dans la configuration globale. L’éditeur n’expose
pas encore toutes les options du modèle (date, online détaillé, ajout libre de
critères). Les corpus précédents restent sans écart après ces changements.
