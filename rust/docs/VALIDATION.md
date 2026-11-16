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

## Jalon 0.6 : presets cartes, charts, flacons et Heist

68 tests Rust passent, ainsi que Clippy sans avertissement. Le nouveau corpus
`compare-presets.cjs` exécute le parseur TypeScript complet et les fonctions de
presets/stat-filters upstream inchangées : 221 objets, dont le vrai Small Life
Flask copié du personnage ; 52 objets avec enchantements et 43 filtres négatifs.
Aucun écart sur les presets disponibles et le preset actif, IDs, valeurs, bornes,
sélections, masquages et exclusions. Le corpus comprend propriétés de zone et
butin, cartes à huit mods, cartes blighted, récompense Valdo/Void, tiers de flacons,
implicites/crafts, enchantements, neuf métiers Heist, cibles Priceless, ailes de
blueprints, charts ordinaires/spéciaux et objets non identifiés.
Rapport : [presets-0.6.json](presets-0.6.json).

Le comparateur exécute aussi les corps AST inchangés de création des filtres
d'objet et de requêtes Trade, avec les constantes et fonctions auxiliaires
upstream. Il omet les clients HTTP, IPC et stores réactifs. Il compare l'identité
recherchée, les champs map_filters/heist_filters et les groupes de stats. Il ne prouve pas la parité des politiques misc/type/trade, des
presets d'équipement/base d'artisanat, ni des règles spécialisées restantes.
Les corpus de 1 651 filtres calculés, 3 144 inspections/1 875 décisions et 55 961
traductions/effets ont également été rejoués sans écart.

Le parseur conserve désormais le tier minimum des sources agrégées et lit les
propriétés de zone, bonus de butin, récompense, niveau et métadonnées Heist.
Les zones/discriminants sont résolus depuis le catalogue conservé. Une copie
incomplète de blueprint (niveau ou ailes) ou une zone de chart/blight inconnue
refuse la recherche plutôt que de retirer ces critères. Les exclusions envoient
un groupe NOT, y compris tous les IDs alternatifs ; aucun ID interne `item.*`
spécialisé n'est envoyé dans les stats.

Le texte hardcodé upstream « Gains no Charges during Flask Effect » n'existe plus
dans ce catalogue, qui contient « Gains no Charges during Effect ». Le comportement
upstream de masquage des enchants est conservé à ce stade, et le corpus emploie
le vrai suffixe `(enchant)` ; il ne prétend pas résoudre cette divergence de données.
Les enchants masqués restent accessibles dans le volet secondaire.

Observations du build release, toujours au-dessus du PoE ouvert :

- **Vrai Small Life Flask**, survol de la barre puis Ctrl+D sans clic en jeu :
  preset exact, aucun faux mod, 176 résultats/10 offres en 732 ms réseau.
  Copie/parsing 72 ms, préparation 0 ms, premier rendu froid 78 ms, état Passive,
  jeu actif. Sa copie anglaise est conservée dans `fixtures/small-life-flask.txt`.
- **Carte synthétique** : preset Propriétés, tier 16, quantité min 72, rareté min
  36, pack size min 22, blighted/uberblighted faux ; zéro résultat, 296 ms réseau.
  Maj+Espace et clic sur le ComboBox : choix Cartes en lot, génération invalidée,
  aucun HTTP automatique, message demandant Rechercher. Clic explicite : critères
  de propriétés retirés, tier conservé, 2 159 résultats/10 offres en 740 ms.
- **Flacon magique synthétique** : charge recovery min 29 et groupe NOT sur
  `explicit.stat_2448920197`. La copie immédiatement après la carte rencontre le
  cooldown local de 5 s, sans retry. Après expiration, clic Rechercher : 13
  résultats/10 offres, 628 ms. Les deux cases, dont « Exclure », sont inspectées.
- **Contrat synthétique** : catégorie `heistmission.contract`, aucun type Bunker
  imposé, niveau de zone min 83, Lockpicking min 3 et cible Priceless ; 5 résultats/
  5 offres, 698 ms. La portée « Contrats de tout type » est visible dans le panneau.
- **Blueprint synthétique** : ailes révélées min 1, ailes totales min 4, niveau 83
  et NOT sur le nombre d'enchants ; 17 résultats/10 offres, 620 ms. État Passive,
  focus jeu ; la liste des contrôles possède son propre défilement.

Aucun de ces objets synthétiques n'a été créé ou manipulé dans le jeu. Le compteur
de filtres a été corrigé après observation de sa valeur périmée lors d'un changement
de preset. Les propriétés n'affichent plus leur identifiant interne comme libellé,
et la hauteur de la liste est adaptée au nombre de lignes. L'échelle de sortie
reste 1 ; autres facteurs, plateformes et langues non validés pour ce jalon.
Les cartes/charts et Heist réels restent à valider avec des copies du personnage.

Paquet 0.6.0-1 : makepkg et ses 68 tests hors ligne passent ; 459 fichiers de
l'archive source comparés aux fichiers locaux ; 347 fichiers du paquet root/root ;
les notices de 188 crates et de la bibliothèque standard sont vérifiées par hash.
Le binaire extrait, sans installation système, passe cinq cycles passifs puis
une vraie copie de la wand : 485 résultats/10 offres, 665 ms réseau, 71 ms copie,
4 ms préparation, 6 ms premier rendu. Le changement de preset de la carte affiche
bien zéro filtre actif en mode lot, aucune requête ni ancien résultat courant.
La nouvelle hauteur de liste montre les trois contrôles du blueprint en entier,
sans identifiant interne comme libellé de propriété ; 17 résultats/10 offres,
666 ms réseau. Une dernière vraie copie par Ctrl+Alt+D donne le DPS local 11,
sans HTTP ni prise de focus. Le presse-papiers est vérifié comme copie de la wand.

Après masquage, RSS 75 744 Kio (73,97 Mio), 0 tick CPU sur 3 s à CLK_TCK=100.
Mesure ponctuelle après parcours de plusieurs nouveaux panneaux ; elle n'est pas
une comparaison mémoire à parcours identique avec le jalon 0.5. Aucune optimisation
revendiquée à ce jalon. PoE et River sont restés ouverts et le panneau est masqué.


## Jalon 0.7 : politique et édition des filtres d’objet

75 tests Rust passent, ainsi que Clippy sur toutes les cibles sans avertissement.
Le corpus `compare-item-filters.cjs` exécute le parseur anglais complet puis les
corps AST inchangés de `createFilters` et `createTradeRequest` upstream. Le modèle
entier et le corps Trade entier sans presets de mods sont comparés : 2 312 cas,
2 299 reconnus, 13 rejets attendus pour des références obsolètes absentes du
catalogue (Charged Compass et Metamorph). Zéro écart, dont 10 politiques éditées
et une gemme imprégnée : [item-filters-0.7.json](item-filters-0.7.json).

Couverture : catégories/bases, raretés, gemmes/transfigurations/Vaal, seuils de
niveau/qualité, influence, synthèse, fracture, miroir, split, foil/Foulborn/Vestigial,
objets non identifiés, mods voilés, paliers de clusters/objets de zone, liens,
piles avec séparateurs de milliers, Agnerod, identité de cartes/Heist et réglages
Trade en ligne/hors ligne, devise, date, regroupement. Les fonctions HTTP, IPC et
stores réactifs ne sont pas chargés. Les corpus antérieurs de 55 961 traductions,
1 651 calculs, 3 144 inspections/1 875 décisions et 221 presets passent sans écart.
La comparaison ne démontre pas la parité complète du parseur, des presets de mods
ou de l’éditeur. Certaines options du modèle n’ont pas encore de contrôle natif.

Observations du build release au-dessus du même PoE ouvert :

- Vraie wand : 572 résultats/10 offres, 614 ms réseau ; copie 74 ms, préparation
  6 ms, premier rendu froid 90 ms, état Passive et focus jeu.
- Maj+Espace et clic ouvrent les filtres d’objet. Modifier ilvl 1 → 2 invalide les
  anciens résultats sans HTTP ; Rechercher donne 551 résultats en 610 ms.
  Min 2/Max 1 est refusé localement. Saisie de Min 3 et clic immédiat Rechercher :
  539 résultats/10 offres, 563 ms ; le signal GTK en attente n’annule pas cette
  nouvelle génération. `min_item_level` global reste null.
- Bague synthétique : nouveau plan, ilvl 83 désactivé au lieu de conserver Min 3,
  catégorie Ring, résistances min 43 et vie min 81. 10 000 résultats/10 offres en
  864 ms. Choix natif Base ou nom exact : `type: Gold Ring`, catégorie retirée,
  862 résultats/10 offres, 581 ms. Prix en chaos, marchands décochés, regroupement
  API coché : statut `available`, `collapse: true`, `price: chaos` ; 744 résultats/
  10 offres, 671 ms. L’indication ninja 2 chaos porte explicitement sur la base.
- Vraie pile de 3 Wisdom : échange en lot, 24 résultats/20 offres, 317 ms.
  Cocher Stock minimum envoie `minimum: 3` ; 17 résultats/17 offres, 353 ms.
  Le stock saisi ne devient pas une préférence globale.

Les contrôles corruption/rareté ne sont proposés que si la politique de l’objet
les définit. Pour les devises sans mods, les options propres à Trade sont
insensibles et le panneau renvoie au stock et à la devise de paiement. Les
variantes uniques reconnues restent visibles dans le titre ; aucune cote ninja
ordinaire n’est réutilisée pour les variantes encore non prises en charge.
Les réglages marchands modifiés pendant les contrôles sont remis à leur valeur
initiale. Les tests n’envoient aucun message de chat ni commande de jeu.

Paquet 0.7.0-1 : build et 75 tests hors ligne réussis ; 464 fichiers de l’archive
source comparés aux fichiers locaux ; 348 fichiers du paquet root/root ; notices
des 188 crates et de la bibliothèque standard vérifiées par hash. Malgré un
message interne de libfakeroot pendant le nettoyage, la création réussit et
l’audit du contenu, des propriétaires et des sommes ne trouve aucun écart.

Le binaire extrait sans installation système passe cinq cycles passifs puis une
vraie copie de la wand : 570 résultats/10 offres, 705 ms réseau, 73 ms copie/parsing,
5 ms préparation et 12 ms premier rendu. Les nouveaux contrôles sont visibles ;
désactiver le minimum ilvl invalide résultats/requête sans HTTP. Une recherche
lancée puis masquée est rejetée à son retour. La copie Wisdom immédiatement après
rencontre le cooldown local, sans retry ; après expiration et clic explicite avec
Stock minimum 3 : 15 résultats/15 offres, 356 ms. Un clic sur Marchands, désactivé
dans ce mode, ne change ni la génération ni la préférence. La nouvelle copie ne
reprend pas l’ancien stock actif.

Dernière vraie copie par Ctrl+Alt+D : DPS physique local 11, aucun HTTP, état
Passive et jeu actif. Le presse-papiers contient la copie avancée réelle de la
wand (seul l’espace final de la ligne Sockets diffère de la fixture normalisée).
Après masquage : RSS 77 820 Kio (76,00 Mio), 0 tick CPU sur 3 s à CLK_TCK=100.
Cette mesure ponctuelle ne démontre aucune optimisation mémoire. PoE et River
restent ouverts, les préférences de départ sont conservées et le panneau est
masqué. Windows, autres compositeurs, langues et facteurs d’écran restent non
validés ; la parité globale des fonctionnalités n’est pas encore atteinte.


## Jalon 0.8 : presets d’équipement et ajustements des mods

78 tests Rust et Clippy sur toutes les cibles sans avertissement. Le corpus
compte 1 145 objets et 18 choix Timeless reconnus, sans écart. Le nouveau
comparateur utilise le parseur anglais et les fonctions complètes upstream de
presets, filtres d’objet, mods et requêtes Trade. Il compare les presets disponibles
et actifs, tags, IDs, valeurs, bornes, sélections, masquages, huiles et groupes,
le modèle d’objet entier puis le corps Trade effectif entier. Les nœuds désactivés
sont retirés des deux corps avant comparaison, car le natif n’envoie que les
critères sélectionnés. Les ajouts facultatifs de sources exactes dans l’UI sont
séparés des critères upstream comparés.

Couverture : équipement normal/magique/rare, objets finis et bases d’artisanat,
uniques constants/variables/catalysés/corrompus et variantes, mods spéciaux par nom,
fractures, influences, qualité, Memory Strands, voilés, clusters, anointments,
emplacements libres/craftés et cinq familles de Timeless. Le test exige un vrai
groupe Timeless ; un simple parsing sans mod reconnu ne suffit pas à le faire
passer. Rapport : [equipment-presets-0.8.json](equipment-presets-0.8.json).
Les corps de presets des objets spéciaux restants et le parseur entier ne sont
pas déclarés équivalents. Les précédents corpus sont rejoués sans écart.

Une différence du moteur de requêtes précédent a été corrigée : les IDs encodés
`enchant.stat_…|…` restent entiers, comme upstream, au lieu d’être convertis en
option numérique. Une requête réelle du format upstream a renvoyé un résultat
pour Allocates Always Angry. Les copies synthétiques emploient les vraies lignes
avancées des données, y compris les bornes de noms des Timeless. La qualité
accessoire est décodée en pseudo dédié ; Memory Strands n’est plus pris à tort
comme propriété d’une carte. Les callbacks des nouvelles lignes et de Min/Max
emploient des références faibles pour éviter de retenir les anciens widgets.
Aucun gain mémoire chiffré n’est déduit de cette correction.

Observations release, au-dessus du PoE resté ouvert :

- Vraie Driftwood Wand : preset exact, implicite Spell Damage min 8 sélectionné,
  564 résultats/10 offres, 681 ms réseau ; copie 74 ms, préparation 3 ms et premier
  rendu froid 87 ms, état Passive et focus jeu. Modifier Min 8 → 9 efface requête
  et offres sans HTTP. La recherche suivante est masquée avant sa réponse.
- Bague synthétique sans craft : preset objet fini, catégorie Ring et pseudos
  résistances/vie ; copie trop proche de la recherche précédente, cooldown local
  sans retry. Le choix natif Base d’artisanat invalide la génération sans HTTP,
  active ilvl 83 et Gold Ring, retire les pseudos ; clic Rechercher : 3 163
  résultats/10 offres en 591 ms. Le panneau reste interactif.
- Lethal Pride synthétique : groupe Timeless et seed Kaom 10 000 exactement,
  requête acceptée, zéro résultat, 277 ms réseau. Cases Min/Max décochées sur
  Kaom puis Rakiata coché : une seule condition count min 1 avec deux choix,
  1 828 résultats/10 offres, 586 ms. Le message du panneau explique le OR.

Ces fixtures n’ont pas été créées dans le jeu. Les uniques/clusters/anointments
réels restent à valider avec des copies du personnage. L’éditeur, les autres
langues/plateformes et les presets spécialisés restants sont toujours partiels.

Paquet 0.8.0-1 : construction et 78 tests hors ligne réussis ; 472 fichiers de
l’archive source comparés aux fichiers locaux ; 349 fichiers du paquet root/root ;
les notices de 188 crates et de la bibliothèque standard sont vérifiées par hash.
Le binaire extrait sans installation système passe cinq cycles passifs, puis la
vraie wand : 567 résultats/10 offres, 658 ms réseau ; copie 75 ms, préparation
4 ms et premier rendu 25 ms, jeu actif. Décocher/cocher le mod met immédiatement
le compteur à 0/1, invalide la génération et ne lance aucune requête. Une bague
synthétique donne ensuite 5 474 résultats/10 offres en 815 ms ; les sources exactes
avancées sont construites sans sélection automatique.

Un premier clic du pointeur virtuel sur le mod a coïncidé avec une perte de focus
et le masquage du panneau, avant traitement de l’édition. Les répétitions au centre
de la ligne puis au même emplacement sur la case restent interactives et éditent
correctement. La cause de cet événement isolé n’est pas établie ; aucune correction
de focus n’est revendiquée à partir de ce seul événement. Le masquage révoque bien
les entrées et le focus revient au jeu.

Dernière copie réelle de la wand par Ctrl+Alt+D : le presse-papiers est comparé
à la fixture normalisée, DPS physique local 11 et aucun HTTP. Avec cette copie,
5 relectures locales d’échauffement puis 20 autres ouvertures/fermetures par
`inspect-check` passent en 2,199 s : Passive, premier rendu, jeu actif, aucun focus
overlay et aucune requête à chaque cycle. RSS avant/après ces 20 cycles : 78 584
Kio (76,74 Mio), stable sur ce parcours. Après masquage : 1 tick CPU sur 3 s à
CLK_TCK=100. Cette fenêtre courte ne mesure pas une charge durable et ne prouve
pas l’absence de toute fuite. Aucun rendu périodique n’est demandé. Les préférences
initiales sont conservées ; PoE et River restent ouverts, panneau masqué.


## Jalon 0.9 : transitions du bureau et menus

85 tests Rust réussis et Clippy toutes cibles sans avertissement. Sept tests
ajoutés couvrent les commits `done` indépendants, la réduction distincte de
l’absence, l’arrêt du moniteur, le retour au jeu sans réouverture, les menus et
les contrôles différés invalidés sans dépendre des générations de recherche.

Le banc `scripts/test-overlay-lifecycle.py` lance un River/XWayland privé à
2560×1600, faux jeu et autre application, avec données synthétiques/locales et
proxy bloquant les services externes. Rapport :
[overlay-lifecycle-0.9.json](overlay-lifecycle-0.9.json). Résultats : 30 vrais clics
virtuels modifient la sélection et la génération sans HTTP ni masquage ; preset
natif sélectionné à la souris ; fermeture explicite d’un menu ouvert sans grab
résiduel ; changement d’application masqué ; retour sans réouverture ; unmap/map
en modes passif et interactif ; destruction puis nouvel XID redécouvert.
La fermeture du faux jeu ne ferme jamais le PoE réel. Un clavier virtuel reste
présent pendant le test, comme le clavier du bureau réel ; aucune touche n’est
émise sur ce parcours. Sans clavier, GTK ne reçoit pas les mêmes événements de
focus, ce qui avait faussé une première vérification après sélection du menu.

Un scénario distinct avec wtype a reproduit le masquage immédiat de l’ancien
code lors d’un transfert vers le menu. Le contrôle différé conserve ce menu
reconnu et ne demande aucune activation. La sélection End/Return de ce scénario
n’a toutefois pas abouti de façon fiable : la validation clavier des menus reste
ouverte. Cette observation n’explique pas le clic isolé décrit au jalon 0.8.

Sur le bureau réel avant le dernier ajustement des menus : vraie wand copiée
avant affichage (75 ms), 560 résultats/10 offres Trade, 732 ms réseau ; premier
rendu 28 ms. Deux clics sur la case du mod invalident les prix sans HTTP et
gardent le panneau interactif. Échap ferme et révoque le clavier ; son événement
de perte de focus tardif ne remplace pas la cause Escape. L’utilisateur confirme
que le panneau disparaît lors d’un passage volontaire vers une autre application.

La demande foreign-toplevel de réduction de PoE est ignorée par ce River :
pendant les 2 s observées, le jeu reste actif et non réduit. La restauration a
également été demandée ; aucune réduction/restauration réelle n’est donc déclarée
réussie. Le drapeau réduit est testé dans la logique, pas graphiquement sur PoE.
Autres compositeurs, Windows, plusieurs sorties et autres facteurs restent non
validés. Aucun gain de performance n’est revendiqué pour ce jalon.


Paquet 0.9.0-1 : build et 85 tests hors ligne réussis ; 475 fichiers de l’archive
source comparés au dépôt ; 350 fichiers du paquet root/root ; notices des 188
crates et de la bibliothèque standard contrôlées par hash. Binaire extrait sans
installation système : cinq cycles passifs sur PoE puis vraie copie Ctrl+D de la
wand, 558 résultats/10 offres, 652 ms réseau, 74 ms copie/parsing, 3 ms préparation
et 10 ms premier rendu. Le jeu garde le focus en mode passif. Un clic sur le mod
invalide les prix et garde le panneau interactif ; Échap ferme avec la bonne cause.

Sur ce même binaire au-dessus du jeu, une bague synthétique permet d’ouvrir le
menu de presets, choisir Base d’artisanat à la souris et rester interactif avec
le focus du panneau. Le menu rouvert puis masqué explicitement disparaît : pas
de grab résiduel, KeyboardNone et jeu actif. Une réponse réseau devenue obsolète
est rejetée. Aucune création d’objet dans le jeu n’est impliquée.

Dernière copie réelle Ctrl+Alt+D de la wand : presse-papiers comparé à la fixture
normalisée, DPS physique local 11 et aucun HTTP. Après masquage, RSS 75 412 Kio
(73,64 Mio), zéro tick CPU sur 3 s à CLK_TCK=100. Cette mesure ponctuelle ne
constitue ni un benchmark comparatif ni une preuve d’absence de fuite. Les
préférences sont identiques à leur sauvegarde initiale. PoE et River restent
ouverts ; le service 0.9 est masqué.


## Jalon 0.10 : zones des logbooks, salles et reflets

91 tests Rust et Clippy toutes cibles réussis. Les nouveaux tests vérifient
l’accessibilité du Locus avec/sans explosifs, l’isolation faction/boss entre les
zones I/II/III, la difficulté maximale des reflets répétés, Delirium sans condition
de difficulté et le refus d’un logbook sans zone. Deux tests réseau vérifient le
message de validation HTTP, l’absence de retry et le rejet d’une page HTML comme
message. Les erreurs JSON sont lues sur 16 Kio au maximum ; seul le message
`error.message`, limité à 300 caractères sans caractères de contrôle, est affiché.

Le comparateur [special-presets-0.10.json](special-presets-0.10.json) compte 2 599
cas, zéro écart : 1 024 cas temple, 848 cas Tablet, 720 cas logbook, les trois
fixtures natives et la vraie wand ordinaire. Il couvre les trois raretés des
logbooks, 1–3 zones, quatre factions et quatre bosses, 1 446 presets de zones,
les sélections initiales et l’activation explicite de tous les filtres retenus.
Sont comparés le choix de preset, les références/IDs/options/valeurs/bornes,
sélections/masquages/tags, le modèle des filtres d’objet et le corps Trade complet
effectif. Les salles sont générées depuis leurs noms imprimés. L’entrée upstream
Splinter Research Lab (Tier 1) possède un matcher mal formé et reste inconnue des
deux parseurs ; ses lignes inconnues sont également comparées. Aucune règle ni
fonction upstream n’est corrigée dans l’oracle pour faire passer le test.

Les corpus précédents sont rejoués sans écart : 55 961 traductions/effets,
1 651 calculs/bornes, 3 144 inspections et 1 875 décisions, 221 presets
cartes/charts/flacons/Heist, 2 312 politiques de filtres d’objet et 1 145 presets
d’équipement. Cette couverture ne démontre pas la parité du parseur entier.

Sur River privé, [overlay-lifecycle-0.10.json](overlay-lifecycle-0.10.json) reprend
les 30 clics, menus/masquage, changement d’application, disparition/réapparition
et nouvel XID du jalon précédent. Il ajoute un changement de zone I → II par le
vrai menu GTK : faction seule de la seconde zone, aucun boss de la première,
prix/requête effacés et panneau toujours interactif. Les services externes sont
bloqués dans ce banc ; aucune touche n’est envoyée au vrai PoE.

Observations release au-dessus du PoE ouvert, avec textes spéciaux synthétiques
collés dans le presse-papiers (aucun objet spécial créé ou copié dans le jeu) :

- Logbook I, Black Scythe/Vorana : 33 résultats/10 offres, 744 ms réseau.
  Le choix natif Zone II efface les prix/requête et retire le boss ; le panneau
  garde son focus. Recherche explicite Druids : 1 370 résultats/10 offres, 616 ms.
- Atzoatl, Doryani accessible et Locus bloqué avec explosifs accessibles : 431
  résultats/10 offres, 613 ms réseau. Le panneau passif laisse le jeu actif.
- Tablet : les trois reflets choisis sont présents ; réponse HTTP 400. Une
  demande diagnostique explicite du même corps précise `Unknown stat provided:
  pseudo.lake_50846`. Les endpoints publics Trade `data/stats` et `data/items`
  listent pourtant l’ID et Mirrored Tablet. La cause de cette incohérence côté
  service n’est pas établie. Le parcours réseau du Tablet reste non validé ; ni
  retry automatique ni retrait silencieux des filtres n’a été ajouté.

Ces contrôles ne valident pas les copies réelles de ces objets spéciaux. La
réduction réelle, le clavier des menus, Windows et les autres configurations
restent dans les limites déjà décrites. Aucun gain de performance n’est revendiqué.


## Jalon 0.11 : compétences et supports des mercenaires

98 tests Rust et Clippy toutes cibles réussis. Les tests ajoutés vérifient les
prérequis de copie, la compétence principale conservée, la fusion des deux IDs de
Gilded Extra Targets et le signalement d’un tier synthétique inconnu. Un test de
prédicats vérifie qu’un groupe N−1 accepte un support optionnel absent mais ne
remplace jamais la compétence ni le support requis ; une exclusion locale force
les six liens sans devenir une exclusion globale. Le lien navigateur conserve le
JSON entier, encode la ligue comme un segment unique et rejette le mode bulk.

[mercenary-0.11.json](mercenary-0.11.json) : 16 900 cas, zéro écart. Le catalogue
compte 64 noms de builds (35 définitions et 29 alias), dont les compétences et
supports atteignables sont parcourus. Le comparateur exécute le parseur anglais,
les presets et le générateur Trade TypeScript inchangés. Il compare build/niveau,
compétences/supports/lignes inconnues, modèle initial puis édité de chaque groupe,
IDs, textes, tags, icônes, familles, tiers/options/borne minimale, sélections,
masquages, modèle complet des filtres d’objet et corps Trade effectif. Les cas
couvrent supports seuls, cinq supports, requis, N−1/mixte, un seul optionnel,
exclusions locales, tous requis, minima absents/hors bornes/fractionnaires et
fixtures multi-compétences aux niveaux 1/83/100. Les textes sont synthétiques.

Tous les corpus précédents sont rejoués sans écart : 55 961 traductions/effets,
1 651 calculs/bornes, 3 144 inspections et 1 875 décisions, 221 presets de zones /
flacons / Heist, 2 312 modèles de filtres d’objet, 1 145 presets d’équipement et
2 599 presets spéciaux. Le parseur entier, les autres langues et l’éditeur global
ne sont pas déclarés équivalents.

[overlay-lifecycle-0.11.json](overlay-lifecycle-0.11.json) reprend le cycle de vie
précédent sur River privé et ajoute l’éditeur de mercenaire : activation d’un
groupe et des six liens, incrément réel du minimum par SpinButton, deux menus
N−1, trois événements de molette sans édition des critères, requête générée puis
groupe décoché sans groupe Mercenary résiduel, panneau toujours interactif. Le
lien navigateur est décodé et comparé à la requête entière après l’erreur réseau
simulée. Une édition l’efface avec les résultats. Les services externes sont
bloqués par le banc ; aucune entrée n’est transmise au vrai PoE.

Sur le PoE resté ouvert, release natif avec la fixture synthétique Bastion :
recherche simple Shield Crush, 202 résultats/10 offres, 665 ms réseau. Le panneau
passif laisse le jeu actif. Les cases, deux menus de supports et le défilement
fonctionnent dans le panneau interactif. Une recherche six liens + deux N−1 reçoit
HTTP 400, « Query is too complex » ; le serveur propose de réduire les filtres ou
de se connecter pour augmenter la limite. Aucune connexion n’est effectuée et
aucun retry n’est envoyé. Décocher le groupe retire ses critères enfants : la
requête simple revient depuis le cache, 202 résultats, zéro ms réseau. Une capture
de l’éditeur et de ses contrôles visibles a été inspectée.

Le lien navigateur pour les recherches refusées est maintenant généré sans
supprimer de critère ; son ouverture réelle dans un navigateur et le comportement
authentifié du site restent à vérifier. La fixture n’est pas une copie réelle de
Warrant du personnage. La limitation anonyme ne prouve pas un défaut des poids,
qui sont comparés au générateur upstream, ni le succès de toutes les recherches
complexes. Windows, autres compositeurs/sorties et réduction réelle restent non
validés. Aucun gain de performance n’est revendiqué.


## Jalon 0.12 : cotations, variantes et ensembles liés

100 tests Rust et Clippy toutes cibles réussis. Les tests couvrent notamment la
quantité maximale des piles et les variantes/références génériques. Les corpus
antérieurs sont rejoués sans écart : 55 961 traductions/effets, 1 651 calculs,
3 144 inspections / 1 875 décisions, 221 presets, 2 312 filtres d’objet,
1 145 presets d’équipement, 2 599 presets spéciaux et 16 900 cas Mercenary.

[economy-0.12.json](economy-0.12.json) : zéro écart sur 6 937 identités d’objet,
39 recherches/routes de feed, 392 ensembles liés, 78 conversions, 508 tendances
et 4 982 slugs. L’oracle utilise le parseur anglais et les fonctions TypeScript
pures upstream inchangés ; les cours sont synthétiques et déterministes. Sont
parcourus les uniques du catalogue, combinaisons de mods spécialisés, liens,
bases/niveaux/influences, gemmes/qualités/corruption, cartes et catégories.
Les slugs incluent les références du catalogue et des cas Unicode. Les prix
absents et nuls restent distincts. La variance échantillonnée et la prévision
nécessitant sept points suivent la source ; le texte est nommé « dispersion »
pour éviter de le confondre avec une variation du cours.

Limites de ce corpus : les quatre anciennes références Vinktar sont absentes des
stats actuelles, Filled Coffin manque dans le catalogue anglais. Ces branches ne
sont pas déclarées validées par des copies complètes ; aucun oracle n’a été
corrigé pour les rendre artificiellement atteignables. Les traits Foulborn,
Vestigial et foil ne sont pas valorisés ; la référence ordinaire est signalée.

[overlay-lifecycle-0.12.json](overlay-lifecycle-0.12.json) reprend les contrôles
précédents sur River privé. Un cache privé version 1 fournit Dawn à 50 chaos,
Divine à 200, Midnight à 201, Noon à zéro et Dusk absent. Le test vérifie le
partage du cache, 150 chaos pour les trois fragments, 2,5 div pour la pile de dix,
la distinction zéro/absence, le lien exact, graphe et 14 lignes liées. Le volet
s’ouvre au clic sans perdre l’interaction ; le test exige une hauteur de liste
supérieure à 100 px pour détecter un conteneur enfant resté masqué. Changer
d’objet efface ces données.
Une édition réelle de l’entrée GTK de ligue efface cote, graphe, lien et lignes
liées, sans HTTP. Toutes les entrées clavier sont confinées au bureau privé et
les services externes y sont bloqués ; le cache réel n’est jamais prérempli.

Sur le bureau réel, release natif au-dessus du PoE resté ouvert : les fixtures
synthétiques Sacrifice at Dawn, Tombfist et Vessel of Vinktar produisent les
identités attendues avec le jeu actif en mode passif. Dawn est absent du feed
Covens observé : aucune cote n’est inventée, mais les relations sont disponibles
et le volet peut être ouvert au clic. Tombfist distingue « 1 Jewel, Steelscale
Gauntlets » (1 chaos observé). Vinktar reçoit la cote générique (2 chaos observés)
avec avertissement. Ce ne sont pas des copies réelles d’objets du personnage.

La vraie pile de trois Scrolls of Wisdom, copiée par Ctrl+D, donne 0,008725 chaos
par objet, 0,026175 pour la pile et 0,349 pour quarante au cours observé. Les trous
du graphe sont conservés. Le jeu garde le focus ; le panneau se masque et les
réponses Trade devenues obsolètes sont rejetées. Ces valeurs sont un instantané,
pas une promesse de prix de vente. Images, navigation navigateur, copies réelles
d’uniques spéciaux, autres langues/configurations et réduction réelle restent
non validées. Aucun gain de performance n’est revendiqué pour ce jalon.

La première vérification du paquet 0.12 a révélé un ScrolledWindow resté masqué
sous le volet lié : la propriété « expanded » seule ne prouvait pas l’affichage.
Le conteneur est maintenant montré explicitement ; le test graphique exige la
surface occupée par la liste avant de réussir. Le Trade live du fragment Dawn
rejette par ailleurs son tag d’échange historique (`No valid exchange want tags`),
sans retry ; les prix liés du feed restent consultables indépendamment. Cela ne
valide pas le parcours Trade des fragments historiques sur Covens.


## Jalon 0.13 : identité des uniques non identifiés

103 tests Rust et Clippy toutes cibles réussis. Trois tests métier couvrent
l’ordre/déduplication des choix et l’isolation par base, la conservation du texte
et des propriétés/drapeaux après le choix, les candidats uniques/absents et les
refus pour d’autres raretés ou une identité déjà résolue.

[unidentified-0.13.json](unidentified-0.13.json) : 5 773 cas, zéro écart. Les
547 groupes de bases et leurs 1 259 identités dédupliquées sont parcourus ;
286 bases ont un candidat unique. Les 5 214 transformations sélectionnées
comparent identité/namespace/base, texte conservé, statut non identifié,
corruption/miroir/foil, poussière, identité poe.ninja, presets, modèles complets
des filtres d’objet et corps Trade effectifs. La comparaison exécute le parseur
anglais, les callbacks `computed` du résolveur et `makeIdentifiedUnique` inchangés.
Les copies sont synthétiques, avec des sections distinctes pour les drapeaux.
Niveaux 1/84, niveaux spéciaux Watcher’s Eye/Agnerod, liens, influences et drapeaux
supplémentaires sont couverts. Les non-candidats, autres raretés et identités
incompatibles sont vérifiés. Aucune fonction de l’oracle n’est corrigée.

Pour 64 sélections nécessitant un nombre de sockets/passifs invisible sur ces
copies, `getDetailsId` upstream lève une exception. Le port Rust retourne une
identité absente : aucune propriété cachée n’est déduite. Cela est distingué
d’une parité du comportement d’erreur. Les 6 937 cas d’identité du jalon économie,
39 routes, 392 ensembles, 78 conversions, 508 tendances et 4 982 slugs sont
rejoués sans écart après l’ajout de l’avertissement sur les non-identifiés.

[overlay-lifecycle-0.13.json](overlay-lifecycle-0.13.json) reprend les contrôles
précédents et ajoute une vraie copie de fixture dans le presse-papiers privé :
12 candidats Heavy Belt, aucune requête même après `prices`, défilement natif sans
édition de génération, choix Mageblood avec une seule recherche et filtre Trade
`identified=false`, avertissement de cote, résolution automatique Lifesprig sans
vol de focus, choix avant inspection/poussière sans HTTP et refus sans candidat.
Les services externes sont bloqués ; ces touches ne quittent pas le bureau privé.

Au-dessus du vrai PoE, release avec les textes synthétiques Heavy Belt et
Driftwood Wand uniques non identifiés : la liste reste passive jusqu’à l’action
explicite, le défilement et le choix Mageblood fonctionnent au clic. La recherche
conserve `identified=false` : trois résultats/trois offres, 610 ms réseau lors de
l’observation. Aucune cote Mageblood n’est présente pour cette identité dans le
feed Covens observé ; l’absence et le statut non identifié sont affichés.
Le cas à candidat unique devient Lifesprig, toujours non identifié, avec le jeu
actif : 76 résultats/10 offres, 587 ms. Les captures du choix et du résultat ont
été inspectées. Aucun de ces deux uniques n’a été créé, identifié ou copié dans
le jeu. Une vraie copie Ctrl+Alt+D de la wand normale est ensuite comparée à la
fixture ; DPS local 11, aucun HTTP, panneau masqué et jeu actif.

Les autres langues, copies réelles d’uniques non identifiés, images, autres
configurations et réduction réelle gardent leurs limites documentées. Aucun
gain de performance n’est revendiqué pour ce jalon.

La première vérification du paquet a aussi mis en évidence un raccourci perdu :
Ctrl+D envoyé moins de 300 ms après Ctrl+Alt+D était supprimé par l’anti-répétition
commun aux deux actions. Les horodatages sont maintenant distincts par raccourci.
Le test `test-shortcut-replay.sh` émule la classe du jeu sur son serveur privé et
vérifie inspection → prix → interaction → fermeture en moins de 250 ms, puis le
refus d’une répétition du même Ctrl+D et son acceptation après 350 ms. Les contrôles
de replay vers une autre application et de libération après conflit restent inclus.
Ce défaut observé explique cette touche ignorée, pas le symptôme de focus signalé
sur l’autre configuration ni le clic isolé du jalon 0.8.
