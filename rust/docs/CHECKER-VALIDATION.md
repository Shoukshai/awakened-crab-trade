# Validation du price checker 0.23 — Linux

Le périmètre actuel est le price checker : sélection des stats et Min/Max,
recherche Trade, offres dans un panneau natif pleine hauteur adjacent à
l’inventaire. Les widgets antérieurs ne sont plus compilés dans l’application.
Leurs champs de configuration sont conservés sans installer leurs raccourcis.

## Automatisation

120 tests Rust couvrent le moteur et ses services, dont le choix de la ligue
actuelle, le cache de ligues et la géométrie du panneau. Clippy tous targets avec
`-D warnings` et le formatage sont vérifiés pendant la préparation du paquet.
Les rapports antérieurs du moteur fournis avec le paquet sont des validations
historiques de ses règles ; ils ne prouvent pas une identité totale à upstream.

[price-checker-lifecycle-0.23.json](price-checker-lifecycle-0.23.json) utilise
uniquement un River privé et un faux jeu GTK. Un serveur HTTP de boucle locale
fournit deux offres synthétiques ; les autres requêtes publiques sont bloquées
par un proxy local invalide. Le changement d’URL Trade est refusé hors du bureau
de test identifié et n’accepte qu’une adresse HTTP 127.0.0.1. Un rare synthétique est copié par le même
geste que dans PoE. Le panneau commence passif avec ses mods ouverts ; les
rectangles passent alors les clics au faux jeu sans sélectionner de stat.
Ctrl+Alt+D autorise l’interaction après le premier rendu, y compris lorsque la
recherche automatique change la génération de requête avant ce rendu.

Le test saisit directement un minimum de vie de 60 puis clique Rechercher sans
validation clavier intermédiaire : la requête contient bien ce minimum.
Cliquer le fond du rectangle retire la stat de la requête suivante ; éditer
Min/Max ne change pas sa sélection. Les anciennes offres sont invalidées à
chaque modification. La zone des stats est bornée et les offres disposent de
l’espace principal restant. L’infobulle native au survol est réellement mappée
et visible : elle affiche 73 de vie sur la première offre, alors que l’objet
recherché a 85 de vie. Quitter la ligne ferme l’infobulle, sans changer le focus
du panneau. Les réglages n’ont que deux pages et cinq raccourcis ;
annulation, capture et nouvelle liaison sont exercées, et les données JSON des
anciens widgets sont préservées. Le changement d’application masque le panneau
sans le rouvrir au retour. Dix cycles de capture/masquage, le repos sans nouvelle
frame et la recréation du faux jeu passent.

Le panneau mesure 480×1600 dans la sortie privée initiale 2560×1600. Son bord
droit correspond au bord gauche d’inventaire calculé à partir du client X11.
Un changement de géométrie du faux jeu sans changement de sortie est suivi par
ConfigureNotify, sans polling ni activation. Les sorties privées 1920×1080 et
2560×1600 aux facteurs 2 et 1 conservent toute la hauteur logique.
Le client X observé peut conserver 2560×1600 après le changement de mode ; les
positions sont donc comparées aux coordonnées X réellement observées. Ce test
ne prouve pas un alignement physique du vrai coffre après une telle modification.

Une liste de ligues en cache propre au bureau privé prouve la sélection de la
ligue actuelle softcore, indépendante de l’ordre de la liste et des variantes
Hardcore. Sa réception ne rouvre pas un panneau masqué. Les combinaisons sont
aussi testées par le probe privé historique de replay : autres applications,
répétition de copie, écriture bloquée et restauration après conflit.

## Session réelle

[price-checker-real-0.23.json](price-checker-real-0.23.json) distingue les essais.
Avant le passage au panneau pleine hauteur, la vraie Driftwood Wand copiée par
l’utilisateur a donné dix offres Trade dans le panneau. Cliquer les contrôles
natifs pour passer le minimum de dégâts des sorts de 8 à 9 a modifié la requête
et les résultats ; les dix offres étaient affichées. Le minimum initial a été
rétabli et le profil est sémantiquement conservé. Les derniers cycles du rapport
utilisaient le cache de 60 s ; zéro milliseconde réseau y signifie un cache hit.

Le premier essai utilisateur du nouveau panneau a révélé une réponse HTTP 403
sur la liste des ligues du site PoE. La détection utilise désormais la liste
publique des ligues économiques de poe.ninja, avec un cache de cinq minutes.
**Covens, sélection automatique prête**, a été observé dans le processus réel
après cette correction. Une liste ambiguë ou indisponible ne déclenche pas de
recherche dans une ligue devinée ; le réglage manuel reste disponible.

L’utilisateur a confirmé que les offres apparaissent après la correction de
ligue. Le journal du processus réel donne un panneau **480×1600 à x=1116, y=0**,
contre un client PoE à x=14, y=14, 2532×1540. Le bord droit 1596 correspond au
bord gauche d’inventaire calculé par la règle upstream. La recherche réelle
Covens a donné **432 résultats, dix offres, 621 ms réseau**, puis un cache hit.

La zone d’offres agrandie et les infobulles ont ensuite été ajoutées à la demande
de l’utilisateur. Elles sont validées graphiquement dans le bureau privé et
chargées dans le processus réel ; l’observation réelle du dernier survol reste
à confirmer. Le jeu n’est jamais activé de force. Les essais avant/après chaque
modification sont séparés dans le rapport ; les vérifications du binaire du
paquet sont enregistrées dans `dist/`.

Windows, plusieurs écrans, X11 natif et les autres compositeurs restent non
validés. La version Windows suit la première version Linux. Les formats riches
du presse-papiers ne sont pas restaurés ; le parseur et l’éditeur de filtres ne
revendiquent pas la totalité des fonctionnalités du site Trade.
