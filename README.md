# MeikiCtrl

L'appli manette à la sauce Meikitoow : correction de dérive des sticks, émulation Xbox 360, overlay manette 3D par-dessus le jeu. Pensée pour ne presque rien consommer en jeu.

## Lancer

Dans un terminal, depuis ce dossier :

```
cargo run
```

`cargo run` = version de test, avec une console qui affiche les logs.

**Installateur** (et version portable) :

```
powershell -ExecutionPolicy Bypass -File .\build-release.ps1
```

Résultat dans `dist\` : `MeikiCtrl-Setup-<version>.exe` (installe MeikiCtrl dans ton profil Windows, avec raccourcis et désinstallation) et `MeikiCtrl-portable.exe` (un seul fichier, modèles 3D intégrés). La première fois, le script installe l'outil Tauri (quelques minutes).

**Publier sur GitHub (dépôt privé)** :

```
powershell -ExecutionPolicy Bypass -File .\publish-github.ps1
```

Pas besoin de Node ni de npm : l'interface (dossier `ui/`) est en HTML/CSS/JS simple, intégrée par Tauri.

## Ce que fait l'appli

- **Profils par jeu** : réglages sticks, gâchettes et boutons par jeu. MeikiCtrl active tout seul le bon profil quand le jeu passe au premier plan.
- **Remappage** : chaque bouton DualSense peut envoyer n'importe quel bouton Xbox (ou rien).
- **Courbes et anti-zone morte** : préréglages « Visée FPS », « Précision max », « Réactif », anti-zone morte pour compenser celle du jeu, gâchette rapide.
- **Masquer la DualSense** (HidHide) pendant l'émulation : plus de doubles entrées.
- **Overlay réglable** : coin de l'écran, taille, opacité, couleur des éclairs.
- **Lancement avec Windows** dans la zone de notification, et une seule instance à la fois.
- **Mode performance façon OBS** : priorité du processus réglable, priorité GPU « temps réel » pour l'overlay (fluide même avec un jeu à 500+ FPS), et **mode administrateur** (plus de demandes d'autorisation pour HidHide, démarrage admin via une tâche planifiée).

- **Tableau de bord** : manette connectée, USB ou Bluetooth, batterie, test des boutons et des sticks en direct, interrupteurs Overlay et Émulation.
- **Sticks & dérive** : zone morte et portée max réglables pour chaque stick, plus un **calibrage automatique** (tu lâches le stick, il mesure pendant 2 s). Les aperçus montrent le stick brut (gris) et ce que voit le jeu (cyan).
- **Émulation Xbox** : ta DualSense devient une manette Xbox 360 virtuelle (pilote **ViGEmBus**), avec la correction de dérive appliquée.
- **Overlay** : la manette 3D animée par-dessus le jeu.
- **Zone de notification** : fermer la fenêtre ne coupe pas l'appli. Clic gauche sur l'icône pour rouvrir, clic droit pour le menu (overlay, émulation, quitter).

Les réglages sont sauvegardés dans `%APPDATA%\MeikiCtrl\settings.json`.

## Raccourcis

| Touche | Action |
|---|---|
| F8 | Afficher / cacher l'overlay |
| F9 | Recentrer l'overlay |

## Performances

- Moteur : lecture bloquante de la manette, donc 0 % de CPU quand rien ne bouge.
- Overlay : 60 images/s quand tu joues, 30 au repos, rien quand il est caché.
- Interface : fermée = détruite, donc aucune WebView ne tourne pendant que tu joues.

## À venir

- Vibrations du jeu renvoyées vers la DualSense, couleur de la barre lumineuse de la vraie manette.

## Modèle 3D

`assets/manette.mkm` est converti depuis **« PS5 Controller » de Taohid Animation**, sous licence CC-BY-4.0 :
https://sketchfab.com/3d-models/ps5-controller-b7bb9c5102a04cb0b1966c6d02bad7d6

`assets/xbox.mkm` est converti depuis **« Xbox Controller-Black » d'ArtStudio640**, sous licence CC-BY-4.0 :
https://sketchfab.com/3d-models/xbox-controller-black-755a3876d0284b629a8e9de89facd1b5

Pour reconvertir (Python 3 + numpy) :
`python tools/convert_glb.py manette.glb assets/manette.mkm` et `python tools/convert_xbox.py xbox.glb assets/xbox.mkm`.
