# Awakened PoE Trade — migration Rust (aperçu)

Programme natif GTK3 + layer-shell, sans Electron ni WebView. Première configuration
vérifiée : Artix x86_64, River, PoE Steam/DWProton dans XWayland, écran 2560×1600
à facteur 1. Le code upstream reste présent et distinct dans le même dépôt.

## Lancer

Depuis ce répertoire :

```sh
cargo build --locked --release -j 3
GDK_BACKEND=wayland ./target/release/awakened-poe-trade-rs daemon
```

Le programme démarre masqué. Dans PoE, survoler un objet :

- **Ctrl+D** : copie avancée, parsing puis recherche et affichage passif.
- **Maj+Espace** : autoriser les clics après le premier rendu ; cliquer dans le
  panneau pour utiliser ses contrôles. Le compositeur garde la décision de focus.
- **Ctrl+Maj+D** : fermer tant que le clavier est encore dans PoE.
- **Échap** ou **Fermer** : fermer lorsque le panneau possède le clavier.

La ligue est visible et éditable ; sélectionner **Covens** pour le personnage
testé. Défaut d'une installation neuve : Standard. Les premières offres comparées
par base ne suffisent pas à estimer un rare : choisir ses mods et leurs valeurs
minimum avant **Rechercher**. Les mods complexes ne sont pas encore portés.

Les raccourcis utilisent des grabs X11, vérifiés avec le jeu XWayland de cette
machine. Ils ne constituent pas des raccourcis globaux Wayland. Aucune règle River
ni configuration système n'est modifiée. Les événements destinés aux autres
applications X11 sont rejoués ; un conflit de raccourci désactive ce backend avec
un message dans le journal. Les applications Wayland natives nécessitent un autre
backend de raccourcis. Le portail GlobalShortcuts est absent ici.

Commandes utiles depuis un outil qui garde PoE actif :

```sh
./target/release/awakened-poe-trade-rs status
./target/release/awakened-poe-trade-rs league Covens
./target/release/awakened-poe-trade-rs check  # après Ctrl+Alt+C manuel
./target/release/awakened-poe-trade-rs hide
./target/release/awakened-poe-trade-rs quit
```

Réglages : `$XDG_CONFIG_HOME/awakened-poe-trade-rs/config.json`, sinon
`~/.config/awakened-poe-trade-rs/config.json`. Le champ `scale` (0,75–2,5) s'applique
au prochain lancement. Les filtres de mods cochés concernent l'objet courant.
Une copie utilise un marqueur pour rejeter l'ancien objet et contourner le cas
Proton 10+ ; elle ne restaure pas le presse-papiers après une copie réussie.

## Paquet Artix / Arch

Avec les dépendances de développement GTK3, gtk-layer-shell et Rust installées :

```sh
cargo fetch --locked
sh scripts/package.sh
```

Le paquet et son archive source sont dans `dist/`, avec les sommes SHA256. Le
script ne les installe pas. Installation explicite :

```sh
sudo pacman -U dist/awakened-poe-trade-rs-0.1.0-1-x86_64.pkg.tar.zst
```

L'entrée du menu lance le service masqué et écrit les journaux dans
`~/.local/state/awakened-poe-trade-rs/overlay.log` (ou `$XDG_STATE_HOME`). Depuis le
terminal, les journaux vont sur stderr. `status` fournit l'état réel par socket
local. Aucun rendu périodique n'est demandé lorsque l'interface est inactive.

## Vérifier et limites

```sh
cargo test --locked
python3 scripts/smoke-overlay.py target/release/awakened-poe-trade-rs
```

Le second test nécessite le service lancé et PoE actif ; il ouvre et ferme cinq
panneaux passifs, sans envoyer de commande de jeu. Les exemples de contrôle du
pointeur et des fenêtres sont des outils de validation manuelle.

L'overlay exige layer-shell v4 et foreign-toplevel. La détection actuelle cible
précisément `steam_app_238960` / `Path of Exile`. Le parsing prend en charge le texte
anglais. La recherche est limitée aux premières 10 offres avec prix, cache de
60 s, pas de retry automatique. Les réponses 429 et les limites serveur sont
respectées. Une requête en vol a un timeout ; sa réponse est invalidée à la
fermeture ou au changement d'objet, sans réouverture ni activation.

Ce n'est pas encore un remplacement complet d'Awakened PoE Trade. Inventaire et
travail restant : [PORTING.md](docs/PORTING.md). Preuves, observations et tests
manuels restants : [VALIDATION.md](docs/VALIDATION.md). X11 natif, Windows, autres
compositeurs, autres langues et autres facteurs d'écran ne sont pas validés.

MIT, copyright Alexander Drozdov ; licence et attributions upstream conservées
dans `../LICENSE` et `../README.md`. Catalogue et traductions de stats réutilisés
depuis upstream ; pas de service web embarqué dans l'interface Rust.
