# Changements

## 0.2.20 — 27 octobre 2026

- Menu au clic droit dans la file du lecteur : lire le titre, envoyer la sélection
  vers une playlist Spotify existante ou nouvelle, retirer la sélection de la
  file ou conserver uniquement les morceaux choisis.
- Un clic droit sur une ligne sélectionnée conserve le groupe ; sur une autre
  ligne, il choisit ce titre. Navigation clavier avec la touche Menu ou Maj+F10.
- Les modifications locales vérifient les occurrences capturées et préservent
  la progression du morceau en cours tant qu’il reste dans la file. Une file
  modifiée entretemps ferme le menu sans appliquer une ancienne sélection.
- Rendu Classique et Modern traduit dans les six langues, sans dépendance ni
  tâche permanente supplémentaire.

## 0.2.19 — 27 octobre 2026

- Sélection multiple dans la file du lecteur : Ctrl/Cmd+clic ajoute ou retire
  un morceau, Maj+clic sélectionne une plage et Ctrl/Cmd+A sélectionne tous
  les titres lorsque la file est active.
- Maintenir le clic gauche sur un morceau sélectionné puis glisser déplace
  tout le groupe, dans son ordre de liste. Le repère d'insertion et le défilement
  aux bords restent disponibles ; un simple clic sans déplacement choisit
  uniquement le morceau visé.
- La sélection suit les occurrences déplacées après confirmation du moteur,
  y compris les doublons. La lecture conserve son titre et sa progression.
- Rendu dans les deux skins, historique traduit dans les six langues,
  sans nouvelle dépendance, requête Spotify ni tâche permanente.

## 0.2.18 — 27 octobre 2026

- Maintenir le clic gauche puis glisser un morceau permet de changer sa place
  dans la file du lecteur. Un repère indique la destination, avec défilement
  automatique aux bords ; la lecture en cours conserve son morceau et sa position.
- Dans une playlist Spotify qui vous appartient, le même geste déplace le morceau
  ou la sélection multiple, en conservant son ordre. Les occurrences identiques
  sont distinguées par leur position. L'ordre confirmé est enregistré sur Spotify.
- Un déplacement sans changement ne déclenche aucune écriture. La source et la
  révision sont contrôlées avant l'envoi, avec une seule modification à la fois
  et sans nouvel envoi automatique en cas de réponse incertaine.
- Les deux skins et les six langues sont conservés, sans nouvelle dépendance
  ni tâche permanente. Le glisser-déposer de la bibliothèque vers le lecteur
  reste disponible pour ajouter des titres sans interrompre la lecture.

## 0.2.17 — 27 octobre 2026

- Un clic droit sur un morceau d'une playlist ou des résultats de recherche
  ouvre les cinq actions du menu Envoyer vers… à proximité du pointeur.
  Le menu reste entièrement dans la fenêtre, y compris près de ses bords.
- Un clic droit sur un morceau déjà sélectionné conserve la sélection multiple ;
  sur un autre morceau, il sélectionne uniquement celui-ci. L'ouverture du menu
  ne lance pas la lecture et ne modifie pas les playlists.
- Même menu, commandes et contrôles de droits que le bouton Envoyer vers…,
  dans les deux skins et les six langues. Aucune dépendance, requête réseau
  ou tâche permanente ajoutée pour ouvrir le menu.

## 0.2.16 — 27 octobre 2026

- Un bouton Envoyer vers… regroupe les actions de la bibliothèque : lire la
  sélection, l'ajouter à la file, l'envoyer vers une playlist existante ou créer
  une nouvelle playlist Spotify avec les morceaux choisis.
- Le même menu permet de retirer la sélection d'une playlist dont vous êtes
  propriétaire. Les raccourcis existants et le glisser-déposer sont conservés.
- Menu intégré aux skins Classique et Modern, traduit dans les six langues,
  utilisant les commandes existantes sans dépendance ni tâche permanente.

## 0.2.15 — 27 octobre 2026

- Suppr retire les morceaux sélectionnés d'une playlist Spotify dont vous êtes
  propriétaire, depuis son panneau de titres. La sélection multiple est prise
  en charge ; la file du lecteur et la lecture en cours sont conservées.
- Vérification du propriétaire, de la révision et du contenu avant l'envoi.
  Seules les occurrences choisies sont retirées, y compris en présence de
  doublons ; les titres sont rechargés après confirmation de Spotify.
- Une seule modification à la fois, sans nouvel envoi automatique. Une réponse
  incertaine invite à vérifier la playlist avant de réessayer. Les résultats
  de recherche et les playlists d'autres personnes ne sont pas modifiés.
- Contrôles et connexion existants réutilisés, sans dépendance ni tâche
  permanente. Messages dans les six langues et tests hors connexion ; le
  retrait réel reste à vérifier avec un compte Spotify connecté.

## 0.2.14 — 27 octobre 2026

- Entrer dans À propos → Mise à jour lance immédiatement la recherche d'une
  nouvelle version, sans devoir cliquer sur Vérifier. Le même comportement
  s'applique à l'ouverture directe de cet onglet.
- La vérification s'applique après un état initial, un résultat à jour ou une
  erreur. Une version déjà proposée garde son bouton Télécharger, un paquet
  prêt garde Installer / Redémarrer ; aucune opération en cours n'est relancée.
- Contrôleur et événements existants réutilisés, sans requête périodique,
  nouveau thread permanent ou dépendance. Historique traduit dans les six langues.

## 0.2.13 — 27 octobre 2026

- Nouvelle playlist… dans Vers playlist… : donner un nom à une playlist Spotify
  et y ajouter les morceaux sélectionnés, depuis la recherche ou une playlist.
  La sélection et son ordre restent mémorisés pendant la saisie ; la lecture
  et la file en cours sont conservées.
- Nom de 1 à 100 caractères, saisie Unicode et presse-papiers, validation par
  Créer ou Entrée et retour sans création avec Retour ou Échap. Dialogue natif
  dans les deux skins, traduit dans les six langues.
- Création, ajout à la bibliothèque Spotify puis ajout des titres avec la
  connexion existante, une seule opération à la fois et sans nouvel envoi
  automatique. Une création confirmée est conservée si une étape suivante
  échoue ; un succès complet exige toutes les confirmations. Une réponse
  incertaine invite à vérifier les playlists avant de réessayer.
- Aucune nouvelle dépendance ni tâche permanente. Tests hors connexion de la
  saisie, du protocole et des échecs ; la création réelle sur Spotify reste à
  vérifier avec un compte connecté.

## 0.2.12 — 27 octobre 2026

- Recherche de musiques Spotify intégrée à la bibliothèque, au même niveau que
  les playlists. Champ, boutons et résultats dans les skins Classique et Modern,
  avec les six langues et le raccourci Ctrl+F.
- Recherche par titre ou artiste après validation avec Entrée ou Chercher.
- Sélection multiple dans les résultats et les titres des playlists : Ctrl/Cmd+clic,
  Maj+clic, plages au clavier et Ctrl/Cmd+A. Lire joue la sélection dans son ordre
  de liste ; À la file ou Maj+Entrée l'ajoute sans interrompre la lecture.
- Glisser-déposer de la sélection dans la file du lecteur, avec repère d'insertion
  et défilement aux bords. L'ordre des titres et le morceau en cours sont conservés ;
  une sélection qui dépasse les places libres est refusée en entier.
- Bouton Vers playlist… pour ajouter la sélection à une playlist Spotify existante,
  après choix de la destination. Une seule écriture à la fois, avec vérification
  des droits et de la révision ; une réponse incertaine demande de vérifier la
  playlist et ne déclenche aucun nouvel envoi automatique.
- Jusqu'à 50 résultats, deux chargements de métadonnées simultanés et annulation
  de la recherche précédente. Les réponses anciennes ne remplacent pas les
  nouveaux résultats ; Effacer, la fermeture et la déconnexion libèrent ceux-ci.
- Connexion Spotify et moteur réseau existants réutilisés, sans nouvelle pile
  réseau, paquet résolu, thread permanent, pochette ni cache de recherche sur disque.
- Tests hors connexion des sélections et du protocole d'ajout. La recherche et
  l'écriture sur Spotify restent à vérifier avec un compte connecté.

## 0.2.11 — 22 octobre 2026

- Six langues disponibles : français, anglais, allemand, espagnol, italien et
  portugais. Détection de la langue du système et de ses variantes régionales,
  avec repli en anglais pour les langues non prises en charge.
- Sélection immédiate et mémorisée dans À propos, accessible à la souris et au
  clavier. Boutons, aide, bibliothèque, connexion, lecture et mises à jour traduits ;
  titres, artistes, noms des playlists et diagnostics externes préservés.
- Catalogues statiques intégrés, sans dépendance, téléchargement ou thread ajouté.
  Tests du changement à chaud et de vrais redémarrages avec les six langues.

## 0.2.10 — 22 octobre 2026

- File, morceau courant et position mémorisés dans les préférences. Au lancement,
  le morceau est révélé dans la file et reste en pause ; Lecture ou Espace reprend
  au temps enregistré sans précharger de flux audio au démarrage.
- Playlist d'origine retrouvée dans la bibliothèque, indépendamment de la
  dernière playlist consultée. Volume, Shuffle et Repeat conservés.
- Sauvegarde locale bornée et remplacée atomiquement à la fermeture, avec un
  point de reprise environ toutes les 15 secondes si l'état a changé. Aucune
  nouvelle dépendance ni tâche de fond ; les sondes de mesure n'utilisent pas
  cette sauvegarde.
- Tests hors connexion du transport, des préférences et de vrais redémarrages
  du processus, dont une file contenant plusieurs occurrences du même titre.

## 0.2.9 — 22 octobre 2026

- Interface en français et en anglais, avec détection de la langue du système
  et choix Auto / Français / English dans À propos → Spotiamp. Le changement
  est immédiat dans les deux skins et la bibliothèque, sans interrompre la lecture.
- Langue mémorisée dans un petit fichier borné, remplacé atomiquement ;
  `--language auto|fr|en` permet un essai sans changer le choix enregistré.
- Traduction des boutons, infobulles, aide, connexion, statuts de lecture,
  bibliothèque et mises à jour. Les métadonnées Spotify ne sont pas traduites.
- Textes statiques intégrés et empruntés sans allocation, sans dépendance ni
  tâche de fond supplémentaire. Tests des changements à chaud, de la persistance
  après redémarrage et de la détection de langue, avec les deux skins.

## 0.2.8 — 22 octobre 2026

- Spotiamp est présenté comme un lecteur Spotify natif : retrait des mentions
  « expérimental » et « prototype » dans l'interface, le titre de fenêtre,
  À propos, l'aide, les métadonnées et la présentation du projet.
- Aperçu du README actualisé et outils de validation adaptés au nouveau titre
  de fenêtre, avec prise en charge des anciennes versions pour les comparaisons
  de performances et les essais de mise à jour.

## 0.2.7 — 22 octobre 2026

- La fermeture normale mémorise les positions du lecteur et de la bibliothèque,
  leur état attaché ou libre, ainsi que la bibliothèque ouverte ou fermée.
  F3 rouvre la bibliothèque à sa position mémorisée ; si elle était attachée,
  elle suit le déplacement du lecteur effectué pendant sa fermeture.
- Les positions accessibles restent inchangées, y compris sur plusieurs écrans
  et avec des coordonnées négatives. Si un écran a disparu, les fenêtres sont
  replacées pour garder leurs bandeaux accessibles ; le groupe conserve son
  orientation quand c'est possible. Le décalage des fenêtres attachées s'adapte
  à un changement d'échelle.
- `--library` et `--no-library` imposent la visibilité pour ce lancement sans
  remplacer le choix enregistré. Les ouvertures et fermetures explicites par
  l'utilisateur restent mémorisées. Un petit fichier `Spotiamp/window-layout`,
  borné et remplacé atomiquement, conserve la disposition séparément des skins,
  de la qualité audio et de la session Spotify. Aucun enregistrement périodique,
  nouveau thread ou dépendance.

## 0.2.6 — 22 octobre 2026

- Fenêtres aimantées : le bandeau de la bibliothèque permet de la détacher,
  tandis que celui du lecteur déplace les deux fenêtres lorsqu'elles sont liées.
  Un simple clic sans déplacement ne les détache pas.
- Au relâchement, rapprocher une fenêtre libre du bord de l'autre les recolle
  à droite, à gauche, au-dessus ou en dessous, avec alignement des coins proches.
  La réduction et la restauration restent groupées même une fois séparées.
- Même géométrie d'accrochage sur Windows, macOS et Linux, aux échelles 1× et 2×.
  Le suivi réutilise les événements et la boucle existants, sans nouveau thread,
  timer, moteur graphique ou dépendance.

## 0.2.5 — 22 octobre 2026

- Un appui bref sur Haut ou Bas ne déplace plus la sélection de deux lignes
  dans le lecteur ou la bibliothèque. Retour arrière ne traite plus deux fois
  le premier effacement dans les champs de saisie.
- La répétition lorsque la touche reste enfoncée conserve son fonctionnement.
  L'appui initial n'est plus traité à la fois par l'événement clavier et par
  la lecture de l'état des touches, sans nouvel état, timer ou thread.

## 0.2.4 — 22 octobre 2026

- Pendant la lecture ou un chargement, les changements explicites de titre
  affichent immédiatement la sélection et regroupent les demandes : après
  200 ms sans nouveau choix, seul le dernier est chargé. Le premier lancement
  à l'arrêt et l'avance naturelle restent immédiats ; Pause et Arrêt restent
  disponibles.
- Les anciens chargements reçoivent une annulation lors d'un remplacement ou
  d'un arrêt. La fermeture libère les récepteurs avant d'attendre les tâches,
  et le suivi des tâches évite de perdre un chargement terminé simultanément.
  L'annulation est coopérative : elle n'interrompt pas une section synchrone
  de décodage ou de recherche de position déjà en cours.
- Les boutons de lecture conservent les clics lorsque la file se recentre
  pendant l'appui. Les clics sur une ancienne ligne de file restent protégés.
- Une seule demande de lecture différée est conservée, sans nouveau thread
  ni sondage périodique.

## 0.2.3 — 22 octobre 2026

- La molette fait défiler la liste survolée dans le lecteur ou la bibliothèque,
  même si sa fenêtre est inactive : il n'est plus nécessaire de cliquer d'abord
  pour l'activer.
- Lors de l'avance automatique, la file ne bouge pas si le nouveau titre est
  déjà visible. Sinon, elle privilégie un décalage d'environ quatre lignes
  pour le rendre visible, en respectant les limites de la liste.
- Un choix ou relancement explicite de titre conserve le centrage. Les
  actualisations de position ne déplacent pas la file pendant la lecture.

## 0.2.2 — 22 octobre 2026

- Boutons du skin Modern aux contours mieux lissés, avec un relief cohérent
  au repos et pendant l'appui.
- Le rendu réutilise les buffers existants, sans dépendance ni animation
  périodique supplémentaire.

## 0.2.1 — 22 octobre 2026

- Quand un titre démarre ou est relancé, le lecteur sélectionne sa ligne et
  la centre dans la partie visible de la file, y compris depuis la bibliothèque.
  Près du début ou de la fin, le défilement est borné aux limites de la liste.
- Le défilement manuel reste libre pendant la lecture : les actualisations
  de position ne recentrent pas la liste. Le comportement est commun aux
  skins Classique et Modern.

## 0.2.0 — 22 octobre 2026

- Deux skins natifs intégrés : **Classique** et **Modern**, sélectionnables
  dans l'en-tête de la file du lecteur ou avec **F4**. Le changement est immédiat,
  conserve les dimensions des deux fenêtres et n'interrompt pas la lecture.
- Le choix est mémorisé dans le fichier de préférence `Spotiamp/skin`, séparé
  de la qualité audio et de la session Spotify. Classique reste le choix par
  défaut si la préférence est absente ou invalide ; un échec d'écriture est signalé.
- `--skin classic` ou `--skin modern` choisit une apparence pour ce lancement,
  sans modifier la préférence enregistrée. Les deux skins réutilisent les
  fenêtres et buffers existants, sans moteur graphique supplémentaire.

## 0.1.9 — 22 octobre 2026

- Le bandeau de la bibliothèque s'atténue lorsqu'elle perd le focus, comme
  celui du lecteur. Ses couleurs actives reviennent lorsqu'elle reprend le focus.
- Ce rendu utilise l'état de focus existant, sans timer ni animation supplémentaire.

## 0.1.8 — 22 octobre 2026

- Double-cliquer sur une playlist dans la bibliothèque charge ses titres,
  remplace la file après chargement réussi et lance le premier titre.
- Un simple clic conserve l'aperçu des titres sans modifier la file ni la
  lecture en cours. Le double-clic sur un titre démarre toujours cette ligne.
- Une playlist vide ou une erreur de chargement conserve la file précédente.

## 0.1.7 — 22 octobre 2026

- Bibliothèque Spotify ouverte automatiquement avec le lecteur, accolée à sa
  droite ; placement en dessous sur les écrans trop étroits.
- Déplacement des deux fenêtres ensemble depuis l'un ou l'autre bandeau,
  avec réduction et restauration groupées. L'accrochage utilise les événements
  natifs de Windows, macOS et X11, sans thread supplémentaire.
- F3, Échap ou la croix ferment la bibliothèque en conservant la lecture ;
  F3 depuis le lecteur la rouvre attachée. `--no-library` démarre le lecteur
  seul et évite d'allouer les buffers de la bibliothèque.
- Placement tenant compte de la zone utile de l'écran, des coordonnées
  négatives des écrans secondaires et de l'échelle 2×.

## 0.1.6 — 22 octobre 2026

- Optimisation globale à l’édition de liens (LTO complet), avec le niveau
  d’optimisation pour la vitesse conservé à 3. Aucune décompression au lancement.
- Retrait du décodeur FLAC inutilisé par les formats Spotify sélectionnables
  dans ce lecteur ; Vorbis, MP3 et les qualités 96 / 160 / 320 kb/s conservés.
- Retrait des traces détaillées passant par `log` en compilation release ;
  avertissements, erreurs et messages de l’interface conservés.
- Comparaison native Windows automatisée avec la 0.1.5 : taille, premier rendu,
  mémoire résidente, mémoire privée engagée et CPU au repos. Les mesures hors
  connexion ne remplacent pas un essai de lecture avec un compte Spotify.

## 0.1.5 — 22 octobre 2026

- L'éclair du bouton À propos clignote doucement entre doré et vert fluo
  lorsqu'une mise à jour est disponible ou prête à installer.
- L'infobulle distingue la mise à jour disponible de celle qui est téléchargée
  et prête à installer ; le clic ou F2 ouvre directement l'onglet Mise à jour.
- Animation suspendue derrière le panneau À propos et l'aide, et pendant le
  téléchargement. La boucle et le buffer existants sont réutilisés.

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
