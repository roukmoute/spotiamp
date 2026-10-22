//! Silver and blue skin, drawn into the same framebuffer and control geometry.
use super::skin::{TransportIcon, format_time};
use super::*;
use crate::appearance::{button, gradient, panel};

const NAVY: u32 = 0x071c30;
const CYAN: u32 = 0x6ce5ff;
const ICE: u32 = 0xeaf8ff;
const MUTED: u32 = 0x91b3cb;
const INK: u32 = 0x233e59;

impl View {
    fn modern_panel(&mut self, rect: (usize, usize, usize, usize)) {
        panel(&mut self.pixels, WIDTH, rect, NAVY, true);
        let (x, y, w, h) = rect;
        gradient(
            &mut self.pixels,
            WIDTH,
            (x + 2, y + 2, w - 4, h - 4),
            0x13364f,
            NAVY,
        );
        self.rect(x + 2, y + 2, w - 4, 1, 0x385a71);
    }

    fn modern_header(&mut self, y: usize, title: &str) {
        gradient(
            &mut self.pixels,
            WIDTH,
            (3, y, WIDTH - 6, 18),
            0xf9fcff,
            0xafbfce,
        );
        self.rect(4, y + 17, WIDTH - 8, 1, 0x70869d);
        let left = (WIDTH - title.len() * 6) / 2;
        for (x, width) in [
            (12, left.saturating_sub(25)),
            (WIDTH - left + 10, left.saturating_sub(25)),
        ] {
            self.rect(x, y + 7, width, 1, 0x7f95ab);
            self.rect(x, y + 8, width, 1, 0xf4faff);
        }
        self.tiny_text(
            left,
            y + 6,
            title,
            if self.focused { 0x1d4365 } else { 0x6a7b8c },
        );
    }

    fn modern_button(&mut self, control: Button, label: &str, enabled: bool) {
        let (x, y, w, h) = control.rect();
        let down = self.button_down(x, y);
        button(&mut self.pixels, WIDTH, (x, y, w, h), enabled, down);
        let d = usize::from(down);
        self.text(
            x + w.saturating_sub(label.chars().count() * 8) / 2 + d,
            y + (h - 8) / 2 + d,
            label,
            if enabled { INK } else { 0x6b7c8d },
            w / 8,
        );
    }

    fn modern_transport(&mut self, x: usize, icon: TransportIcon, enabled: bool) {
        let down = self.button_down(x, 116);
        // Capsule profile stays wholly inside the original 38-by-22 hitbox.
        for row in 0..22 {
            let edge = row.min(21 - row);
            let inset = [8, 5, 3, 2, 1, 1, 0, 0, 0, 0, 0][edge];
            self.rect(x + inset, 116 + row, 38 - 2 * inset, 1, 0x567087);
            if row > 0 && row < 21 {
                let (top, bottom) = if !enabled {
                    (0xd1dce5, 0xa2b2c2)
                } else if down {
                    (0x7d9ab2, 0xc8dce9)
                } else if row < 10 {
                    (0xffffff, 0xd4e7f2)
                } else {
                    (0xc0d5e5, 0x8ca8bf)
                };
                gradient(
                    &mut self.pixels,
                    WIDTH,
                    (x + inset + 1, 116 + row, 36 - 2 * inset, 1),
                    if row < 10 { top } else { bottom },
                    bottom,
                );
            }
        }
        let d = usize::from(down);
        self.transport_glyph(
            x + 20 + d,
            128 + d,
            icon,
            if enabled { 0xf2fcff } else { 0xc4d1dd },
        );
        self.transport_glyph(
            x + 19 + d,
            127 + d,
            icon,
            if enabled { 0x205779 } else { 0x71889b },
        );
    }

    fn modern_mode(&mut self, control: Button, active: bool) {
        let (x, y, w, h) = control.rect();
        let down = self.button_down(x, y);
        button(&mut self.pixels, WIDTH, (x, y, w, h), true, down || active);
        let d = usize::from(down);
        if control == Button::Shuffle {
            self.rect(
                x + 5 + d,
                y + 8 + d,
                3,
                6,
                if active { 0x247b99 } else { 0x8193a5 },
            );
            self.rect(
                x + 5 + d,
                y + 8 + d,
                2,
                2,
                if active { CYAN } else { 0xb5c2cd },
            );
            self.tiny_text(x + 11 + d, y + 9 + d, "SHUFFLE", INK);
        } else {
            let color = if active { 0x007ca5 } else { INK };
            self.rect(x + 6 + d, y + 6 + d, 9, 2, color);
            self.rect(x + 5 + d, y + 7 + d, 2, 3, color);
            self.rect(x + 6 + d, y + 13 + d, 9, 2, color);
            self.rect(x + 14 + d, y + 11 + d, 2, 3, color);
            self.triangle(x + 13 + d, y + 5 + d, 3, true, color);
            self.triangle(x + 5 + d, y + 11 + d, 3, false, color);
        }
    }

    fn modern_grip(&mut self, rect: (usize, usize, usize, usize), down: bool) {
        button(&mut self.pixels, WIDTH, rect, true, down);
        let (x, y, w, h) = rect;
        let d = usize::from(down);
        for dx in (4..w.saturating_sub(3)).step_by(3) {
            self.rect(x + dx + d, y + 3 + d, 1, h - 6, 0x648198);
            self.rect(x + dx + 1 + d, y + 3 + d, 1, h - 6, 0xf4fcff);
        }
    }

    pub(super) fn render_modern(&mut self, s: &Snapshot) {
        gradient(
            &mut self.pixels,
            WIDTH,
            (0, 0, WIDTH, HEIGHT),
            0xe2e8ef,
            0xa4b2c2,
        );
        // Fixed fine highlights suggest brushed metal without noise, textures,
        // or animation that would cause idle repainting.
        for y in (22..HEIGHT - 2).step_by(3) {
            self.rect(
                3,
                y,
                WIDTH - 6,
                1,
                if y < 145 { 0xd2dce7 } else { 0xb9c6d3 },
            );
        }
        self.rect(0, 0, WIDTH, 1, 0xf9fcff);
        self.rect(0, 0, 1, HEIGHT, 0xf9fcff);
        self.rect(WIDTH - 1, 0, 1, HEIGHT, 0x50667c);
        self.rect(0, HEIGHT - 1, WIDTH, 1, 0x50667c);
        self.modern_header(2, "SPOTIAMP");
        self.titlebar_buttons();

        self.modern_panel((14, 28, 187, 61));
        let sec = s.position_ms / 1000;
        let min = (sec / 60).min(99);
        for (x, n) in [
            (24, min / 10),
            (54, min % 10),
            (98, sec % 60 / 10),
            (128, sec % 10),
        ] {
            self.digit(x, 37, n as u8);
        }
        self.rect(87, 49, 3, 4, CYAN);
        self.rect(87, 67, 3, 4, CYAN);
        if s.playing {
            self.triangle(169, 39, 5, true, CYAN);
        } else {
            self.rect(168, 39, 3, 9, MUTED);
            self.rect(174, 39, 3, 9, MUTED);
        }
        self.tiny_text(164, 72, "PCM", MUTED);
        self.modern_button(
            Button::Connection,
            ConnectionAction::for_state(s).label(),
            true,
        );
        self.rect(214, 36, 4, 4, if s.connected { 0x008faf } else { 0x8393a4 });
        self.text(
            225,
            34,
            if s.connected {
                "SPOTIFY / EN LIGNE"
            } else {
                "SPOTIFY / HORS LIGNE"
            },
            INK,
            24,
        );
        self.modern_panel((213, 57, 333, 32));
        let row = s.current.and_then(|i| s.queue.get(i));
        self.text(
            219,
            62,
            row.map(|r| r.title.as_str())
                .unwrap_or("Votre musique. Tout simplement."),
            ICE,
            40,
        );
        self.text(
            219,
            76,
            row.map(|r| r.artist.as_str())
                .unwrap_or("SPOTIAMP / MODERN"),
            MUTED,
            40,
        );

        panel(&mut self.pixels, WIDTH, (14, 97, 532, 9), 0x3d5a73, true);
        self.rect(17, 100, 526, 2, 0x7591a8);
        if s.duration_ms > 0 {
            let progress =
                (s.position_ms.min(s.duration_ms) as u64 * 526 / s.duration_ms as u64) as usize;
            self.rect(17, 100, progress, 2, 0x0b95bd);
            let knob = (15 + progress).saturating_sub(10).clamp(15, 523);
            self.modern_grip((knob, 96, 22, 11), self.dragging == 1);
        }
        let notice = match self.update {
            UpdateStatus::Available { .. } => Some("Mise à jour disponible - F2"),
            UpdateStatus::Ready { .. } => Some("Mise à jour prête - F2"),
            _ => None,
        };
        if let Some(notice) = notice {
            self.text(236, 108, notice, INK, 38);
        }
        for (x, icon) in [
            (15, TransportIcon::Previous),
            (57, TransportIcon::Play),
            (99, TransportIcon::Pause),
            (141, TransportIcon::Stop),
            (183, TransportIcon::Next),
        ] {
            self.modern_transport(x, icon, s.connected);
        }
        self.modern_mode(Button::Shuffle, s.shuffle);
        self.modern_mode(Button::Repeat, s.repeat);
        let down = self.button_down(308, 116);
        button(&mut self.pixels, WIDTH, Button::About.rect(), true, down);
        let d = usize::from(down);
        let bolt = if self.update_indicator_lit() {
            GREEN
        } else {
            0x126b97
        };
        for (row, bits) in [
            0x018u16, 0x030, 0x060, 0x0c0, 0x1fe, 0x03c, 0x078, 0x070, 0x060, 0x040,
        ]
        .iter()
        .enumerate()
        {
            for col in 0..9 {
                if bits & (1 << (8 - col)) != 0 {
                    self.rect(315 + col + d, 122 + row + d, 1, 1, 0xeafaff);
                    self.rect(314 + col + d, 121 + row + d, 1, 1, bolt);
                }
            }
        }
        if notice.is_some() {
            self.rect(325 + d, 119 + d, 3, 3, 0x285e24);
            self.rect(325 + d, 119 + d, 2, 2, GREEN);
        }
        panel(&mut self.pixels, WIDTH, (337, 123, 207, 8), 0x35536b, true);
        let volume = s.volume as usize * 201 / 65535;
        self.rect(340, 126, volume, 2, 0x16a6d0);
        self.modern_grip(
            (
                (337 + volume).saturating_sub(5).clamp(337, 531),
                118,
                13,
                18,
            ),
            self.dragging == 2,
        );
        self.tiny_text(410, 136, "VOLUME", INK);

        self.rect(4, 142, WIDTH - 8, 1, 0x667e94);
        self.rect(4, 143, WIDTH - 8, 1, 0xf1f8fe);
        self.modern_header(145, "LISTE DE LECTURE");
        self.modern_button(Button::Library, "BIBLIO. F3", true);
        self.modern_panel((14, 171, 408, 23));
        if self.input_active {
            self.rect(16, 192, 404, 1, CYAN);
        }
        let input = if self.input.is_empty() {
            "Coller un lien Spotify : titre, album...".to_owned()
        } else {
            self.input
                .chars()
                .rev()
                .take(49)
                .collect::<String>()
                .chars()
                .rev()
                .collect()
        };
        if self.select_all && !self.input.is_empty() {
            self.rect(19, 177, input.chars().count() * 8, 11, 0x245b83);
        }
        self.text(
            20,
            179,
            &input,
            if self.input.is_empty() { MUTED } else { ICE },
            49,
        );
        self.modern_button(Button::Paste, "COL.", true);
        self.modern_button(Button::Add, "+ AJ.", s.connected && !s.busy);
        gradient(
            &mut self.pixels,
            WIDTH,
            (15, 203, 530, 14),
            0x436681,
            0x23435d,
        );
        self.rect(15, 203, 530, 1, 0x7b99b1);
        self.tiny_text(19, 208, "TITRE / ARTISTE", ICE);
        self.tiny_text(502, 208, "DUREE", ICE);
        self.modern_panel((14, 219, 532, 195));
        self.offset = self.offset.min(s.queue.len().saturating_sub(ROWS));
        if self.selected.is_some_and(|i| i >= s.queue.len()) {
            self.selected = s.queue.len().checked_sub(1);
        }
        if s.queue.is_empty() {
            self.text(78, 266, "LE PLAISIR D'UN PETIT LECTEUR.", CYAN, 49);
            for (y, text) in [
                (299, "1. Cliquez sur CONNEXION."),
                (320, "2. Ouvrez vos playlists : BIBLIO. F3."),
                (341, "3. Double-cliquez une playlist pour lire."),
            ] {
                self.text(57, y, text, ICE, 58);
            }
            self.text(57, 378, "Compte Premium requis. Aide : F1", MUTED, 55);
        } else {
            for (display, (index, row)) in s
                .queue
                .iter()
                .enumerate()
                .skip(self.offset)
                .take(ROWS)
                .enumerate()
            {
                let y = 221 + display * 24;
                let selected = self.selected == Some(index);
                let current = s.current == Some(index);
                if selected {
                    gradient(
                        &mut self.pixels,
                        WIDTH,
                        (16, y, 520, 24),
                        0x28618a,
                        0x163d60,
                    );
                    self.rect(16, y, 520, 1, 0x5da3c6);
                } else if display % 2 == 1 {
                    self.rect(16, y, 520, 24, 0x0b243a);
                }
                if current {
                    self.triangle(17, y + 8, 4, true, CYAN);
                }
                self.text(24, y + 8, &format!("{:03}", index + 1), MUTED, 3);
                self.text(
                    59,
                    y + 3,
                    &row.title,
                    if current || selected { ICE } else { 0xc3e4f5 },
                    52,
                );
                self.text(59, y + 14, &row.artist, MUTED, 52);
                self.text(493, y + 8, &format_time(row.duration_ms), CYAN, 6);
            }
            if s.queue.len() > ROWS {
                self.rect(538, 221, 6, 190, 0x1d3b52);
                let h = (190 * ROWS / s.queue.len()).max(6);
                let top = self.offset * (190 - h) / (s.queue.len() - ROWS);
                gradient(
                    &mut self.pixels,
                    WIDTH,
                    (539, 221 + top, 4, h),
                    0xb9e6f4,
                    0x568fab,
                );
            }
        }
        self.modern_button(Button::Remove, "SUPPRIMER", self.selected.is_some());
        self.modern_button(Button::Clear, "VIDER", !s.queue.is_empty());
        self.modern_panel((197, 419, 83, 17));
        self.text(202, 424, &format!("{:03}", s.queue.len()), CYAN, 3);
        self.tiny_text(232, 426, "TITRES", MUTED);
        self.tiny_text(300, 426, "KB/S", INK);
        for (control, quality) in [
            (Button::Quality96, AudioQuality::Kbps96),
            (Button::Quality160, AudioQuality::Kbps160),
            (Button::Quality320, AudioQuality::Kbps320),
        ] {
            let (x, y, w, h) = control.rect();
            let selected = quality == s.quality;
            let down = self.button_down(x, y);
            button(
                &mut self.pixels,
                WIDTH,
                (x, y, w, h),
                true,
                down || selected,
            );
            let d = usize::from(down);
            self.rect(
                x + 6 + d,
                425 + d,
                3,
                5,
                if selected { 0x008eac } else { 0x7b8c9c },
            );
            self.rect(
                x + 6 + d,
                425 + d,
                2,
                2,
                if selected { CYAN } else { 0xbac8d3 },
            );
            let label = quality.kbps().to_string();
            self.text(
                x + (w - label.len() * 8) / 2 + 3 + d,
                424 + d,
                &label,
                INK,
                3,
            );
        }
        self.rect(13, 443, 534, 1, 0x6d8499);
        self.rect(13, 444, 534, 1, 0xe1edf6);
        let status = self
            .transient
            .as_ref()
            .filter(|(_, at)| at.elapsed() < Duration::from_secs(8))
            .map(|(text, _)| text.as_str())
            .unwrap_or(&s.status);
        let first: String = status.chars().take(66).collect();
        let second: String = status.chars().skip(66).take(66).collect();
        self.text(15, 450, &first, INK, 66);
        self.text(15, 460, &second, INK, 66);
        if self.about.is_none() && !self.help {
            let tooltip = match self.hovered_button {
                Some(Button::About) => Some((284, self.about_tooltip())),
                Some(Button::Shuffle) => Some((
                    225,
                    if s.shuffle {
                        "Lecture aléatoire : activée"
                    } else {
                        "Lecture aléatoire : désactivée"
                    },
                )),
                Some(Button::Repeat) => Some((
                    225,
                    if s.repeat {
                        "Répéter la file : activé"
                    } else {
                        "Répéter la file : désactivé"
                    },
                )),
                _ => None,
            };
            if let Some((left, text)) = tooltip {
                panel(
                    &mut self.pixels,
                    WIDTH,
                    (left, 140, 264, 19),
                    0xe8f5fd,
                    false,
                );
                self.text(left + 8, 146, text, INK, 31);
            }
        }
    }
}
