//! Static About dialog, rendered into the player's existing bitmap.
use super::*;

impl View {
    pub(super) fn render_about(&mut self) {
        let Some(tab) = self.about else {
            return;
        };

        self.rect(29, 29, 512, 416, 0x0c0c14);
        self.bevel(24, 24, 512, 416, BG, false);
        self.bevel(30, 30, 500, 26, 0x353249, false);
        self.rect(35, 35, 3, 15, GOLD);
        self.text(45, 39, "À PROPOS DE SPOTIAMP", WHITE, 54);

        let close_down = self.button_down(505, 33);
        self.metal_face(505, 33, 20, 18, true, close_down);
        let offset = usize::from(close_down);
        for i in 0..7 {
            self.rect(511 + i + offset, 38 + i + offset, 2, 1, 0x303346);
            self.rect(517 - i + offset, 38 + i + offset, 2, 1, 0x303346);
        }

        for (index, (candidate, label)) in [
            (AboutTab::Player, "Spotiamp"),
            (AboutTab::Credits, "Crédits"),
            (AboutTab::Shortcuts, "Raccourcis"),
            (AboutTab::History, "Historique"),
            (AboutTab::Updates, "Mise à jour"),
        ]
        .into_iter()
        .enumerate()
        {
            let x = 40 + index * 96;
            let selected = candidate == tab;
            let pressed = self.button_down(x, 65);
            self.metal_face(x, 65, 96, 25, true, selected || pressed);
            let offset = usize::from(pressed);
            self.text(
                x + (96 - label.chars().count() * 8) / 2 + offset,
                74 + offset,
                label,
                0x242b40,
                11,
            );
            if selected {
                self.rect(x + 4, 87, 88, 2, GOLD);
            }
            if self.about_focus == index {
                self.about_focus_ring(x + 4, 69, 88, 16);
            }
        }

        self.bevel(40, 96, 480, 300, BLACK, true);
        match tab {
            AboutTab::Player => self.about_player(),
            AboutTab::Credits => self.about_credits(),
            AboutTab::Shortcuts => self.about_shortcuts(),
            AboutTab::History => self.about_history(),
            AboutTab::Updates => self.about_updates(),
        }

        let notice = self
            .transient
            .as_ref()
            .filter(|(_, at)| at.elapsed() < Duration::from_secs(8))
            .map(|(text, _)| text.clone());
        if let Some(notice) = notice {
            self.rect(43, 380, 474, 12, 0x171724);
            self.text(48, 382, &notice, GOLD, 58);
        }

        self.about_action(40, 186, "PROJET GITHUB", 5);
        self.about_action(236, 174, "COPIER LES INFOS", 6);
        self.about_action(420, 100, "FERMER", 7);
    }

    fn about_action(&mut self, x: usize, width: usize, label: &str, focus: usize) {
        let down = self.button_down(x, 405);
        self.metal_face(x, 405, width, 22, true, down);
        let offset = usize::from(down);
        self.text(
            x + (width - label.chars().count() * 8) / 2 + offset,
            412 + offset,
            label,
            0x242b40,
            width / 8,
        );
        if self.about_focus == focus {
            self.about_focus_ring(x + 4, 409, width - 8, 14);
        }
    }

    fn about_focus_ring(&mut self, x: usize, y: usize, width: usize, height: usize) {
        for dx in (0..width).step_by(2) {
            self.rect(x + dx, y, 1, 1, 0x35334a);
            self.rect(x + dx, y + height - 1, 1, 1, 0x35334a);
        }
        for dy in (0..height).step_by(2) {
            self.rect(x, y + dy, 1, 1, 0x35334a);
            self.rect(x + width - 1, y + dy, 1, 1, 0x35334a);
        }
    }

    fn about_centered(&mut self, y: usize, text: &str, color: u32) {
        let width = text.chars().count().min(56) * 8;
        self.text((WIDTH - width) / 2, y, text, color, 56);
    }

    fn about_wordmark(&mut self) {
        // Original angular S monogram; no Winamp artwork or external asset.
        self.bevel(150, 119, 40, 40, 0x353249, false);
        let rows = [0b011110, 0b110000, 0b111100, 0b001111, 0b000011, 0b111110];
        for (y, row) in rows.iter().enumerate() {
            for x in 0..6 {
                if row & (1 << (5 - x)) != 0 {
                    self.rect(158 + x * 4, 127 + y * 4, 4, 4, 0x786a38);
                    self.rect(157 + x * 4, 126 + y * 4, 4, 4, 0xd2bc6a);
                }
            }
        }
        for (index, c) in "SPOTIAMP".chars().enumerate() {
            if let Some(glyph) = font8x8::BASIC_FONTS.get(c) {
                for (row, bits) in glyph.iter().enumerate() {
                    for column in 0..8 {
                        if bits & (1 << column) != 0 {
                            let x = 208 + index * 24 + column * 3;
                            let y = 126 + row * 3;
                            self.rect(x + 2, y + 2, 3, 3, 0x4d475f);
                            self.rect(x, y, 3, 3, WHITE);
                        }
                    }
                }
            }
        }
    }

    fn about_player(&mut self) {
        self.about_wordmark();
        self.rect(152, 174, 256, 1, 0x625a3c);
        self.rect(152, 176, 256, 1, GOLD);
        self.about_centered(191, concat!("Version ", env!("CARGO_PKG_VERSION")), GREEN);
        self.about_centered(227, "Un lecteur Spotify natif au style classique.", WHITE);
        self.about_centered(247, "Un seul skin, vos titres, votre musique.", WHITE);
        self.rect(64, 269, 432, 1, 0x303042);
        self.about_centered(285, "Spotify Premium requis pour écouter.", WHITE);
        self.about_centered(307, "Projet expérimental - Code sous licence MIT.", WHITE);
        self.about_centered(331, "github.com/roukmoute/spotiamp", GREEN);
        self.about_centered(353, "(c) 2026 Spotiamp contributors", DIM);
        self.about_centered(369, "Non affilié à Spotify ou à Winamp.", DIM);
    }

    fn about_credits(&mut self) {
        self.text(54, 111, "SPOTIAMP CONTRIBUTORS", GREEN, 56);
        self.text(
            54,
            132,
            "Copyright (c) 2026 Spotiamp contributors",
            WHITE,
            56,
        );
        self.text(54, 150, "Code du projet : licence MIT.", WHITE, 56);
        self.rect(54, 173, 452, 1, 0x303042);
        for (y, name, role) in [
            (184, "librespot 0.8", "Protocole et lecture Spotify"),
            (201, "Symphonia", "Décodage audio"),
            (218, "Rodio / CPAL", "Sortie audio native"),
            (235, "minifb", "Fenêtre et pixels"),
            (252, "font8x8", "Caractères bitmap"),
            (269, "arboard", "Presse-papiers"),
        ] {
            self.text(54, y, name, GOLD, 17);
            self.text(202, y, role, WHITE, 38);
        }
        self.rect(54, 283, 452, 1, 0x303042);
        self.text(
            54,
            301,
            "Les dépendances gardent leurs licences propres.",
            WHITE,
            56,
        );
        self.text(
            54,
            320,
            "Notices et sources MPL fournies dans le paquet.",
            WHITE,
            56,
        );
        self.text(
            54,
            350,
            "Projet indépendant de Spotify et de Winamp.",
            DIM,
            56,
        );
        self.text(
            54,
            370,
            "Les noms restent ceux de leurs propriétaires.",
            DIM,
            56,
        );
    }

    fn about_shortcuts(&mut self) {
        self.text(54, 111, "DANS CETTE FENÊTRE", GREEN, 56);
        self.rect(54, 126, 452, 1, 0x303042);
        for (y, keys, action) in [
            (140, "Tab / Maj+Tab", "Choisir un onglet ou bouton"),
            (158, "Flèches", "Changer d'onglet"),
            (176, "Entrée / Espace", "Activer le contrôle choisi"),
            (194, "F1", "Afficher les raccourcis"),
            (212, "Échap / F2", "Fermer À propos"),
        ] {
            self.text(54, y, keys, GOLD, 20);
            self.text(230, y, action, WHITE, 35);
        }
        self.text(54, 235, "DANS LE LECTEUR", GREEN, 56);
        self.rect(54, 250, 452, 1, 0x303042);
        for (y, keys, action) in [
            (265, "F1 / F2 / F3", "Aide / À propos / Bibliothèque"),
            (285, "Espace", "Lecture / pause (hors saisie)"),
            (305, "Ctrl+V / Cmd+V", "Coller un lien Spotify"),
            (325, "Entrée", "Ajouter le lien / lire"),
            (345, "Flèches / molette", "Parcourir la file"),
            (365, "Suppr", "Retirer le titre choisi"),
        ] {
            self.text(54, y, keys, GOLD, 20);
            self.text(230, y, action, WHITE, 35);
        }
    }

    fn about_history(&mut self) {
        for (y, version, first, second) in [
            (
                109,
                "0.1.3 / RECONNEXION AUTOMATIQUE",
                "Session mémorisée réutilisée au lancement.",
                "Déconnecter efface la session enregistrée.",
            ),
            (
                142,
                "0.1.2 / SHUFFLE ET REPEAT",
                "Ordre aléatoire et répétition de la file.",
                "Boutons classiques, voyants verts et relief.",
            ),
            (
                175,
                "0.1.1 / BOUTON LECTURE",
                "En pause : reprise à la même position.",
                "En lecture : retour au début du morceau.",
            ),
            (
                208,
                "0.1.0 / INSTALLATEUR WINDOWS",
                "Installation par utilisateur et raccourcis.",
                "Désinstallation et mises à jour intégrées.",
            ),
            (
                241,
                "ALPHA.8 / PLAYLISTS SPOTIFY",
                "Bibliothèque native, playlists du compte.",
                "Titres à la demande, lecture et ajout à la file.",
            ),
            (
                274,
                "ALPHA.7 / FENÊTRE INTÉGRÉE",
                "Bandeau déplaçable, boutons Réduire et Fermer.",
                "Cadre système retiré, même interface légère.",
            ),
            (
                307,
                "ALPHA.6 / MISES À JOUR",
                "Détection, téléchargement et installation.",
                "Vérification du paquet puis redémarrage.",
            ),
            (
                340,
                "ALPHA.5 / À PROPOS",
                "Version, crédits, raccourcis et historique.",
                "Quatre onglets, accessibles aussi au clavier.",
            ),
        ] {
            self.text(54, y, version, GOLD, 56);
            self.text(54, y + 14, first, WHITE, 56);
            self.text(54, y + 25, second, DIM, 56);
        }
        self.text(
            54,
            382,
            "Historique complet : CHANGELOG.md dans les sources.",
            DIM,
            56,
        );
    }

    fn about_updates(&mut self) {
        self.text(54, 111, "MISES À JOUR DE SPOTIAMP", GREEN, 56);
        self.text(
            54,
            132,
            concat!("Version actuelle : ", env!("CARGO_PKG_VERSION")),
            WHITE,
            56,
        );
        self.rect(54, 148, 452, 1, 0x303042);

        let status = self.update.clone();
        let (message, color) = match &status {
            UpdateStatus::Disabled => {
                ("Mises à jour indisponibles dans cette version.".into(), DIM)
            }
            UpdateStatus::Idle => ("Recherchez une nouvelle version du lecteur.".into(), WHITE),
            UpdateStatus::Checking => ("Recherche d'une nouvelle version...".into(), GOLD),
            UpdateStatus::Current => (
                "Vous disposez de la dernière version compatible.".into(),
                GREEN,
            ),
            UpdateStatus::Available { version } => {
                (format!("Version disponible : {version}"), GREEN)
            }
            UpdateStatus::Downloading { percent } => (
                format!("Téléchargement du paquet : {} %", (*percent).min(100)),
                GOLD,
            ),
            UpdateStatus::Ready { version } => {
                (format!("Version {version} prête à installer."), GREEN)
            }
            UpdateStatus::Installing => ("Installation en cours, puis redémarrage...".into(), GOLD),
            UpdateStatus::Failed(_) => ("La mise à jour n'a pas abouti.".into(), GOLD),
        };
        self.text(54, 162, &message, color, 56);
        if let UpdateStatus::Failed(error) = &status {
            // Bound error text to the content area, including long server errors.
            let chars: Vec<char> = error.chars().take(112).collect();
            for (index, line) in chars.chunks(56).enumerate() {
                self.text(
                    54,
                    180 + index * 14,
                    &line.iter().collect::<String>(),
                    WHITE,
                    56,
                );
            }
        }
        if let UpdateStatus::Downloading { percent } = status {
            self.bevel(64, 192, 432, 13, 0x171724, true);
            self.rect(
                67,
                195,
                426 * usize::from(percent.min(100)) / 100,
                7,
                0x69a23c,
            );
        }

        self.text(
            54,
            228,
            "La lecture continue pendant le téléchargement.",
            WHITE,
            56,
        );
        self.text(
            54,
            253,
            "Installer redémarre le lecteur et interrompt",
            WHITE,
            56,
        );
        self.text(
            54,
            269,
            "la lecture. La file n'est pas conservée.",
            GOLD,
            56,
        );

        self.update_button(64, "VÉRIFIER", 8, self.can_check_update());
        let (label, enabled) = match status {
            UpdateStatus::Available { .. } => ("TÉLÉCHARGER", true),
            UpdateStatus::Ready { .. } => ("INSTALLER / REDÉMARRER", true),
            UpdateStatus::Downloading { .. } => ("TÉLÉCHARGEMENT...", false),
            UpdateStatus::Installing => ("INSTALLATION...", false),
            _ => ("TÉLÉCHARGER", false),
        };
        self.update_button(286, label, 9, enabled);
        self.text(
            54,
            354,
            if env!("CARGO_PKG_VERSION").contains('-') {
                "Canal préversion : préversions incluses."
            } else {
                "Versions stables uniquement."
            },
            DIM,
            56,
        );
        self.text(
            54,
            370,
            "Le paquet est vérifié avant son installation.",
            DIM,
            56,
        );
    }

    fn update_button(&mut self, x: usize, label: &str, focus: usize, enabled: bool) {
        let down = enabled && self.button_down(x, 315);
        self.metal_face(x, 315, 208, 24, enabled, down);
        let offset = usize::from(down);
        self.text(
            x + (208 - label.chars().count() * 8) / 2 + offset,
            323 + offset,
            label,
            if enabled { 0x242b40 } else { 0x494d62 },
            24,
        );
        if self.about_focus == focus {
            self.about_focus_ring(x + 4, 319, 200, 16);
        }
    }
}
