# Spotiamp — un petit lecteur Spotify natif

**L'esprit Winamp, dans un lecteur Spotify natif et léger pour Windows, macOS et Linux.**

Spotiamp partage les mêmes skins **Classique** et **Modern** et la même base Rust
sur les trois systèmes : une file de lecture, vos playlists Spotify, la recherche
de musiques et des mises à jour intégrées, en **six langues**. Le moteur audio
actuel est librespot 0.8. Un compte Spotify Premium est nécessaire.

Des binaires natifs sont publiés pour **Windows x64, macOS Apple Silicon et
Linux x64**. La compilation, les tests et la création des paquets sont automatisés
sur les trois systèmes. Les mesures de performances, les essais de lecture et
leurs résultats sont détaillés dans [PERFORMANCE.md](PERFORMANCE.md).

## Télécharger et lancer

| Système | Architecture | Dernière version stable |
|---|---|---|
| Windows 10/11 | x64 — Intel/AMD 64 bits | [Installateur](https://github.com/roukmoute/spotiamp/releases/latest/download/Spotiamp-Setup-Windows-x64.exe) · [ZIP portable](https://github.com/roukmoute/spotiamp/releases/latest/download/Spotiamp-Windows-x64.zip) |
| macOS | Apple Silicon — ARM64 | [ZIP portable](https://github.com/roukmoute/spotiamp/releases/latest/download/Spotiamp-macOS-arm64.zip) |
| Linux | x64 — Intel/AMD 64 bits | [ZIP portable](https://github.com/roukmoute/spotiamp/releases/latest/download/Spotiamp-Linux-x64.zip) |

**Windows.**
Lancer `Spotiamp-Setup-Windows-x64.exe`, puis suivre l'assistant en français,
anglais, allemand, espagnol, italien ou portugais. L'installation concerne votre
compte, sans droits administrateur.
Le lecteur apparaît dans le menu Démarrer ; le raccourci sur le Bureau est optionnel.
L'assistant propose de lancer Spotiamp à la fin. Pour la version portable,
décompresser le ZIP et lancer `spotiamp.exe`.

**macOS et Linux.** Décompresser le ZIP dans un dossier personnel, puis ouvrir
un terminal dans ce dossier et lancer l'exécutable portable :

```sh
./spotiamp
```

Si l'outil d'extraction n'a pas conservé le droit d'exécution, le rétablir avec
`chmod +x ./spotiamp`. Le paquet macOS contient un exécutable pour Apple Silicon.
Il n'est pas signé avec Developer ID ni notarié ; si macOS bloque son ouverture,
suivre la [procédure Apple](https://support.apple.com/fr-fr/102445).
Le binaire Linux nécessite **glibc 2.39 ou plus**, les bibliothèques système
ALSA, D-Bus et X11, et une session X11 ou XWayland ; il est compilé et testé en CI
sur Ubuntu 24.04.

Aucun environnement Rust n'est requis pour lancer les binaires publiés.
Les [releases](https://github.com/roukmoute/spotiamp/releases/latest) fournissent
aussi les sources et les empreintes SHA-256. Les mises à jour intégrées
sélectionnent automatiquement le paquet adapté au système et à l'architecture.

La version **0.1.6** réduit la taille du binaire par optimisation globale à la
compilation et retrait de code inutilisé, tout en conservant le niveau
d’optimisation pour la vitesse. Les comparaisons et leurs limites sont dans
[PERFORMANCE.md](PERFORMANCE.md). L’éclair du bouton À propos signale toujours
les mises à jour disponibles.

## Langue de l'interface

Spotiamp propose **français, anglais, allemand, espagnol, italien et portugais**.
Au premier lancement, il utilise la langue du système, avec l'anglais comme choix
de repli pour les langues non prises en charge. Dans **À propos (F2) → Spotiamp**,
choisir **Auto**, **Français**, **English**, **Deutsch**, **Español**, **Italiano** ou
**Português**. Le changement s'applique
immédiatement au lecteur, à la bibliothèque et aux messages, sans interrompre
la musique. Les noms des playlists, titres et artistes restent ceux de Spotify.

Le choix est conservé dans le petit fichier `Spotiamp/language`, à côté des autres
préférences. **Auto** suit à nouveau la langue du système. Pour essayer une langue
pendant un seul lancement, utiliser `spotiamp --language CODE`, avec `auto`, `fr`,
`en`, `de`, `es`, `it` ou `pt` (`.\spotiamp.exe` sous PowerShell, `./spotiamp` sous
macOS/Linux). Les variantes régionales, par exemple `de-AT`, `es-MX` ou `pt-BR`,
utilisent la traduction commune de leur langue.
Les textes sont intégrés au binaire : aucun téléchargement, moteur de traduction,
thread ni dépendance supplémentaire. Les diagnostics techniques des bibliothèques
externes gardent leur texte d'origine.

## Deux skins natifs et légers

Les boutons **Classique** et **Modern**, dans l'en-tête de la file du lecteur,
permettent de choisir l'apparence ; **F4** passe de l'une à l'autre. Le changement
s'applique immédiatement au lecteur et à sa bibliothèque, sans interrompre
la musique ni modifier leurs dimensions. Le choix est conservé pour les prochains lancements.

La préférence tient dans un petit fichier `Spotiamp/skin`, à côté du réglage
audio dans le dossier de configuration du système. Une préférence absente ou
invalide utilise **Classique** ; une erreur de sauvegarde est signalée dans le lecteur.
Pour essayer un skin sans changer le choix enregistré, lancer
`spotiamp --skin classic` ou `spotiamp --skin modern`
(`.\spotiamp.exe` sous PowerShell, `./spotiamp` sous macOS/Linux).

Sous Windows, Spotiamp dessine son interface en logiciel, sur le processeur,
puis l'affiche avec **GDI**, le système graphique classique de Windows :
**aucune accélération graphique n'est requise pour dessiner le lecteur**.
Le rendu du lecteur ne crée aucun contexte OpenGL ou Direct3D et ne demande
pas de carte graphique dédiée. macOS et Linux utilisent leurs propres backends
d'affichage pour présenter le skin choisi.

La différence avec Spotifast se situe aussi dans ce choix d'affichage :
[Spotifast utilise egui/eframe avec le moteur OpenGL Glow](https://github.com/crmne/spotifast/blob/e86d3de0cb72edba8f24b447218daa7f3af1f127/Cargo.toml#L31-L36),
tandis que Spotiamp utilise le rendu logiciel et GDI sous Windows.
Les [détails techniques et les mesures](PERFORMANCE.md) expliquent cette approche.

Au premier lancement, la **bibliothèque de vos playlists Spotify** s'ouvre avec
le lecteur, dans une fenêtre utilisant le même skin **collée à sa droite**. Faire
glisser le bandeau du lecteur déplace les deux fenêtres lorsqu'elles sont
attachées. Faire glisser le bandeau de la bibliothèque permet de la détacher.
Pour les recoller, rapprocher leurs bords puis relâcher la souris : elles
s'aimantent à droite, à gauche, au-dessus ou en dessous. Une fois séparées,
l'une ou l'autre peut être déplacée pour les réunir. Elles se réduisent toujours
ensemble. Sur un écran trop étroit, la bibliothèque se place initialement
juste en dessous. Les lancements suivants retrouvent les positions mémorisées,
l'état attaché ou libre et la bibliothèque ouverte ou fermée.
**BIBLIO. F3** ou **F3** la ferme ou la rouvre : **Recherche**, playlists
et dossiers à gauche, résultats ou titres de la playlist choisie à droite, en
lecture seule.
Le catalogue est chargé à l'ouverture et les titres seulement à la sélection.
La découverte de bibliothèque reste à vérifier avec un compte Spotify réel ;
ses tests automatisés fonctionnent hors connexion.

Le lecteur et sa bibliothèque gardent le **bandeau sans cadre système**, avec
les boutons **Réduire** et **Fermer** intégrés et le déplacement par glissement.

Les **mises à jour intégrées** restent accessibles dans l'onglet **Mise à jour**
du panneau **À propos** : détection d'une nouvelle version, téléchargement,
puis installation et redémarrage à votre demande.
Le skin Classique conserve les touches métalliques, les boutons enfoncés pendant
l'appui, les cadres encastrés et les poignées rainurées. Modern utilise des
boutons aux contours lissés, avec un relief cohérent au repos et pendant l'appui.
Les deux skins gardent la qualité **320 kb/s par défaut** et la molette d'un titre par cran.

![Interface de Spotiamp](preview.png)

L'installateur et l'archive Windows visent **Windows 10/11 x64** (processeur Intel/AMD
64 bits), conformément au [socle de la cible Rust](https://doc.rust-lang.org/rustc/platform-support.html).
Le binaire Windows et l'installateur ne sont pas signés. La CI compile le binaire
avec le runtime C statique. L'interface minifb utilise Win32/GDI et l'audio WASAPI.

## Utilisation

1. Télécharger le paquet de votre système et lancer Spotiamp en suivant les
   instructions ci-dessus.
2. À la première utilisation, cliquer sur **CONNEXION**. Autoriser Spotify dans
   le navigateur, puis revenir dans le lecteur. Aux lancements suivants,
   Spotiamp tente automatiquement de réutiliser la session mémorisée.
   Aucun mot de passe n'est demandé dans Spotiamp.
3. Dans la bibliothèque, choisir **Recherche**, saisir un titre ou un artiste,
   puis valider avec **Entrée** ou **Chercher**. Il est aussi possible de copier
   un lien Spotify de titre, d'album ou de playlist dans le lecteur avec
   **Ctrl+V** (**Cmd+V** sous macOS), puis **+ AJ.** ou Entrée.
4. Double-cliquer sur un titre. Les boutons et la barre de position pilotent
   le moteur audio local. Le client Spotify officiel peut être fermé.

Le bouton **Lecture (▶)** reprend le morceau courant à sa position lorsqu'il
est en pause. S'il est déjà en lecture, il revient au début de ce même morceau.
Ces actions conservent le titre chargé, même si une autre ligne est sélectionnée,
ainsi que sa qualité audio. Après **Arrêt**, le bouton démarre le titre sélectionné
(sinon le titre courant, ou le premier titre de la file).
Un double-clic sur une ligne ou **Entrée** lance explicitement ce titre depuis
le début. **Pause (Ⅱ)** et **Espace** alternent entre pause et reprise.

### Retrouver sa musique au redémarrage

Spotiamp mémorise la file de lecture dans son ordre exact, le morceau courant
et sa position. Au prochain lancement, ce morceau est sélectionné et visible
dans la file. La lecture reste en pause : **Lecture (▶)** ou **Espace** reprend
à l'endroit mémorisé, après la reconnexion à Spotify. Aucun flux audio n'est
chargé avant cette action.

Lorsque la file provient d'une playlist Spotify, celle-ci est retrouvée dans
la bibliothèque après le chargement du catalogue. Consulter une autre playlist
avant de fermer ne change pas cette association. Le volume, Shuffle et Repeat
sont également conservés. **Vider** efface la file mémorisée ; **Arrêt** remet
la position au début du morceau.

La sauvegarde utilise `Spotiamp/playback-session.json`, à côté des autres
préférences : `%APPDATA%` sous Windows, `~/Library/Application Support` sous
macOS et `$XDG_CONFIG_HOME` (ou `~/.config`) sous Linux. Elle contient uniquement
les identifiants et métadonnées de la file, sans audio ni identifiants de connexion.
Elle est remplacée atomiquement à la fermeture et, en cas de changement, toutes
les 15 secondes environ. Aucun thread ni dépendance supplémentaire ; la limite
reste de 500 titres.

Pendant la lecture ou un chargement, la navigation affiche immédiatement le
titre choisi. Le moteur attend **200 ms sans nouveau choix** avant de charger
le dernier demandé : plusieurs clics rapides sur **Suivant** ne lancent donc
pas tous les titres intermédiaires. Le premier lancement à l'arrêt et l'avance
naturelle vers le titre suivant restent immédiats. **Pause** et **Arrêt**
restent disponibles ; les chargements remplacés ou arrêtés sont annulés lorsque
leur exécution le permet. Un recentrage de la file pendant l'appui n'annule
pas les clics sur les boutons de lecture.

Les boutons **Shuffle** et **Repeat (↻)** se trouvent après **Suivant**.
Un voyant vert indique qu'un mode est actif ; cliquer à nouveau le désactive.
**Shuffle** parcourt les titres dans un ordre mélangé, sans rejouer une entrée
pendant le même tour. **Précédent** permet de revenir dans cet ordre.
**Repeat** relance la file lorsqu'elle se termine ; avec Shuffle, un nouvel
ordre est préparé à chaque tour. Sans Repeat, la lecture s'arrête à la fin.
Avec un seul titre et Repeat activé, ce titre joue en boucle.
Les deux modes sont désactivés au lancement et n'altèrent pas vos playlists Spotify.

Pour **changer l'ordre de la file du lecteur**, maintenez le clic gauche sur
un morceau et faites-le glisser à la position souhaitée. Relâchez sur le repère
entre les lignes. La liste défile automatiquement près de ses bords. Déplacer
le morceau en cours ne le relance pas : sa position de lecture est conservée.

Pour retrouver vos playlists sans copier de lien, connectez Spotify et ouvrez
la bibliothèque avec **BIBLIO. F3** si elle est fermée. Elle affiche les playlists
enregistrées ou suivies de votre compte, avec leur dossier en sous-titre.
Un **simple clic sur une playlist**
affiche ses titres sans modifier la file ni la lecture en cours. Un
**double-clic sur la playlist** charge ses titres, remplace la file après
chargement réussi et lance le premier titre. **Actualiser** ou **F5** recharge
le catalogue Spotify.

Le bouton **Envoyer vers…** regroupe les actions de la bibliothèque dans un menu.
Hors des champs de saisie, **Maj+F10** ou la touche **Menu** l'ouvre au clavier.
Choisissez une action à la souris ou avec **Haut/Bas**, puis **Entrée**.
**Échap**, un clic à l'extérieur ou le changement de fenêtre ferme le menu.
Les actions indisponibles restent désactivées.

Un **clic droit sur un morceau**, dans les résultats de recherche ou les titres
d'une playlist, ouvre ce même menu près du pointeur. Sur un morceau déjà choisi,
la sélection multiple est conservée ; sur un autre, seul ce morceau est sélectionné.
Le menu reste dans les limites de la fenêtre. L'ouvrir ne lance pas la lecture
et n'envoie aucune modification à Spotify.

Dans la colonne des playlists, **Envoyer vers… → Lire la sélection** remplace
la file par la playlist choisie et démarre son premier titre.
**Ajouter à la file** ajoute cette playlist à la fin de la file existante.
Le remplacement n'a lieu qu'après le chargement réussi des titres :
une erreur d'accès ou une playlist vide conserve la file précédente.

Dans les titres d'une playlist comme dans les résultats de recherche, **Ctrl+clic**
(**Cmd+clic** sous macOS) ajoute ou retire une ligne à la sélection. **Maj+clic**
sélectionne une plage ; **Maj+Haut/Bas**, **Maj+Début/Fin** et **Maj+Page précédente/
suivante** l'étendent au clavier. **Ctrl+A** (**Cmd+A**) sélectionne tous les titres
lorsque la liste des titres est active. La sélection reste dans l'ordre de la liste,
même si vous avez choisi les lignes dans un autre ordre.

Pour **réordonner une playlist Spotify qui vous appartient**, maintenez le clic
gauche sur un titre choisi, puis glissez-le vers le repère d'insertion dans la
même liste. Si plusieurs titres sont sélectionnés, ils sont déplacés ensemble,
dans leur ordre de liste. Le défilement aux bords permet d'atteindre le début ou
la fin de la playlist. Le nouvel ordre est enregistré sur Spotify après
confirmation ; la file et la lecture du lecteur restent indépendantes.
**Échap** ou un dépôt hors de la liste annule le déplacement interne.
Les résultats de recherche et les playlists d'autres personnes ne se réordonnent
pas ; leurs titres peuvent toujours être glissés vers la file du lecteur.

Avec des titres sélectionnés, **Envoyer vers… → Lire la sélection** ou **Entrée**
remplace la file par cette sélection et démarre son premier titre.
**Envoyer vers… → Ajouter à la file**, **Maj+Entrée** ou
**Ctrl+Entrée** (**Cmd+Entrée**) l'ajoute à la fin, sans interrompre la musique.
Un double-clic sur un titre sélectionne ce titre seul et le lance.
Vous pouvez aussi **glisser la sélection vers la file du lecteur** : un repère
indique l'insertion entre les lignes, et la file défile près de ses bords.
Le dépôt insère les titres dans leur ordre de liste, sans interrompre le morceau
courant. La sélection n'est pas retirée de la playlist Spotify d'origine.

Pour retirer des morceaux d'une playlist Spotify dont vous êtes propriétaire,
activez son panneau de titres, sélectionnez les lignes puis choisissez
**Envoyer vers… → Retirer de cette playlist** ou appuyez sur **Suppr**.
Le retrait est envoyé à Spotify sans dialogue de confirmation et concerne
uniquement les occurrences sélectionnées, même si un titre apparaît plusieurs
fois. La file du lecteur et la lecture en cours sont conservées. Ce raccourci
n'agit ni sur les résultats de recherche ni sur les playlists d'autres personnes.
La playlist est vérifiée avant l'envoi : si son contenu a changé, actualisez-la
avant de recommencer. Après confirmation, ses titres sont rechargés depuis Spotify.

Pour chercher une musique dans le catalogue Spotify, choisir **Recherche** dans
la colonne de gauche, au-dessus des playlists. **Ctrl+F** (**Cmd+F** sous macOS)
dans la bibliothèque ouvre aussi la recherche et place le curseur dans le champ.
Saisir un titre,
un artiste ou les deux, puis appuyer sur **Entrée** ou cliquer sur **Chercher**.
La recherche utilise la session Spotify déjà connectée ; aucune clé d'API
supplémentaire n'est nécessaire.

Les résultats s'affichent à droite avec le titre, l'artiste et la durée, avec
la même sélection multiple et les mêmes actions de lecture, d'ajout à la file
et de glisser-déposer que les titres d'une playlist. **Effacer** annule la recherche
et vide ses résultats, en conservant la file et la playlist consultée. Revenir
à une playlist retrouve sa vue.

Pour enregistrer les morceaux choisis sur Spotify,
**Envoyer vers… → Vers une playlist…** ouvre la liste des playlists de votre
bibliothèque. Choisissez une destination,
puis cliquez sur **Ajouter** ou appuyez sur **Entrée** ; **Annuler** ou **Échap**
ferme ce choix. La sélection est ajoutée à la fin de la playlist Spotify,
dans l'ordre affiché, sans changer la lecture en cours. Les occurrences répétées
d'un même titre sont conservées. Spotify doit autoriser la modification de
la destination ; une playlist en lecture seule est refusée.

Pour créer une playlist avec cette sélection, choisissez
**Envoyer vers… → Nouvelle playlist…**, ou cliquez sur **Nouvelle…** dans
le choix de destination. Saisissez un nom de **1 à 100 caractères**,
puis validez avec **Créer** ou **Entrée**. Le nom accepte les accents et les
raccourcis de presse-papiers habituels. **Retour** ou **Échap** revient au choix
de destination sans créer de playlist. La sélection retenue lors de l'ouverture
est conservée pendant la saisie, qu'elle vienne d'une recherche ou d'une playlist.
Spotiamp crée la playlist, l'ajoute à votre bibliothèque Spotify, puis y ajoute
les morceaux choisis dans leur ordre de liste. Le succès n'est annoncé qu'après
confirmation de ces trois étapes par Spotify.

Une seule création, demande d'ajout ou de retrait à Spotify est traitée à la fois,
pour au plus 500 titres, sans découpage de la sélection. Attendez la confirmation avant
de recommencer. Si la playlist est créée mais qu'une étape suivante échoue,
Spotiamp conserve sa destination connue et précise l'étape en échec ; il ne
recrée pas une autre playlist. Si Spotify ne confirme pas une opération après
l'envoi, vérifiez vos playlists et leur contenu avant de réessayer. **Actualiser**
recharge le catalogue renvoyé par Spotify. Aucune demande d'écriture n'est
renvoyée automatiquement, pour éviter de répéter une modification.

Une recherche affiche jusqu'à **50 titres** : si la liste est limitée, préciser
le titre ou l'artiste. Le champ accepte **256 caractères**. Les requêtes partent
seulement après validation ; une nouvelle recherche annule la précédente et
ignore ses réponses tardives. Les résultats ne sont pas sauvegardés et sont
libérés à la fermeture de la bibliothèque ou à la déconnexion. Les six langues
et les deux skins utilisent ces mêmes contrôles natifs.

Quand vous lancez ou relancez explicitement un titre, sa ligne est sélectionnée
et centrée dans la partie visible de la file du lecteur, y compris depuis la
bibliothèque. Près du début ou de la fin, le défilement s'arrête aux limites
de la liste. Lors de l'avance automatique, la file ne bouge pas si le nouveau
titre est déjà visible ; sinon, elle privilégie un décalage d'environ quatre
lignes pour le rendre visible. Vous pouvez parcourir la file librement : les
actualisations de position ne la recentrent pas pendant la lecture.

La molette agit sur la liste survolée dans le lecteur ou la bibliothèque,
même si sa fenêtre n'est pas active. Aucun clic préalable n'est nécessaire.
Au clavier, un appui bref sur **Haut** ou **Bas** déplace la sélection d'une
seule ligne dans le lecteur comme dans la bibliothèque. Garder la touche
enfoncée conserve la répétition ; **Retour arrière** suit la même règle dans
les champs de saisie.

La playlist affichée et la file sont limitées à **500 titres accessibles**.
L'ajout d'une sélection à la file est refusé en entier si les places restantes
ne suffisent pas ; la file et la lecture sont conservées. L'ajout d'une playlist
entière depuis la colonne de gauche garde son fonctionnement précédent : seuls
les titres qui tiennent dans la file sont ajoutés, avec un message de limitation.
Les noms, artistes et durées arrivent progressivement, avec deux requêtes de
métadonnées simultanées au maximum. Dans **Envoyer vers…**, l'ajout à une
playlist, la création et le retrait de titres modifient vos playlists sur Spotify.
**Suppr** effectue également le retrait dans une playlist dont vous êtes propriétaire.
Le catalogue est plafonné à **1 000 playlists** et **2 000 entrées** en comptant
les marqueurs de dossiers ; un dépassement est signalé au lieu de charger une
liste partielle silencieusement.

Dans la bibliothèque, hors du champ de saisie, **Tab** ou **Gauche/Droite** change de panneau ;
**Haut/Bas**, **Début/Fin** et **Page précédente/suivante** parcourent les lignes.
Dans les listes, **Entrée** lance la lecture, **Maj+Entrée** ou **Ctrl+Entrée**
ajoute à la file, et la molette déplace d'une ligne par cran le panneau survolé.
Les raccourcis de presse-papiers **Ctrl/Cmd+C**, **X** et **V** concernent les champs
de recherche et de nom de playlist ; ils ne copient pas les lignes de titres.
**F3**, **Échap** ou la croix ferme cette fenêtre ; lorsque le champ de recherche est actif,
le premier **Échap** quitte seulement la saisie. La lecture du lecteur continue ; les données de bibliothèque
et son buffer graphique sont relâchés à la fermeture.
**F3** depuis le lecteur la rouvre à sa position mémorisée, attachée ou libre.
Si elle était attachée, son emplacement suit les déplacements du lecteur
effectués pendant sa fermeture. Fermer le lecteur ferme également la bibliothèque.

À la fermeture normale de Spotiamp, les positions et l'état ouvert ou fermé de
la bibliothèque sont conservés dans `Spotiamp/window-layout`, à côté des autres
préférences. Ce fichier est petit, borné et remplacé atomiquement ; il ne modifie
ni le skin, ni la qualité audio, ni la session Spotify. Une préférence absente
ou invalide utilise le placement initial. Aucun enregistrement périodique n'est ajouté.
Les positions restent identiques tant que les bandeaux sont accessibles, même
sur plusieurs écrans. Si un écran est débranché, les fenêtres sont replacées
dans une zone accessible. Lors d'un changement d'échelle, le décalage des fenêtres
attachées est adapté ; les fenêtres libres gardent leurs positions absolues
tant qu'elles restent accessibles.

Pour ce lancement uniquement, `--no-library` démarre avec le lecteur seul et
`--library` ouvre la bibliothèque, sans changer la préférence de visibilité.
Une ouverture ou fermeture explicite avec F3, Échap ou la croix est mémorisée.
Comme celui du lecteur, le bandeau de la bibliothèque s'atténue lorsqu'elle
n'est pas active et retrouve ses couleurs lorsqu'elle redevient active.
Les touches **Ctrl**, **F3** et les **flèches** sont réinitialisées aux changements
de focus entre les deux fenêtres. Sous Linux, restaurer une fenêtre réduite
réaffiche l'image conservée, même si son contenu n'a pas changé. La fermeture
libère aussi les ressources natives de la fenêtre.

Les boutons s'activent au relâchement de la souris dans le bouton pressé.
Relâcher à l'extérieur annule le clic ; une perte de focus annule aussi l'appui.
Les barres de volume et de position réagissent dès l'appui et pendant le glissement.

Pour déplacer les fenêtres attachées ensemble, maintenir le bouton gauche de
la souris sur une zone libre du bandeau du lecteur et la faire glisser.
Le bandeau de la bibliothèque la détache au premier déplacement ; un simple
clic conserve l'attachement. Relâcher une fenêtre libre près du bord de l'autre
les recolle automatiquement et aligne les coins proches. Ce comportement est
identique sous Windows, macOS et Linux, dans les deux skins et aux deux échelles.
**Réduire** et **Fermer**, à
droite du bandeau, remplacent les boutons du cadre système. **Alt+F4** continue
de fermer la fenêtre active sous Windows : la bibliothèque seule si elle a le
focus, ou l'ensemble si le lecteur a le focus. La fenêtre garde une taille fixe, à
l'échelle **1×** ou **2×** avec `--scale 2` ; la maximisation est désactivée.

Le bouton **À propos**, près du volume, ou **F2** ouvre un panneau à cinq
onglets : **Spotiamp**, **Crédits**, **Raccourcis**, **Historique** et **Mise à jour**.
**Projet GitHub** ouvre le dépôt dans le navigateur du système.
**Copier les infos** copie la version, le système, l'architecture, le moteur et
la qualité préférée ; ces informations ne contiennent aucune donnée de compte.
Un message confirme la copie ou signale une erreur d'accès au presse-papiers
ou d'ouverture du navigateur.

Dans ce panneau, **Tab/Maj+Tab** parcourt les onglets et boutons, les quatre
**flèches** changent d'onglet et **Entrée/Espace** active le contrôle sélectionné.
**F1** affiche les raccourcis ; **F2**, **Échap**, la croix ou **Fermer** ferment
le panneau. La lecture continue pendant sa consultation ; les commandes du
lecteur restent bloquées derrière le panneau.

Une recherche de mise à jour commence trois secondes après le lancement, une
seule fois par session. À partir de la version **0.1.0**, seules les versions
stables sont proposées ; les releases du projet sont publiées sans le statut
« pre-release ».
La recherche ne propose qu'un paquet
compatible avec le système et l'architecture du lecteur. Une notification
persistante et un repère sur le bouton **À propos** signalent une version
disponible. Son éclair alterne doucement entre doré et vert fluo ; l'infobulle
indique **Mise à jour disponible (F2)**, puis **Mise à jour à installer (F2)**
quand le téléchargement est terminé. Cliquer sur l'éclair ou appuyer sur **F2**
ouvre directement l'onglet **Mise à jour**.
Le clignotement s'arrête pendant le téléchargement et lorsque le panneau
À propos ou l'aide masque le bouton. Il reprend si une mise à jour attend
encore une action. L'animation utilise la boucle et le buffer graphique
existants, sans thread ni moteur graphique supplémentaire.

Entrer dans l'onglet **Mise à jour** lance immédiatement une recherche si aucune
version n'est déjà proposée : après un lancement sans vérification, un résultat
« à jour » ou une erreur. Cela fonctionne aussi lors de l'ouverture directe de
cet onglet, à la souris ou au clavier. Une recherche, un téléchargement ou une
installation en cours continue sans être relancé. Une version déjà disponible
ou un paquet prêt à installer conserve directement son bouton **Télécharger**
ou **Installer / Redémarrer**.

**Vérifier** permet toujours de relancer la recherche manuellement. **Télécharger**
récupère le paquet sur disque et vérifie son empreinte SHA-256 ; la lecture peut
continuer. Aucune vérification périodique ni tâche permanente n'est ajoutée.
Lorsque le paquet est prêt, **Installer / Redémarrer** ferme le lecteur,
remplace l'exécutable puis le relance. La lecture s'arrête ; la file et sa position
sont enregistrées avant la fermeture, puis retrouvées en pause au redémarrage.
Les identifiants Spotify mémorisés dans le coffre du système, la qualité préférée
et le skin choisi sont conservés.
Un échec s'affiche dans le panneau. Le dossier du programme doit être accessible
en écriture pour l'installation. `--no-update-check` désactive la recherche
automatique au démarrage ; ouvrir l'onglet **Mise à jour** ou utiliser **Vérifier**
permet toujours de chercher une nouvelle version.

Sous Windows, l'installateur utilise par défaut `%LOCALAPPDATA%\Programs\Spotiamp` : les mises
à jour intégrées peuvent y remplacer les fichiers sans élévation. Il n'ajoute
aucun service, lancement automatique ni processus résident. Le **S doré** du
panneau À propos et les informations de version sont intégrés à l'exécutable.
Le lecteur et sa bibliothèque utilisent cette même icône pour leurs fenêtres,
la barre des tâches et Alt+Tab, sans fichier d'image externe à charger.
Les raccourcis continuent
de fonctionner après une mise à jour ; la version dans les paramètres Windows
est actualisée au lancement du lecteur installé.

Pour désinstaller, utiliser **Paramètres Windows → Applications → Spotiamp →
Désinstaller**. Fermer le lecteur avant une installation ou désinstallation.
Les réglages dans `%APPDATA%\Spotiamp` et les identifiants du coffre Windows sont
conservés ; **DECONNECTER** dans le lecteur permet d'effacer ces identifiants.
Les fichiers personnels ajoutés à la racine du dossier d'installation sont
conservés. Les sous-dossiers `licenses`, `mpl-source` et `scripts` appartiennent
au programme et sont supprimés intégralement, y compris les fichiers apportés
par les mises à jour ; ne pas y ranger de données personnelles.

L'installation réutilise les réglages et la connexion mémorisée d'une ancienne
copie portable. Elle ne supprime pas cette copie et ne migre pas une file de
lecture en cours, qui n'est pas sauvegardée.

Les boutons **96 / 160 / 320**, à côté de **KB/S** sous la liste, règlent la
qualité demandée. Le choix est mémorisé et appliqué au prochain chargement de
titre : le morceau en cours continue sans coupure. Pour l'appliquer immédiatement,
relancer le morceau par double-clic (il reprend au début). Pause/reprise et
déplacement dans le morceau conservent la qualité de son chargement initial.

**320 kb/s** est le maximum du moteur intégré actuellement.

**Limitation du mode sans perte.** Spotiamp utilise
librespot 0.8, qui ne prend pas en charge l'accès aux flux Spotify sans perte.
Cet accès dépend des protections techniques de Spotify : les mainteneurs de
librespot ont indiqué, après un échange avec Spotify, qu'ils ne peuvent pas
poursuivre les développements qui contournent ces protections
([explication du projet librespot](https://github.com/librespot-org/librespot/issues/1583#issuecomment-3499343020)).
Un abonnement Premium ou l'ajout d'un décodeur FLAC ne suffit donc pas à activer
le sans-perte dans Spotiamp. Cette limitation existait déjà avant le retrait du
décodeur FLAC inutilisé en version 0.1.6.

Spotify propose le sans-perte dans ses applications officielles et sur certains
appareils Spotify Connect compatibles
([appareils pris en charge par Spotify](https://support.spotify.com/fr/article/lossless-audio-quality/)).
Il s'agit d'une limitation de l'intégration actuelle de Spotiamp ; cela ne
signifie pas que le sans-perte est impossible pour tout logiciel ou appareil tiers.

Le moteur peut choisir un format de repli
si le débit demandé n'existe pas pour ce titre. La barre d'état indique donc la
**qualité demandée**, pas une mesure du débit réel. Le réglage est enregistré
dans `%APPDATA%/Spotiamp/audio-quality` sous Windows,
`~/Library/Application Support/Spotiamp/audio-quality` sous macOS et
`$XDG_CONFIG_HOME/Spotiamp/audio-quality` (sinon `~/.config/Spotiamp/audio-quality`)
sous Linux. Ce petit fichier ne contient aucun identifiant.

Le débit réseau audio nominal passe de 160 à 320 kb/s. Cela ne signifie pas que
la RAM double ; l'impact sur les buffers et le décodage reste à mesurer en lecture
réelle. Aucun nouveau moteur, processus ou préchargement n'est ajouté.

Le compte doit avoir Spotify Premium. La session est mémorisée dans le coffre
du système : Gestionnaire d'identifiants Windows, Trousseau macOS ou Secret
Service Linux. Si ce service est indisponible, la connexion reste limitée à
la session courante ; il n'y a pas de repli vers un fichier de jetons en clair.
La connexion est annoncée après la sauvegarde de la session, ou avec un message
si la sauvegarde n'a pas pu être confirmée.
Fermer la fenêtre conserve cette session : au prochain lancement, une tentative
de reconnexion s'effectue en arrière-plan, sans démarrer la musique.
Si aucune session n'est enregistrée, si le coffre est inaccessible ou si la
reconnexion échoue, le navigateur ne s'ouvre pas automatiquement ; le bouton
**CONNEXION** reste disponible. Un échec réseau n'efface pas la session mémorisée.
**ANNULER** interrompt une tentative de connexion sans oublier le compte.
**DECONNECTER** supprime les identifiants mémorisés : après effacement réussi,
la reconnexion automatique n'aura plus lieu au lancement suivant. Le navigateur du système
sert à l'autorisation Spotify et au lien **Projet GitHub** du panneau À propos ;
il n'est pas embarqué dans le lecteur.

Les liens acceptés sont `https://open.spotify.com/track/...`, `/album/...`,
`/playlist/...`, y compris les URL `intl-fr`, et les URI `spotify:track:...`,
`spotify:album:...`, `spotify:playlist:...`. Les liens raccourcis ne sont pas
résolus. Les playlists sont limitées aux titres accessibles renvoyés par Spotify.

Raccourcis du lecteur : Espace lecture/pause, flèches sélection, Entrée lecture
ou ajout, Suppr retirer, Tab champ de lien, F1 aide, F2 À propos, F3 bibliothèque,
F4 changer de skin, Échap fermer
l'aide ou le panneau. Sous macOS, Cmd+V colle également un lien. Le texte utilise
une petite police bitmap : les glyphes absents sont affichés avec `?`.

## Choix pour limiter la mémoire

- Rendu logiciel bitmap sur le CPU, affichage **Win32/GDI sous Windows**,
  sans dépendance à l'accélération 3D pour l'interface du lecteur.
- Deux skins intégrés utilisant les mêmes fenêtres et buffers, aucune WebView
  ni moteur de navigateur. Le changement de skin ne crée pas de nouvelle fenêtre.
- Un processus pendant la lecture ; le réseau et l'audio restent hors du thread
  d'interface. Un processus temporaire remplace l'exécutable lors d'une mise à jour.
- Recherche de mise à jour ponctuelle, sans thread ni client HTTP gardé au repos.
  Téléchargement sur disque et progression affichée dans le buffer existant.
- Pas de pochettes, de visualiseur, de cache audio persistant ni de téléchargement
  des titres de toutes les playlists. Le catalogue de playlists est paginé,
  et une seule playlist est chargée pour l'affichage à la fois.
- Bibliothèque ouverte au lancement, désactivable avec `--no-library`. Son
  framebuffer, le catalogue et les titres affichés sont relâchés à la fermeture ; la file
  du lecteur reste indépendante. Les données partagées utilisent des `Arc` et
  une révision évite de recopier ou parcourir les titres à chaque image.
- Accrochage des fenêtres géré par les événements natifs, sans thread ni
  redessin périodique supplémentaire. Garder la bibliothèque ouverte consomme
  ses buffers graphiques et les données du catalogue affiché ; les mesures du
  lecteur seul doivent donc préciser que la bibliothèque est fermée.
- File limitée à 500 titres, deux requêtes de métadonnées simultanées.
- Recherche limitée à 50 résultats, sans pochettes ni cache persistant,
  réutilisant la connexion et le moteur asynchrone existants. Aucun thread
  supplémentaire ni requête à chaque caractère saisi.
- Sélections bornées à 500 titres, une seule création ou un seul ajout à une
  playlist Spotify à la fois.
  Le glisser-déposer réutilise les événements souris et la boucle d'interface ;
  aucune nouvelle pile réseau, dépendance résolue ni tâche permanente.
- Buffer graphique réutilisé ; rendu seulement lorsque l'état visible change.
- Pas de préchargement du morceau suivant ; cela peut introduire un silence
  entre les titres. Les buffers internes de librespot ne sont pas tous bornés
  par Spotiamp ; une grande réponse de playlist peut causer un pic temporaire.
- Un worker asynchrone pour le moteur, plus son thread de lecture et les
  threads nécessaires aux chargements et à la sortie audio. Ce plafond est
  documenté dans `vendor/librespot-playback/PATCH.md`.

Le lecteur comprend connexion, import par lien, recherche de musiques, bibliothèque
de playlists Spotify, file mémorisée, lecture locale, pause, arrêt, précédent/suivant,
volume et déplacement dans le titre. Il n'inclut pas encore de vue albums/artistes/
titres likés, d'égaliseur, de skins `.wsz`, de lecture hors ligne, d'icône de
notification ni de touches multimédia globales. L'accessibilité et la saisie complexe sont
limitées par cette interface bitmap.
Un déplacement dans le titre peut laisser entendre brièvement les données
déjà présentes dans le petit buffer de sortie.

Ce client utilise une intégration Spotify non officielle ; une modification
du service peut interrompre son fonctionnement. Il n'est affilié ni à Spotify
ni à Winamp. Le graphisme est dessiné dans le code ; aucun fichier de skin
Winamp n'est redistribué.

## Compiler

Installer Rust avec rustup et les outils natifs de compilation. Le fichier
`rust-toolchain.toml` fixe Rust **1.95.0**, avec rustfmt et Clippy.

```sh
cargo build --release --locked --bin spotiamp
cargo test --locked
cargo test --locked -p librespot-playback --lib quality_tests
```

Les tests du défilement couvrent l'accès aux titres entre deux vues, les crans,
les fractions, les rafales et les limites de liste. Sous Windows, `cargo test`
injecte aussi des événements de molette natifs dans une fenêtre de test.
Les tests de qualité couvrent la persistance, la conservation du flux en cours,
la demande liée à chaque chargement et l'invalidation du précédent flux quand
le débit change. Ils fonctionnent hors ligne ; le débit servi par Spotify doit
encore être vérifié pendant une lecture authentifiée.
Le test X11 demande un écran virtuel et `xdotool` et se lance explicitement :

```sh
LC_ALL=C XMODIFIERS=@im=none xvfb-run -a cargo test --locked --bin spotiamp x11_wheel_press_and_release_count_as_one_notch -- --ignored
```

Les tests d'interface vérifient les états visuels pressé/relâché, l'annulation
hors bouton, les contrôles désactivés et la perte de focus. Ceux du panneau
À propos couvrent les onglets, la navigation clavier, les boutons et le blocage
des interactions avec le lecteur en arrière-plan. Ceux des mises à jour couvrent
les états et actions disponibles, la capture souris et le rendu sans remplacer
le buffer graphique. Un appui sur **Télécharger** ne devient pas un clic sur
**Installer** si l'état change avant le relâchement. Les tests natifs Windows et
X11 injectent aussi des clics courts et vérifient leurs coordonnées en taille
double. La CI exécute les tests X11 dans Xvfb, avec `xdotool`.

Les tests de bibliothèque utilisent des données synthétiques : sélection et
défilement dans chaque panneau, changements de liste pendant un chargement,
double-clic sur le bon titre, commandes désactivées et conservation du framebuffer.
Ils ne remplacent pas un essai authentifié de découverte des playlists et de
lecture Spotify. Ceux de la recherche couvrent les limites et l'encodage des
requêtes, les réponses anciennes, l'annulation, la sélection du bon résultat,
la conservation de la lecture pendant la saisie et les traductions. Les sélections
multiples, l'ordre des titres, la capacité de la file et les messages sont également
vérifiés hors connexion. Les réponses de création et de modification de playlist
sont simulées, y compris les refus et l'absence de confirmation après envoi.
Les retraits vérifient aussi les droits du propriétaire, les révisions et les
occurrences répétées. La recherche, la création, l'ajout et le retrait réels
doivent encore être essayés avec un compte Spotify
connecté ; les tests automatisés n'en constituent pas une validation.

Windows : Rust avec la cible `x86_64-pc-windows-msvc` et les outils de compilation
Visual Studio C++. macOS : outils Xcode. Linux Debian/Ubuntu :

```sh
sudo apt install build-essential pkg-config libasound2-dev libdbus-1-dev libx11-dev libxcursor-dev libxrandr-dev
```

Linux utilise X11, ou XWayland dans une session Wayland. L'audio passe par
Rodio/CPAL : WASAPI sous Windows, CoreAudio sous macOS, ALSA sous Linux.
La connexion OAuth écoute temporairement sur `127.0.0.1:5588`.

```sh
cargo run --release -- --scale 2
# Interface seule : lecture et mises à jour désactivées dans ce binaire.
cargo build --release --no-default-features --bin spotiamp
# Initialisation du moteur hors ligne, sans morceau ni sortie sonore réelle.
cargo run --release --bin engine_probe
```

Le workflow `.github/workflows/build.yml` automatise la compilation native,
les tests et les paquets sur **Windows x64, macOS Apple Silicon et Linux x64**.
La [CI de la version 0.1.6](https://github.com/roukmoute/spotiamp/actions/runs/39702450154)
a réussi sur les trois systèmes, avec vérification des archives par le moteur
de mise à jour. Elle teste aussi les fenêtres natives macOS, les interactions
X11 et l'installation, la mise à jour et la désinstallation sous Windows.
La compilation Windows MSVC du workflow utilise le runtime C statique
(`-C target-feature=+crt-static`).
Les scripts `measure-windows.ps1` et `measure-linux.py` enregistrent les mesures
brutes ; une étiquette `playing` ne prouve pas qu'une musique joue réellement.

## Licences

Code Spotiamp : MIT. Les dépendances gardent leurs licences propres. Le paquet
binaire inclut leurs notices ainsi que les sources exactes des composants MPL
concernés. `Cargo.lock` fixe les versions pour reconstruire le programme.
