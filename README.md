# Spotiamp — un petit lecteur Spotify classique

Prototype natif en Rust, inspiré de l'interface Winamp. Windows est la cible
prioritaire ; le même code vise macOS et Linux. Ce projet expérimente une
interface très sobre et plusieurs pistes de moteur audio. Le moteur actuel
est librespot 0.8 : il reste remplaçable si les mesures réelles ne conviennent pas.

**État : prototype expérimental.** Un premier essai utilisateur sous Windows
montre une session connectée, une playlist chargée et un morceau en état de
lecture. Sa capture affiche 5,7 Mo de mémoire et 0,4 % de CPU à cet instant ;
ce n'est pas une mesure de la mémoire totale ni un test de stabilité prolongé.
Les résultats et leurs limites sont dans `PERFORMANCE.md`.

**[Télécharger la préversion Windows et ses sources](https://github.com/roukmoute/spotiamp/releases).**
Choisir l'archive `Spotiamp-Windows-x64.zip`, puis la décompresser.

La préversion **0.1.0-alpha.7** retire le **cadre et la barre de titre du système**
sous Windows, macOS et Linux. Les boutons **Réduire** et **Fermer** font partie
du bandeau classique de Spotiamp, qui permet aussi de déplacer la fenêtre.
Ce changement conserve le rendu bitmap existant, sans dépendance ni service ajouté.

Les **mises à jour intégrées** restent accessibles dans l'onglet **Mise à jour**
du panneau **À propos** : détection d'une nouvelle version, téléchargement,
puis installation et redémarrage à votre demande.
Les touches métalliques, boutons enfoncés pendant l'appui, cadres encastrés et
poignées rainurées sont conservés, comme la qualité **320 kb/s par défaut** et
la molette d'un titre par cran.

![Interface de Spotiamp](preview.png)

L'archive Windows fournie vise **Windows 10/11 x64** (processeur Intel/AMD
64 bits), conformément au [socle de la cible Rust](https://doc.rust-lang.org/rustc/platform-support.html).
Le binaire Windows publié en préversion est portable et non signé. La CI le
compile avec le runtime C statique. L'interface
minifb utilise Win32/GDI et l'audio WASAPI. Aucun environnement Rust n'est requis
pour lancer l'exécutable. Le premier essai Windows porte sur le prototype
précédemment nommé Spool ; le paquet Spotiamp est reconstruit après renommage.

## Utilisation

1. Décompresser l'archive Windows et lancer `spotiamp.exe`.
2. Cliquer sur **CONNEXION**. Autoriser Spotify dans le navigateur, puis revenir
   dans le lecteur. Aucun mot de passe n'est demandé dans Spotiamp.
3. Copier un lien Spotify de titre, d'album ou de playlist. Dans Spotiamp :
   **Ctrl+V**, puis **+ AJ.** ou Entrée.
4. Double-cliquer sur un titre. Les boutons et la barre de position pilotent
   le moteur audio local. Le client Spotify officiel peut être fermé.

Les boutons s'activent au relâchement de la souris dans le bouton pressé.
Relâcher à l'extérieur annule le clic ; une perte de focus annule aussi l'appui.
Les barres de volume et de position réagissent dès l'appui et pendant le glissement.

Pour déplacer la fenêtre, maintenir le bouton gauche de la souris sur une zone
libre du bandeau supérieur et la faire glisser. **Réduire** et **Fermer**, à
droite du bandeau, remplacent les boutons du cadre système. **Alt+F4** continue
de fermer le lecteur sous Windows. La fenêtre garde une taille fixe, à
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
seule fois par session. Cette version alpha inclut les préversions dans sa
recherche ; une future version stable ne proposera que les versions stables.
La recherche ne propose qu'un paquet
compatible avec le système et l'architecture du lecteur. Une notification
persistante et un repère sur le bouton **À propos** signalent une version
disponible ; **F2** ouvre alors directement l'onglet **Mise à jour**.

Dans cet onglet, **Vérifier** relance la recherche. **Télécharger** récupère le
paquet sur disque et vérifie son empreinte SHA-256 ; la lecture peut continuer.
Lorsque le paquet est prêt, **Installer / Redémarrer** ferme le lecteur,
remplace l'exécutable puis le relance. **La lecture s'arrête et la file est
perdue**, car elle n'est pas encore sauvegardée. Les identifiants Spotify déjà
mémorisés dans le coffre du système et la qualité préférée sont conservés.
Un échec s'affiche dans le panneau. Le dossier du programme doit être accessible
en écriture pour l'installation. `--no-update-check` désactive la recherche
automatique au démarrage ; **Vérifier** reste disponible.

Les boutons **96 / 160 / 320**, à côté de **KB/S** sous la liste, règlent la
qualité demandée. Le choix est mémorisé et appliqué au prochain chargement de
titre : le morceau en cours continue sans coupure. Pour l'appliquer immédiatement,
relancer le morceau par double-clic (il reprend au début). Pause/reprise et
déplacement dans le morceau conservent la qualité de son chargement initial.

**320 kb/s** est le maximum du moteur intégré actuellement ; le mode Spotify
sans perte n'est pas pris en charge. Le moteur peut choisir un format de repli
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
**DECONNECTER** supprime les identifiants mémorisés. Le navigateur du système
sert à l'autorisation Spotify et au lien **Projet GitHub** du panneau À propos ;
il n'est pas embarqué dans le lecteur.

Les liens acceptés sont `https://open.spotify.com/track/...`, `/album/...`,
`/playlist/...`, y compris les URL `intl-fr`, et les URI `spotify:track:...`,
`spotify:album:...`, `spotify:playlist:...`. Les liens raccourcis ne sont pas
résolus. Les playlists sont limitées aux titres accessibles renvoyés par Spotify.

Raccourcis du lecteur : Espace lecture/pause, flèches sélection, Entrée lecture
ou ajout, Suppr retirer, Tab champ de lien, F1 aide, F2 À propos, Échap fermer
l'aide ou le panneau. Sous macOS, Cmd+V colle également un lien. Le texte utilise
une petite police bitmap : les glyphes absents sont affichés avec `?`.

## Choix pour limiter la mémoire

- Rendu logiciel bitmap, un seul thème, aucune WebView ni moteur de navigateur.
- Un processus pendant la lecture ; le réseau et l'audio restent hors du thread
  d'interface. Un processus temporaire remplace l'exécutable lors d'une mise à jour.
- Recherche de mise à jour ponctuelle, sans thread ni client HTTP gardé au repos.
  Téléchargement sur disque et progression affichée dans le buffer existant.
- Pas de pochettes, de visualiseur, de cache audio persistant ni de téléchargement
  de la bibliothèque complète.
- File limitée à 500 titres, deux requêtes de métadonnées simultanées.
- Buffer graphique réutilisé ; rendu seulement lorsque l'état visible change.
- Pas de préchargement du morceau suivant ; cela peut introduire un silence
  entre les titres. Les buffers internes de librespot ne sont pas tous bornés
  par Spotiamp ; une grande réponse de playlist peut causer un pic temporaire.
- Un worker asynchrone pour le moteur, plus son thread de lecture et les
  threads nécessaires aux chargements et à la sortie audio. Ce plafond est
  documenté dans `vendor/librespot-playback/PATCH.md`.

La première version comprend connexion, import par lien, liste, lecture locale,
pause, arrêt, précédent/suivant, volume et déplacement dans le titre. Elle
n'inclut pas encore recherche de catalogue, navigation de bibliothèque, égaliseur,
skins `.wsz`, lecture hors ligne, icône de notification, touches multimédia
globales ni sauvegarde de la file. L'accessibilité et la saisie complexe sont
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

Le workflow `.github/workflows/build.yml` automatise les tests et artefacts sur
les trois systèmes. Les tests ont réussi sur Windows x64, Linux x64 et macOS
Apple Silicon lors de la [première exécution](https://github.com/roukmoute/spotiamp/actions/runs/39630012360).
La compilation Windows MSVC du workflow utilise le runtime C statique
(`-C target-feature=+crt-static`).
Les scripts `measure-windows.ps1` et `measure-linux.py` enregistrent les mesures
brutes ; une étiquette `playing` ne prouve pas qu'une musique joue réellement.

## Licences

Code Spotiamp : MIT. Les dépendances gardent leurs licences propres. Le paquet
binaire inclut leurs notices ainsi que les sources exactes des composants MPL
concernés. `Cargo.lock` fixe les versions pour reconstruire le programme.
