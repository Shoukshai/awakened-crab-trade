# Awakened PoE Trade — migration Rust (aperçu)

Programme natif GTK3 + layer-shell, sans Electron ni WebView. Première configuration
vérifiée : Artix x86_64, River, PoE Steam/DWProton dans XWayland, écran 2560×1600
à facteur 1. [Démarrage rapide](docs/QUICKSTART.md). Le code upstream reste présent et distinct dans le même dépôt.

## Lancer

Depuis ce répertoire :

```sh
cargo build --locked --release -j 3
GDK_BACKEND=wayland ./target/release/awakened-poe-trade-rs daemon
```

Le programme démarre masqué. Dans PoE, survoler un objet. Raccourcis par défaut :

- **Ctrl+D** : copie avancée, parsing puis recherche et affichage passif.
- **Ctrl+Alt+D** : copie avancée et évaluation locale de l’objet ou de la carte.
- **Maj+Espace** : autoriser les clics après le premier rendu ; cliquer dans le
  panneau pour utiliser ses contrôles. Le compositeur garde la décision de focus.
  Si le panneau est masqué, ouvrir le menu des widgets.
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
Le volet **Filtres d’objet** reprend la politique upstream pour la base/catégorie,
corruption, rareté, influences, fracture, miroir, split, niveaux, qualité, liens et
stock. Les valeurs concernent la copie courante et se réinitialisent à la suivante ;
modifier un filtre invalide les offres sans lancer HTTP. Les plages inversées sont
refusées localement. Les choix Marchands et regroupement API sont mémorisés ; les
bornes d’objet ne deviennent pas des réglages globaux. Les anciens minima JSON
restent reconnus pour les profils existants. Les options Trade sont désactivées
pour les devises sans mods en échange en lot ; utiliser stock et paiement.
Les seuils de gemmes, variantes uniques reconnues, objets voilés et contraintes de
zone suivent également cette politique. Cela ne prouve pas la parité du parseur
entier. L’éditeur reste partiel : pas d’ajout libre de critère, filtre de date ou
sélection de toutes les options d’online/groupement upstream.
Les presets **Objet fini** et **Base d’artisanat** suivent maintenant les critères
upstream : niveau, fracture, synthèse, influences, craft, qualité et corruption.
La base conserve ses critères d’artisanat ; l’objet fini utilise les propriétés et
pseudos. Les constantes des uniques, implicites de base, mods voilés et variantes
ont leurs règles de visibilité/sélection. Les sources consommées restent proposées
sans sélection dans le volet avancé.

Les clusters appliquent les plages de nombre de passifs. Les options Timeless
cochées forment un groupe « au moins une » ; on peut ajuster les bornes de seed.
Le volet avancé propose un préfixe/suffixe libre ou crafté lorsque les mods copiés
le justifient. Les anointments affichent leurs huiles et suivent les règles de
coût/talisman/corruption upstream. Modifier un mod invalide les anciennes offres
sans démarrer de recherche. Les presets de logbooks à plusieurs zones, Atzoatl, Mirrored Tablet et
Mercenary Warrant sont décrits plus bas avec leurs limites réseau.

Les objets possédant un identifiant d'échange upstream utilisent la recherche
en lot lorsqu'aucun filtre de mod n'est sélectionné : devise de paiement,
stock minimum, prix unitaire et quantités de chaque offre. Les autres objets
utilisent la recherche Trade normale. Le prix indicatif poe.ninja apparaît
séparément ; il ne valorise pas les mods d’un rare. Son cache mémoire/disque dure
31 minutes. L’identité et les variantes suivent les règles upstream : gemmes,
bases et influences, uniques et liens/rolls spécialisés. Le panneau distingue les
prix absents d’un prix nul, convertit chaos/divine selon le feed et affiche la
valeur de la pile copiée ainsi que celle d’une pile complète. Le volet **Objets
liés et prix indicatifs** utilise le même instantané, sans requête supplémentaire.
Le lien source vise la fiche exacte. Modifier la ligue efface immédiatement
l’ancienne cote ; cliquer **Rechercher** pour charger la suivante.

La tendance et sa dispersion suivent le calcul upstream ; la dispersion n’est
pas un taux de variation du prix. Les trous du graphe restent des données absentes.
Foulborn, Vestigial et foil peuvent afficher la référence ordinaire avec un
avertissement : ce prix ne valorise pas le trait. Les règles historiques de Vinktar
ne distinguent pas les descriptions actuelles du flacon ; une cote générique est
signalée. Filled Coffin manque dans le catalogue anglais actuel : sa copie
complète n’est pas validée. Images des objets liés et prédiction poeprices ne sont
pas portées.

Un unique non identifié propose les identités de sa base dans une liste native.
Passer en interaction avec **Maj+Espace**, puis choisir l’identité à consulter.
Une seule correspondance est sélectionnée automatiquement ; aucune correspondance
bloque la recherche. Ce choix conserve le statut non identifié, le texte copié,
les propriétés et les mods visibles. Il ne révèle aucun mod caché et n’effectue
aucune identification dans le jeu. La recherche Trade conserve le critère
**non identifié** ; la cote poe.ninja décrit seulement l’identité choisie.
Une nouvelle copie réinitialise le choix. Les variantes d’un même unique restent
regroupées selon le premier résultat du catalogue upstream ; pas d’icônes pour
cette liste. Le choix fonctionne aussi avant l’évaluation locale de poussière.

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
Les images de cartes et les autres fonctions restantes sont inventoriées dans PORTING.md.

Le bouton **Cibles Heist** ouvre la recherche native du catalogue upstream :
répliques d’uniques et anciennes gemmes à qualité alternative. Tous les mots
doivent correspondre, avec au moins trois caractères et au plus cinq résultats.
Le catalogue actuel contient 94 répliques et aucune de ces anciennes gemmes ; les
gemmes transfigurées et les bases ne sont pas incluses dans ce widget upstream.
La saisie reste locale. Sélectionner charge les cotes poe.ninja, trie jusqu’à
cinq objets et ignore les doublons ; une sixième sélection remplace la liste.
**Actualiser les cotes** réessaie explicitement en cas d’erreur.

Cliquer une sélection consulte les prix Trade d’un **objet virtuel du catalogue**,
sans remplacer le presse-papiers ni supposer des mods, sockets ou niveau d’objet.
La base connue est ajoutée à l’identité : le parcours virtuel upstream l’omet et
sa requête est refusée par Trade. Ouvrir le widget ne lance pas HTTP ; fermer ou
changer de ligue révoque les réponses en attente. Les images et l’OCR ne sont pas
portés. Un sixième raccourci facultatif se configure dans **Réglages**.

**Grille Delve** affiche une surface transparente indépendante, sans capter les
clics ni le clavier. Elle reprend les 11 colonnes, neuf lignes, proportions et
zones coupées du widget upstream. Sa taille suit la sortie logique, sans utiliser
l’échelle du panneau de prix. Le bouton ferme le panneau interactif pour afficher
la grille ; `delve` la bascule et `delve-hide` masque seulement la grille. Le
raccourci facultatif se configure dans Réglages → Raccourcis, avec défilement.

Les panneaux de prix, d’évaluation et de réglages sont exclusifs : ils masquent
la grille tout en conservant sa demande d’affichage tant qu’ils restent passifs.
La recherche Heist est non exclusive et peut coexister avec elle. Autoriser
l’interaction annule son affichage, comme le drapeau upstream `hide-on-focus`. Changer
vers une autre application ou perdre le jeu annule aussi son affichage ; revenir
au jeu ne la restaure pas. `hide` et `quit` masquent les deux surfaces. Le widget
ne lance aucun réseau et ne programme pas de rendu périodique. L’alignement avec
l’interface Delve du jeu reste à vérifier ; les proportions, la traversée des
clics et des changements de sortie sont testés séparément.

**Chronos** ouvre le gestionnaire natif de chronomètres ; `stopwatches` l’ouvre
également en mode passif. Ajouter une instance, puis **Maj+Espace** autorise les
contrôles de ses widgets. Chaque instance a son propre démarrage/pause et sa
remise à zéro. En marche, la remise à zéro affiche une seconde, conformément au
widget upstream ; en pause, elle revient à zéro. Les durées ne sont pas sauvegardées
entre deux lancements.

Les widgets en marche et ceux arrêtés après au moins une seconde restent visibles
passivement dans PoE. Les widgets à zéro ou arrêtés avant une seconde apparaissent
pendant l’interaction. Survoler un widget interactif expose ses commandes ;
**Déplacer** active sa poignée centrale. **Supprimer** demande un maintien d’une
seconde, annulé au relâchement ou à la sortie. **Réglages → Chronomètres** permet
de changer les noms, positions et deux raccourcis par instance, ou de retirer une
instance au brouillon. Enregistrer applique ces champs et les raccourcis ensemble ;
un conflit ou un refus d’écriture conserve l’ancien profil. Limite native : 32
instances. La disposition des commandes est adaptée à GTK. Cliquer une partie
visible d’un chronomètre ou d’une bande d’images la place devant les autres ; cet
ordre est mémorisé entre deux lancements.

Les chronomètres et les bandes d’images partagent une seule surface transparente.
Les chronomètres gardent une seule échéance
de mise à jour. Aucun réveil périodique n’est programmé lorsque tous les widgets
et le gestionnaire sont masqués ; les durées en marche continuent par calcul à la
réouverture. Une autre application masque la surface et révoque ses entrées.
Revenir dans PoE peut restaurer les chronomètres persistants, passivement ; cela
ne rouvre pas le panneau de prix. `hide` masque toutes les surfaces jusqu’à une
nouvelle action explicite. Les vues prix, évaluation et réglages masquent les
chronomètres pendant leur affichage.

**Images** ouvre le gestionnaire de bandes d’images (`images` en ligne de commande).
Appuyer sur **Maj+Espace** pour autoriser les clics, puis activer une bande avec **Afficher**.
Les bandes apparaissent uniquement pendant l’interaction. Cliquer une vignette
l’agrandit dans la sortie ; quitter la vignette ferme l’aperçu. **Déplacer** expose
six poignées d’ancrage, en haut et en bas. **Supprimer** demande un maintien d’une
seconde, annulé au relâchement ou à la sortie. Une autre application masque bandes,
aperçu et entrées ; le retour au jeu ne les réactive pas. Une action explicite
**Maj+Espace** ouvre le menu et restaure les bandes demandées après son premier
rendu. Les vues Prix, Évaluation et Réglages les masquent.

**Réglages → Images** permet d’ajouter des bandes, changer leur nom, position et
ancrage, importer des fichiers locaux et réordonner les images avec les flèches.
Les modifications du profil restent au brouillon jusqu’à **Enregistrer**.
Les fichiers sont copiés dans `$XDG_DATA_HOME/awakened-poe-trade-rs/images`, sinon
`~/.local/share/awakened-poe-trade-rs/images`. Annuler conserve les copies déjà
importées, sans modifier le profil ni les fichiers d’origine. Limites natives :
16 bandes, 32 images par bande, 16 Mo et 48 millions de pixels par fichier ; le
décodage d’affichage est borné à 4096 pixels par dimension. Les GIF s’animent
seulement lorsqu’ils sont visibles. Les URL distantes ne sont pas prises en charge.
La feuille Syndicate fournie est l’image upstream inchangée, marquée **version
3.24**, avec son crédit à elrincondelexiliado.com/syndicate ; elle ne constitue pas
une référence des mécaniques de la ligue actuelle. L’ordre de superposition
entre chronomètres et images est mémorisé ; la disposition exacte des commandes
upstream reste à porter.

**Menu** : lorsque le panneau est masqué, **Maj+Espace** ouvre le menu flottant
des widgets. Les boutons proposent les réglages, les cibles Heist et les instances
masquées ; **Configurer → Toujours afficher** conserve aussi les boutons des
instances déjà affichées. Cliquer une instance l’affiche et la place devant les
autres. Le bouton **+** ajoute un chronomètre ou une bande d’images ; la recherche
coffre crée un panneau de filtres. Le champ de collage accepte le
texte d’un objet et prépare ses prix selon le réglage de recherche automatique,
sans envoyer de raccourci de copie au jeu. Limite du collage : 1 Mio.

Au survol, **Déplacer** expose les quatre ancrages de coin. Position, ancrage et
option d’affichage sont mémorisés ; les valeurs par défaut sont 5 % depuis le
coin supérieur gauche. Le menu partage la surface et l’ordre des chronomètres
images et recherches coffre. Son contenu défile si nécessaire. Les icônes et la présentation sont
adaptées à GTK. Le menu peut coexister avec les cibles Heist ; les gestionnaires
Images/Chronos et les vues Prix/Évaluation/Réglages le masquent. Un changement
d’application le ferme, et le retour au jeu ne le rouvre pas. **Échap**, **Fermer**
ou `hide` ferment les surfaces selon le contexte d’interaction.

**Recherche coffre** : ouvrir le coffre dans PoE, puis **Maj+Espace** et le
bouton du widget souhaité. Les deux listes par défaut reprennent Map rolling et
Dump sorting upstream. Cliquer un filtre masque l’overlay, attend le focus du jeu,
ouvre sa recherche avec Ctrl+F, colle le texte puis valide avec Entrée. Le backend
vérifie le contexte et la même fenêtre XWayland avant chaque combinaison ; il ne
force pas son activation. Un changement d’application annule l’action en attente.
Ce parcours d’application des filtres est validé sur le faux jeu privé ; sur le
PoE réel, seuls l’affichage et les contrôles du widget ont été vérifiés.

**+ → Recherche coffre** ajoute une instance. Le bouton **Configurer** au survol
ouvre **Réglages → Coffre** : titre, coordonnées, quatre ancrages, affichage,
filtres, noms facultatifs, réordonnement et raccourcis par filtre. Les changements
restent au brouillon jusqu’à **Enregistrer**. Le commutateur **Raccourcis** du
widget active ou libère ses combinaisons ; elles peuvent agir lorsque le panneau
est masqué. Un conflit conserve l’ancien profil. **Déplacer**, **Masquer** et la
suppression au maintien d’une seconde suivent les autres widgets. Limites natives :
16 instances, 64 filtres par instance et 250 unités UTF-16 par filtre sur une ligne.
Le bouton **Trouver dans le coffre** de l’inspecteur utilise le nom du catalogue
entre guillemets, selon la règle upstream.

Le huitième champ de **Réglages → Raccourcis**, **Objet similaire dans le
coffre**, est désactivé par défaut. Une combinaison configurée copie l’objet
survolé, puis cherche son nom du catalogue entre guillemets dans le coffre déjà
ouvert. Elle suit les mêmes contrôles de contexte et de restauration du texte.

**Général → Défiler les onglets du coffre avec Ctrl + molette** est activé par
défaut, comme upstream. Hors de la zone du coffre, Ctrl+molette envoie une flèche
gauche ou droite au même jeu XWayland, en conservant les modificateurs encore
maintenus. La molette d’origine continue d’arriver au jeu. L’observateur XRecord
n’enregistre aucune touche. Si cette extension manque, le réglage est désactivé
avec son diagnostic ; les raccourcis clavier restent indépendants. Le backend
refuse une fenêtre ou une géométrie devenue indisponible. Le défilement et la
recherche similaire sont validés sur le faux jeu privé ; leur effet dans le vrai
coffre reste à vérifier manuellement.

L’option **Général → Restaurer le texte du presse-papiers après une action** est
désactivée par défaut. Elle restaure le texte précédent après une copie d’objet,
ou après le délai de 120 ms d’une recherche coffre, s’il est disponible et si la
sélection contient encore le texte de l’action. Les actions de modification du
presse-papiers sont sérialisées ; une action concurrente est refusée. Les formats
riches et images ne sont pas sauvegardés. Une lecture de texte existant par `check`
ou un collage dans le menu ne déclenche pas cette restauration.

Les raccourcis utilisent des grabs X11, vérifiés avec le jeu XWayland de cette
machine. Ils ne constituent pas des raccourcis globaux Wayland. Aucune règle River
ni configuration système n'est modifiée. Les événements destinés aux autres
applications X11 sont rejoués ; un conflit au démarrage est signalé dans le journal et dans Réglages. Une
modification refusée conserve les anciens réglages et raccourcis. Les applications Wayland natives nécessitent un autre
backend de raccourcis. Le portail GlobalShortcuts est absent ici.

Commandes utiles depuis un outil qui garde PoE actif :

```sh
./target/release/awakened-poe-trade-rs status
./target/release/awakened-poe-trade-rs menu # menu passif, sans activation
./target/release/awakened-poe-trade-rs stopwatches
./target/release/awakened-poe-trade-rs league Covens
./target/release/awakened-poe-trade-rs check  # après Ctrl+Alt+C manuel
./target/release/awakened-poe-trade-rs catalogue # recherche des cibles Heist, passive
./target/release/awakened-poe-trade-rs catalogue replica "Replica Headhunter" # préremplir, sans HTTP
./target/release/awakened-poe-trade-rs settings  # ouverture passive des réglages
./target/release/awakened-poe-trade-rs interact  # autoriser les clics explicitement
./target/release/awakened-poe-trade-rs delve # basculer la grille passive
./target/release/awakened-poe-trade-rs delve-hide
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
`~/.config/awakened-poe-trade-rs/config.json`. Le bouton **Réglages** ouvre un brouillon natif :
ligue, échelle (0,75–2,5), plage, devises, recherche automatique, stock copié,
marchands, regroupement, profil de carte, huit raccourcis globaux, chronomètres, images et recherches coffre. **Annuler** ne
persiste rien. **Enregistrer** applique l’échelle et les raccourcis sans relancer
l’application, puis masque le panneau ; cela ne lance aucune recherche.
Désactiver la recherche automatique permet de vérifier les filtres après Ctrl+D
avant **Rechercher**. La devise automatique suit les règles de l’objet upstream.

Un champ de raccourci vide le désactive ; saisir une combinaison ou utiliser
**Capturer**. Les doublons, combinaisons réservées à la copie et touches
indisponibles sont refusés. Un échec d’écriture ou de réservation conserve les
anciens réglages et raccourcis. **Réinitialiser ces champs** remplit le brouillon
avec les valeurs par défaut et conserve les règles avancées de mods. L’éditeur
ne couvre pas encore toutes les préférences upstream. Les filtres de mods
cochés concernent l’objet courant.
Une copie utilise un marqueur pour rejeter l'ancien objet et contourner le cas
Proton 10+ ; la restauration du texte précédent dépend de l’option Général décrite plus haut.

## Paquet Artix / Arch

Avec les dépendances de développement GTK3, gtk-layer-shell et Rust installées :

```sh
cargo fetch --locked
sh scripts/package.sh
```

Le paquet et son archive source sont dans `dist/`, avec les sommes SHA256. Le
script ne les installe pas. Installation explicite :

```sh
sudo pacman -U dist/awakened-poe-trade-rs-0.22.0-1-x86_64.pkg.tar.zst
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
cargo build --locked --example item-filters
node scripts/compare-item-filters.cjs
cargo build --locked --example equipment-presets
node scripts/export-mod-mechanics.cjs --check
node scripts/compare-equipment-presets.cjs
node scripts/export-special-rules.cjs --check
node scripts/compare-special-presets.cjs
cargo build --locked --example mercenary
node scripts/compare-mercenary.cjs
cargo build --locked --example economy-identity
node scripts/compare-economy.cjs
cargo build --locked --example unidentified
node scripts/compare-unidentified.cjs
node scripts/compare-hotkeys.cjs target/debug/examples/hotkeys docs/hotkeys-0.14.json
node scripts/compare-item-search.cjs target/debug/examples/item-search docs/item-search-0.15.json
python3 scripts/test-item-search.py target/release/awakened-poe-trade-rs docs/item-search-lifecycle-0.15.json
node scripts/compare-delve.cjs target/debug/examples/delve-grid docs/delve-parity.json
python3 scripts/test-delve.py target/release/awakened-poe-trade-rs docs/delve-lifecycle-0.17.json
sh scripts/test-shortcut-replay.sh
python3 scripts/test-settings.py target/release/awakened-poe-trade-rs docs/settings-0.14.json
python3 scripts/test-overlay-lifecycle.py target/release/awakened-poe-trade-rs
python3 scripts/smoke-overlay.py target/release/awakened-poe-trade-rs
```

La comparaison TypeScript est un test de développement hors ligne : elle exige
Node, les en-têtes X11 xorgproto pour les noms de touches, et les dépendances upstream (`npm ci` dans `../renderer`, voir DEVELOPING.md).
Elle n'est pas requise pour construire ou utiliser l'application Rust.
Le test de raccourcis exige River et xdotool, démarre son propre serveur sans écran
et ne transmet ses touches qu'à sa fenêtre de test. Il vérifie aussi la libération
des grabs partiels après conflit. Le test de cycle de vie exige aussi wl-copy,
wlr-randr et wtype ; il utilise un bureau River privé, un faux jeu et un proxy
bloquant les services externes. Il ne touche pas au PoE réel. Le test Delve exige aussi grim et Pillow pour lire les pixels du bureau privé.
Le test `smoke-overlay.py` nécessite le service
lancé et PoE actif ; il ouvre et ferme cinq
panneaux passifs, sans envoyer de commande de jeu. Les exemples de contrôle du
pointeur et des fenêtres sont des outils de validation manuelle.

L'overlay exige layer-shell v4 et foreign-toplevel. La détection actuelle cible
précisément `steam_app_238960` / `Path of Exile`. Le parsing prend en charge le texte
anglais. La recherche est limitée aux premières 10 offres Trade ou 20 offres
en lot, cache de 60 s, pas de retry automatique. Les réponses 429 et les limites serveur sont
respectées. Une requête en vol a un timeout ; sa réponse est invalidée à la
fermeture ou au changement d'objet, sans réouverture ni activation.

Les logbooks proposent une zone I, II ou III : choisir une autre zone efface
les offres précédentes sans lancer de recherche. Les salles d’Atzoatl tiennent
compte des explosifs accessibles. Le Tablet utilise la difficulté maximale des
reflets répétés ; son parcours réseau reste limité : les filtres lake ont été
rejetés par le serveur Covens malgré leur présence dans son catalogue public.
Le panneau affiche l’erreur serveur et ne retire pas ces critères automatiquement.

Les Mercenary Warrants disposent d’un éditeur de compétences, supports requis ou
N−1, six liens et exclusions. Décocher un groupe suspend ses critères enfants en
conservant leurs choix pour sa réactivation. Les compétences principales restent
recherchées. Une requête très complexe peut dépasser la limite du Trade anonyme :
réduire ses critères, ou utiliser **Ouvrir Trade** pour conserver la requête exacte
dans le navigateur. Aucun compte n’est connecté par l’application native.

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

Validation des chronomètres et du bureau isolé :

```sh
cargo build --locked --examples
node scripts/compare-stopwatch.cjs
node scripts/compare-delve.cjs
python3 scripts/test-stopwatch.py
node scripts/compare-image-strips.cjs target/debug/examples/image-strip docs/image-strips-parity-0.18.json
python3 scripts/test-image-strips.py target/release/awakened-poe-trade-rs docs/image-strips-lifecycle-0.18.json
```

Les deux comparaisons exigent les dépendances Node upstream dans `renderer/`.
Elles ne sont pas utilisées à l’exécution du programme natif. Le test graphique
crée son propre River headless et son faux jeu, sans toucher au PoE réel.

Validation de l’ordre des widgets :

```sh
cargo build --locked --example widget-order
node scripts/compare-widget-order.cjs target/debug/examples/widget-order docs/widget-order-parity-0.21.json
python3 scripts/test-widget-order.py target/release/awakened-poe-trade-rs docs/widget-order-lifecycle-0.21.json
```

Validation du menu sur le bureau privé :

```sh
cargo build --locked --examples
node scripts/compare-widget-menu.cjs target/debug/examples/widget-menu docs/widget-menu-parity-0.21.json
python3 scripts/test-widget-menu.py target/release/awakened-poe-trade-rs docs/widget-menu-lifecycle-0.21.json
```

Ce scénario exige aussi `xkbcli`, grim et Pillow. Les outils de clavier virtuel
et de presse-papiers retardé refusent le bureau réel : ils vérifient le runtime
privé et son nom de socket. Ils ne font pas partie du backend de production.

Validation des recherches coffre :

```sh
cargo build --locked --examples
node scripts/compare-stash-search.cjs target/debug/examples/stash-search docs/stash-search-parity-0.21.json
python3 scripts/test-stash-search.py target/release/awakened-poe-trade-rs docs/stash-search-lifecycle-0.21.json
```

Le scénario crée une fenêtre GTK X11 de test dans son River privé. Les touches
Ctrl+F, Ctrl+V et Entrée ne sont envoyées qu’à ce faux jeu. Il vérifie aussi la
restauration du texte après une copie avancée et les annulations tardives.

Tests du jalon 0.22, sur des bureaux River privés :

```sh
cargo build --locked --examples -j 3
node scripts/compare-stash-scroll.cjs target/debug/examples/stash-scroll docs/stash-scroll-parity-0.22.json
python3 scripts/test-stash-shortcuts.py target/release/awakened-poe-trade-rs docs/stash-shortcuts-lifecycle-0.22.json
APT_TEST_NO_RECORD=1 python3 scripts/test-stash-shortcuts.py target/release/awakened-poe-trade-rs docs/wheel-unavailable-0.22.json
```
