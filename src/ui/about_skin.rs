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
        self.text(
            45,
            39,
            self.language.text("À PROPOS DE SPOTIAMP", "ABOUT SPOTIAMP"),
            WHITE,
            54,
        );

        let close_down = self.button_down(505, 33);
        self.metal_face(505, 33, 20, 18, true, close_down);
        let offset = usize::from(close_down);
        for i in 0..7 {
            self.rect(511 + i + offset, 38 + i + offset, 2, 1, 0x303346);
            self.rect(517 - i + offset, 38 + i + offset, 2, 1, 0x303346);
        }

        for (index, (candidate, label)) in [
            (AboutTab::Player, "Spotiamp"),
            (AboutTab::Credits, self.language.text("Crédits", "Credits")),
            (
                AboutTab::Shortcuts,
                self.language.text("Raccourcis", "Shortcuts"),
            ),
            (
                AboutTab::History,
                self.language.text("Historique", "History"),
            ),
            (
                AboutTab::Updates,
                self.language.text("Mise à jour", "Updates"),
            ),
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
            .map(|(text, _)| text.text(self.language).to_owned());
        if let Some(notice) = notice {
            self.rect(43, 380, 474, 12, 0x171724);
            self.text(48, 382, &notice, GOLD, 58);
        }

        self.about_action(
            40,
            186,
            self.language.text("PROJET GITHUB", "GITHUB PROJECT"),
            5,
        );
        self.about_action(
            236,
            174,
            self.language.text("COPIER LES INFOS", "COPY INFO"),
            6,
        );
        self.about_action(420, 100, self.language.text("FERMER", "CLOSE"), 7);
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
        self.about_centered(
            183,
            &format!(
                "{} {}",
                self.language.text("Version", "Version"),
                env!("CARGO_PKG_VERSION")
            ),
            GREEN,
        );
        self.text(54, 207, self.language.text("LANGUE", "LANGUAGE"), WHITE, 12);
        for (index, preference) in LANGUAGE_CHOICES.into_iter().enumerate() {
            let (x, y, width, height) = Button::Language(preference).rect();
            let label = match preference {
                LanguagePreference::Auto => "Auto",
                LanguagePreference::Specific(language) => language.native_name(),
            };
            let down = self.button_down(x, y);
            self.metal_face(
                x,
                y,
                width,
                height,
                true,
                down || self.language_preference == preference,
            );
            let offset = usize::from(down);
            self.text(
                x + (width - label.chars().count() * 8) / 2 + offset,
                y + 7 + offset,
                label,
                0x242b40,
                12,
            );
            if self.about_focus == 10 + index {
                self.about_focus_ring(x + 4, y + 4, width - 8, height - 8);
            }
        }
        self.about_centered(
            264,
            self.language.text(
                "Un lecteur Spotify natif, léger et compact.",
                "A small, lightweight native Spotify player.",
            ),
            WHITE,
        );
        self.about_centered(
            281,
            self.language.text(
                "Deux skins intégrés : Classique et Modern.",
                "Two built-in skins: Classic and Modern.",
            ),
            WHITE,
        );
        self.rect(64, 296, 432, 1, 0x303042);
        self.about_centered(
            310,
            self.language.text(
                "Spotify Premium requis pour écouter.",
                "Spotify Premium is required to listen.",
            ),
            WHITE,
        );
        self.about_centered(
            327,
            self.language.text(
                "Logiciel libre sous licence MIT.",
                "Free and open-source software. MIT license.",
            ),
            WHITE,
        );
        self.about_centered(344, "github.com/roukmoute/spotiamp", GREEN);
        self.about_centered(355, "(c) 2026 Spotiamp contributors", DIM);
        self.about_centered(
            369,
            self.language.text(
                "Non affilié à Spotify ou à Winamp.",
                "Not affiliated with Spotify or Winamp.",
            ),
            DIM,
        );
    }

    fn about_credits(&mut self) {
        self.text(
            54,
            111,
            self.language
                .text("CONTRIBUTEURS SPOTIAMP", "SPOTIAMP CONTRIBUTORS"),
            GREEN,
            56,
        );
        self.text(
            54,
            132,
            "Copyright (c) 2026 Spotiamp contributors",
            WHITE,
            56,
        );
        self.text(
            54,
            150,
            self.language.text(
                "Code du projet : licence MIT.",
                "Project code: MIT license.",
            ),
            WHITE,
            56,
        );
        self.rect(54, 173, 452, 1, 0x303042);
        for (y, name, role) in [
            (
                184,
                "librespot 0.8",
                self.language.text(
                    "Protocole et lecture Spotify",
                    "Spotify protocol and playback",
                ),
            ),
            (
                201,
                "Symphonia",
                self.language.text("Décodage audio", "Audio decoding"),
            ),
            (
                218,
                "Rodio / CPAL",
                self.language
                    .text("Sortie audio native", "Native audio output"),
            ),
            (
                235,
                "minifb",
                self.language.text("Fenêtre et pixels", "Window and pixels"),
            ),
            (
                252,
                "font8x8",
                self.language.text("Caractères bitmap", "Bitmap fonts"),
            ),
            (
                269,
                "arboard",
                self.language.text("Presse-papiers", "Clipboard"),
            ),
        ] {
            self.text(54, y, name, GOLD, 17);
            self.text(202, y, role, WHITE, 38);
        }
        self.rect(54, 283, 452, 1, 0x303042);
        self.text(
            54,
            301,
            self.language.text(
                "Les dépendances gardent leurs licences propres.",
                "Dependencies retain their own licenses.",
            ),
            WHITE,
            56,
        );
        self.text(
            54,
            320,
            self.language.text(
                "Notices et sources MPL fournies dans le paquet.",
                "Notices and MPL sources are included in the package.",
            ),
            WHITE,
            56,
        );
        self.text(
            54,
            350,
            self.language.text(
                "Projet indépendant de Spotify et de Winamp.",
                "An independent project, separate from Spotify/Winamp.",
            ),
            DIM,
            56,
        );
        self.text(
            54,
            370,
            self.language.text(
                "Les noms restent ceux de leurs propriétaires.",
                "All names belong to their respective owners.",
            ),
            DIM,
            56,
        );
    }

    fn about_shortcuts(&mut self) {
        self.text(
            54,
            111,
            self.language.text("DANS CETTE FENÊTRE", "IN THIS WINDOW"),
            GREEN,
            56,
        );
        self.rect(54, 126, 452, 1, 0x303042);
        for (y, keys, action) in [
            (
                140,
                self.language.text("Tab / Maj+Tab", "Tab / Shift+Tab"),
                self.language
                    .text("Choisir un onglet ou bouton", "Choose a tab or button"),
            ),
            (
                158,
                self.language.text("Flèches", "Arrows"),
                self.language.text("Changer d'onglet", "Switch tabs"),
            ),
            (
                176,
                self.language.text("Entrée / Espace", "Enter / Space"),
                self.language.text(
                    "Activer le contrôle choisi",
                    "Activate the selected control",
                ),
            ),
            (
                194,
                "F1",
                self.language
                    .text("Afficher les raccourcis", "Show shortcuts"),
            ),
            (
                212,
                self.language.text("Échap / F2", "Esc / F2"),
                self.language.text("Fermer À propos", "Close About"),
            ),
        ] {
            self.text(54, y, keys, GOLD, 20);
            self.text(230, y, action, WHITE, 35);
        }
        self.text(
            54,
            235,
            self.language.text("DANS LE LECTEUR", "IN THE PLAYER"),
            GREEN,
            56,
        );
        self.rect(54, 250, 452, 1, 0x303042);
        for (y, keys, action) in [
            (
                265,
                "F1 / F2 / F3",
                self.language
                    .text("Aide / À propos / Bibliothèque", "Help / About / Library"),
            ),
            (
                282,
                "F4",
                self.language.text("Changer de skin", "Switch skins"),
            ),
            (
                299,
                self.language.text("Espace", "Space"),
                self.language.text(
                    "Lecture / pause (hors saisie)",
                    "Play / pause (outside text input)",
                ),
            ),
            (
                316,
                "Ctrl+V / Cmd+V",
                self.language
                    .text("Coller un lien Spotify", "Paste a Spotify link"),
            ),
            (
                333,
                self.language.text("Entrée", "Enter"),
                self.language
                    .text("Ajouter le lien / lire", "Add the link / play"),
            ),
            (
                350,
                self.language.text("Flèches / molette", "Arrows / wheel"),
                self.language.text("Parcourir la file", "Browse the queue"),
            ),
            (
                367,
                self.language.text("Suppr", "Delete"),
                self.language
                    .text("Retirer le titre choisi", "Remove the selected track"),
            ),
        ] {
            self.text(54, y, keys, GOLD, 20);
            self.text(230, y, action, WHITE, 35);
        }
    }

    fn about_history(&mut self) {
        for (y, version, first, second) in [
            (
                109,
                self.language
                    .text("0.2.12 / RECHERCHE SPOTIFY", "0.2.12 / SPOTIFY SEARCH"),
                self.language.text(
                    "Rechercher un titre ou un artiste dans la bibliothèque.",
                    "Find a song or artist in the library.",
                ),
                self.language.text(
                    "Double-clic : lire. À la file : ajouter le titre.",
                    "Double-click: play. Add to queue: append the track.",
                ),
            ),
            (
                154,
                self.language
                    .text("0.2.11 / SIX LANGUES", "0.2.11 / SIX LANGUAGES"),
                self.language.text(
                    "Français, anglais, allemand, espagnol,",
                    "French, English, German, Spanish,",
                ),
                self.language.text(
                    "italien et portugais. Choix mémorisé.",
                    "Italian and Portuguese. Remembered choice.",
                ),
            ),
            (
                199,
                self.language
                    .text("0.2.10 / REPRISE MÉMORISÉE", "0.2.10 / REMEMBERED PLAYBACK"),
                self.language.text(
                    "Playlist, morceau et position retrouvés au lancement.",
                    "Playlist, track and position restored at startup.",
                ),
                self.language.text(
                    "Lecture reprend où vous vous êtes arrêté.",
                    "Play resumes where you left off.",
                ),
            ),
            (
                244,
                self.language.text(
                    "0.2.9 / LANGUES DE L'INTERFACE",
                    "0.2.9 / INTERFACE LANGUAGES",
                ),
                self.language.text(
                    "Français et anglais, avec détection du système.",
                    "French and English, with system language detection.",
                ),
                self.language.text(
                    "Choix dans À propos, mémorisé au redémarrage.",
                    "Choose in About; remembered across restarts.",
                ),
            ),
            (
                289,
                self.language.text(
                    "0.2.8 / LECTEUR SPOTIFY NATIF",
                    "0.2.8 / NATIVE SPOTIFY PLAYER",
                ),
                self.language.text(
                    "Présentation harmonisée dans le lecteur et l'aide.",
                    "Consistent presentation in the player and help.",
                ),
                self.language.text(
                    "À propos et documentation mis à jour.",
                    "About and documentation updated.",
                ),
            ),
            (
                334,
                self.language.text(
                    "0.2.7 / DISPOSITION MÉMORISÉE",
                    "0.2.7 / REMEMBERED WINDOW LAYOUT",
                ),
                self.language.text(
                    "Positions et fenêtres attachées ou libres mémorisées.",
                    "Window positions and attached/free layout remembered.",
                ),
                self.language.text(
                    "Bibliothèque ouverte ou fermée retrouvée au lancement.",
                    "Library open/closed state restored at startup.",
                ),
            ),
        ] {
            self.text(54, y, version, GOLD, 56);
            self.text(54, y + 14, first, WHITE, 56);
            self.text(54, y + 25, second, DIM, 56);
        }
        self.text(
            54,
            382,
            self.language.text(
                "Historique complet : CHANGELOG.md dans les sources.",
                "Full history: CHANGELOG.md in the source package.",
            ),
            DIM,
            56,
        );
    }

    fn about_updates(&mut self) {
        self.text(
            54,
            111,
            self.language
                .text("MISES À JOUR DE SPOTIAMP", "SPOTIAMP UPDATES"),
            GREEN,
            56,
        );
        let version_label = self.language.text("Version actuelle :", "Current version:");
        self.text(54, 132, version_label, WHITE, 40);
        self.text(
            54 + (version_label.chars().count() + 1) * 8,
            132,
            env!("CARGO_PKG_VERSION"),
            WHITE,
            15,
        );
        self.rect(54, 148, 452, 1, 0x303042);

        let status = self.update.clone();
        let (message, color) = match &status {
            UpdateStatus::Disabled => (
                self.language
                    .text(
                        "Mises à jour indisponibles dans cette version.",
                        "Updates are unavailable in this build.",
                    )
                    .into(),
                DIM,
            ),
            UpdateStatus::Idle => (
                self.language
                    .text(
                        "Recherchez une nouvelle version du lecteur.",
                        "Check for a new version of the player.",
                    )
                    .into(),
                WHITE,
            ),
            UpdateStatus::Checking => (
                self.language
                    .text(
                        "Recherche d'une nouvelle version...",
                        "Checking for a new version...",
                    )
                    .into(),
                GOLD,
            ),
            UpdateStatus::Current => (
                self.language
                    .text(
                        "Vous disposez de la dernière version compatible.",
                        "You have the latest compatible version.",
                    )
                    .into(),
                GREEN,
            ),
            UpdateStatus::Available { version } => (
                match self.language {
                    crate::locale::Language::French => format!("Version disponible : {version}"),
                    crate::locale::Language::English => format!("Available version: {version}"),
                    crate::locale::Language::German => format!("Verfügbare Version: {version}"),
                    crate::locale::Language::Spanish => format!("Versión disponible: {version}"),
                    crate::locale::Language::Italian => format!("Versione disponibile: {version}"),
                    crate::locale::Language::Portuguese => format!("Versão disponível: {version}"),
                },
                GREEN,
            ),
            UpdateStatus::Downloading { percent } => (
                match self.language {
                    crate::locale::Language::French => {
                        format!("Téléchargement du paquet : {} %", (*percent).min(100))
                    }
                    crate::locale::Language::English => {
                        format!("Downloading package: {} %", (*percent).min(100))
                    }
                    crate::locale::Language::German => {
                        format!("Paket wird geladen: {} %", (*percent).min(100))
                    }
                    crate::locale::Language::Spanish => {
                        format!("Descargando paquete: {} %", (*percent).min(100))
                    }
                    crate::locale::Language::Italian => {
                        format!("Download del pacchetto: {} %", (*percent).min(100))
                    }
                    crate::locale::Language::Portuguese => {
                        format!("A descarregar pacote: {} %", (*percent).min(100))
                    }
                },
                GOLD,
            ),
            UpdateStatus::Ready { version } => (
                match self.language {
                    crate::locale::Language::French => {
                        format!("Version {version} prête à installer.")
                    }
                    crate::locale::Language::English => {
                        format!("Version {version} is ready to install.")
                    }
                    crate::locale::Language::German => {
                        format!("Version {version} ist bereit zur Installation.")
                    }
                    crate::locale::Language::Spanish => {
                        format!("Versión {version} lista para instalar.")
                    }
                    crate::locale::Language::Italian => {
                        format!("Versione {version} pronta da installare.")
                    }
                    crate::locale::Language::Portuguese => {
                        format!("Versão {version} pronta a instalar.")
                    }
                },
                GREEN,
            ),
            UpdateStatus::Installing => (
                self.language
                    .text(
                        "Installation en cours, puis redémarrage...",
                        "Installing, then restarting...",
                    )
                    .into(),
                GOLD,
            ),
            UpdateStatus::Failed(_) => (
                self.language
                    .text(
                        "La mise à jour n'a pas abouti.",
                        "The update did not complete.",
                    )
                    .into(),
                GOLD,
            ),
        };
        self.text(54, 162, &message, color, 56);
        if let UpdateStatus::Failed(error) = &status {
            // Bound error text to the content area, including long server errors.
            let chars: Vec<char> = error.text(self.language).chars().take(112).collect();
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
            self.language.text(
                "La lecture continue pendant le téléchargement.",
                "Playback continues while downloading.",
            ),
            WHITE,
            56,
        );
        self.text(
            54,
            253,
            self.language.text(
                "Installer redémarre le lecteur et interrompt",
                "Installing restarts the player and interrupts",
            ),
            WHITE,
            56,
        );
        self.text(
            54,
            269,
            self.language.text(
                "la lecture. La file et la position sont mémorisées.",
                "playback. The queue and position are remembered.",
            ),
            GOLD,
            56,
        );

        self.update_button(
            64,
            self.language.text("VÉRIFIER", "CHECK"),
            8,
            self.can_check_update(),
        );
        let (label, enabled) = match status {
            UpdateStatus::Available { .. } => (self.language.text("TÉLÉCHARGER", "DOWNLOAD"), true),
            UpdateStatus::Ready { .. } => (
                self.language
                    .text("INSTALLER / REDÉMARRER", "INSTALL / RESTART"),
                true,
            ),
            UpdateStatus::Downloading { .. } => (
                self.language.text("TÉLÉCHARGEMENT...", "DOWNLOADING..."),
                false,
            ),
            UpdateStatus::Installing => (
                self.language.text("INSTALLATION...", "INSTALLING..."),
                false,
            ),
            _ => (self.language.text("TÉLÉCHARGER", "DOWNLOAD"), false),
        };
        self.update_button(286, label, 9, enabled);
        self.text(
            54,
            354,
            if env!("CARGO_PKG_VERSION").contains('-') {
                self.language.text(
                    "Canal préversion : préversions incluses.",
                    "Preview channel: includes pre-releases.",
                )
            } else {
                self.language
                    .text("Versions stables uniquement.", "Stable versions only.")
            },
            DIM,
            56,
        );
        self.text(
            54,
            370,
            self.language.text(
                "Le paquet est vérifié avant son installation.",
                "The package is verified before installation.",
            ),
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
