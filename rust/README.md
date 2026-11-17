# Awakened PoE Trade — price checker Rust

Overlay natif de consultation des prix : copier un objet dans PoE, choisir ses
stats et leurs bornes, rechercher les objets comparables sur Trade et lire les
offres dans le panneau. Le périmètre est limité au **price checker**.

Configuration Linux vérifiée : Artix x86_64, River avec layer-shell, PoE Steam /
DWProton sous XWayland, écran 2560×1600 au facteur 1. **Windows x64 dispose d’une version portable 0.29.1**, avec intégration
Win32 et validation dans un bureau Wine isolé. Le test réel sous Windows 11 /
PoE Steam reste à faire : voir [Windows](docs/windows.md). Aucun Electron ni WebView.

## Lancement

Depuis `rust/`, avec Rust, GTK3, gtk-layer-shell, wl-clipboard et xdotool :

```sh
cargo build --locked --release -j 3
GDK_BACKEND=wayland ./target/release/awakened-poe-trade-rs daemon
```

Le programme démarre masqué. Dans PoE, survoler un objet :

- **Ctrl+D** : copie et affichage passif du price checker.
- **Ctrl+Alt+D** : même panneau, interaction autorisée après le premier rendu.
- **Maj+Espace** : autoriser les clics dans un panneau déjà affiché.
- **Ctrl+Maj+D** : masquer depuis le jeu ; **Échap** ou **Fermer** depuis le panneau.

La ligue actuelle PC softcore est sélectionnée automatiquement depuis la liste
poe.ninja au lancement (Covens dans la session testée). Pour une autre ligue, désactiver
la sélection automatique dans Réglages. Le panneau occupe toute la hauteur de
l’écran, au bord gauche de l’inventaire. Un léger voile sombre couvre le jeu ;
cliquer en dehors du panneau le ferme sans transmettre ce clic au jeu. Pour un équipement rare, choisir
les stats pertinentes en cliquant sur leurs rectangles, activer Min et/ou Max,
saisir les bornes puis cliquer
**Rechercher**. Modifier un filtre invalide les anciennes offres sans lancer de
recherche. Les offres occupent la zone principale sous les filtres. Survoler n’importe où dans la case d’une offre
affiche rapidement son nom, sa base, son ilvl, ses mods et son prix ; les bornes de recherche
ne sont pas utilisées comme statistiques de l’offre. **Ouvrir Trade** ouvre la
requête dans le navigateur sur une action explicite.

Les puces de **base** et d’**ilvl** sont désactivées par défaut. La base porte
son nom ; ilvl active un champ de minimum prérempli avec le niveau de l’objet.
Rareté et corruption se choisissent dans leurs listes. Qualité, niveau de
gemme et liens restent disponibles selon l’objet. **Options** conserve les options complémentaires, dont les
influences et le regroupement Trade. Les presets proposent les filtres
adaptés à son type. **Autres stats** donne accès aux stats
masquées par les calculs pseudo. Une comparaison par base seule ne suffit pas à
estimer un rare. Les cotes poe.ninja sont indicatives.

La version 0.29 accepte la copie réelle des Viscera sans ligne « Rarity », dont
Siren’s Tentacle. Les 164 bases et leurs emplacements ont été comparés à PoEDB.
Les sliders règlent des seuils entiers. Depuis 0.29.1, le jaune remplit la
barre jusqu’au seuil minimum ; le reste est gris foncé. La molette sur une liste fermée fait défiler les filtres sans
changer son choix. Les modes se nomment **Recherche par stats** et
**Recherche pour craft** ; les sections secondaires affichent leur nombre de stats.

Après une copie manuelle Ctrl+Alt+C, on peut aussi lancer :

```sh
./target/release/awakened-poe-trade-rs check
./target/release/awakened-poe-trade-rs locked-check
./target/release/awakened-poe-trade-rs hide
./target/release/awakened-poe-trade-rs quit
```

Réglages : `~/.config/awakened-poe-trade-rs/config.json` ou `$XDG_CONFIG_HOME`.
Général et Raccourcis utilisent un brouillon : **Annuler** conserve le profil,
**Enregistrer** applique sans redémarrer. Les champs historiques des fonctions
annexes sont préservés mais leurs raccourcis ne sont plus installés.

## Paquet et validation

`sh scripts/package.sh` construit le paquet Artix/Arch dans `dist/` sans
l’installer. Les dépendances Cargo doivent être disponibles localement
(`cargo fetch --locked` avant une construction hors ligne).

```sh
cargo test --locked -j 3
python3 scripts/test-price-checker.py target/release/awakened-poe-trade-rs dist/price-checker-lifecycle-0.27.json
```

Le bloc supérieur affiche l’image et la cote poe.ninja, sans courbe. Les monnaies
utilisent cette cote seule ; gemmes, uniques, cartes et Viscera disposent de leurs
filtres propres. Les données et crédits sont documentés dans `docs/`.

Voir [périmètre et état](docs/PORTING.md), [démarrage rapide](docs/QUICKSTART.md)
et [validation actuelle](docs/CHECKER-VALIDATION.md). Les essais graphiques
automatisés utilisent un faux jeu dans un River privé. Les observations réelles,
les tests automatisés et les limites sont distingués dans la validation.

Le parseur anglais et l’éditeur couvrent de nombreux cas upstream, sans revendiquer
la parité totale. L’ajout libre de toute stat Trade, toutes les options avancées,
les autres compositeurs, X11 natif, plusieurs écrans et Windows restent à traiter
ou valider. Les anciens widgets et leurs rapports appartiennent à l’historique.
