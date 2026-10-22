# Résultats Linux : optimisation 0.1.6

**Choix retenu : `pruned-fat3`.** Le binaire local passe de 9 386 480 à 7 588 544 octets (−19,15 %), sans régression observable de décodage ou de réactivité dans ce banc. Aucun compte Spotify ni périphérique audio utilisé.

Les variantes `s`, `z` et mixte réduisent davantage le fichier, mais elles augmentent le coût CPU du décodage. Le mixte gagne 39,04 % en taille ; il reste 30,6 % plus lent en Ogg et 65,8 % plus lent en MP3 que `pruned-fat3` dans leur comparaison directe. Il est écarté pour préserver la vitesse, sans défaut de fidélité : les PCM sont strictement identiques.

## Taille des exécutables locaux

| Profil | Octets | Gain vs baseline locale |
|---|---:|---:|
| `baseline-thin3` | 9 386 480 | 0.00 % |
| `fat3` | 7 649 504 | 18.51 % |
| `pruned-fat3` | 7 588 544 | 19.15 % |
| `fats` | 5 608 768 | 40.25 % |
| `fatz` | 5 312 440 | 43.40 % |
| `mixeds` | 5 721 920 | 39.04 % |

L’exécutable Linux officiel 0.1.5 fait 9 338 384 octets. Il constitue un repère séparé ; les pourcentages ci-dessus utilisent la baseline locale compilée avec le même outil et les mêmes sources que les variantes.

## Décodage CPU

Médiane de trois répétitions, 100 décodages du même signal de 30 s par répétition. Unité : millisecondes de CPU par seconde d’audio décodée. Plus bas est mieux ; ces chiffres ne représentent pas le CPU total d’une lecture Spotify.

**decode-round1**

| Profil | Ogg Vorbis | MP3 | FLAC |
|---|---:|---:|---:|
| `baseline-thin3` | 0.664 | 0.600 | 0.518 |
| `fat3` | 0.669 | 0.587 | 0.507 |
| `pruned-fat3` | 0.644 | 0.588 | — |
| `fats` | 0.897 | 1.232 | 0.677 |
| `fatz` | 1.167 | 1.452 | 0.764 |

**decode-round2**

| Profil | Ogg Vorbis | MP3 | FLAC |
|---|---:|---:|---:|
| `baseline-thin3` | 0.656 | 0.615 | — |
| `pruned-fat3` | 0.656 | 0.582 | — |
| `mixeds` | 0.856 | 0.964 | — |

Chaque format produit 2 646 000 échantillons f64 par fichier. Avant chronométrage, le SHA256 de tous leurs bits a été comparé : identique entre chaque variante qui prend en charge le format, sans paquet sauté ni échantillon non fini. Les empreintes sont conservées dans `results-linux.json`.

## Fenêtre et repos

**round1** — huit démarrages par variante ; mesures au repos après 20 s de stabilisation, 30 s par répétition.

| Variante | Démarrage médian (ms) | CPU au repos (% d’un cœur) | RSS (Mio) | Privée résidente (Mio) |
|---|---:|---:|---:|---:|
| `official` | 43.62 | 0.267 | 9.828 | 5.910 |
| `baseline-thin3` | 43.56 | 0.267 | 9.992 | 6.086 |
| `fat3` | 43.28 | 0.267 | 10.004 | 5.930 |

**round2** — huit démarrages par variante ; mesures au repos après 20 s de stabilisation, 30 s par répétition.

| Variante | Démarrage médian (ms) | CPU au repos (% d’un cœur) | RSS (Mio) | Privée résidente (Mio) |
|---|---:|---:|---:|---:|
| `baseline-thin3` | 42.54 | 0.267/0.233 | 10.391/10.207 | 6.324/6.262 |
| `pruned-fat3` | 43.83 | 0.233/0.233 | 10.125/10.051 | 6.055/6.055 |

Les valeurs séparées par `/` sont les deux répétitions. La résolution d’interrogation du démarrage est de 5 ms ; un écart d’environ 1 ms ne démontre donc pas un ralentissement. Le gain de taille sur disque ne se traduit pas ici par une baisse importante de RAM au repos. Ces données Linux sous Xvfb/Openbox ne sont pas extrapolées à Windows, au GPU ou à la lecture Spotify authentifiée.

Méthode et reproduction : `README.md`. Paramètres, samples, hashes, profils et provenance : `results-linux.json`.
