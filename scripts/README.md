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
Les archives contiennent `BUILD-INFO.json` : `app_version` vient du paquet racine
de `cargo metadata`, `binary_name` indique l'exécutable à la racine du ZIP, et
`git_commit` identifie le checkout Git propre utilisé pour préparer l'archive.
Ce dernier vaut `null` si le projet a des modifications, des fichiers non suivis,
ou aucun historique Git. Les empreintes et tailles du binaire, la cible et le
compilateur restent présents. Compilez et empaquetez le même checkout : le script
décrit les sources courantes et ne déduit pas la version d'un ancien binaire passé
avec `--binary`.

Les dépendances Rust sont fixées par `Cargo.lock` ; la chaîne système peut encore
faire varier le binaire. Le workflow CI produit les trois cibles sans publier de
release. Après réussite des trois jobs sur le même commit, les archives internes
des artefacts CI sont publiées avec les noms suivants :

| Cible | Asset de release | Exécutable dans le ZIP |
| --- | --- | --- |
| Windows x64 | `Spotiamp-Windows-x64.zip` | `spotiamp.exe` |
| Linux x64 | `Spotiamp-Linux-x64.zip` | `spotiamp` |
| macOS ARM64 | `Spotiamp-macOS-arm64.zip` | `spotiamp` |
| Sources | `Spotiamp-source.zip` | Sources sous `spotiamp/` |

L'installateur `Spotiamp-Setup-Windows-x64.exe` s'ajoute aux quatre ZIP.
`Spotiamp-SHA256SUMS.txt` accompagne ces cinq fichiers et contient une ligne par
fichier : empreinte SHA-256, deux espaces, puis nom exact de l'asset. Calculez les
empreintes après les renommages et vérifiez les métadonnées avant de publier le
brouillon complet. Les fichiers d'authentification et de configuration utilisateur
ne font pas partie des archives.

## Installateur Windows

Avec Python 3.11+ et Inno Setup 6.3+ sous Windows, après le packaging :

```powershell
python scripts/build_windows_installer.py --archive dist/spotiamp-x86_64-pc-windows-msvc.zip --output-dir dist
```

Le script cherche `ISCC.exe` dans le PATH et dans les dossiers standards
d'Inno Setup 6 ; `--iscc` permet de fournir son chemin. `--expected-commit` et
`--expected-version` vérifient les métadonnées attendues. `--validate-only`
valide le paquet sans compiler l'installateur et fonctionne aussi sous Linux.
Le ZIP doit provenir d'un checkout propre, porter un commit Git et contenir le
binaire MSVC x64 identifié par son empreinte dans `BUILD-INFO.json`.

La recette `scripts/windows/installer.iss` installe tous les fichiers du ZIP
validé, sans modification. Le rapport `Spotiamp-Setup-Windows-x64.build.json`
relie les empreintes de l'installateur, de l'archive et du binaire au commit.
L'icône est construite à partir de `assets/windows/spotiamp.svg` ; le fichier
ICO multi-résolution est suivi dans Git et incorporé à la compilation Windows
par `build.rs`. Aucun compilateur d'icône n'est requis pour reconstruire le projet.

La CI Windows exécute `scripts/test_windows_installer.ps1` avec le nouvel
installateur, son ZIP et le ZIP officiel alpha.8 vérifié par SHA-256. Ce test
installe dans un dossier avec espaces et accent, vérifie les fichiers et les
raccourcis, lance l'interface native, puis exerce le vrai helper de mise à jour
alpha.8 vers le nouveau binaire. L'ancienne installation est une fixture : le
contenu portable alpha.8 est superposé à une installation Inno courante. Ce
test couvre la transaction locale, pas la découverte GitHub ni la lecture Spotify.
Il vérifie ensuite la réinstallation et la désinstallation, avec des fichiers
témoins personnels et des préférences conservées. Il exige un environnement CI
Windows jetable et refuse une installation ou des raccourcis préexistants.
Les rapports et logs sont téléversés même si ce test échoue.

## Publication des versions

Publier toutes les versions comme **releases définitives**, jamais avec le
statut GitHub « pre-release ». Utiliser des numéros SemVer sans suffixe alpha
pour les prochaines versions et désigner la plus récente comme **Latest**.
Attendre la réussite complète des trois jobs CI du même commit, y compris les
tests natifs, le packaging, le test de l'installateur et l'envoi des artefacts.
Publier les fichiers issus de ces artefacts, sans recompilation locale, après
vérification des commits, versions et empreintes. Conserver les noms des ZIP
ci-dessus : la mise à jour intégrée les utilise, y compris sous Windows installé.
