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
- Saisie de ligue à vérifier graphiquement (la commande de ligue et sa persistance
  sont vérifiées) ; replay des raccourcis dans une autre application X11 ; conflits de raccourcis.
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
