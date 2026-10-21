# Changements

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
