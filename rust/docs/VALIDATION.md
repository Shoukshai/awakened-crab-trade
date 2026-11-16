# État de validation — 16 novembre 2026

L'objectif global n'est **pas achevé**. Ce document distingue le premier parcours
utilisable des fonctionnalités et configurations encore à porter.

## Tests automatisés

`cargo test --locked` : 22 tests réussis au jalon initial, couvrant noms/bases,
CRLF, propriétés de gemmes, sockets, rejet de texte invalide, filtres rare/unique,
tier et liens, traduction des mods simples implicites et craftés, états/focus avant
premier rendu, générations obsolètes, 429/Retry-After et fenêtres de limites,
JSON invalide, annulation avant réseau et entre recherche et fetch, cache et offres.
`cargo clippy --locked --all-targets -- -D warnings` passe également.
Ces tests ne prouvent pas la parité du parser TypeScript ni de tous les filtres.

`python3 scripts/smoke-overlay.py` avec le service et PoE actifs : cinq ouvertures
et fermetures réelles réussies, état Passive/visible/rendered confirmé par socket,
focus X11 toujours `Path of Exile`, masquage confirmé. Ce test n'envoie aucune
commande de jeu et ne remplace pas l'inspection d'une capture.

## Comportements observés sur PoE

- Panneau natif layer-shell au-dessus du jeu plein écran, capture inspectée.
  Pas de fenêtre séparée comptée comme overlay.
- Copie manuelle fournie par l'utilisateur : Driftwood Wand normale, ilvl 1,
  sockets W-W, mod implicite `8(8-12)% increased Spell Damage` en description avancée.
- Ligue donnée par l'utilisateur « Coven », identifiant API vérifié `Covens`.
- Premier vrai search/fetch : 362 résultats, 10 offres affichées, 624 ms réseau.
  Une nouvelle recherche a renvoyé 357 puis 361/359 résultats (marché vivant).
  Capture des 10 offres inspectée, focus resté au jeu.
- L'API de cette date renvoie des identifiants de requête base64url contenant
  `-`/`_`. La première validation trop restrictive a été corrigée et couverte.
- Copie automatique explicite de l'objet survolé : 51 ms avant affichage, puis
  prix dans l'overlay. Aucun Enter, message de chat, déplacement ou clic sur un
  contrôle de jeu n'a été envoyé pour cette vérification.
- Ctrl+D enregistré par XGrabKey sur XWayland : vraie copie avancée et prix
  affichés ; 166 ms capture/parsing à froid avec l'index de mods, 623 ms réseau.
- Maj+Espace passe en Interactive sans demander de focus exclusif.
- Clic réel sur Fermer avec le pointeur virtuel Wayland, après vérification
  visuelle de la position dans le bouton : panneau masqué ; focus revenu à PoE.
- Binaire extrait du paquet Artix : cinq cycles graphiques supplémentaires, copie
  Ctrl+D, recherche et 10 offres visibles (356 résultats, 597 ms réseau). Échec
  de copie lorsque le pointeur n'était plus sur un objet : message d'erreur
  passif, aucun ancien objet accepté.
- Cache réel vérifié : réouverture du même objet, `cached=true`, réseau 0 ms.
- Mod implicite coché dans le panneau, minimum saisi à 12, clic Rechercher :
  69 résultats au lieu de 356, 565 ms réseau ; capture inspectée. Échap depuis
  l'interface focalisée masque le panneau et rend le focus au jeu.
- Ctrl+Maj+D ferme aussi immédiatement après Maj+Espace : la fermeture n'est pas
  soumise au throttling de copie.
- Changement d'application pendant une vraie recherche : génération 16 rejetée
  après le masquage ; retour au jeu sans réouverture. Ce cas est observé en plus
  du test automatisé de génération.
- Échelle d'application 1,25 dans un profil temporaire : panneau 775×525,
  capture inspectée, PoE garde le focus. Profil normal conservé à 1. Le facteur
  de sortie Wayland reste 1 : ce test ne valide pas un écran HiDPI.
- Fenêtre du jeu réapparue avec un nouveau PID pendant la séance : suivi natif
  retrouvé sans redémarrer le prototype. Aucun arrêt ni relancement de PoE par
  l'agent. Le service et le jeu sont des processus distincts.

Captures et journaux de ces essais restent locaux dans `/tmp/apt-*` et
`/tmp/poe-*` ; ils ne sont pas inclus dans le paquet ni dans Git. Ils sont
temporaires et peuvent disparaître au redémarrage.

## Symptôme upstream

Upstream `3188208957422b21b69b12083a03edf3d1bac187` compilé suivant DEVELOPING.md,
lancé sous X11 dans un profil temporaire `/tmp/apt-upstream-profile`. Test ciblé
du même `OverlayController.activateOverlay()` utilisé par l'application via
inspecteur Node local : Electron déclare sa fenêtre visible, non focalisée,
2560×1600 ; focus X11 encore PoE. Le symptôme « perte de focus sans overlay »
n'a pas été reproduit par cet essai. Il ne valide pas tous les raccourcis upstream.

Le code upstream active sans accusé du premier rendu ; son backend Linux utilise
`xcb_set_input_focus`, et non les mécanismes Wayland natifs. Ce sont deux pistes
statiques de diagnostic, pas la preuve d'une cause sur l'autre configuration.
Les logs upstream signalent aussi un échec `XkbGetKeyboard` dans libuiohook.
L'application upstream de test a été fermée ; PoE a été conservé.

## Vérifications encore nécessaires

- Changement volontaire d'application : confirmé par l'utilisateur (« Le panneau
  disparais bien »), et confirmé dans les logs par `other_active=true` → Hidden,
  puis retour `game active=true` sans réouverture. Les tentatives automatisées
  River `focus-view`/activation avaient été ignorées pendant le plein écran ;
  elles ne sont pas comptées comme des tests réussis. Pas de mapping Alt+Tab par
  défaut dans la configuration lue.
- Réduction/restauration, fermeture et réapparition contrôlées du jeu ; la
  réapparition a été observée, pas une campagne complète de ces transitions.
- La saisie native de ligue, le replay et les conflits ont depuis été testés au
  jalon 0.2 ci-dessous ; le replay utilise un serveur isolé.
- Plusieurs sorties, facteur d'écran différent de 1, mise à l'échelle de
  l'application, branchements d'écran et géométrie au-dessus d'un jeu fenêtré.
- Fenêtres de jeu natives Wayland, PoE hors Steam, X11 natif et Windows : non testés.
- Paquet construit avec makepkg et tests, inspecté par pacman, extrait et exécuté
  sans installation système. Installation sur système propre, mise à jour et
  désinstallation restent à vérifier. makepkg LTO C/C++ désactivé uniquement dans
  ce PKGBUILD, car les objets GCC LTO de ring ne sont pas lus par rust-lld ; le
  thin-LTO Rust du profil Cargo est conservé.

## Mesures

Mesures disponibles à ce jalon : capture/parsing à froid 119–166 ms avec index
de mods, premier rendu 4–68 ms après demande d'affichage, réseau 611–657 ms dans
les essais cités. Elles proviennent de journaux distincts et ne constituent pas
une étude statistique. Sur le binaire release extrait du paquet : parsing à froid 22 ms, copie/parsing
à chaud 50 ms, premier rendu 3–4 ms après demande à chaud, réseau 565–597 ms.
Après parcours complet et masquage : RSS 57,79 MiB et 0 tick CPU mesuré sur 3 s
(/proc, résolution CLK_TCK). Cela signifie sous le seuil de mesure, pas un coût
CPU mathématiquement nul. Un panneau statique avant parsing avait donné 46,76 MiB
et 0 tick sur 2 s. Pas d'optimisation spécifique revendiquée ; la boucle reste
événementielle, sans rendu permanent. Optimisations supplémentaires à décider
sur une campagne de mesures après extension fonctionnelle.

## Jalon 0.2 : bulk et économie

37 tests unitaires passent, ainsi que Clippy avec avertissements refusés.
Les ajouts couvrent les identifiants de bulk, piles et influences, les ratios
et stocks des offres, les devises différentes, les variantes poe.ninja,
les erreurs/cache/annulations et les réglages compatibles avec la version 0.1.

Sur le PoE resté ouvert, copie Ctrl+D d'une pile de 3 Scroll of Wisdom : recherche
bulk Covens, 23 résultats et 20 offres visibles, 409 ms réseau (build debug).
Prix poe.ninja séparé à 0,008005 chaos/unité, total de la pile et graphe inspectés.
Le panneau passif reste sans focus. Saisie native d'un stock minimum de 100 :
une offre restante, stock 1 925, 19 chaos pour 200 parchemins, soit 0,095/unité.
Sélection de Divine Orb dans la liste native puis recherche : 3 offres, 352 ms
réseau. Échap masque le panneau et rend le focus au jeu. Les réglages de test
devise/stock ont été restaurés à chaos/sans minimum après ces essais.
Le build release intermédiaire donne 72 ms copie/parsing, 73 ms premier rendu,
347 ms réseau bulk ; ces échantillons dépendent de la charge et du marché vivant.

Le paquet Artix 0.2.0-1 a été construit hors ligne par makepkg avec ses 37 tests,
inspecté par pacman, extrait et exécuté sans installation système. Propriétaires
root/root vérifiés pour ses 343 fichiers ; les hashes des licences de 188 crates
de l'hôte et de la bibliothèque standard Rust correspondent à l'index embarqué.
L'archive source correspond aux fichiers locaux et exclut le bytecode Python.
Le PKGBUILD généré vérifie le SHA256 concret de l'archive source.

Depuis ce binaire extrait : cinq cycles passifs réussis ; Ctrl+D sur la pile
réelle donne 24 résultats, 20 offres bulk, 450 ms réseau et un cache poe.ninja âgé
de 27 min. Une seconde copie sur la baguette donne 411 résultats, 10 offres Trade,
605 ms réseau et aucun agrégat poe.ninja de mauvaise variante. Captures inspectées,
focus conservé au jeu dans les deux cas. Copie/parsing : 104 ms à froid, 50 ms
à chaud ; premier rendu 5 puis 4 ms. Lecture du cache économie : 15,767 ms.
Maj+Espace puis clic dans le champ Ligue donne le focus au panneau visible ;
saisie native de `Covens`, clic Rechercher, résultat depuis le cache Trade,
puis Échap : panneau masqué et focus vérifié sur Path of Exile.

Après ces parcours et masquage, le processus graphique complet utilise
71 640 Kio de RSS (69,96 Mio), avec 0 tick CPU supplémentaire sur 3 s à CLK_TCK=100.
Ce RSS inclut maintenant les données économie et n'est pas directement comparable
au diagnostic de parsing ni une réduction revendiquée par rapport à la 0.1.

Le premier essai de replay sur le bureau réel est invalide : River n'a pas donné
le focus à la fenêtre de test devant PoE. Un clic destiné à cette fenêtre a pu
atteindre le jeu ; cet essai a été abandonné et n'est pas compté comme réussite.
`scripts/test-shortcut-replay.sh` démarre désormais son propre River sans écran,
avec un répertoire runtime et un serveur XWayland distincts. Le module de
production y rejoue Ctrl+D, Maj+Espace et Ctrl+Maj+D vers une autre application
X11, avec snapshot de jeu actif faux puis volontairement obsolète/vrai ; aucune
action d'overlay n'est déclenchée. Un conflit sur le troisième raccourci rejette
l'enregistrement et libère les grabs précédents. Le programme de test refuse
l'exécution hors de ce serveur isolé. Cela teste le protocole XWayland, sans
constituer une nouvelle preuve graphique de l'overlay réel.

## Optimisations mesurées en 0.2

Le catalogue conserve toutes ses données upstream ; le chargement ne désérialise
plus ses propriétés inutilisées dans le diagnostic. Cinq processus froids par
objet/version, release, sans réseau, résultats item/requête strictement égaux :
parsing médian de la wand 23,553 → 18,802 ms, Wisdom 24,094 → 18,221 ms ; pic RSS
médian du diagnostic respectivement 31,69 → 24,50 et 31,29 → 24,09 Mio.
Données brutes : [parser-measurements-0.2.json](parser-measurements-0.2.json).

Le cache poe.ninja était désérialisé directement depuis un fichier non tamponné.
Lecture et écriture utilisent maintenant BufReader/BufWriter. Trois paires de
diagnostics froids avec le même cache réel compact de 2 436 243 octets, profil
temporaire, proxy réseau fermé et résultats imposés depuis le cache : médiane
murale 1 418,429 → 46,813 ms, CPU 1 417,531 → 46,541 ms. Prix et graphes identiques,
seul l'âge est exclu de la comparaison. Le timestamp local du cache de test est
rafraîchi pour conserver le parcours hors ligne, sans modifier le fichier réel.
Données : [economy-cache-measurements-0.2.json](economy-cache-measurements-0.2.json).
Ces mesures concernent les processus diagnostics, pas le RSS de toute l'interface.

Reproduction avec deux binaires conservés des versions à comparer :

```sh
python3 scripts/benchmark.py parser /chemin/avant /chemin/apres --runs 5 --output /tmp/parser.json
python3 scripts/benchmark.py economy-cache /chemin/avant /chemin/apres --cache "$HOME/.cache/awakened-poe-trade-rs/economy/Covens.json" --output /tmp/cache.json
```

Les binaires de référence de cette séance sont conservés localement dans
`dist/benchmarks/` (hors Git et paquet). Le script mesure CPU/RSS avec wait4 Linux
et vérifie l'égalité des sorties. Il n'envoie ni touche ni requête HTTP utile.

## Jalon 0.3 : traduction des mods et bornes

47 tests Rust et Clippy avec avertissements refusés passent. Le métier gère
maintenant les matchers à deux valeurs (moyenne), les nombres littéraux dans
les descriptions, les lignes multiples, les flags sans valeur, les options,
les résolutions `select`, `trivial-merge`, `percent-merge`, `flag-merge`, les
bounds avancées, les valeurs négatives et anciennes, l'effet des modificateurs
et les contributions cumulées par type. Les uniques récupèrent la catégorie
de leur base pour distinguer un mod local d'arme d'un mod global sur bijou.
Les requêtes conservent les identifiants alternatifs en groupe « au moins un »,
les transformations de valeur upstream et les options après `|`. Les plages
inversées et les valeurs non finies sont rejetées avant mise en file réseau.
Une nouvelle recherche invalide la génération précédente avant la validation
locale, pour qu'une ancienne réponse ne remplace pas une erreur de saisie.

`scripts/compare-stat-translations.cjs` exécute le TypeScript upstream inchangé
avec son vrai index binaire de matchers. 55 961 cas générés depuis le catalogue,
dont 1 132 avec effet augmenté ou valeur non scalable : 51 045 traductions
reconnues et zéro différence sur référence, IDs, valeur, bornes, décimales,
legacy, option, inversion et sens de comparaison. Le reste est rejeté par les
deux implémentations. Le calcul de l'effet est comparé à `applyIncr` upstream.
L'index natif reproduit son tri stable et sa recherche binaire, y compris les
matchers avancés et les collisions/ambiguïtés du catalogue. Rapport conservé :
[stat-translations-0.3.json](stat-translations-0.3.json).

Ce corpus est synthétique et porte sur traduction/effet. Il ne prouve pas la
parité du parsing d'objets entiers, du regroupement de tous les types de mods,
des propriétés, des pseudo-stats ni de l'interface upstream complète. Des tests
Rust distincts couvrent les blocs, types, rappels ignorés et sommes de sources.
La comparaison exige Node et les dépendances de développement renderer ; aucune
interface JavaScript n'est embarquée dans le binaire Rust.

Sur le PoE resté ouvert : Ctrl+D copie de nouveau la vraie Driftwood Wand avant
affichage, 401 résultats et 10 offres Trade, 779 ms réseau ; panneau passif sans
focus. Le premier rendu mesure 76 ms et la copie/parsing 69 ms sur cet essai froid.
Maj+Espace puis ouverture de l'expandeur : mod implicite affiché, contrôle Min
actif à 8, Max activé puis saisi à 12. Après clic Rechercher, `status.request`
confirme exactement `implicit.stat_2974417149` avec `{min:8,max:12}` ; 404 résultats,
10 offres, 608 ms réseau. Capture inspectée. Échap masque le panneau et le focus
X11 est vérifié sur Path of Exile. Cinq ouvertures/fermetures passives réussissent
ensuite. Ces mesures sont des observations ponctuelles, sans optimisation
supplémentaire revendiquée à ce jalon.

Paquet 0.3.0-1 : makepkg hors ligne et ses 47 tests réussis, source comparée aux
383 fichiers locaux, 344 fichiers du paquet vérifiés root/root, licences/hashes
de 188 crates et de la bibliothèque standard contrôlés. Le binaire extrait a
réussi cinq cycles passifs puis une vraie copie de la wand : 407 résultats,
10 offres, 628 ms réseau, 70 ms copie/parsing et 4 ms premier rendu à chaud.
La saisie Min=8, Max=4 affiche l'erreur locale ; la génération passe de 13 à 14,
sans requête ni résultat actif, aucun `trade search` pour 14 dans le journal.
Après correction à Max=12 : génération 15, requête vérifiée avec les deux bornes,
405 résultats, 10 offres et 599 ms réseau. Échap masque le panneau, PoE reprend
le focus. RSS ensuite à 69 248 Kio (67,63 Mio), 0 tick CPU sur 3 s à CLK_TCK=100.
Les offres varient avec le marché. Les branches UI des flags et options ont une
couverture de traduction/requête, mais pas encore d'inspection graphique sur
des objets réels correspondants.

## Jalon 0.4 : pseudo-stats et propriétés

55 tests Rust passent, ainsi que Clippy avec avertissements refusés. La table des
35 règles pseudo générales est exportée depuis le TypeScript upstream inchangé
avec son hash et son commit de provenance. `export-pseudo-rules.cjs --check`
vérifie son identité JSON, y compris les multiplicateurs, sources obligatoires,
groupes remplacés, ordre des règles et conservation des sources. Le traitement
spécifique de visibilité du chaos crafté est porté dans le moteur Rust.

`compare-calculated-filters.cjs` compare 1 651 entrées au parseur d'objet complet
upstream, puis à ses fonctions de propriétés, pseudo-stats et bornes initiales :
zéro écart sur les IDs, valeurs affichées, bornes, sélections, masquages et sources
restantes. Le corpus comprend les contributions simples/cumulées, résistances
duales et générales, attributs, sources requises, uniques corrompus/non corrompus,
sources craftées/fracturées, valeurs négatives, qualité 0/10/20/30, armes et armures,
la vraie copie de la wand et une fixture d'anneau. Les autres objets sont
synthétiques ; ils vérifient les calculs et ne constituent pas des objets copiés
du jeu. Rapport : [calculated-filters-0.4.json](calculated-filters-0.4.json).

L'hôte Node charge les ressources Vite depuis les fichiers locaux, sans réseau,
et exécute les modules parser/calculs upstream. Pour éviter de charger l'interface
web et les presets, il extrait les corps AST inchangés des trois fonctions de
conversion en filtres utilisées. Cette comparaison ne couvre pas tous les presets,
les ajustements finaux d'UI ni les règles pseudo spécialisées (maps, flacons,
Heist, clusters, etc.). Les 55 961 comparaisons traduction/effet du jalon 0.3 ont
également été rejouées sans différence après ajout des métadonnées de sources.

Vérifications graphiques du build release sur PoE :

- Vraie wand copiée par Ctrl+D, 439 résultats et 10 offres, 673 ms réseau ; 71 ms
  copie/parsing, 4 ms préparation des cinq lignes de filtres, 81 ms premier rendu
  à froid. Panneau passif, focus conservé au jeu.
- Physical DPS calculé à 12,6 après normalisation de qualité, affiché à 12 selon
  l'arrondi upstream. Clic dans la ligne puis Rechercher : requête vérifiée avec
  `weapon_filters.filters.pdps.min=12`, aucun faux ID `item.*` dans les stats ;
  10 offres, 602 ms réseau. Les mods de dégâts de sorts ne contribuent pas au DPS.
- Défilement natif de la liste, capture inspectée, sans transmettre de touche
  de défilement X11 au jeu. L'exemple pointer exige `--scroll` explicite.
- Fixture `synthetic-calculated-ring.txt` chargée par presse-papiers et commande
  check, pas par un objet du jeu. Suggestions automatiques vérifiées par socket :
  `pseudo_total_life.min=81` et `pseudo_total_elemental_resistance.min=43`, 65
  résultats, 10 offres, 630 ms réseau ; panneau passif. La restauration du
  presse-papiers a été tentée dans finally ; elle n'a pas persisté après la fin
  de l'outil de test. Une nouvelle copie réelle depuis PoE a été vérifiée ensuite
  (voir contrôle du paquet ci-dessous). Le prix poe.ninja était explicitement
  étiqueté « Prix de la base seule », pas une estimation des mods de ce rare.

Le volet conserve les sources exactes dans un expandeur secondaire. La hauteur
de la liste et sa remise à zéro entre objets ont été ajustées après observation
des contrôles trop serrés. La plage suggérée est persistée (défaut ±10 %, anciens
profils compatibles) et s'applique à la prochaine copie. Les anciens contrôles
déjà saisis ne sont pas réécrits lors d'un changement de plage.

Paquet 0.4.0-1 : build et 55 tests hors ligne par makepkg, archive source comparée
aux 410 fichiers locaux, 345 fichiers du paquet root/root, notices de 188 crates
et bibliothèque standard contrôlées par leurs hashes. Binaire extrait exécuté
sans installation système. Cinq cycles passifs supplémentaires passent. La vraie
wand donne 433 résultats, 10 offres, 770 ms réseau ; préparation des cinq lignes
de filtres en 3 ms, premier rendu 6 ms. La nouvelle liste agrandie et le défilement
vers les lignes pseudo/sources sont inspectés sur captures.

Sélection de pDPS : `weapon_filters.filters.pdps.min=12`, 442 résultats, 739 ms
réseau. Sélection supplémentaire du pseudo dégâts des sorts :
`pseudo.pseudo_increased_spell_damage.min=8`, 10 offres et 618 ms réseau.
Une consultation immédiate de la fixture est bloquée par le cooldown local
de 5 s, sans retry automatique ni activation. Après expiration, clic explicite
sur Rechercher : 64 résultats, 10 offres, 628 ms réseau avec les deux suggestions
vie/résistances attendues. La liste revient bien à son début entre deux objets.

Échap rend le focus au jeu. Une dernière copie de la vraie wand donne 447 résultats
et 10 offres en 616 ms, préparation des lignes 6 ms et premier rendu 9 ms. Après
masquage, wl-paste confirme toujours le vrai texte avancé de la wand (8% spell
damage), puis RSS du processus à 70 896 Kio (69,23 Mio), 0 tick CPU sur 3 s à
CLK_TCK=100. Ce sont des observations ponctuelles, sans optimisation spécifique
revendiquée à ce jalon. PoE et le compositeur sont restés ouverts.

## Jalon 0.5 : évaluation locale et profils de cartes

61 tests Rust passent, Clippy sans avertissement. Les valeurs par défaut du
panneau sont exportées depuis `WidgetItemCheck.vue` upstream inchangé et vérifiées
avec leur SHA256. `compare-inspection.cjs` compare au parseur TypeScript complet
3 144 entrées : les 340 traductions de mods de zone, valeurs simples/plages et
négatives dans trois profils, inconnus, carte synthétique et vraie wand ; DPS
physique/élémentaire à la qualité copiée ; poussière des uniques aux seuils
ilvl 46/68/84, qualité catalyst, influences et implicites de corruption.
Les 1 875 combinaisons des fonctions de décisions sont identiques à upstream.
Le texte rendu (signes et placeholders compris) et les tags du catalogue sont
également comparés. Zéro écart : [inspection-0.5.json](inspection-0.5.json).
Les corpus de 1 651 filtres calculés et 55 961 traductions/effets ont été rejoués,
sans écart. Cette couverture ne prouve pas la parité complète du parseur ou des
presets.

Les deux règles par défaut « Monsters reflect #% of Physical/Elemental Damage »
n'existent plus dans les traductions de ce commit upstream. Le port conserve ces
réglages et les signale comme obsolètes ; aucune ancienne ligne n'est présentée
comme reconnue. Le catalogue comporte 340 règles actuelles, plus ces deux règles
obsolètes dans le profil I. Une ligne inconnue reste visible même si aucun mod
n'est reconnu ; son avertissement est hors de la liste défilante.

Observations graphiques, build release natif, au-dessus du même PoE ouvert :

- Ctrl+Alt+D sur la vraie Driftwood Wand : copie avant panneau, DPS physique 11
  à qualité copiée, aucun appel Trade/poe.ninja, état Passive et focus jeu. Une
  copie prend 72 ms, préparation 4 ms, premier rendu à froid 61 ms. Les liens
  Wiki/PoEDB/CoE sont visibles ; leur ouverture navigateur n'a pas été déclenchée.
- Maj+Espace, clic **Consulter les prix** : page Trade interactive, 464 résultats,
  10 offres et 741 ms réseau dans Covens. Échap masque et rend le focus au jeu.
- Une carte **synthétique**, chargée par presse-papiers et `inspect-check`, affiche
  Beach, les thorns physiques, −12 % de résistances maximales, deux boss et une
  ligne future inconnue. Aucune carte réelle n'a été copiée depuis le personnage.
  Couleurs prudence/favorable par défaut inspectées ; clic thorns → danger.
- Profil II initialement neutre ; activation des nouveaux mods et marquage vu.
  Retour profil I : danger conservé. Recherche « physical thorns » : trois lignes
  (résultat et deux règles obsolètes), modification par radio puis effacement :
  règle `-s-`, l'autre profil est préservé. Après redémarrage, profil I nouveau et
  profil II vu sont conservés. Réglages de test ensuite remis à leur état initial,
  sans changer la ligue, les filtres ou l'échelle de l'utilisateur.
- Les trois colonnes Danger/Prudence/Bon du TreeView sont visibles à largeur 620.
  La première sélection de page GTK ignorait les enfants encore invisibles ;
  correction vérifiée sur capture. La hauteur des mods et l'avertissement inconnu
  ont également été ajustés après inspection réelle.
- Une requête Trade de la wand étant en vol, nouvelle commande `inspect-copy` :
  nouvelle vraie copie et panneau local passif en 85 ms, sans attendre HTTP.
  Aucun résultat ni requête de prix dans cette nouvelle génération, focus jeu.
  Le journal enregistre le rejet de l'ancienne réponse. La capture possède son
  worker borné distinct de Trade ; le timeout réseau reste 15 s.

Le test isolé River/XWayland vérifie désormais les quatre raccourcis, leur replay
vers une autre application X11 même avec un ancien état game_active=true, et la
libération des grabs après conflit. Aucune touche de ce test n'atteint le jeu.
L'utilisateur a confirmé que le panneau disparaît lors d'un changement volontaire
d'application. La réduction/fermeture du jeu, les cartes réelles, les images de
cartes, l'ouverture navigateur, les autres langues et plateformes restent non
validées. PoE et River sont restés ouverts pendant ces contrôles.

Paquet 0.5.0-1 : makepkg hors ligne et ses 61 tests passent. Archive source
comparée aux 426 fichiers locaux ; 346 fichiers du paquet root/root ; notices des
188 crates et de la bibliothèque standard vérifiées par leurs hashes. Le binaire
extrait, après les opérations de strip du paquet, est lancé sans installation
système : cinq cycles passifs passent ; copie réelle de la wand → 456 résultats,
10 offres, 611 ms réseau, 71 ms copie/parsing, 3 ms filtres, 6 ms premier rendu.
Ctrl+Alt+D produit ensuite le panneau local avec DPS 11 et aucune requête de prix,
toujours passif et jeu actif. Le vrai texte avancé de la wand est conservé dans le
presse-papiers après les fixtures. Après masquage : RSS 70 676 Kio (69,02 Mio),
0 tick CPU sur 3 s à CLK_TCK=100. Mesure ponctuelle ; aucune optimisation mémoire
revendiquée pour ce jalon. Le panneau reste masqué, PoE et River restent ouverts.
