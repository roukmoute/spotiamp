# Changements

## 0.1.4 — 22 octobre 2026

- Icône Windows reprenant le **S doré du panneau À propos**, avec plusieurs
  tailles intégrées à l'exécutable et à l'installateur.
- Icônes natives attribuées au lecteur et à la bibliothèque : la barre des
  tâches et Alt+Tab ne dépendent plus de l'icône générique de Windows.
- Chargement depuis les ressources intégrées, sans fichier externe, décodeur
  d'image ou processus supplémentaire.

## 0.1.3 — 22 octobre 2026

- Reconnexion Spotify automatique au lancement à partir de la session du coffre
  système. Une seule tentative en arrière-plan, sans démarrage de la lecture.
- Aucune ouverture automatique du navigateur si la session est absente,
  inaccessible ou refusée, ou si le réseau est indisponible. La connexion
  manuelle reste disponible et les échecs réseau n'effacent pas les identifiants.
- La fermeture du lecteur conserve la session ; **Déconnecter** l'efface
  explicitement. Les résultats retardés d'une connexion annulée sont ignorés.
- Les identifiants réutilisables sont sauvegardés avant l'annonce de connexion
  réussie, y compris après reconnexion. Un échec de sauvegarde est signalé.
- **Annuler** interrompt une tentative sans oublier le compte. Une connexion
  terminée pendant un clic ne transforme pas ce clic en déconnexion.

## 0.1.2 — 22 octobre 2026

- Boutons **Shuffle** et **Repeat** après Suivant, avec relief métallique,
  effet d'enfoncement et voyant vert pour les modes actifs.
- Lecture aléatoire sans doublon d'entrée dans un même tour et navigation
  Précédent/Suivant dans cet ordre. La liste affichée conserve son ordre.
- Répétition de la file à sa fin, ou d'un titre seul. Associée à Shuffle,
  elle prépare un nouvel ordre à chaque tour sans enchaîner deux fois la
  même entrée lorsque la file compte plusieurs titres.
- Modes désactivés au lancement ; aucune modification des playlists Spotify.

## 0.1.1 — 22 octobre 2026

- Bouton **Lecture** corrigé : après une pause, le morceau reprend à sa
  position ; pendant la lecture, il recommence au début. Le morceau courant
  reste la cible, même si une autre ligne de la file est sélectionnée.
- Reprise et retour au début utilisent le titre déjà chargé et conservent sa
  qualité audio. Après **Arrêt**, Lecture démarre le titre sélectionné.
  Le double-clic et Entrée continuent de lancer explicitement une ligne ;
  Pause et Espace conservent leur fonction pause/reprise.

## 0.1.0 — 22 octobre 2026

- Installateur Windows x64 en français et en anglais, pour le compte courant,
  sans droits administrateur. Installation par défaut dans
  `%LOCALAPPDATA%\Programs\Spotiamp`, raccourci du menu Démarrer, raccourci Bureau
  optionnel et lancement proposé à la fin de l'assistant.
- Désinstallation depuis les paramètres Windows, avec conservation des réglages,
  des identifiants mémorisés et des fichiers personnels à la racine du dossier.
  Nettoyage des sous-dossiers appartenant au programme après les mises à jour.
- Icône et métadonnées de version intégrées à l'exécutable Windows.
- Mises à jour intégrées conservées pour les installations et les copies
  portables. Les raccourcis et le désinstallateur restent en place ; la version
  enregistrée dans Windows est synchronisée au lancement du lecteur installé.
- Protection native contre les installations et désinstallations simultanées
  au lecteur ou à ses processus temporaires de mise à jour.
- Installateur construit à partir du ZIP Windows validé, avec le même binaire,
  les mêmes licences et sources MPL. Aucun service ni processus résident ajouté.
- Publication standard **0.1.0**, sans statut GitHub « pre-release ».

## 0.1.0-alpha.8 — 22 octobre 2026

- Bibliothèque de vos playlists Spotify en lecture seule, ouverte avec
  **BIBLIO. F3** ou **F3**, dans une fenêtre classique séparée. Playlists et
  dossiers à gauche, titres de la playlist sélectionnée à droite.
- Catalogue paginé à l'ouverture ; les titres sont chargés seulement à la
  sélection. **Actualiser / F5** recharge la liste. Noms, artistes et durées
  arrivent progressivement, avec deux requêtes de métadonnées simultanées.
- **Lire** remplace la file après chargement réussi de la playlist et démarre
  au titre sélectionné. Un double-clic démarre à la ligne choisie.
  **À la file** complète la file existante sans la remplacer ni lancer la lecture.
  Une erreur d'accès ou une playlist vide conserve la file précédente.
- Limite explicite de **500 titres accessibles** par playlist affichée et
  dans la file ; les ajouts respectent les places restantes et signalent la limite.
- Navigation au clavier et molette d'une ligne par cran dans chaque panneau.
  Réduire, fermer et déplacer la bibliothèque utilisent le même bandeau intégré.
- Gestion des deux fenêtres : touches Ctrl/F3/flèches réinitialisées au changement
  de focus ; restauration Linux réaffichant l'image conservée ; ressources
  natives libérées à la fermeture.
- Fenêtre et framebuffer créés à l'ouverture ; catalogue et titres affichés
  relâchés à la fermeture, sans interrompre la lecture du lecteur. Les instantanés
  partagés et leur révision évitent la copie des listes à chaque image.
- Cette nouvelle bibliothèque reste à vérifier avec un compte Spotify réel.
  Les tests hors ligne utilisent des données synthétiques et ne prouvent pas
  la découverte ni la lecture authentifiée.

## 0.1.0-alpha.7 — 21 octobre 2026

- Cadre et barre de titre du système retirés sous Windows, macOS et Linux.
  Les boutons **Réduire** et **Fermer** sont intégrés au bandeau classique.
- Déplacement de la fenêtre en faisant glisser une zone libre du bandeau
  supérieur avec le bouton gauche de la souris.
- Taille fixe conservée aux échelles 1× et 2× (`--scale 2`), avec maximisation
  désactivée. Les raccourcis natifs, dont **Alt+F4** sous Windows, sont conservés.
- Rendu dans le buffer graphique existant, sans nouvelle dépendance ni service.

## 0.1.0-alpha.6 — 21 octobre 2026

- Mises à jour intégrées depuis les releases GitHub, préversions comprises :
  recherche trois secondes après le lancement, notification persistante, puis
  téléchargement et installation sur demande. `--no-update-check` désactive
  la recherche automatique ; la vérification manuelle reste disponible.
- Cinquième onglet **Mise à jour** dans À propos, avec version disponible,
  progression, erreurs et boutons adaptés à l'état. F2 l'ouvre directement
  lorsqu'une version est disponible, en téléchargement, prête ou en erreur.
- Paquet compatible téléchargé sur disque et vérifié avant installation.
  Un processus temporaire remplace l'exécutable puis relance le lecteur.
  Le redémarrage interrompt la lecture et perd la file non sauvegardée ;
  les identifiants déjà mémorisés dans le coffre et la qualité sont conservés.
- Recherche et téléchargement hors du thread d'interface, sans client HTTP ni
  thread de vérification permanent. Le panneau réutilise le buffer graphique.
- Tests de l'interface pour chaque état, la navigation clavier, les contrôles
  désactivés et les changements d'état pendant un clic. Aucun clic du panneau
  ne traverse vers les commandes de lecture.

## 0.1.0-alpha.5 — 21 octobre 2026

- Panneau À propos intégré au skin, avec quatre onglets : Spotiamp, Crédits,
  Raccourcis et Historique. Version du programme, crédits des composants et
  licence du projet y figurent.
- Accès au dépôt GitHub et copie de la version, du système, de l'architecture,
  du moteur et de la qualité préférée, sans donnée de compte. Un message
  confirme la copie ou signale une erreur de presse-papiers ou de navigateur.
- Navigation à la souris et au clavier, focus visible et boutons pressés.
  F2 ouvre ou ferme le panneau ; Échap le ferme et F1 affiche ses raccourcis.
  Les interactions du lecteur sont bloquées derrière le panneau, pendant que
  la lecture audio continue.
- Le panneau réutilise le buffer graphique existant, sans nouvelle dépendance,
  image externe ni animation permanente.

## 0.1.0-alpha.4 — 21 octobre 2026

- Skin plus proche du Winamp classique : touches métalliques, pictogrammes de
  transport, bordures encastrées, bandeaux compacts et curseurs rainurés.
- Boutons enfoncés pendant l'appui, activation au relâchement dans le même bouton.
  Un relâchement à l'extérieur ou une perte de focus/capture annule l'action.
- Clics courts conservés même si l'appui et le relâchement arrivent dans le même
  rafraîchissement, avec coordonnées adaptées au mode `--scale 2`.
- État actif/inactif de la fenêtre macOS corrigé.
- Tests natifs des événements souris Windows et X11 ; CI déclenchée une seule
  fois par changement de branche, sans nouvelle compilation pour le tag de release.

## 0.1.0-alpha.3 — 21 octobre 2026

- Qualité demandée par défaut : 320 kb/s au lieu de 160, maximum du moteur actuel.
- Sélection 96 / 160 / 320 sous la liste, mémorisée entre les lancements.
- Changement appliqué au prochain chargement audio, sans couper le titre en cours.
- La barre d'état identifie la qualité demandée du titre chargé ; un éventuel
  format de repli n'est pas présenté comme un débit réel mesuré.

## 0.1.0-alpha.2 — 21 octobre 2026

- Molette Windows : un cran déplace la liste d'un titre, au lieu de dix.
  Les titres 9 et 10 peuvent ainsi être atteints depuis la vue initiale 1–8.
- Les petits mouvements sont accumulés, sans arrondi systématique vers une
  ligne entière. Les rafales de plusieurs événements ne perdent plus de crans.
- Sous X11, le relâchement de la molette ne double plus le déplacement.
  Sous macOS, les événements d'une même mise à jour sont cumulés.
- Le numéro de version figure dans le titre de la fenêtre et dans `--help`.

## 0.1.0-alpha.1 — 21 octobre 2026

Première préversion Spotiamp : lecteur Spotify natif, connexion, import de
titres/albums/playlists par lien, file de lecture et commandes audio locales.
