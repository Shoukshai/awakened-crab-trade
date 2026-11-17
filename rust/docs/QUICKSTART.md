# Utiliser le price checker

1. Lancer `GDK_BACKEND=wayland awakened-poe-trade-rs daemon` depuis le paquet,
   ou `GDK_BACKEND=wayland ./target/release/awakened-poe-trade-rs daemon` après compilation.
2. Dans PoE, survoler un objet et faire **Ctrl+Alt+D**.
3. Le bloc du haut affiche la cote poe.ninja et l’image de l’objet. Pour une
   monnaie, cette consultation suffit : aucune recherche Trade n’est lancée.
   La ligue actuelle est sélectionnée automatiquement. Cliquer les rectangles
   des stats à comparer et ajuster Min/Max. Les puces base et ilvl sont
   désactivées par défaut ; cliquer pour les activer. Rareté et corruption
   se choisissent dans leurs listes. Les gemmes utilisent niveau et qualité
   exacts, modifiables ou désactivables ; les cartes utilisent leur tier.
4. Cliquer **Rechercher** et lire les offres sous les filtres. Survoler une offre
   n’importe où dans sa case pour voir rapidement son identité, son ilvl, ses mods et son prix.
5. Un léger voile sombre accompagne le panneau. Cliquer à l’extérieur ou
   **Échap** le ferme. **Ctrl+D** permet une consultation passive ;
   **Maj+Espace** autorise ensuite les clics.

Les filtres d’objet et les presets complètent les mods. Les prix indicatifs et
une recherche par base seule ne constituent pas une estimation fiable d’un rare.
Les réglages se limitent au price checker et à ses raccourcis.

Linux validé : Artix/River, PoE Steam/DWProton sous XWayland, un écran au facteur 1.
Windows vient ensuite ; il n’est pas encore implémenté. Voir PORTING et
CHECKER-VALIDATION pour les fonctionnalités et les vérifications restantes.
