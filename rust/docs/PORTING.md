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
| Overlay, zones de clic, focus (`main/src/windowing`) | oui | layer-shell natif, suivi Wayland | panneau au-dessus de PoE, clics, 5 cycles, focus passif, changement d’app confirmé par l’utilisateur ; cycle de vie et menus sur River privé | réduction, plusieurs écrans/facteurs de sortie |
| Copie et restauration presse-papiers (`HostClipboard`) | oui | marqueur Proton, copie XWayland, lecture data-control | vraie Driftwood Wand copiée avant affichage | restauration optionnelle, autres jeux/langues/backends |
| Raccourcis (`Shortcuts`) | oui | grabs X11 limités à XWayland + IPC ; configuration/capture natives, remplacement transactionnel | copie réelle ; 816 comparaisons de noms ; capture, conflit, rollback et replay pendant écriture bloquée sur serveur privé | backend River/portail pour Wayland natif, suivi des changements de disposition clavier |
| Parsing multilingue, descriptions avancées (`renderer/src/parser`) | oui | anglais, catalogue typé, propriétés, identifiants Trade/bulk, piles, influences ; traduction des plages, flags, multilignes, variantes par catégorie ; cumul et effet des mods | copies réelles wand/Wisdom ; 55 961 comparaisons traduction/effet avec le TypeScript inchangé ; tests des blocs | parité du parser d’objet entier et enrichissements spécialisés restants, autres langues |
| Filtres et recherche Trade, bulk | oui | base, unique, ilvl, gemme, qualité, tier, liens, bornes Min/Max, options, alternatives OR ; 35 règles pseudo générales ; propriétés équipement/Q20, bornes suggérées et IDs internes ; Trade et bulk | recherches réelles Covens ; 1 651 comparaisons objet→calculs/bornes avec TypeScript ; DPS de la vraie wand et suggestions d'une fixture dans l'UI | presets cartes/charts/flacons/Heist portés au jalon 0.6 (voir validation) ; équipement/base, ajustements finaux, clusters/Timeless/anointments portés au jalon 0.8 ; logbooks multi-zones/Atzoatl/Mirrored Tablet portés au jalon 0.10 (rejet serveur Tablet documenté) ; Mercenary Warrant porté au jalon 0.11 ; politique des filtres d’objet portée au jalon 0.7, éditeur encore partiel |
| Prix poe.ninja, tendances, objets liés, valeurs des piles (`expected-value`) | oui | identités/variantes upstream, feed dense/cache 31 min partagé, chaos/divine, pile copiée/complète, graphe et dispersion, 392 relations, lien exact | 6 937 identités, 39 routes, 392 ensembles, 78 conversions, 508 graphes et 4 982 slugs comparés ; feed réel Covens, Wisdom réelle et fixtures au-dessus du jeu ; cycle privé incluant édition de ligue | images ; copies réelles des uniques spéciaux ; Filled Coffin absent du catalogue actuel ; cote générique Vinktar signalée |
| Prédiction des rares (`price-prediction/poeprices.ts`) | oui | non | endpoints vérifiés sans donnée d’objet : HTML de parking le 16/11/2026, voir rapport 0.14 | revérifier le service, partage et rendu natif |
| Résolution des non-identifiés (`unidentified-resolver`) | oui | choix natif avec défilement, première variante par référence, résolution automatique si choix unique ; propriétés/texte/statut préservés ; aucune recherche avant choix | 5 773 comparaisons sur 547 bases et 1 259 identités ; sélection/automatisme/sans candidat/inspection testés sur River privé ; fixtures consultées au-dessus de PoE | icônes, variantes distinguées ; copies réelles de ces uniques non identifiés |
| Évaluation objets, cartes et mods dangereux | oui | calcul local DPS/poussière ; trois profils persistés, couleurs/vu/nouveaux ; catalogue 340 mods, inconnus et obsolètes | 3 144 cas parseur/calculs et 1 875 décisions comparés au TypeScript upstream ; tests natifs documentés dans VALIDATION | images des cartes, autres objets réels et langues |
| Recherche objets, wiki et liens externes | oui | liens Wiki/PoEDB/CoE ; recherche Heist native, cinq sélections/cotes et consultation virtuelle ; raccourci configurable | 1 552 recherches, 1 025 consultations virtuelles, 228 opérations comparées ; scénario graphique privé et refus serveur de la requête upstream | images, OCR, autres recherches que les cibles du widget upstream, validation navigateur |
| Recherche coffre et défilement | oui | non | non | intégration et configuration |
| Widgets minuteur, images, grille Delve, menu | oui | grille Delve sur une couche séparée, toujours passive ; raccourci et bascule native | 12 966 géométries et 3 840 transitions comparées ; pixels, quatre clics traversants, vingt passages par le bouton et changement de sortie/facteur 2 sur River privé | alignement dans la vraie interface Delve ; minuteurs, images et menu de widgets |
| Réglages, raccourcis configurables, langues, échelle | oui | JSON compatible 0.1, brouillon natif Général/Raccourcis/Cartes ; échelle à chaud, recherche manuelle, devises et stock par défaut ; filtres d’objet transitoires | migration JSON, validation, annulation, refus IO, conservation des règles avancées, échelle 1,25 et nouvelles liaisons sur River privé | préférences upstream restantes, langues, plein écran de réglages dédié |
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

`presets/equipment.rs` porte la sélection objet fini/base et les ajustements des
uniques, clusters, anointments et groupes Timeless. Les métadonnées des sources
préservent génération, mécanique, rolls et effets ; les 61 noms de mods spéciaux
proviennent d’un export vérifiable de client_strings upstream. Les critères
« emplacement libre ou crafté » sont deux contraintes count, pas un ID interne
émis au serveur. Les choix Timeless sont un groupe OR. Chaque contrôle de mod
invalide la génération sans HTTP ; ses callbacks utilisent des références faibles
aux widgets. Une liaison circulaire du contrôle Min/Max des filtres d’objet a
également été supprimée. Les sources exactes avancées restent facultatives et
séparées des critères par défaut du preset upstream.


## Jalon 0.9 : suivi des fenêtres et menus natifs

Le suivi foreign-toplevel publie chaque fenêtre seulement après son événement
`done`. Les mises à jour partielles d’une autre fenêtre ne contaminent plus
l’instantané. La fin du moniteur efface les fenêtres et révoque les entrées.
La machine d’états distingue jeu absent, réduit, inactif et autre application ;
le retour au jeu ne rouvre pas le panneau. Les journaux et `status` exposent la
cause du masquage, les surfaces suivies et la géométrie des contrôles.

Un contrôle différé de 30 ms distingue la perte de focus d’un transfert vers un
menu GTK appartenant au panneau. Son jeton est indépendant de la génération des
prix : éditer un filtre n’annule pas une véritable perte de focus. Aucun appel
ne force l’activation. Masquer révoque aussi les grabs des menus ouverts.
Le test privé vérifie les clics, les presets à la souris, le masquage avec menu,
le changement d’application et la disparition/recréation du faux jeu. Il ne
valide ni la réduction réelle de PoE ni le clavier des menus. Voir VALIDATION.


## Jalon 0.10 : logbooks, Atzoatl et Mirrored Tablet

`item/special.rs` conserve les sections des trois zones du logbook et sépare
leurs factions/boss des mods globaux. `Mode::Logbook` expose I/II/III dans le même
sélecteur natif ; chaque choix recrée les filtres, invalide les prix et attend
Rechercher. Une description sans zone reconnue est refusée pour la recherche.

`presets/special.rs` applique les règles upstream exportées avec hash dans
`src/data/special-rules.json` : accessibilité des salles, explosifs et masquages,
choix des reflets et difficulté maximale. Le comparateur exécute le parseur et
les fonctions TypeScript inchangés sur 2 599 cas, corps Trade complet compris.
Les trois fixtures spéciales sont synthétiques. Les recherches logbook/Atzoatl
sont acceptées sur Covens ; le Tablet reçoit un rejet pour un ID lake, encore
présent dans le catalogue public. Cette limite ne justifie aucune suppression
silencieuse de critère. Le panneau expose maintenant le message JSON d’une
réponse HTTP en erreur, lu sur au plus 16 Kio et limité à 300 caractères.
Mercenary Warrant porté au jalon 0.11 ; les autres enrichissements du parser, la parité globale de
l’éditeur, les widgets et les autres fonctions de l’inventaire restent à porter.


## Jalon 0.11 : Mercenary Warrant

`item/mercenary.rs` porte les sections du presse-papiers, les alias de build, les
familles/canoniques/tiers des supports et les groupes de presets. Les données
proviennent des mêmes catalogues embarqués, avec résolution des IDs fusionnés.
`item/mercenary/query.rs` produit les groupes Trade spécifiques et leurs poids.
Les critères de compétences principales, supports requis/N−1, six liens, minimum
de meilleurs tiers et exclusions locales/globales sont comparés au TypeScript
inchangé sur 16 900 cas. Les descriptions sans build/compétences sont refusées
pour la recherche ; les supports de familles synthétiques non reconnus restent
signalés dans les lignes inconnues.

`ui/mercenary_panel.rs` expose ces choix avec des widgets GTK, séparés du métier.
Les choix enfants sont conservés dans les widgets ; décocher le groupe les marque
inactifs dans le modèle envoyé à la recherche, y compris N−1. Le moteur métier
reste compatible avec les états upstream, dont les combinaisons parent décoché /
supports optionnels cochés. La molette sur les menus/compteurs fait défiler la
liste sans éditer les critères. Toute édition invalide les résultats sans HTTP.

Les recherches complexes peuvent dépasser la limite anonyme du service. Le lien
Trade ordinaire conserve le corps complet par paramètre `q` dès la recherche,
puis utilise l’ID serveur si elle aboutit. Une erreur ne supprime pas les critères.
La navigation reste un clic explicite, après révocation des entrées du panneau.
Authentification native, images des compétences, copies réelles de Warrants et
navigation réelle du navigateur restent non validées/non implémentées.


## Jalon 0.12 : identités, tendances et objets liés

`economy/identity.rs` porte `getDetailsId.ts` ; `economy/model.rs` porte les valeurs,
conversions chaos/divine, dispersion/prévision et relations `item-drop.json`.
Le catalogue et le feed sont partagés entre prix principal et objets liés.
Les sources numériques conservent leur première contribution avant agrégation,
nécessaire aux variantes. Le parseur conserve la quantité maximale des piles.
Le format de cache version 1 reste lisible ; les requêtes, limites et annulations
conservent les workers bornés. Les messages de résultat sont indirects pour ne
pas agrandir tous les événements UI.

Le panneau natif ajoute un volet défilant d’objets liés, les valeurs des piles,
les avertissements de référence générique et un lien vers la fiche exacte.
Modifier la ligue révoque immédiatement cote, graphe, lien et objets liés sans
HTTP ; la recherche explicite charge la nouvelle ligue. Une réponse obsolète ne
peut pas restaurer ces données. Les images upstream ne sont pas portées.

Les comparaisons exécutent le parseur anglais et les fonctions TypeScript pures
inchangées avec un feed déterministe, y compris prix absents et nuls. Elles ne
prouvent pas la parité du parseur entier ni celle des cotations live. Les quatre
anciennes références Vinktar ne correspondent plus au catalogue courant ; la
cote générique disponible est signalée. Filled Coffin est absent du catalogue
courant : les tests Rust de sa branche historique ne valident pas sa copie
complète. Foulborn/Vestigial/foil utilisent uniquement une référence ordinaire,
avec avertissement explicite, sans valorisation de leur trait.


## Jalon 0.13 : uniques non identifiés

`item/unidentified.rs` porte les candidats de `UnidentifiedResolver.vue` et la
transformation `makeIdentifiedUnique` : ordre du catalogue, déduplication par
référence, première variante. Le choix remplace les métadonnées d’identité et
recalcule la poussière ; il conserve le texte brut, la base copiée, les propriétés,
mods, liens, influences et drapeaux, notamment « non identifié ». Une identité
d’une autre base, inconnue ou déjà résolue est refusée.

`ui/unidentified_panel.rs` fournit les boutons et le défilement natifs. L’affichage
initial reste passif ; le choix demande l’interaction explicite. Sans candidat,
ou tant que le choix manque, aucun job Trade/poe.ninja n’est émis. Un seul candidat
est résolu avant la première recherche. Le changement d’identité recrée les
contrôles, invalide les anciennes générations et garde l’état interactif sans
activation. Les callbacks portent un jeton de copie : une sélection tardive ne
peut s’appliquer à un nouvel objet ou rouvrir une fenêtre masquée.

La cote indicative rappelle que l’objet reste non identifié et que ses mods
cachés ne sont pas évalués. Les variantes sans stats nécessaires restent sans
identité poe.ninja plutôt que de deviner ces stats ; dans 64 cas synthétiques,
la fonction upstream lève une exception et le port Rust retourne une absence.
Les images et la distinction des variantes ignorées par le résolveur upstream
restent hors de cette implémentation. Cela ne prouve pas la parité du parseur
complet ni la valeur d’un unique réel du personnage.

L’anti-répétition XWayland est maintenant propre à chaque raccourci de copie :
passer rapidement de l’inspection aux prix ne supprime plus le second appel.
Les répétitions du même raccourci gardent leur garde de 300 ms, vérifiée avec
une fenêtre de jeu émulée sur le serveur privé.


## Jalon 0.14 : réglages et raccourcis natifs

`hotkeys.rs` conserve les noms et l’ordre des modificateurs de `ipc/KeyToCode.ts`.
Les keysyms sont isolés du backend ; l’extension Super et NumpadEnter n’est pas
une implémentation Windows. Les cinq actions peuvent être configurées ou
désactivées ; les doublons et réservations du jeu/copie sont refusés. La capture
GTK utilise la touche non modifiée et ses modificateurs. Une disposition clavier
changée pendant l’exécution exige encore un redémarrage ou un nouvel enregistrement.

`desktop/preferences.rs` sérialise les modifications hors du thread GTK. Le lecteur
X11 tourne avant toute acquisition de touches. Le backend réserve les nouvelles
combinaisons, persiste le profil puis publie les nouvelles liaisons avant de
libérer les anciennes. Un refus de réservation ou d’écriture libère seulement
les nouvelles acquisitions. Aucun verrou du lecteur n’est tenu pendant l’IO.
Une touche réservée mais pas encore publiée est rejouée vers l’application ;
les relâchements armés par une ancienne révision ne déclenchent pas d’action.
Un arrêt de connexion est signalé ; un nouvel enregistrement tente une connexion
neuve, sans boucle de retry. La perte d’une connexion ne peut conserver ses grabs.

`ui/settings_panel.rs` édite une copie du profil. Général expose ligue, échelle,
plage, devises, en ligne, automatique, marchands, regroupement et stock copié ;
Cartes expose le profil et le signalement des nouveaux mods. Annuler ne persiste
rien. Réinitialiser remplit seulement les champs exposés et conserve règles de
mods et décisions des cartes. Enregistrer n’émet aucun HTTP et ne rouvre jamais
un panneau masqué entre-temps. Les résultats réseau sont invalidés ; la prochaine
copie applique les nouveaux défauts. Les valeurs par défaut des nouveaux champs
sont omises du JSON pour conserver les anciens profils.

La prédiction des rares reste à porter. Les quatre GET sans paramètres des
endpoints poeprices ont renvoyé du HTML avec redirection de parking, sans JSON.
Le rapport horodaté `poeprices-availability-0.14.json` est une observation de
disponibilité, sans donnée d’objet transmise ni affirmation de fermeture durable.


## Jalon 0.15 : recherche des cibles Heist

`item/search.rs` porte `findItems` et la sélection de `WidgetItemSearch.vue` :
trim/espaces JavaScript, longueur UTF-16 minimale de trois, recherche insensible
à la casse, présence de tous les mots et préfixe du mot le plus long. L’ordre du
catalogue, les variantes et le seuil de cinq résultats sont conservés. Le panier
ignore les doublons par nom, trie par cote et remplace les cinq objets par le
sixième. Le catalogue courant contient 94 répliques, sans ancienne gemme à
qualité alternative ; le panneau explique cette absence. Il ne remplace pas ces
gemmes par des gemmes modernes et ne prétend pas offrir une recherche générale.

`Choice::virtual_item` reproduit l’objet virtuel upstream : aucun texte copié,
mod, socket, niveau d’objet ou catégorie d’équipement n’est inventé. Le parcours
upstream omet `uniqueBase`, donc ses filtres envoient le nom de l’unique comme
type de base ; Trade a refusé ce corps avec HTTP 400 « Unknown item base type ».
`trade_item` ajoute seulement la base connue du catalogue, vérifiée avant
consultation. L’oracle compare séparément le modèle d’origine et cette entrée
enrichie aux fonctions de filtres upstream inchangées. Aucune fonction de
l’oracle ni source TypeScript n’est corrigée.

`ui/item_search_panel.rs` fournit recherche, choix, panier, cotes et actualisation
explicite. Les événements portent session et révision pour rejeter les anciens
boutons ; les réponses ont la génération globale et ne peuvent ouvrir une
fenêtre. Le worker poe.ninja existant traite les paniers bornés à cinq avec le
même cache et les mêmes limites que le panneau de prix. Saisir/effacer ne lance
aucun réseau ; sélectionner ne lance pas Trade. Une consultation explicite
prépare les filtres de l’objet virtuel sans toucher au presse-papiers. Les cotes
sont invalidées à la réouverture, au changement de ligue et en erreur ; leur
actualisation reste explicite. Le widget est accessible par le bouton Cibles
Heist, la commande `catalogue` et un raccourci facultatif désactivé par défaut.

Les images, l’OCR des anciennes gemmes (upstream Windows), les autres widgets et
recherches non proposées par ce widget restent dans l’inventaire. Les liens
externes du panneau d’évaluation gardent leur portée et leurs limites documentées.


## Jalon 0.16 : grille Delve passive

`delve.rs` porte les proportions de `WidgetDelveGrid.vue` et une politique
indépendante du rendu. L’arrondi positif reproduit `Math.round` ; le rectangle
vient de la hauteur logique de sortie, avec 11 × 9 cases et le découpage upstream.
La palette est celle de `tailwind.config.js`. L’échelle du panneau de prix ne
modifie pas cette géométrie liée au jeu.

`ui/delve_grid.rs` crée une surface layer-shell RGBA, ancrée aux quatre bords,
zone exclusive −1, région d’entrée vide, clavier None et refus de focus GTK.
Cairo efface en transparent, peint le fond blanc à 10 %, clippe puis dessine
les bordures. La première frame est comptée après `after-paint` ; les anciens
jetons ne peuvent restaurer une surface masquée. Il n’y a ni animation ni timer
de rafraîchissement. Une erreur de dessin masque la fenêtre et garde les entrées
révoquées. Le réseau et le presse-papiers ne sont pas sollicités.

Les vues Prix, Évaluation et Réglages sont exclusives : elles masquent la surface
sans annuler sa demande tant que l’affichage reste passif. La vue Recherche Heist
est non exclusive. L’interaction du panneau annule la demande de grille, selon
`hide-on-focus` upstream. Les pertes de contexte
jeu, changements d’application et erreurs de suivi annulent aussi la demande,
sans réouverture au retour. Le bouton natif ferme le panneau puis attend le jeu
avant l’affichage passif ; aucune activation n’est forcée. Le raccourci facultatif
et les commandes `delve`/`delve-hide` permettent aussi la bascule. `hide` et `quit`
révoquent les deux surfaces. Le septième raccourci est omis du JSON lorsqu’il est
désactivé ; les anciens profils restent lisibles et leurs défauts conservés.

Les mesures et observations restent limitées aux configurations nommées dans
VALIDATION. Les changements de résolution et facteur 2 concernent un serveur
privé, pas l’écran réel du jeu. L’alignement avec la véritable interface Delve,
plusieurs moniteurs et les autres backends restent à valider. Les autres widgets
restent à porter.
