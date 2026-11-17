# Périmètre : price checker Awakened PoE Trade

Périmètre corrigé avec l’utilisateur le 17 novembre 2026 : reproduire le price
checker, avec sélection des stats et consultation Trade dans un overlay.
Première version Linux utilisable, puis version Windows. Les widgets, le chat,
les chronomètres, les images, le coffre et la grille Delve sont hors périmètre.
Le travail antérieur est conservé dans Git ; ses composants graphiques ne sont
plus compilés dans le panneau actuel. L’ancien inventaire est dans
[MIGRATION-HISTORY.md](MIGRATION-HISTORY.md), à titre historique.

| Fonction du price checker | État Linux | Vérification / limite |
| --- | --- | --- |
| Copie de l’objet survolé | Implémentée, avant affichage | PoE XWayland ; geste Ctrl+Alt+C ; capture annulée si le contexte change |
| Parsing anglais, propriétés, mods et pseudo stats | Implémentés | Corpus comparés aux règles upstream ; parseur complet et autres langues non revendiqués |
| Sélection des mods, Min/Max, presets | Implémentée | Rectangles entièrement cliquables, slider bicolore et champs Min/Max accolés ; filtres calculés et sources exactes ; tests graphiques privés réussis |
| Filtres d’objet | Implémentés | Puces base et ilvl désactivées par défaut ; listes rareté et corruption ; influences et autres options disponibles ; éditeur avancé encore partiel |
| Recherche Trade et échange en lot | Implémentés | Zone principale pour les offres, propriétés et mods de chaque objet vendu au survol ; limites réseau, cache, annulation et résultats obsolètes gérés |
| Panneaux par famille | Implémentés | Gemmes : niveau/qualité/corruption ; uniques : identité conservée ; cartes : tier ; Viscera : emplacement et effet ; monnaies : poe.ninja seul |
| Prix indicatifs poe.ninja | Implémentés | Image et cote compacte, variante exacte, total de pile ; aucune courbe ni prédiction automatique des rares |
| Données actuelles | Complément au catalogue upstream | Identifiants officiels Trade, 164 bases Viscera et leurs emplacements ; icônes des influences liées à la source du mod |
| Overlay et focus | Implémentés sous River | Panneau pleine hauteur à côté de l’inventaire ; premier rendu avant interaction ; léger voile sombre, fermeture au clic extérieur ; masquage après changement d’application |
| Ligue actuelle | Automatique | Liste publique poe.ninja, PC softcore ; cache de 5 min ; choix manuel possible dans Réglages |
| Réglages | Limités au price checker | Général et cinq raccourcis ; brouillon, annulation, validation et écriture transactionnelle |
| Distribution Linux | Paquet Artix/Arch | Construction reproductible ; version 0.27 avec sources et licences |
| Windows | Étape suivante | Moteur PoE/réseau réutilisable ; intégration native fenêtres, presse-papiers, raccourcis, rendu et focus à implémenter puis tester sur Windows |

Le moteur PoE (`item`, `modifiers`, `calculated`, `presets`, `item_filters`,
`filters`) et les services (`trade`, `economy`) ne dépendent pas de GTK.
`overlay.rs` décrit les états indépendamment du bureau. `desktop/` contient
l’intégration Linux et `ui.rs` le panneau GTK. Un backend Windows devra utiliser
les API Windows pour l’overlay et les entrées ; une compilation Linux ne le valide pas.

Ctrl+D ouvre la consultation passive. Ctrl+Alt+D ouvre le même price checker
avec interaction autorisée après son premier rendu. Maj+Espace autorise
l’interaction avec un panneau déjà affiché. Les anciens champs JSON des fonctions
annexes sont conservés sans lancer leurs composants ou installer leurs raccourcis.
