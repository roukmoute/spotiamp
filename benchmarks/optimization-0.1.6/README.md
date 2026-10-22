# Comparaison des profils pour Spotiamp 0.1.6

Ces mesures Linux servent à choisir le profil de compilation. Elles ne mesurent
ni la lecture Spotify authentifiée, ni Windows, ni la sortie audio.

## Méthode

- Sources Spotiamp 0.1.5 identiques pour `baseline-thin3`, `fat3`, `fats` et `fatz`.
  `pruned-fat3` ajoute seulement la suppression des messages détaillés passant
  par `log` en release (sans filtrer tous les événements `tracing`) et du décodeur
  FLAC, inutilisé dans la sélection des pistes Spotify.
- Profils : opt-level 3/thin LTO, 3/fat LTO, s/fat LTO et z/fat LTO. Tous utilisent
  codegen-units=1, panic=abort et strip=symbols. Le compilateur est Rust 1.95.0.
- Démarrage : huit lancements par variante, ordre alterné déterministe, cache de
  fichiers chaud. Mesure jusqu'à la fenêtre gérée par Openbox et un pixel vert
  réellement peint dans le compteur. Interrogation Xlib toutes les 5 ms.
- Repos : fenêtre 560×470 sur le même Xvfb/Openbox, 20 s de stabilisation puis
  30 s de mesures toutes les 500 ms. CPU utilisateur+système du processus,
  exprimé en pourcentage d'un cœur. RSS, PSS et mémoire résidente privée viennent
  de `/proc/PID/smaps_rollup`. La mémoire privée résidente Linux n'est pas
  l'indicateur Windows « Private Bytes ».
- Chaque lancement reçoit des dossiers XDG isolés et une adresse D-Bus
  inexistante. `--no-update-check` désactive les requêtes de mise à jour. Aucun
  compte Spotify, navigateur, bibliothèque personnelle ou périphérique audio
  n'est utilisé par le banc. Les compilations sont arrêtées pendant les mesures.
- Décodage : signal original de 30 s, stéréo 44,1 kHz, mélange de sinusoïdes,
  chirps et bruit déterministe, encodé par libsndfile 1.2.2. Le probe appelle le
  `SymphoniaDecoder` de la même dépendance `librespot-playback` compilée avec
  chaque profil, y compris sa conversion des paquets en PCM f64. Ogg Vorbis et
  MP3 couvrent les formats conservés ; FLAC est un contrôle des profils complets.
- Avant chronométrage, un décodage distinct écrit chaque échantillon PCM f64 et
  compare son SHA256 complet. Les fichiers PCM temporaires ne sont pas livrés.
  Le benchmark ne calcule pas ce hash : il décode 100 fois le fichier et conserve
  les résultats via `black_box`. Trois répétitions par couple profil/format sont
  entrelacées ; CPU du processus enfant mesuré par `getrusage`.

Les résultats détaillés et les identités des binaires sont dans
`results-linux.json`. Les différences de quelques millisecondes au démarrage ou
de quelques pages mémoire ne sont pas une preuve de gain reproductible. Les
processus Xvfb/Openbox sont exclus des mesures et identiques pour toutes les
variantes. Le poste conserve ses activités de fond habituelles.

## Reproduire le banc

Les scripts utilisent Python 3 avec `ctypes`, Xvfb, Openbox, `xprop`, `xdotool`,
libsndfile 1.2.2 avec encodeurs Vorbis/MP3 et un compilateur Rust identique.
`measure-runtime.py --help` décrit les paramètres ; chaque sortie doit être un
nouveau dossier. Exemple :

```sh
python3 measure-runtime.py \
  --binary baseline=/chemin/spotiamp-baseline \
  --binary candidat=/chemin/spotiamp-candidat \
  --output runtime-resultats --runs 8 --idle-repeats 2 --warmup 20 --seconds 30
```

Les versions comparées ici s'identifient comme 0.1.5 ; adapter la vérification
explicite de version et le préfixe de titre pour comparer une autre version.
Le script accepte `--wm-root` pour un arbre de paquets Openbox extrait sans
installation ; sinon il utilise Openbox du système.

`make-fixtures.py` génère les trois fichiers synthétiques et leur manifeste.
Un identifiant de conteneur Ogg peut varier entre générations : toutes les
variantes doivent partager les mêmes fichiers au cours d'une comparaison.

Les sources de base sont le commit
`f5219186c825cc5bfb92f3230ef241932a467b5e`. Créer des copies séparées avec
`git archive`. Appliquer `pruned-source.patch` sur une copie propre pour
`pruned-fat3`, ou le patch combiné `mixed-source.patch` sur une autre copie
propre pour `mixeds` : ne pas appliquer ces deux patches successivement.
Les fichiers de provenance joints identifient leur base et leurs empreintes.
Aucune modification n'est nécessaire dans le checkout actif.

`build-benchmark.py` construit une copie explicite, crée `builds/PROFIL/build.json`,
l'inventaire Cargo `dependency-artifacts.json` et un instantané des rlibs dans
`deps/`. Il n'exécute pas l'application. Les dépendances natives habituelles du
projet doivent être installées ; `--offline` peut être ajouté si Cargo dispose
déjà du cache complet. Exemple :

```sh
python3 build-benchmark.py --source /chemin/copiedesources \
  --source-commit f5219186c825cc5bfb92f3230ef241932a467b5e --profile baseline-thin3
python3 build-decode-probe.py baseline-thin3
```

`build-decode-probe.py PROFIL` lie le probe aux rlibs exactes avec les mêmes
options d'optimisation du binaire racine. Les crates ayant des overrides gardent
leurs options de compilation enregistrées par Cargo. Puis :

```sh
python3 measure-decode.py --profile baseline-thin3 --profile pruned-fat3 \
  --format ogg --format mp3 --passes 100 --runs 3 --output decode-resultats
```

`--verify-only` exécute seulement la vérification PCM complète. Les binaires,
rlibs, fixtures et fichiers PCM ne sont pas inclus dans ce dossier.
