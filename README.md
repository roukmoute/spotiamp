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

![Interface de Spotiamp](preview.png)

L'archive Windows fournie vise **Windows 10/11 x64** (processeur Intel/AMD
64 bits), conformément au [socle de la cible Rust](https://doc.rust-lang.org/rustc/platform-support.html).
Le binaire GNU publié en préversion est portable et non signé. Ses imports ont
été inspectés : seules des DLL système Windows sont requises, sans DLL MinGW
supplémentaire. L'interface
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

Le compte doit avoir Spotify Premium. La session est mémorisée dans le coffre
du système : Gestionnaire d'identifiants Windows, Trousseau macOS ou Secret
Service Linux. Si ce service est indisponible, la connexion reste limitée à
la session courante ; il n'y a pas de repli vers un fichier de jetons en clair.
**DECONNECTER** supprime les identifiants mémorisés. Le navigateur est utilisé
pour l'autorisation seulement ; il n'est pas embarqué dans le lecteur.

Les liens acceptés sont `https://open.spotify.com/track/...`, `/album/...`,
`/playlist/...`, y compris les URL `intl-fr`, et les URI `spotify:track:...`,
`spotify:album:...`, `spotify:playlist:...`. Les liens raccourcis ne sont pas
résolus. Les playlists sont limitées aux titres accessibles renvoyés par Spotify.

Raccourcis : Espace lecture/pause, flèches sélection, Entrée lecture ou ajout,
Suppr retirer, Tab champ de lien, F1 aide, Échap fermer l'aide. Sous macOS,
Cmd+V colle également un lien. Le texte utilise une petite police bitmap :
les glyphes absents sont affichés avec `?`.

## Choix pour limiter la mémoire

- Rendu logiciel bitmap, un seul thème, aucune WebView ni moteur de navigateur.
- Un seul processus ; le réseau et l'audio restent hors du thread d'interface.
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
```

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
# Mesure d'interface uniquement : aucune lecture disponible dans ce binaire.
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
