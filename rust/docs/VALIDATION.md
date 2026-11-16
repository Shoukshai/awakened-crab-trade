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
