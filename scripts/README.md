# Mesurer et distribuer Spotiamp

Les scripts mesurent un vrai processus. Le choix `idle` ou `playing` décrit le
scénario que vous préparez ; il ne déclenche ni ne vérifie la lecture.
Utilisez une compilation **release**, fermez les outils de profilage et relevez
les mêmes conditions (qualité audio, durée, morceau, volume, état du cache).
Conservez les JSON bruts et la version du binaire avant de comparer des chiffres.

## Windows

Depuis le dossier de distribution, dans PowerShell :

```powershell
.\scripts\measure-windows.ps1 -ExePath .\spotiamp.exe -Scenario idle -MeasureStartup -Seconds 60 -OutputPath .\measurements\idle.json
```

Pour mesurer la lecture, ouvrez Spotiamp, connectez-vous et lancez un morceau.
Relevez son PID dans le Gestionnaire des tâches, puis :

```powershell
.\scripts\measure-windows.ps1 -TargetProcessId 1234 -Scenario playing -Seconds 120 -WarmupSeconds 10 -OutputPath .\measurements\playing.json
```

`private_bytes` correspond à la mémoire privée engagée et `working_set_bytes`
à la mémoire résidente, qui peut inclure des pages partagées. Les colonnes du
Gestionnaire des tâches peuvent utiliser une autre définition. Divisez les octets
par 1 048 576 pour obtenir des Mio. Médiane, 95e percentile et maximum sont
calculés sur les échantillons ; la fréquence par défaut est une seconde.

`-MeasureStartup` observe l'apparition d'une fenêtre principale visible toutes
les 10 ms pendant au plus 10 secondes. Ce temps comprend le lancement du
processus, ne mesure pas le premier rendu exact et ne garantit pas que l'audio
est prêt. Une mesure impossible reste `null`.

## Linux

```sh
python3 scripts/measure-linux.py --exe ./spotiamp --scenario idle --seconds 60 --output measurements/idle.json
python3 scripts/measure-linux.py --pid 1234 --scenario playing --warmup 10 --seconds 120 --output measurements/playing.json
```

Le script utilise `/proc`, sans dépendance Python. Il relève RSS, PSS et les pages
privées résidentes ; ces dernières ne sont **pas** équivalentes aux Private Bytes
de Windows. PSS et mémoire privée restent `null` si `smaps_rollup` n'est pas
accessible. La visibilité de la fenêtre n'est pas mesurée sous Linux.

Les deux scripts excluent les processus enfants et laissent l'application ouverte.
L'empreinte SHA-256 de l'exécutable accompagne les résultats.

## Archives

Après une compilation release avec la cible choisie :

```sh
python3 scripts/package.py --target x86_64-pc-windows-msvc
```

Le script crée dans `dist/` une archive portable et `spotiamp-source.zip`. Il inclut
les notices des dépendances résolues pour cette cible et les sources des
composants MPL. Les textes de licence manquants provoquent une erreur explicite.
`--binary /chemin/vers/spotiamp.exe` accepte un autre dossier de compilation.
Les archives contiennent `BUILD-INFO.json` (compilateur et empreintes). Les
dépendances Rust sont fixées par `Cargo.lock` ; la chaîne système peut encore
faire varier le binaire. Le workflow CI produit les trois cibles sans publier de release.
