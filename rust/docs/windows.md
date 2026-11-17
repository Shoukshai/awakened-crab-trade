# Price checker Windows 0.30.0

Version portable x64 pour Windows 11, préparée pour PoE 1 Steam et standalone.

La version [en un seul EXE](windows-portable.md) se lance directement et prépare
automatiquement ses bibliothèques dans le cache Windows. Le ZIP reste disponible.

Décompresser entièrement le ZIP dans un dossier, puis lancer
`awakened-poe-trade-rs.exe`. Garder les DLL et les sous-dossiers à côté de l’EXE.
Les réglages s’ouvrent au premier lancement. L’icône de notification permet de
rouvrir les réglages et de quitter. Relancer l’EXE ouvre les réglages de
l’instance existante.

Dans PoE, survoler un objet :

- `Ctrl+Alt+D` : copier l’objet et ouvrir le panneau interactif.
- `Ctrl+D` : ouvrir le panneau passif ; `Maj+Espace` permet d’interagir.
- `Échap`, la croix ou un clic extérieur : fermer.
- `Ctrl+Maj+D` : masquer le panneau.

Les raccourcis sont configurables. Ils sont limités à PoE ; la copie envoie
uniquement `Ctrl+Alt+C` à la fenêtre de jeu active. Le panneau utilise les mêmes
filtres, offres, aperçus et prix poe.ninja que la version Linux. Aucun moteur
d’estimation de prix des rares n’est inclus.

Utiliser le mode fenêtré ou plein écran fenêtré pour l’overlay. Lancer PoE et le
checker avec les mêmes privilèges, normalement sans administrateur. La version
Windows native doit encore être validée dans un vrai PoE sous Windows 11 ; les
tests automatisés utilisent Wine, un faux jeu et des réponses réseau locales.

Les préférences sont dans `%APPDATA%\awakened-poe-trade-rs\config.json`.
Le cache et le journal `checker.log` sont dans
`%LOCALAPPDATA%\awakened-poe-trade-rs`.

## Reconstruction sous Linux

Prérequis : Rust stable avec rustup, Python 3.11+, curl ou accès HTTPS Python,
tar avec zstd et xz, pkg-config, glib-compile-schemas. Aucun paquet Windows
n’est installé dans le système ; les entrées sont locales à `rust/dist`.

```sh
python3 rust/scripts/setup-windows.py
sh rust/scripts/cargo-windows.sh build --release --locked --bin awakened-poe-trade-rs -j 3
python3 rust/scripts/package-windows.py
```

Les versions et SHA256 des 60 paquets de construction MSYS2 sont dans
`rust/packaging/windows/runtime-lock.json`. Le paquet final ne contient que la
fermeture des imports DLL nécessaires au programme et aux chargeurs d’images,
les ressources GTK et les licences. `runtime-imports.json` décrit les imports
PE ; `SHA256FILES.json` permet de vérifier les fichiers extraits.

Les scripts `windows-sandbox.py` et `test-windows.py` testent le binaire dans un
préfixe Wine et un bureau River privés. Le faux jeu n’est reconnu qu’avec le
jeton de garde du répertoire temporaire. Aucun profil Steam ni fenêtre du jeu
réel n’est utilisé.

Pour ces tests, `setup-windows-test-layout.py` compile une disposition US
[ReactOS](https://github.com/reactos/reactos/tree/344ebfe594e4cf6a498bac08105e1996a03a2c61/dll/keyboard/kbdus),
sous LGPL, qui complète le composant clavier absent de Wine. Elle est installée
uniquement dans le préfixe privé de test et reste absente du ZIP distribué.
L’application utilise les dispositions du système Windows, sans les modifier.
