//! Classic bitmap skin. Geometry is shared with the input code in `ui.rs`.
//! All decoration is drawn directly into the existing software framebuffer.
use super::*;

#[derive(Clone, Copy)]
pub(super) enum TransportIcon {
    Previous,
    Play,
    Pause,
    Stop,
    Next,
}

impl View {
    pub(super) fn rect(&mut self, x: usize, y: usize, w: usize, h: usize, color: u32) {
        for yy in y.min(HEIGHT)..y.saturating_add(h).min(HEIGHT) {
            self.pixels[yy * WIDTH + x.min(WIDTH)..yy * WIDTH + x.saturating_add(w).min(WIDTH)]
                .fill(color);
        }
    }

    pub(super) fn bevel(&mut self, x: usize, y: usize, w: usize, h: usize, fill: u32, inset: bool) {
        if self.skin == crate::appearance::Skin::Modern {
            let fill = match fill {
                BG => 0x1b344c,
                BLACK => 0x071c30,
                0x353249 => 0x294c69,
                other => other,
            };
            crate::appearance::panel(&mut self.pixels, WIDTH, (x, y, w, h), fill, inset);
            return;
        }
        self.rect(x, y, w, h, fill);
        if w < 3 || h < 3 {
            return;
        }
        let (top, bottom, inner_top, inner_bottom) = if inset {
            (SHADOW, LIGHT, 0x080912, 0x48485b)
        } else {
            (LIGHT, SHADOW, 0x5a5a70, 0x1b1b2a)
        };
        self.rect(x, y, w, 1, top);
        self.rect(x, y, 1, h, top);
        self.rect(x, y + h - 1, w, 1, bottom);
        self.rect(x + w - 1, y, 1, h, bottom);
        self.rect(x + 1, y + 1, w - 2, 1, inner_top);
        self.rect(x + 1, y + 1, 1, h - 2, inner_top);
        self.rect(x + 1, y + h - 2, w - 2, 1, inner_bottom);
        self.rect(x + w - 2, y + 1, 1, h - 2, inner_bottom);
    }

    pub(super) fn text(&mut self, x: usize, y: usize, text: &str, color: u32, max: usize) {
        let color = self.skin_text_color(color);
        for (index, c) in text.chars().take(max).enumerate() {
            let glyph = font8x8::BASIC_FONTS
                .get(c)
                .or_else(|| font8x8::LATIN_FONTS.get(c))
                .or_else(|| font8x8::BASIC_FONTS.get('?'));
            if let Some(glyph) = glyph {
                for (row, bits) in glyph.iter().enumerate() {
                    for column in 0..8 {
                        if bits & (1 << column) != 0 {
                            self.rect(x + index * 8 + column, y + row, 1, 1, color);
                        }
                    }
                }
            }
        }
    }

    // Five-pixel capitals keep title strips as compact as the original skin.
    pub(super) fn tiny_text(&mut self, x: usize, y: usize, text: &str, color: u32) {
        let color = self.skin_text_color(color);
        for (index, c) in text.chars().enumerate() {
            let rows = match c {
                'A' => [14, 17, 31, 17, 17],
                'B' => [30, 17, 30, 17, 30],
                'C' => [15, 16, 16, 16, 15],
                'D' => [30, 17, 17, 17, 30],
                'E' => [31, 16, 30, 16, 31],
                'F' => [31, 16, 30, 16, 16],
                'G' => [15, 16, 23, 17, 15],
                'H' => [17, 17, 31, 17, 17],
                'I' => [7, 2, 2, 2, 7],
                'J' => [7, 1, 1, 17, 14],
                'K' => [17, 18, 28, 18, 17],
                'L' => [16, 16, 16, 16, 31],
                'M' => [17, 27, 21, 17, 17],
                'N' => [17, 25, 21, 19, 17],
                'O' => [14, 17, 17, 17, 14],
                'P' => [30, 17, 30, 16, 16],
                'Q' => [14, 17, 17, 19, 15],
                'R' => [30, 17, 30, 18, 17],
                'S' => [15, 16, 14, 1, 30],
                'T' => [31, 4, 4, 4, 4],
                'U' => [17, 17, 17, 17, 14],
                'V' => [17, 17, 17, 10, 4],
                'W' => [17, 17, 21, 27, 17],
                'X' => [17, 10, 4, 10, 17],
                'Y' => [17, 10, 4, 4, 4],
                'Z' => [31, 2, 4, 8, 31],
                '/' => [1, 2, 4, 8, 16],
                _ => [0; 5],
            };
            for (row, bits) in rows.iter().enumerate() {
                for column in 0..5 {
                    if bits & (1 << (4 - column)) != 0 {
                        self.rect(x + index * 6 + column, y + row, 1, 1, color);
                    }
                }
            }
        }
    }

    pub(super) fn metal_face(
        &mut self,
        x: usize,
        y: usize,
        w: usize,
        h: usize,
        enabled: bool,
        down: bool,
    ) {
        if self.skin == crate::appearance::Skin::Modern {
            crate::appearance::button(&mut self.pixels, WIDTH, (x, y, w, h), enabled, down);
            return;
        }
        self.rect(x, y, w, h, 0x0b0d17);
        let (high, face, low) = if enabled {
            (0xe8edf0, 0xb6bec8, 0x747d91)
        } else {
            (0x9194a3, 0x7a7e90, 0x53586d)
        };
        let (top, bottom) = if down { (low, high) } else { (high, low) };
        self.rect(x + 1, y + 1, w - 2, h - 2, face);
        self.rect(x + 1, y + 1, w - 2, 1, top);
        self.rect(x + 1, y + 1, 1, h - 2, top);
        self.rect(x + 1, y + h - 2, w - 2, 1, bottom);
        self.rect(x + w - 2, y + 1, 1, h - 2, bottom);
        if down {
            self.rect(x + 2, y + 2, w - 4, 1, 0x646b7d);
            self.rect(x + 2, y + 2, 1, h - 4, 0x646b7d);
        } else {
            self.rect(
                x + 2,
                y + 2,
                w - 4,
                2,
                if enabled { 0xd6dee4 } else { 0x8b8fa0 },
            );
            self.rect(
                x + 2,
                y + 4,
                w - 4,
                2,
                if enabled { 0xc6cfd8 } else { 0x85899b },
            );
            self.rect(
                x + 2,
                y + h - 5,
                w - 4,
                2,
                if enabled { 0x9aa5b5 } else { 0x666d83 },
            );
            self.rect(x + 2, y + h - 3, w - 4, 1, 0x5b6378);
        }
    }

    fn button(&mut self, x: usize, y: usize, w: usize, label: &str, active: bool) {
        let down = self.button_down(x, y);
        self.metal_face(x, y, w, 22, active, down);
        let offset = usize::from(down);
        let left = x + w.saturating_sub(label.chars().count() * 8) / 2 + offset;
        self.text(
            left,
            y + 7 + offset,
            label,
            if active { 0x242b40 } else { 0x51586b },
            w / 8,
        );
    }

    pub(super) fn triangle(&mut self, x: usize, y: usize, size: usize, right: bool, color: u32) {
        for column in 0..size {
            let half = if right { size - 1 - column } else { column };
            self.rect(x + column, y + size - 1 - half, 1, half * 2 + 1, color);
        }
    }

    pub(super) fn transport_glyph(
        &mut self,
        cx: usize,
        cy: usize,
        icon: TransportIcon,
        color: u32,
    ) {
        match icon {
            TransportIcon::Previous => {
                self.rect(cx - 8, cy - 5, 2, 11, color);
                self.triangle(cx - 4, cy - 5, 6, false, color);
            }
            TransportIcon::Play => self.triangle(cx - 4, cy - 5, 6, true, color),
            TransportIcon::Pause => {
                self.rect(cx - 5, cy - 5, 3, 11, color);
                self.rect(cx + 2, cy - 5, 3, 11, color);
            }
            TransportIcon::Stop => self.rect(cx - 5, cy - 5, 10, 11, color),
            TransportIcon::Next => {
                self.triangle(cx - 4, cy - 5, 6, true, color);
                self.rect(cx + 4, cy - 5, 2, 11, color);
            }
        }
    }

    fn transport_button(&mut self, x: usize, icon: TransportIcon, enabled: bool) {
        let down = self.button_down(x, 116);
        self.metal_face(x, 116, 38, 22, enabled, down);
        let offset = usize::from(down);
        let (cx, cy) = (x + 19 + offset, 127 + offset);
        self.transport_glyph(
            cx + 1,
            cy + 1,
            icon,
            if enabled { 0xe2e8ec } else { 0x969bae },
        );
        self.transport_glyph(cx, cy, icon, if enabled { 0x293247 } else { 0x555c70 });
    }

    fn mode_button(&mut self, button: Button, active: bool) {
        let (x, y, w, h) = button.rect();
        let pressed = self.button_down(x, y);
        self.metal_face(x, y, w, h, true, pressed || active);
        let d = usize::from(pressed);
        if button == Button::Shuffle {
            self.rect(x + 5 + d, y + 8 + d, 3, 6, 0x414854);
            self.rect(
                x + 5 + d,
                y + 8 + d,
                2,
                5,
                if active { 0x398b22 } else { 0x717986 },
            );
            self.rect(
                x + 5 + d,
                y + 8 + d,
                2,
                2,
                if active { GREEN } else { 0x52586e },
            );
            self.tiny_text(x + 11 + d, y + 9 + d, "SHUFFLE", 0x252c40);
        } else {
            // Two return arrows denote looping the whole queue, without a "1".
            let color = if active { GREEN } else { 0x293247 };
            let (left, top) = (x + 5 + d, y + 6 + d);
            self.rect(left + 1, top, 9, 2, color);
            self.rect(left, top + 1, 2, 3, color);
            self.rect(left + 1, top + 7, 9, 2, color);
            self.rect(left + 9, top + 5, 2, 3, color);
            self.triangle(left + 8, top - 1, 3, true, color);
            self.triangle(left, top + 5, 3, false, color);
        }
    }

    fn header(&mut self, y: usize, name: &str) {
        self.bevel(3, y, WIDTH - 6, 18, BG, false);
        self.rect(6, y + 2, WIDTH - 12, 1, 0x171725);
        self.rect(6, y + 14, WIDTH - 12, 1, 0x444255);
        let left = (WIDTH - name.len() * 6) / 2;
        let line_color = if self.focused { GOLD } else { 0x6c6870 };
        let label_color = if self.focused { WHITE } else { 0x86838e };
        for (start, length) in [(13, left - 22), (WIDTH - left + 8, left - 22)] {
            self.rect(start, y + 5, length, 1, line_color);
            self.rect(start, y + 6, length, 1, 0x5e5948);
            self.rect(start, y + 8, length, 1, line_color);
            self.rect(start, y + 9, length, 1, 0x15151e);
            self.rect(start, y + 11, length, 1, 0x77715c);
        }
        self.tiny_text(left, y + 6, name, label_color);
    }

    pub(super) fn digit(&mut self, x: usize, y: usize, digit: u8) {
        let masks = [
            0b0111111, 0b0000110, 0b1011011, 0b1001111, 0b1100110, 0b1101101, 0b1111101, 0b0000111,
            0b1111111, 0b1101111,
        ];
        let mask = masks[(digit % 10) as usize];
        for (bit, (dx, dy, w, h)) in [
            (4, 0, 17, 3),
            (21, 4, 3, 15),
            (21, 23, 3, 15),
            (4, 39, 17, 3),
            (0, 23, 3, 15),
            (0, 4, 3, 15),
            (4, 19, 17, 3),
        ]
        .iter()
        .enumerate()
        {
            self.rect(
                x + dx,
                y + dy,
                *w,
                *h,
                if mask & (1 << bit) != 0 {
                    if self.skin == crate::appearance::Skin::Modern {
                        0x8ceaff
                    } else {
                        GREEN
                    }
                } else {
                    if self.skin == crate::appearance::Skin::Modern {
                        0x163348
                    } else {
                        0x10200c
                    }
                },
            );
        }
    }

    fn grip(&mut self, x: usize, y: usize, w: usize, h: usize, down: bool) {
        self.metal_face(x, y, w, h, true, down);
        let offset = usize::from(down);
        for dx in (4..w.saturating_sub(3)).step_by(3) {
            self.rect(x + dx + offset, y + 4 + offset, 1, h - 8, 0x646c80);
            self.rect(x + dx + 1 + offset, y + 4 + offset, 1, h - 8, 0xdce1e4);
        }
    }

    pub fn render(&mut self, s: &Snapshot) {
        match self.skin {
            crate::appearance::Skin::Classic => self.render_classic(s),
            crate::appearance::Skin::Modern => self.render_modern(s),
        }
        if let Some(index) = self
            .drop_marker
            .filter(|_| self.about.is_none() && !self.help)
        {
            let y = (221 + index.saturating_sub(self.offset).min(ROWS) * 24).min(411);
            let color = if self.skin == crate::appearance::Skin::Modern {
                0x55e4ff
            } else {
                GREEN
            };
            self.rect(17, y, 518, 2, color);
            self.rect(17, y.saturating_sub(2), 3, 6, color);
            self.rect(532, y.saturating_sub(2), 3, 6, color);
        }
        self.render_skin_selector();
        if self.help {
            self.render_help();
        }
        self.render_about();
    }

    fn render_classic(&mut self, s: &Snapshot) {
        self.pixels.fill(BG);
        self.bevel(0, 0, WIDTH, HEIGHT, BG, false);
        self.rect(3, 20, 1, HEIGHT - 23, 0x535166);
        self.rect(WIDTH - 4, 20, 1, HEIGHT - 23, 0x141422);
        self.header(2, "SPOTIAMP");
        self.titlebar_buttons();

        self.bevel(14, 28, 187, 61, BLACK, true);
        self.rect(17, 31, 181, 1, 0x161d15);
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
        self.rect(87, 49, 3, 4, GREEN);
        self.rect(87, 67, 3, 4, GREEN);
        if s.playing {
            self.triangle(169, 39, 5, true, GREEN);
        } else {
            self.rect(168, 39, 3, 9, DIM);
            self.rect(174, 39, 3, 9, DIM);
        }
        self.tiny_text(164, 72, "PCM", DIM);

        self.button(
            421,
            29,
            125,
            ConnectionAction::for_state(s).label(self.language),
            true,
        );
        self.rect(214, 36, 4, 4, if s.connected { GREEN } else { 0x56515a });
        self.text(
            225,
            34,
            if s.connected {
                self.language.text("SPOTIFY / EN LIGNE", "SPOTIFY / ONLINE")
            } else {
                self.language
                    .text("SPOTIFY / HORS LIGNE", "SPOTIFY / OFFLINE")
            },
            if s.connected { GREEN } else { DIM },
            24,
        );
        self.bevel(213, 57, 333, 32, BLACK, true);
        let row = s.current.and_then(|i| s.queue.get(i));
        self.text(
            219,
            62,
            row.map(|r| r.display_title(self.language)).unwrap_or(
                self.language
                    .text("Votre musique. Tout simplement.", "Your music. Simply."),
            ),
            GREEN,
            40,
        );
        self.text(
            219,
            76,
            row.map(|r| r.artist.as_str()).unwrap_or(
                self.language
                    .text("LECTEUR SPOTIFY NATIF", "NATIVE SPOTIFY PLAYER"),
            ),
            DIM,
            40,
        );

        self.bevel(14, 97, 532, 9, 0x11121a, true);
        self.rect(17, 100, 526, 2, 0x383647);
        if s.duration_ms > 0 {
            let progress =
                ((s.position_ms.min(s.duration_ms) as u64 * 526) / s.duration_ms as u64) as usize;
            self.rect(17, 100, progress, 2, 0x6c714a);
            let knob = (15 + progress).saturating_sub(10).clamp(15, 523);
            self.metal_face(knob, 96, 22, 11, true, self.dragging == 1);
            for dx in [7, 10, 13] {
                self.rect(knob + dx, 99, 1, 5, 0x596077);
                self.rect(knob + dx + 1, 99, 1, 5, 0xdce1e4);
            }
        }
        let update_notice = match self.update {
            UpdateStatus::Available { .. } => Some(
                self.language
                    .text("Mise à jour disponible - F2", "Update available - F2"),
            ),
            UpdateStatus::Ready { .. } => Some(
                self.language
                    .text("Mise à jour prête - F2", "Update ready - F2"),
            ),
            _ => None,
        };
        if let Some(notice) = update_notice {
            self.text(236, 108, notice, GOLD, 38);
        }
        for (x, icon) in [
            (15, TransportIcon::Previous),
            (57, TransportIcon::Play),
            (99, TransportIcon::Pause),
            (141, TransportIcon::Stop),
            (183, TransportIcon::Next),
        ] {
            self.transport_button(x, icon, s.connected);
        }
        self.mode_button(Button::Shuffle, s.shuffle);
        self.mode_button(Button::Repeat, s.repeat);
        let about_down = self.button_down(308, 116);
        self.metal_face(308, 116, 22, 22, true, about_down);
        let d = usize::from(about_down);
        // Alternate just the bolt's palette; its geometry and pressed offset,
        // the metal face, and the permanent update dot remain unchanged.
        let (bolt, bolt_shadow) = if self.update_indicator_lit() {
            (GREEN, 0x285e24)
        } else {
            (0xe7c969, 0x77632f)
        };
        for (row, bits) in [
            0x018u16, 0x030, 0x060, 0x0c0, 0x1fe, 0x03c, 0x078, 0x070, 0x060, 0x040,
        ]
        .iter()
        .enumerate()
        {
            for col in 0..9 {
                if bits & (1 << (8 - col)) != 0 {
                    self.rect(315 + col + d, 122 + row + d, 1, 1, bolt_shadow);
                    self.rect(314 + col + d, 121 + row + d, 1, 1, bolt);
                }
            }
        }
        if update_notice.is_some() {
            self.rect(325 + d, 119 + d, 3, 3, 0x285e24);
            self.rect(325 + d, 119 + d, 2, 2, GREEN);
        }
        self.bevel(337, 123, 207, 8, BLACK, true);
        let volume = s.volume as usize * 201 / 65535;
        self.rect(340, 126, volume, 2, 0x76a631);
        let knob = (337 + volume).saturating_sub(5).clamp(337, 531);
        self.grip(knob, 118, 13, 18, self.dragging == 2);
        self.tiny_text(410, 136, self.language.text("VOLUME", "VOLUME"), WHITE);

        self.rect(5, 142, WIDTH - 10, 1, 0x11121d);
        self.rect(5, 143, WIDTH - 10, 1, 0x656174);
        self.header(145, self.language.text("LISTE DE LECTURE", "PLAY QUEUE"));
        let library_down = self.button_down(448, 145);
        self.metal_face(448, 145, 100, 16, true, library_down);
        let offset = usize::from(library_down);
        self.text(
            458 + offset,
            150 + offset,
            self.language.text("BIBLIO. F3", "LIBRARY F3"),
            0x242b40,
            10,
        );
        self.bevel(
            14,
            171,
            408,
            23,
            if self.input_active { 0x0b140b } else { BLACK },
            true,
        );
        let text = if self.input.is_empty() {
            self.language
                .text(
                    "Coller un lien Spotify : titre, album...",
                    "Paste a Spotify link: track, album...",
                )
                .to_owned()
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
            self.rect(19, 177, text.chars().count() * 8, 11, 0x222b67);
        }
        self.text(
            20,
            179,
            &text,
            if self.input.is_empty() { DIM } else { GREEN },
            49,
        );
        self.button(430, 171, 48, self.language.text("COL.", "PASTE"), true);
        self.button(
            484,
            171,
            62,
            self.language.text("+ AJ.", "+ ADD"),
            s.connected && !s.busy,
        );
        self.rect(15, 203, 530, 14, 0x343247);
        self.rect(15, 203, 530, 1, 0x555367);
        self.rect(15, 216, 530, 1, 0x141420);
        self.tiny_text(
            19,
            208,
            self.language.text("TITRE / ARTISTE", "TITLE / ARTIST"),
            WHITE,
        );
        self.tiny_text(502, 208, self.language.text("DUREE", "TIME"), WHITE);
        self.bevel(14, 219, 532, 195, BLACK, true);
        self.sync_queue_selection(s);
        self.offset = self.offset.min(s.queue.len().saturating_sub(ROWS));
        if self.selected.is_some_and(|i| i >= s.queue.len()) {
            self.selected = s.queue.len().checked_sub(1);
        }
        if s.queue.is_empty() {
            self.text(
                78,
                266,
                self.language.text(
                    "LE PLAISIR D'UN PETIT LECTEUR.",
                    "THE JOY OF A SMALL PLAYER.",
                ),
                GREEN,
                49,
            );
            self.text(
                57,
                299,
                self.language
                    .text("1. Cliquez sur CONNEXION.", "1. Click CONNECT."),
                WHITE,
                56,
            );
            self.text(
                57,
                320,
                self.language.text(
                    "2. Collez un lien Spotify (Ctrl+V).",
                    "2. Paste a Spotify link (Ctrl+V).",
                ),
                WHITE,
                56,
            );
            self.text(
                57,
                341,
                self.language.text(
                    "3. AJOUTER, puis double-clic pour lire.",
                    "3. ADD, then double-click to play.",
                ),
                WHITE,
                58,
            );
            self.text(
                57,
                378,
                self.language.text(
                    "Compte Premium requis. Aide : F1",
                    "Premium account required. Help: F1",
                ),
                DIM,
                55,
            );
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
                let selected = self.queue_row_selected(index);
                let current = s.current == Some(index);
                if selected {
                    self.rect(16, y, 520, 24, 0x141c54);
                    self.rect(16, y, 520, 1, 0x303971);
                }
                if current {
                    self.triangle(17, y + 8, 4, true, GREEN);
                }
                self.text(24, y + 8, &format!("{:03}", index + 1), DIM, 3);
                self.text(
                    59,
                    y + 3,
                    row.display_title(self.language),
                    if current || selected { WHITE } else { GREEN },
                    52,
                );
                self.text(59, y + 14, &row.artist, DIM, 52);
                self.text(493, y + 8, &format_time(row.duration_ms), GREEN, 6);
            }
            if s.queue.len() > ROWS {
                self.rect(538, 221, 6, 190, 0x171724);
                self.rect(538, 221, 1, 190, 0x080812);
                let h = (190 * ROWS / s.queue.len()).max(6);
                let top = self.offset * (190 - h) / (s.queue.len() - ROWS);
                self.bevel(539, 221 + top, 4, h, 0xaaa18a, false);
            }
        }

        self.button(
            14,
            417,
            98,
            self.language.text("SUPPRIMER", "REMOVE"),
            self.queue_removal_index(s).is_some(),
        );
        self.button(
            118,
            417,
            74,
            self.language.text("VIDER", "CLEAR"),
            !s.queue.is_empty(),
        );
        self.bevel(197, 419, 83, 17, BLACK, true);
        self.text(202, 424, &format!("{:03}", s.queue.len()), GREEN, 3);
        self.tiny_text(232, 426, self.language.text("TITRES", "TRACKS"), DIM);
        self.rect(285, 420, 1, 16, 0x171722);
        self.rect(286, 420, 1, 16, 0x57536b);
        self.tiny_text(300, 426, "KB/S", WHITE);
        for (x, w, quality) in [
            (350, 60, AudioQuality::Kbps96),
            (415, 60, AudioQuality::Kbps160),
            (480, 66, AudioQuality::Kbps320),
        ] {
            let selected = quality == s.quality;
            let pressed = self.button_down(x, 417);
            self.metal_face(x, 417, w, 21, true, pressed || selected);
            let offset = usize::from(pressed);
            self.rect(
                x + 6 + offset,
                425 + offset,
                3,
                5,
                if selected { 0x398b22 } else { 0x717986 },
            );
            self.rect(
                x + 6 + offset,
                425 + offset,
                2,
                2,
                if selected { GREEN } else { 0x52586e },
            );
            let label = quality.kbps().to_string();
            self.text(
                x + (w - label.len() * 8) / 2 + 3 + offset,
                424 + offset,
                &label,
                0x252c40,
                3,
            );
        }

        self.rect(13, 443, 534, 1, 0x171722);
        self.rect(13, 444, 534, 1, 0x656174);
        let status = self
            .transient
            .as_ref()
            .filter(|(_, at)| at.elapsed() < Duration::from_secs(8))
            .map(|(text, _)| text.text(self.language))
            .unwrap_or_else(|| s.status.text(self.language));
        let chars: Vec<char> = status.chars().collect();
        self.text(
            15,
            450,
            &chars.iter().take(66).collect::<String>(),
            GOLD,
            66,
        );
        self.text(
            15,
            460,
            &chars.iter().skip(66).take(66).collect::<String>(),
            GOLD,
            66,
        );
        if self.about.is_none() && !self.help {
            let tooltip = match self.hovered_button {
                Some(Button::About) => Some((284, self.about_tooltip())),
                Some(Button::Shuffle) => Some((
                    225,
                    if s.shuffle {
                        self.language
                            .text("Lecture aléatoire : activée", "Shuffle: on")
                    } else {
                        self.language
                            .text("Lecture aléatoire : désactivée", "Shuffle: off")
                    },
                )),
                Some(Button::Repeat) => Some((
                    225,
                    if s.repeat {
                        self.language
                            .text("Répéter la file : activé", "Repeat queue: on")
                    } else {
                        self.language
                            .text("Répéter la file : désactivé", "Repeat queue: off")
                    },
                )),
                _ => None,
            };
            if let Some((left, text)) = tooltip {
                self.bevel(left, 140, 264, 19, 0xefe7bc, false);
                self.text(left + 8, 146, text, SHADOW, 31);
            }
        }
    }

    pub(super) fn titlebar_buttons(&mut self) {
        for button in Button::TITLEBAR {
            let (x, y, width, height) = button.rect();
            let down = self.button_down(x, y);
            self.metal_face(x, y, width, height, true, down);
            let offset = usize::from(down);
            let color = if self.focused { 0x303346 } else { 0x62677a };
            match button {
                Button::Help => self.text(x + 4 + offset, y + 3 + offset, "?", color, 1),
                Button::Minimize => self.rect(x + 5 + offset, y + 9 + offset, 8, 2, color),
                Button::Close => {
                    for i in 0..7 {
                        self.rect(x + 6 + i + offset, y + 3 + i + offset, 1, 1, color);
                        self.rect(x + 12 - i + offset, y + 3 + i + offset, 1, 1, color);
                    }
                }
                _ => unreachable!(),
            }
        }
    }

    fn render_help(&mut self) {
        self.bevel(24, 24, 512, 416, BG, false);
        self.rect(28, 28, 504, 29, 0x171724);
        self.text(
            42,
            43,
            self.language
                .text("SPOTIAMP / MODE D'EMPLOI", "SPOTIAMP / HELP"),
            GREEN,
            58,
        );
        for (i, line) in [
            self.language.text(
                "CONNEXION ouvre votre navigateur pour Spotify.",
                "CONNECT opens your browser to sign in to Spotify.",
            ),
            self.language.text(
                "La musique est lue directement par ce lecteur.",
                "Music plays directly in this player.",
            ),
            self.language.text(
                "Le client Spotify officiel peut rester ferme.",
                "The official Spotify app can stay closed.",
            ),
            self.language.text(
                "F3 : ouvrir vos playlists Spotify",
                "F3: open your Spotify playlists",
            ),
            self.language.text(
                "Ajoutez un lien de titre, d'album ou de playlist.",
                "Add a link to a track, album or playlist.",
            ),
            self.language.text(
                "Exemple : https://open.spotify.com/track/...",
                "Example: https://open.spotify.com/track/...",
            ),
            self.language.text(
                "Double-clic : lire. Liste limitee a 500 titres.",
                "Double-click to play. Queue limited to 500 tracks.",
            ),
            self.language.text(
                "Qualite KB/S : 96 / 160 / 320 (defaut, memorise).",
                "Quality KB/S: 96 / 160 / 320 (default, remembered).",
            ),
            self.language.text(
                "Ctrl+V / Cmd+V : coller un lien",
                "Ctrl+V / Cmd+V: paste a link",
            ),
            self.language.text(
                "Entree : ajouter le lien ou lire la selection",
                "Enter: add the link or play the selected track",
            ),
            self.language.text(
                "Espace : lecture / pause (hors saisie)",
                "Space: play / pause (outside the text field)",
            ),
            self.language.text(
                "Fleches : selection / molette : defilement",
                "Arrows: select / mouse wheel: scroll",
            ),
            self.language.text(
                "Suppr : retirer un titre ; Tab : champ de lien",
                "Delete: remove a track; Tab: link field",
            ),
            self.language.text(
                "Barres : position de lecture et volume",
                "Sliders: playback position and volume",
            ),
            self.language.text(
                "F2 : À propos, langue, crédits et raccourcis",
                "F2: About, language, credits and shortcuts",
            ),
            self.language.text(
                "Qualite modifiee : au prochain chargement audio.",
                "Quality changes apply on the next audio load.",
            ),
            self.language.text(
                "Identifiants gardes dans le coffre du systeme.",
                "Credentials are kept in the system keyring.",
            ),
            self.language.text(
                "Sans coffre disponible : connexion non memorisee.",
                "Without a keyring, sign-in cannot be remembered.",
            ),
            self.language.text(
                "DECONNECTER oublie la session enregistree.",
                "DISCONNECT removes the saved session.",
            ),
            self.language.text(
                "Le moteur peut choisir un debit de repli.",
                "The engine may fall back to a lower bitrate.",
            ),
            self.language
                .text("Egaliseur non inclus.", "Equalizer is not included."),
            self.language.text(
                "F1, Echap ou clic : fermer cette aide.",
                "F1, Esc or click: close this help.",
            ),
        ]
        .iter()
        .enumerate()
        {
            self.text(42, 69 + i * 17, line, WHITE, 59);
        }
    }

    fn skin_text_color(&self, color: u32) -> u32 {
        if self.skin == crate::appearance::Skin::Classic {
            return color;
        }
        match color {
            GREEN | GOLD => 0x6ce5ff,
            WHITE => 0xeaf8ff,
            DIM => 0x91b3cb,
            other => other,
        }
    }

    pub fn save_ppm(&self, path: &str) -> std::io::Result<()> {
        let mut file = File::create(path)?;
        write!(file, "P6\n{WIDTH} {HEIGHT}\n255\n")?;
        let mut data = Vec::with_capacity(self.pixels.len() * 3);
        for p in &self.pixels {
            data.extend_from_slice(&[(p >> 16) as u8, (p >> 8) as u8, *p as u8]);
        }
        file.write_all(&data)
    }
}

pub(super) fn format_time(ms: u32) -> String {
    format!("{}:{:02}", ms / 60_000, ms / 1000 % 60)
}
