# Utiliser le price checker

1. Lancer `GDK_BACKEND=wayland awakened-poe-trade-rs daemon` depuis le paquet,
   ou `GDK_BACKEND=wayland ./target/release/awakened-poe-trade-rs daemon` après compilation.
2. Dans PoE, survoler un objet et faire **Ctrl+Alt+D**.
3. La ligue actuelle est sélectionnée automatiquement. Cliquer les rectangles
   des stats à comparer et ajuster Min/Max.
4. Cliquer **Rechercher** et lire les offres sous les filtres. Survoler une offre
   pour voir ses propriétés et ses mods.
5. **Échap** ferme le panneau. **Ctrl+D** permet une consultation passive ;
   **Maj+Espace** autorise ensuite les clics.

Les filtres d’objet et les presets complètent les mods. Les prix indicatifs et
une recherche par base seule ne constituent pas une estimation fiable d’un rare.
Les réglages se limitent au price checker et à ses raccourcis.

Linux validé : Artix/River, PoE Steam/DWProton sous XWayland, un écran au facteur 1.
Windows vient ensuite ; il n’est pas encore implémenté. Voir PORTING et
CHECKER-VALIDATION pour les fonctionnalités et les vérifications restantes.
