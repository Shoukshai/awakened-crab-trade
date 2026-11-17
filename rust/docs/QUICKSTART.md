# Démarrage rapide — aperçu Linux 0.22

Configuration vérifiée : Artix x86_64, River avec layer-shell, PoE Steam/DWProton
sous XWayland, un écran 2560×1600 au facteur 1. Windows et X11 natif non validés.

Depuis le répertoire `rust`, construire et lancer :

```sh
cargo build --locked --release -j 3
GDK_BACKEND=wayland ./target/release/awakened-poe-trade-rs daemon
```

Le service démarre masqué. Dans PoE, survoler un objet puis :

- **Ctrl+D** : copier et consulter ses prix.
- **Ctrl+Alt+D** : inspection locale, dont DPS et mods dangereux des cartes.
- **Maj+Espace** : autoriser les clics après affichage ; si tout est masqué,
  ouvrir le menu des widgets.
- **Ctrl+Maj+D** : masquer depuis le jeu ; **Échap** ferme depuis le panneau.

Dans Réglages, choisir la ligue du personnage : **Covens** pour la session testée,
**Standard** par défaut. Vérifier les mods et leurs bornes avant d’estimer un rare.
**Annuler** conserve le profil ; **Enregistrer** applique sans redémarrer.

Alternative après une copie manuelle Ctrl+Alt+C :

```sh
./target/release/awakened-poe-trade-rs check
./target/release/awakened-poe-trade-rs hide
./target/release/awakened-poe-trade-rs quit
```

`sh scripts/package.sh` construit le paquet Artix/Arch dans `dist/` sans l’installer.
Le lanceur `awakened-poe-trade-rs-launch` appartient au paquet. Les réglages sont
conservés dans `~/.config/awakened-poe-trade-rs/config.json`.

Limites : migration progressive ; chat, tray, mises à jour et préférences
upstream restantes à porter. Ouvrir le coffre avant d’appliquer un filtre ou une
recherche similaire. Ces actions et Ctrl+molette ont été testés sur un faux jeu
privé ; leur effet dans le vrai coffre reste à vérifier. XRecord est nécessaire
au défilement, indépendamment des raccourcis clavier. Plusieurs écrans et autres
compositeurs restent à valider. Voir README, PORTING et VALIDATION pour le détail.
