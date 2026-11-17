# Windows : un seul EXE

Lancer `awakened-crab-trade-0.31.2-windows-x64-portable.exe` directement.
Aucune extraction manuelle, installation ni élévation administrateur.

Au premier lancement, les bibliothèques GTK, le programme et leurs licences sont
décompressés automatiquement dans
`%LOCALAPPDATA%\awakened-poe-trade-rs\portable`. Les lancements suivants
réutilisent ce cache. Aucun dossier n’est créé à côté de l’EXE portable.

Les préférences restent dans `%APPDATA%\awakened-poe-trade-rs\config.json`.
L’EXE contient le même price checker que le ZIP 0.31.2. Dans PoE, survoler
un objet puis faire `Ctrl+Alt+D`.

Pour retrouver les fichiers et les licences sans ouvrir le panneau, lancer
l’EXE avec `--extract-only` depuis un terminal et rediriger sa sortie vers un
fichier. Le dossier extrait permet aussi de lancer directement le checker et
de remplacer ses bibliothèques partagées. Le ZIP classique reste disponible.

Le lanceur vérifie chaque fichier lors de l’extraction et publie le dossier
complet une fois la préparation terminée. Deux lancements simultanés partagent
la même préparation. Une extraction interrompue est reprise au prochain
lancement.

## Reconstruction

Après la construction du ZIP Windows décrit dans [windows.md](windows.md) :

```sh
python3 rust/scripts/package-windows-portable.py
```

Le lanceur est un petit programme Rust indépendant de GTK, avec compression et
vérification SHA256. Ses dépendances et celles du checker sont incluses dans
les licences extraites. Ses seuls imports DLL sont des composants du système
Windows. Les tests Wine restent distincts du test réel dans PoE sous Windows.
