# Mesures et choix techniques

Spotiamp est distribué en binaires natifs pour **Windows x64, macOS Apple Silicon
et Linux x64**. La CI compile, teste et prépare les paquets sur les trois systèmes.
Les mesures ci-dessous précisent le système et le scénario de chaque essai.

Mesures réalisées le 21 octobre 2026 sur Linux x86-64, 16 processeurs logiques,
Rust 1.95.0, compilation `release`. **Aucun compte Spotify n'a été authentifié
pendant ces mesures Linux.** Un premier retour Windows est décrit séparément
ci-dessous ; aucune série de mesures Windows en lecture n'est encore disponible.

## Interface Windows : rendu logiciel et GDI

**L'interface de Spotiamp ne requiert pas d'accélération graphique sous Windows.**
Les boutons, textes et listes sont dessinés par le CPU dans un buffer de pixels
réutilisé. Le backend Windows de minifb affiche ce bitmap avec
[`StretchDIBits`](https://learn.microsoft.com/en-us/windows/win32/api/wingdi/nf-wingdi-stretchdibits),
une fonction de **GDI**, le système graphique classique de Windows.
Le chemin est visible dans le [backend embarqué](vendor/minifb/src/os/windows/mod.rs) ;
le [lecteur](src/main.rs) ne redessine le buffer que lorsque l'état visible change.
L'interface ne crée ni contexte OpenGL ni périphérique Direct3D.

Ce choix évite d'initialiser un moteur de rendu 3D dans le lecteur. Il concerne
le rendu de Spotiamp, pas l'ensemble du système : le
[compositeur de bureau Windows](https://learn.microsoft.com/en-us/windows/win32/dwm/dwm-overview)
peut utiliser le GPU pour afficher les fenêtres. Il ne s'agit donc pas d'une
promesse de « zéro utilisation du GPU » dans le Gestionnaire des tâches.
GDI est ici spécifique à Windows ; les versions macOS et Linux utilisent
leurs propres backends d'affichage.

### Comparaison avec Spotifast

Au commit [`e86d3de`](https://github.com/crmne/spotifast/tree/e86d3de0cb72edba8f24b447218daa7f3af1f127),
consulté le 22 octobre 2026, Spotifast configure **egui/eframe avec Glow**, son
moteur de rendu **OpenGL** : voir les
[dépendances activées](https://github.com/crmne/spotifast/blob/e86d3de0cb72edba8f24b447218daa7f3af1f127/Cargo.toml#L31-L36)
et l'[initialisation du rendu](https://github.com/crmne/spotifast/blob/e86d3de0cb72edba8f24b447218daa7f3af1f127/src/entrypoint.rs#L643-L659).
Spotiamp dessine son skin sur le CPU et présente les pixels avec GDI sous Windows.
Les deux applications sont natives et n'embarquent pas de navigateur.
OpenGL peut lui aussi fonctionner en rendu logiciel, par exemple avec
[Mesa LLVMpipe](https://docs.mesa3d.org/drivers/llvmpipe.html) : son utilisation
ne signifie pas qu'une carte graphique dédiée est obligatoire.

Cette différence décrit leur architecture graphique. Elle ne suffit pas à
attribuer à OpenGL un écart de consommation mémoire entre les deux applications :
les fonctionnalités, buffers, caches et conditions de mesure comptent aussi.

## Réduction du binaire — 0.1.6

La compilation release utilise désormais **LTO complet avec `opt-level = 3`**.
L'optimiseur peut ainsi travailler entre toutes les bibliothèques tout en
conservant les optimisations de vitesse. Le niveau `z`, orienté taille et
désactivant notamment la vectorisation des boucles, n'est pas choisi par défaut
([documentation Cargo](https://doc.rust-lang.org/cargo/reference/profiles.html#opt-level)).
Les symboles étaient déjà retirés, le runtime C Windows était déjà statique,
et les paniques terminaient déjà le processus : ces réglages sont conservés.

Le décodeur FLAC a été retiré : aucun des formats choisis par le chargeur de ce
lecteur, y compris ses replis, ne l'utilisait. Vorbis et MP3 restent présents,
avec les qualités 96 / 160 / 320 kb/s. Cette application ne propose pas de lecture
sans perte ni de fichiers locaux. Les traces `info`, `debug` et `trace` passant par
la bibliothèque `log` sont exclues en release ; avertissements, erreurs et
messages utilisateur sont conservés. Aucun buffer audio n'est réduit et aucun traitement
de décompression du programme n'est ajouté au lancement.

Les comparaisons de profils utilisent les sources de la 0.1.5 au commit
`f5219186c825cc5bfb92f3230ef241932a467b5e`, le même compilateur Rust 1.95.0 et
le même environnement Linux. Le patch des dépendances est enregistré à part.
Ces variantes expérimentales portent donc encore le numéro 0.1.5 ; elles ne
sont pas présentées comme les binaires définitifs de la release 0.1.6.
La validation native Windows compare ensuite l'exécutable officiel 0.1.5 au
véritable nouveau binaire de chaque exécution CI, sur le même runner.

| Variante Linux, même poste | Octets du binaire | Écart à la référence |
|---|---:|---:|
| Référence : niveau 3, Thin LTO | 9 386 480 | — |
| Niveau 3, LTO complet | 7 649 504 | −18,51 % |
| **Retenue : niveau 3, LTO complet, code inutilisé retiré** | **7 588 544** | **−19,15 %** |
| Niveau s, LTO complet | 5 608 768 | −40,25 % |
| Niveau z, LTO complet | 5 312 440 | −43,40 % |
| Mixte : niveau s, codecs/moteur niveau 3, code retiré | 5 721 920 | −39,04 % |

Le décodage de signaux synthétiques stéréo de 30 secondes produit exactement
les mêmes bits PCM en Vorbis et MP3 dans toutes ces variantes. La variante
retenue conserve un temps CPU comparable à la référence. Les variantes s et z
demandent nettement plus de CPU ; même le profil mixte demande environ 31 % de
plus en Vorbis et 66 % en MP3 que la variante retenue lors de leur comparaison.
Ces écarts concernent le **décodeur seul**, sans réseau, déchiffrement Spotify,
sortie audio ni interface ; ils ne sont pas des pourcentages de charge du lecteur
complet. Le choix conserve donc la vitesse de décodage au prix d'un binaire un
peu plus grand que les profils orientés exclusivement vers la taille.

Les paramètres, patches, empreintes, échantillons et scripts du banc Linux
sont conservés dans [benchmarks/optimization-0.1.6](benchmarks/optimization-0.1.6/README.md).
Sur huit lancements par variante, le premier rendu médian passe de 42,54 à
43,83 ms ; l'interrogation toutes les 5 ms ne permet pas d'attribuer de sens à
cet écart. Lors des deux essais de repos de 30 secondes après 20 secondes de
stabilisation, le candidat utilise environ 10,1 Mio RSS et 6,1 Mio privés
résidents, avec deux threads et 0,23 % d'un cœur CPU. La référence utilise
10,2–10,4 Mio RSS et 0,23–0,27 % d'un cœur. Aucun surcoût significatif n'est
observé ici ; la réduction de RAM reste modeste par rapport au gain sur disque.

La taille du fichier, celle du ZIP, la mémoire résidente et la mémoire privée
engagée sont des mesures différentes. Une réduction du code sur disque
n'entraîne pas une baisse proportionnelle de RAM : le système ne charge que les
pages nécessaires et le lecteur conserve ses mêmes buffers et objets en mémoire.
Le benchmark Windows est décrit dans [scripts/README.md](scripts/README.md).
Les résultats sont joints à chaque CI dans `spotiamp-windows-performance`, avec
les empreintes exactes des deux exécutables. Aucun compte Spotify n'est utilisé ;
ces observations ne valident pas la consommation en lecture connectée prolongée.

## Résultats historiques — premier prototype

RSS est la mémoire résidente du processus. Mio = 1 048 576 octets. La mémoire
privée ci-dessous est la partie privée résidente sous Linux ; ce n'est pas le
compteur Windows « Private Bytes », qui mesure la mémoire privée engagée.
Les fichiers JSON bruts, contenant les empreintes des exécutables quand
disponibles, sont conservés dans `benchmarks/`.

| Programme / état exact | RSS médiane | Privée résidente | Threads observés |
|---|---:|---:|---:|
| Spotiamp, interface seule, moteur exclu à la compilation | 5,89 Mio | 3,09 Mio | 1 |
| Spotiamp complet, fenêtre ouverte, déconnecté | 8,60 Mio | 4,90 Mio | 2 |
| librespot Rust 0.8, Session + Player initialisés, sans réseau, sortie factice | 4,67 Mio | 2,63 Mio | 18 |
| Même binaire, `TOKIO_WORKER_THREADS=1` | 4,33 Mio | 2,38 Mio | 3 |
| Sonde finale avec patch embarqué, hors ligne | 4,39 Mio | 2,39 Mio | 3 |
| Spotiamp final, fenêtre ouverte, déconnecté | 8,84 Mio | 5,18 Mio | 2 |
| librespot-c 0.9 initialisé, sans décodeur ni sortie audio | 8,80 Mio | 2,56 Mio | 2 |
| cspot en attente de connexion, sortie ALSA nulle | 6,73 Mio | 3,09 Mio | 7 |

Les quatre premières lignes ont été échantillonnées toutes les 500 ms après
3 secondes de stabilisation, pendant 7 à 10 secondes. L'interface seule a
consommé 0,02 seconde CPU entre le premier et le dernier échantillon ; le
lecteur déconnecté 0,03 seconde. Ce sont de courts essais au repos, pas un test
de stabilité de plusieurs heures.

La mesure finale de Spotiamp porte sur 18 échantillons pendant neuf secondes,
après trois secondes de stabilisation. Le CPU cumulé a augmenté de 0,01 seconde
entre le premier et le dernier échantillon. Les JSON `*-final-*` correspondent
aux exécutables Linux finaux.

La sortie audio native a également été exercée avec du PCM synthétique et une
sortie ALSA `null` : deux cycles démarrage/écriture/arrêt réussis, puis deux
tentatives sans périphérique renvoyant une erreur sans arrêter le processus.
La sonde release consomme environ 3,59 Mio RSS, **sans décodeur, réseau ni
interface**. Le périphérique nul n'a pas l'horloge d'une vraie carte son :
le CPU mesuré est élevé (7,39 secondes sur environ 9,5 secondes de fenêtre),
et ne constitue pas un benchmark de lecture sur un périphérique physique.
Ces essais vérifient le fonctionnement et les erreurs du chemin de sortie.

Les deux alternatives ont été échantillonnées pendant six secondes. cspot a
consommé environ 0,19 seconde CPU sur ces six secondes ; librespot-c était
sous la résolution de mesure de 0,01 seconde. Les détails de compilation,
commits et dépendances sont dans `benchmarks/alternatives/alternative-findings.txt`.

**Ces lignes ne constituent pas un classement des moteurs en lecture.** Les
fonctions actives et bibliothèques partagées diffèrent. La fenêtre virtuelle
Xvfb est exclue de la mémoire du processus, comme le compositeur du bureau.
Un éventuel navigateur d'autorisation n'est pas ouvert dans ces mesures.

## Ce que les essais changent

L'annonce de 100–250 Mo du [README de Spotifast](https://github.com/crmne/spotifast)
concerne son application entière. Elle n'établit pas un minimum pour librespot.
Son interface egui utilise déjà un rendu natif, sans navigateur embarqué ;
il ne faut donc pas expliquer sa consommation par Electron.

Spotiamp conserve Rust pour l'application et librespot comme **premier moteur
testable**, tout en gardant une interface de commandes et un état indépendants
du moteur. Le langage ne décide pas à lui seul de l'empreinte mémoire. Les
essais d'alternatives en C/C++ servent à éclairer un remplacement éventuel.

Le runtime interne de librespot créait un worker par processeur logique. La
version embarquée conserve le thread dédié au lecteur et utilise un seul worker
asynchrone. Le gain constaté au repos est modeste, environ 0,34 Mio : il ne
faut pas le transformer en promesse de réduction spectaculaire. Les threads
de chargement et du périphérique audio peuvent encore s'ajouter pendant la
lecture. Le patch est documenté dans `vendor/librespot-playback/PATCH.md`.

[librespot-c](https://github.com/ejurgensen/librespot-c) est une piste crédible,
mais exige un portage Windows et l'ajout du décodage et de la sortie audio.
[cspot](https://github.com/feelfreelinux/cspot) a été compilé et exécuté, mais
sa compatibilité actuelle avec Spotify et sa configuration TLS doivent être
résolues avant d'envisager une authentification. Le client officiel
[Soloist](https://developer.spotify.com/documentation/soloist) cible Linux et
ne couvre pas les trois plateformes de Spotiamp.

## Validation et limites

- Compilation native Linux et compilation croisée Windows x64 réalisées.
- Interface réelle lancée sous Xvfb ; aide, saisie, soumission d'un lien hors
  connexion et fermeture vérifiées. Les pressions de touches rapides sont
  conservées même si relâchées entre deux rafraîchissements.
- Erreur X11 de méthode de saisie reproduite puis corrigée dans minifb :
  l'application renvoie une erreur au lieu de planter. Échelles 1× et 2× lancées.
- Première préversion : neuf tests distincts réussis sur les liens, la sélection, la file, les
  événements périmés, les touches rapides et l'annulation de sauvegarde.
  `cargo fmt --check` et Clippy avec avertissements traités comme erreurs passent.
- Windows : le fichier PE et ses dépendances ont été inspectés. Un premier
  essai utilisateur du prototype Spool a été réalisé ; voir ci-dessous.
  Les tests automatisés ont aussi réussi sur le runner Windows 2022 x64.
- Linux x64 et macOS Apple Silicon : compilation native, tests, packaging des
  licences et mise à disposition des artefacts réussis sur GitHub Actions.
  L'interface et la lecture audio macOS n'ont pas été testées manuellement.
- Les playlists privées, la qualité audio, la stabilité prolongée et les
  compteurs mémoire complets en lecture restent à vérifier.

Exécution CI du code : [commit 32b03a5, Windows / Linux / macOS](https://github.com/roukmoute/spotiamp/actions/runs/39630012360).

Le binaire Windows GNU x64 de la première préversion (`v0.1.0-alpha.1`),
reconstruit localement après renommage, fait
7 474 688 octets (7,13 Mio) **sur disque**.
Cette taille n'est pas sa consommation mémoire. Empreinte SHA-256 :
`6c8bb52403db049ff0b79c772cbb98d1121d5e22c2af06ee119276e7ae2fb65f`.
Les binaires produits par d'autres cibles ou machines de compilation peuvent
différer ; chaque archive binaire indique son empreinte dans `BUILD-INFO.json`.

## Premier retour Windows — 21 octobre 2026

L'utilisateur a testé le prototype encore nommé Spool. Sa capture montre une
session connectée, six titres importés, un morceau affiché en lecture à 160 kb/s,
et le processus `spool.exe` à **5,7 Mo de mémoire et 0,4 % de CPU** dans le
Gestionnaire des tâches. Il s'agit d'une observation ponctuelle fournie par
l'utilisateur, sans journal de mesures, durée totale d'essai ni configuration
matérielle. Une capture ne permet pas d'évaluer le son ni les transitions.

La colonne mémoire par défaut montre la mémoire privée résidente. Elle exclut
notamment les pages partagées et la mémoire privée engagée non résidente.
Comparer aussi **Working Set** (mémoire résidente privée et partagée) et
**Commit size / Private Bytes** (mémoire privée engagée). Ces mesures se
recouvrent et ne s'additionnent pas. Le moteur librespot est une bibliothèque
dans le processus du lecteur ; il n'y a pas de processus audio auxiliaire
propre à l'application. Les services audio et d'affichage du système restent
des coûts partagés du système.

Références Microsoft : [working set](https://learn.microsoft.com/en-us/windows/win32/procthread/process-working-set)
et [mémoire privée résidente et commit](https://learn.microsoft.com/en-us/troubleshoot/windows-server/performance/troubleshoot-application-service-memory-leaks).

## Mesurer sur Windows

Depuis la version 0.1.7, la bibliothèque est ouverte par défaut et attachée au
lecteur. Les mesures antérieures du lecteur seul n'incluent pas ses buffers ni
son catalogue. Notez donc si elle est ouverte pendant chaque mesure ; fermez-la
avec F3 ou lancez `spotiamp.exe --no-library` pour reproduire le périmètre du
lecteur seul. L'accrochage utilise les événements de fenêtre existants, sans
thread supplémentaire ni redessin périodique.

La comparaison automatique Windows conserve cinq démarrages et deux mesures
au repos pour chaque version, bibliothèque fermée. Elle mesure aussi le
candidat avec ses deux fenêtres ouvertes dans un essai distinct. Tous ces
essais CI restent hors connexion et sans lecture audio.

Décompresser le paquet et ouvrir PowerShell dans son dossier :

```powershell
.\scripts\measure-windows.ps1 -ExePath .\spotiamp.exe -Scenario idle -Seconds 60 -MeasureStartup
```

Puis se connecter, importer une playlist, démarrer effectivement une musique
et fermer l'onglet d'autorisation. Garder une seule instance de Spotiamp :

```powershell
$spotiampId = (Get-Process spotiamp).Id
.\scripts\measure-windows.ps1 -TargetProcessId $spotiampId -Scenario playing -Seconds 120
```

Le script conserve working set, mémoire privée engagée, CPU et les échantillons
bruts. Le délai de démarrage optionnel s'arrête à l'apparition d'une fenêtre
visible, pas à une preuve que le premier pixel est peint ou qu'un son sort.
La valeur `scenario=playing` est une annotation manuelle, pas une détection
automatique d'audio. Aucun script ne lit ou n'exporte les jetons du trousseau.

Pour choisir le moteur définitivement : même machine, même compte, même débit,
même playlist ; mesurer repos connecté, lecture, changements de titres,
import, puis plateau après 30 minutes. Comparer **les mêmes compteurs** avec
Winamp et Spotify. Les captures du Gestionnaire des tâches motivent la recherche,
mais ne remplacent pas ce protocole.

## Refaire les sondes sans compte

```sh
cargo build --release --locked --bin engine_probe
python3 scripts/measure-linux.py --exe target/release/engine_probe --scenario idle --warmup 3 --seconds 7 --interval 0.5
```

La sonde finale utilise le patch à un worker. Les deux JSON `engine-default`
et `engine-one-worker` ont été produits avant ce patch, avec **le même binaire**,
seule la variable d'environnement changeant. Pour reproduire cette comparaison,
retirer temporairement l'entrée `librespot-playback` de `[patch.crates-io]`,
régénérer le verrouillage, reconstruire la sonde et comparer avec/sans
`TOKIO_WORKER_THREADS=1`. Réappliquer ensuite le patch et le verrouillage fourni.

Pour l'interface, compiler avec `--no-default-features --bin spotiamp`. Les tests
en écran virtuel utilisent `XMODIFIERS=@im=none xvfb-run -a ...` ; l'application
ne force pas cette variable sur le bureau de l'utilisateur.

Pour reproduire les tests de sortie sans jouer de son :

```sh
cargo build --release --locked --bin audio_probe
ALSA_CONFIG_PATH="$PWD/benchmarks/audio/null.conf" target/release/audio_probe --seconds 3
ALSA_CONFIG_PATH="$PWD/benchmarks/audio/no-device.conf" target/release/audio_probe --expect-no-device
```

Ces commandes sont réservées à Linux et aux configurations de test fournies.
La source du comportement non cadencé de `null` est le
[module ALSA](https://github.com/alsa-project/alsa-lib/blob/v1.2.14/src/pcm/pcm_null.c).
## Nom des mesures historiques

Les fichiers bruts de `benchmarks/` ont été produits avant le renommage de
Spool en Spotiamp. Leurs noms, chemins de programme et empreintes ont été
conservés pour assurer la traçabilité ; les préfixes de chemin personnels ont
été anonymisés. Le changement de nom ne constitue pas une nouvelle mesure.
