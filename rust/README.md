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
- **Ctrl+Alt+D** : copie avancée et évaluation locale de l’objet ou de la carte.
- **Maj+Espace** : autoriser les clics après le premier rendu ; cliquer dans le
  panneau pour utiliser ses contrôles. Le compositeur garde la décision de focus.
- **Ctrl+Maj+D** : fermer tant que le clavier est encore dans PoE.
- **Échap** ou **Fermer** : fermer lorsque le panneau possède le clavier.

La ligue est visible et éditable ; sélectionner **Covens** pour le personnage
testé. Défaut d'une installation neuve : Standard. Les premières offres comparées
par base ne suffisent pas à estimer un rare : choisir ses mods et leurs valeurs
minimum/maximum avant **Rechercher**. Les mods traduits utilisent désormais les
règles upstream : dégâts X à Y (moyenne), lignes multiples, flags, variantes,
types implicite/craft/fracture, effet des modificateurs et contributions cumulées.
Les cases Min/Max activent chaque borne ; un mod sans valeur numérique demande
seulement sa présence. Les variantes utilisent leur option Trade exacte.
Les groupes d'identifiants alternatifs sont une condition « au moins un », et
non plusieurs conditions obligatoires.

Les filtres calculés reprennent les règles upstream : résistances cumulées,
attributs, vie/mana apportés par les attributs, dégâts et vitesses globaux ; DPS,
vitesse d'attaque, critique, défenses et percentile de base. Les suggestions sont
cochées selon leurs règles ; vérifier les filtres actifs avant d'estimer un rare.
Les défenses et dégâts physiques utilisent la qualité normalisée du site Trade
(`max(20, qualité)` sauf enchantement supprimant ce bonus). Les sources exactes
consommées par un calcul restent dans **Autres filtres et sources exactes**.
La plage suggérée, initialement ±10 %, s'applique à la prochaine copie et reste
bornée aux valeurs possibles ; les uniques suivent leur plage de rolls.
Les presets des cartes et charts proposent **Propriétés de zone** et **Cartes en
lot** selon l’objet. Ce dernier sélectionne les critères de comparaison des cartes
sur Trade ; il ne change pas le moteur en échange de devises. Les cartes à huit
mods conservent leur compteur exact ; quantités, rareté, pack size et bonus de
butin ont leurs règles de sélection upstream. Les cartes blighted filtrent leur
zone, et les cartes à récompense excluent le Void si le mod est absent de la copie.

Les flacons non uniques et les objets Heist ont un preset exact, avec une plage
plafonnée à 2 %. Il reprend les tiers de mods, l’exclusion du préfixe hybride des
flacons, les métiers, cibles Priceless et ailes révélées. Les contrats se comparent
par catégorie ; le panneau indique la portée de la recherche. Changer de preset
recrée les filtres et invalide la réponse en cours ; cliquer ensuite **Rechercher**.
La plage suggérée s’applique à la prochaine copie ou sélection de preset.
Les liens entre corruption, rareté, split/fracture et les autres options d’objet
ne sont pas encore tous portés. Les presets d’équipement/base d’artisanat, bijoux
de cluster et les règles spécialisées restantes restent à porter.

Les objets possédant un identifiant d'échange upstream utilisent la recherche
en lot lorsqu'aucun filtre de mod n'est sélectionné : devise de paiement,
stock minimum, prix unitaire et quantités de chaque offre. Les autres objets
utilisent la recherche Trade normale. Le prix indicatif poe.ninja apparaît
séparément, avec sa tendance et le total de la pile ; il ne valorise pas les mods
d'un rare. Son cache mémoire/disque dure 31 minutes. Les variantes complexes de
certains uniques ne sont pas encore prises en charge et restent sans estimation.

Le bouton **Évaluer** passe aux informations locales du dernier objet. Pour les
cartes, contrats, blueprints, invitations, charts et logbooks, trois profils
mémorisent les décisions danger/prudence/favorable/vu. Cliquer un mod fait avancer
sa décision ; **Signaler les nouveaux mods** permet de marquer les mods déjà vus.
Le catalogue **Configurer les mods de zone** propose 340 traductions, une recherche
par mots et un filtre des règles colorées. Effacer une règle conserve les deux
autres profils. Les décisions décrivent votre build, pas un verdict universel.
Les lignes inconnues restent visibles, même si aucune ligne n’est reconnue.
Deux anciennes règles de réflexion des réglages upstream sont actuellement
obsolètes : le panneau les signale et permet de les effacer pour chaque profil.

Pour les autres objets, le DPS utilise la qualité réellement copiée, distincte
de la qualité normalisée des filtres Trade. La poussière de désenchantement des
uniques utilise les données et facteurs upstream. Wiki, PoEDB et import Craft of
Exile s’ouvrent sur clic explicite ; le panneau se ferme avant le navigateur.
L’import CoE est désactivé pour les objets corrompus ou mirrored. Ce mode ne
consulte ni Trade ni poe.ninja. **Consulter les prix** revient à la recherche.
Les images de cartes, la recherche coffre et les widgets restent à porter.

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
./target/release/awakened-poe-trade-rs --version
```

Diagnostics sans interface ni entrée envoyée au jeu :

```sh
./target/release/awakened-poe-trade-rs inspect src/fixtures/driftwood-wand.txt
./target/release/awakened-poe-trade-rs analyze src/fixtures/synthetic-map.txt
./target/release/awakened-poe-trade-rs lookup src/fixtures/scroll-of-wisdom.txt Covens chaos
./target/release/awakened-poe-trade-rs economy src/fixtures/scroll-of-wisdom.txt Covens
```

`lookup` et `economy` consultent les services publics et leurs caches ; `inspect`
et `analyze` restent locaux. Les journaux séparent traitement, réseau et cache.

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
sudo pacman -U dist/awakened-poe-trade-rs-0.6.0-1-x86_64.pkg.tar.zst
```

L'entrée du menu lance le service masqué et écrit les journaux dans
`~/.local/state/awakened-poe-trade-rs/overlay.log` (ou `$XDG_STATE_HOME`). Depuis le
terminal, les journaux vont sur stderr. `status` fournit l'état réel par socket
local. Aucun rendu périodique n'est demandé lorsque l'interface est inactive.

## Vérifier et limites

```sh
cargo test --locked
cargo clippy --locked --all-targets -- -D warnings
cargo build --locked --example stat-translation
node scripts/compare-stat-translations.cjs
cargo build --locked --example calculated-filters
node scripts/export-pseudo-rules.cjs --check
node scripts/compare-calculated-filters.cjs
cargo build --locked --example inspection
node scripts/export-map-defaults.cjs --check
node scripts/compare-inspection.cjs
cargo build --locked --example presets
node scripts/compare-presets.cjs
sh scripts/test-shortcut-replay.sh
python3 scripts/smoke-overlay.py target/release/awakened-poe-trade-rs
```

La comparaison TypeScript est un test de développement hors ligne : elle exige
Node et les dépendances upstream (`npm ci` dans `../renderer`, voir DEVELOPING.md).
Elle n'est pas requise pour construire ou utiliser l'application Rust.
Le test de raccourcis exige River et xdotool, démarre son propre serveur sans écran
et ne transmet ses touches qu'à sa fenêtre de test. Il vérifie aussi la libération
des grabs partiels après conflit. Le test `smoke-overlay.py` nécessite le service
lancé et PoE actif ; il ouvre et ferme cinq
panneaux passifs, sans envoyer de commande de jeu. Les exemples de contrôle du
pointeur et des fenêtres sont des outils de validation manuelle.

L'overlay exige layer-shell v4 et foreign-toplevel. La détection actuelle cible
précisément `steam_app_238960` / `Path of Exile`. Le parsing prend en charge le texte
anglais. La recherche est limitée aux premières 10 offres Trade ou 20 offres
en lot, cache de 60 s, pas de retry automatique. Les réponses 429 et les limites serveur sont
respectées. Une requête en vol a un timeout ; sa réponse est invalidée à la
fermeture ou au changement d'objet, sans réouverture ni activation.

Ce n'est pas encore un remplacement complet d'Awakened PoE Trade. Inventaire et
travail restant : [PORTING.md](docs/PORTING.md). Preuves, observations et tests
manuels restants : [VALIDATION.md](docs/VALIDATION.md). X11 natif, Windows, autres
compositeurs, autres langues et autres facteurs d'écran ne sont pas validés.

MIT, copyright Alexander Drozdov ; licence et attributions upstream conservées
dans `../LICENSE` et `../README.md`. Le paquet inclut les licences des dépendances
de l'hôte et la notice de la bibliothèque standard Rust. Le collecteur utilise
les sources Cargo locales et `share/doc/rust/COPYRIGHT-library.html` du sysroot ;
les compléments GTK sont fournis avec leurs références exactes dans `packaging/`.
Catalogue et traductions de stats réutilisés
depuis upstream ; pas de service web embarqué dans l'interface Rust.
