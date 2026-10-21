use crate::model::{Command, Snapshot};
use font8x8::UnicodeFonts;
use std::{
    fs::File,
    hash::{Hash, Hasher},
    io::Write,
    time::{Duration, Instant},
};

pub const WIDTH: usize = 560;
pub const HEIGHT: usize = 470;
const BG: u32 = 0x29293d;
const LIGHT: u32 = 0x8c8caa;
const SHADOW: u32 = 0x11111c;
const GREEN: u32 = 0x84ef38;
const DIM: u32 = 0x768675;
const WHITE: u32 = 0xd6d8df;
const BLACK: u32 = 0x080c09;
const GOLD: u32 = 0xc7bd83;
const ROWS: usize = 8;

pub enum Action {
    Send(Command),
    Paste,
    Submit,
}

pub struct View {
    pub pixels: Vec<u32>,
    pub input: String,
    pub input_active: bool,
    pub select_all: bool,
    pub selected: Option<usize>,
    pub help: bool,
    offset: usize,
    dragging: u8,
    last_click: Option<(usize, Instant)>,
    transient: Option<(String, Instant)>,
}

impl Default for View {
    fn default() -> Self {
        Self {
            pixels: vec![BG; WIDTH * HEIGHT],
            input: String::new(),
            input_active: false,
            select_all: false,
            selected: None,
            help: false,
            offset: 0,
            dragging: 0,
            last_click: None,
            transient: None,
        }
    }
}

impl View {
    pub fn notice(&mut self, text: &str) {
        self.transient = Some((text.into(), Instant::now()));
    }
    pub fn insert_char(&mut self, c: char) {
        if self.select_all {
            self.input.clear();
            self.select_all = false;
        }
        if self.input.len() < 2048 {
            self.input.push(c);
        }
    }
    pub fn scroll(&mut self, y: f32, len: usize) {
        let step = (y.abs().ceil() as usize).min(10);
        if y > 0. {
            self.offset = self.offset.saturating_sub(step);
        } else if y < 0. {
            self.offset = (self.offset + step).min(len.saturating_sub(ROWS));
        }
    }
    pub fn move_selection(&mut self, delta: i32, len: usize) {
        if len == 0 {
            self.selected = None;
            return;
        }
        let next = self
            .selected
            .map_or(0, |i| (i as i32 + delta).clamp(0, len as i32 - 1) as usize);
        self.selected = Some(next);
        if next < self.offset {
            self.offset = next;
        }
        if next >= self.offset + ROWS {
            self.offset = next + 1 - ROWS;
        }
    }
    pub fn click(&mut self, x: usize, y: usize, s: &Snapshot) -> Option<Action> {
        if self.help {
            self.help = false;
            return None;
        }
        self.input_active = (170..195).contains(&y) && (14..422).contains(&x);
        self.select_all = false;
        if (1..19).contains(&y) && x >= 530 {
            self.help = true;
            return None;
        }
        if (30..51).contains(&y) && (421..546).contains(&x) {
            return Some(Action::Send(if s.connected || s.busy {
                Command::Logout
            } else {
                Command::Connect
            }));
        }
        if (96..106).contains(&y) && (15..545).contains(&x) && s.duration_ms > 0 {
            self.dragging = 1;
            return self.drag(x, y, s);
        }
        if (116..139).contains(&y) {
            let c = match x {
                15..=52 => Some(Command::Previous),
                57..=94 => Some(Command::PlayIndex(self.selected.or(s.current).unwrap_or(0))),
                99..=136 => Some(Command::Toggle),
                141..=178 => Some(Command::Stop),
                183..=220 => Some(Command::Next),
                337..=543 => {
                    self.dragging = 2;
                    return self.drag(x, y, s);
                }
                _ => None,
            };
            return c.map(Action::Send);
        }
        if (170..195).contains(&y) {
            if (430..=477).contains(&x) {
                return Some(Action::Paste);
            }
            if (484..=545).contains(&x) {
                return Some(Action::Submit);
            }
        }
        if (221..413).contains(&y) && (14..535).contains(&x) {
            let i = self.offset + (y - 221) / 24;
            if i < s.queue.len() {
                self.selected = Some(i);
                let double = self
                    .last_click
                    .is_some_and(|(old, at)| old == i && at.elapsed() < Duration::from_millis(400));
                self.last_click = Some((i, Instant::now()));
                if double {
                    return Some(Action::Send(Command::PlayIndex(i)));
                }
            }
        }
        if (417..438).contains(&y) {
            if (14..=111).contains(&x) {
                return self.selected.map(|i| Action::Send(Command::Remove(i)));
            }
            if (118..=191).contains(&x) {
                return Some(Action::Send(Command::Clear));
            }
        }
        None
    }
    pub fn drag(&self, x: usize, _y: usize, s: &Snapshot) -> Option<Action> {
        match self.dragging {
            1 => Some(Action::Send(Command::Seek(
                ((x.clamp(15, 545) - 15) as u64 * s.duration_ms as u64 / 530) as u32,
            ))),
            2 => Some(Action::Send(Command::Volume(
                ((x.clamp(337, 543) - 337) * 65535 / 206) as u16,
            ))),
            _ => None,
        }
    }
    pub fn end_drag(&mut self) {
        self.dragging = 0;
    }
    pub fn fingerprint(&self, s: &Snapshot) -> u64 {
        let mut hash = std::hash::DefaultHasher::new();
        s.hash(&mut hash);
        self.input.hash(&mut hash);
        (
            self.input_active,
            self.select_all,
            self.selected,
            self.help,
            self.offset,
        )
            .hash(&mut hash);
        self.transient
            .as_ref()
            .filter(|(_, t)| t.elapsed() < Duration::from_secs(8))
            .map(|(text, _)| text)
            .hash(&mut hash);
        hash.finish()
    }
    fn rect(&mut self, x: usize, y: usize, w: usize, h: usize, color: u32) {
        for yy in y.min(HEIGHT)..y.saturating_add(h).min(HEIGHT) {
            self.pixels[yy * WIDTH + x.min(WIDTH)..yy * WIDTH + x.saturating_add(w).min(WIDTH)]
                .fill(color);
        }
    }
    fn bevel(&mut self, x: usize, y: usize, w: usize, h: usize, fill: u32, inset: bool) {
        self.rect(x, y, w, h, fill);
        let (a, b) = if inset {
            (SHADOW, LIGHT)
        } else {
            (LIGHT, SHADOW)
        };
        self.rect(x, y, w, 1, a);
        self.rect(x, y, 1, h, a);
        self.rect(x, y + h - 1, w, 1, b);
        self.rect(x + w - 1, y, 1, h, b);
    }
    fn text(&mut self, x: usize, y: usize, text: &str, color: u32, max: usize) {
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
    fn button(&mut self, x: usize, y: usize, w: usize, label: &str, active: bool) {
        self.bevel(x, y, w, 22, if active { 0x45465f } else { 0x363648 }, false);
        self.text(
            x + (w.saturating_sub(label.chars().count() * 8)) / 2,
            y + 7,
            label,
            if active { WHITE } else { DIM },
            w / 8,
        );
    }
    fn header(&mut self, y: usize, name: &str) {
        self.bevel(3, y, WIDTH - 6, 18, BG, false);
        self.rect(13, y + 6, 126, 1, GOLD);
        self.rect(13, y + 9, 126, 1, 0x736c52);
        self.rect(420, y + 6, 126, 1, GOLD);
        self.rect(420, y + 9, 126, 1, 0x736c52);
        self.text((WIDTH - name.len() * 8) / 2, y + 5, name, WHITE, 34);
    }
    fn digit(&mut self, x: usize, y: usize, digit: u8) {
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
                    GREEN
                } else {
                    0x142710
                },
            );
        }
    }
    pub fn render(&mut self, s: &Snapshot) {
        self.pixels.fill(BG);
        self.bevel(0, 0, WIDTH, HEIGHT, BG, false);
        self.header(2, "S P O T I A M P");
        self.text(534, 7, "?", GOLD, 1);
        self.bevel(14, 28, 187, 61, BLACK, true);
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
        self.text(165, 39, if s.playing { ">" } else { "=" }, GREEN, 1);
        self.text(160, 72, "PCM", DIM, 3);
        self.button(
            421,
            29,
            125,
            if s.busy {
                "ANNULER"
            } else if s.connected {
                "DECONNECTER"
            } else {
                "CONNEXION"
            },
            true,
        );
        self.text(
            214,
            36,
            if s.connected {
                "SPOTIFY / EN LIGNE"
            } else {
                "SPOTIFY / HORS LIGNE"
            },
            if s.connected { GREEN } else { DIM },
            25,
        );
        self.bevel(213, 57, 333, 32, BLACK, true);
        let row = s.current.and_then(|i| s.queue.get(i));
        self.text(
            219,
            62,
            row.map(|r| r.title.as_str())
                .unwrap_or("Votre musique. Tout simplement."),
            GREEN,
            40,
        );
        self.text(
            219,
            76,
            row.map(|r| r.artist.as_str())
                .unwrap_or("LECTEUR CLASSIQUE / EXPERIMENTAL"),
            DIM,
            40,
        );
        self.bevel(14, 97, 532, 9, BLACK, true);
        if s.duration_ms > 0 {
            let progress =
                ((s.position_ms.min(s.duration_ms) as u64 * 526) / s.duration_ms as u64) as usize;
            self.rect(17, 100, progress, 3, GREEN);
            self.rect(15 + progress, 97, 4, 9, GOLD);
        }
        for (x, label) in [(15, "|<"), (57, ">"), (99, "||"), (141, "[]"), (183, ">|")] {
            self.button(x, 116, 38, label, s.connected);
        }
        self.text(240, 123, "VOLUME", WHITE, 8);
        self.bevel(337, 123, 207, 8, BLACK, true);
        let volume = s.volume as usize * 201 / 65535;
        self.rect(340, 126, volume, 2, GREEN);
        self.bevel(337 + volume, 118, 6, 18, GOLD, false);
        self.header(145, "LISTE DE LECTURE");
        self.bevel(
            14,
            171,
            408,
            23,
            if self.input_active { 0x111a13 } else { BLACK },
            true,
        );
        let text = if self.input.is_empty() {
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
            self.rect(19, 177, text.chars().count() * 8, 11, 0x343f72);
        }
        self.text(
            20,
            179,
            &text,
            if self.input.is_empty() { DIM } else { GREEN },
            49,
        );
        self.button(430, 171, 48, "COL.", true);
        self.button(484, 171, 62, "+ AJ.", s.connected && !s.busy);
        self.text(17, 205, "#   TITRE / ARTISTE", WHITE, 40);
        self.text(492, 205, "DUREE", WHITE, 6);
        self.bevel(14, 219, 532, 195, BLACK, true);
        self.offset = self.offset.min(s.queue.len().saturating_sub(ROWS));
        if self.selected.is_some_and(|i| i >= s.queue.len()) {
            self.selected = s.queue.len().checked_sub(1);
        }
        if s.queue.is_empty() {
            self.text(78, 266, "LE PLAISIR D'UN PETIT LECTEUR.", GREEN, 49);
            self.text(57, 299, "1. Cliquez sur CONNEXION.", WHITE, 56);
            self.text(57, 320, "2. Collez un lien Spotify (Ctrl+V).", WHITE, 56);
            self.text(
                57,
                341,
                "3. AJOUTER, puis double-clic pour lire.",
                WHITE,
                58,
            );
            self.text(57, 378, "Compte Premium requis. Aide : F1", DIM, 55);
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
                if self.selected == Some(index) {
                    self.rect(16, y, 520, 24, 0x282c54);
                }
                if s.current == Some(index) {
                    self.rect(17, y + 7, 3, 8, GREEN);
                }
                self.text(24, y + 8, &format!("{:03}", index + 1), DIM, 3);
                self.text(
                    59,
                    y + 3,
                    &row.title,
                    if s.current == Some(index) {
                        GREEN
                    } else {
                        WHITE
                    },
                    52,
                );
                self.text(59, y + 14, &row.artist, DIM, 52);
                self.text(493, y + 8, &format_time(row.duration_ms), GREEN, 6);
            }
            if s.queue.len() > ROWS {
                self.rect(539, 221, 4, 190, 0x343445);
                let h = (190 * ROWS / s.queue.len()).max(6);
                let top = self.offset * (190 - h) / (s.queue.len() - ROWS);
                self.rect(539, 221 + top, 4, h, GOLD);
            }
        }
        self.button(14, 417, 98, "SUPPRIMER", self.selected.is_some());
        self.button(118, 417, 74, "VIDER", !s.queue.is_empty());
        self.text(
            208,
            424,
            &format!("{:03} TITRES / DOUBLE-CLIC : LIRE", s.queue.len()),
            GREEN,
            43,
        );
        self.rect(13, 444, 534, 1, 0x53536e);
        let notice = self
            .transient
            .as_ref()
            .filter(|(_, at)| at.elapsed() < Duration::from_secs(8))
            .map(|(t, _)| t.clone());
        let status = notice.unwrap_or_else(|| s.status.clone());
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
        if self.help {
            self.render_help();
        }
    }
    fn render_help(&mut self) {
        self.bevel(24, 24, 512, 416, BG, false);
        self.text(42, 43, "SPOTIAMP / MODE D'EMPLOI", GREEN, 58);
        for (i, line) in [
            "CONNEXION ouvre votre navigateur pour Spotify.",
            "La musique est lue directement par ce lecteur.",
            "Le client Spotify officiel peut rester ferme.",
            "",
            "Ajoutez un lien de titre, d'album ou de playlist.",
            "Exemple : https://open.spotify.com/track/...",
            "La liste est limitee a 500 titres en memoire.",
            "",
            "Ctrl+V / Cmd+V : coller un lien",
            "Entree : ajouter le lien ou lire la selection",
            "Espace : lecture / pause (hors saisie)",
            "Fleches : selection / molette : defilement",
            "Suppr : retirer un titre ; Tab : champ de lien",
            "Barres : position de lecture et volume",
            "",
            "Identifiants gardes dans le coffre du systeme.",
            "Sans coffre disponible : connexion non memorisee.",
            "DECONNECTER oublie la session enregistree.",
            "",
            "Prototype : recherche et egaliseur non inclus.",
            "F1, Echap ou clic : fermer cette aide.",
        ]
        .iter()
        .enumerate()
        {
            self.text(42, 69 + i * 17, line, WHITE, 59);
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

fn format_time(ms: u32) -> String {
    format!("{}:{:02}", ms / 60_000, ms / 1000 % 60)
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn queue_selection_stays_valid_when_scrolling_and_removing() {
        let mut view = View::default();
        view.move_selection(-1, 0);
        assert_eq!(view.selected, None);
        view.move_selection(1, 500);
        assert_eq!(view.selected, Some(0));
        view.move_selection(999, 500);
        assert_eq!(view.selected, Some(499));
        assert_eq!(view.offset, 492);
        view.render(&Snapshot::default());
        assert_eq!(view.selected, None);
        assert_eq!(view.offset, 0);
    }
    #[test]
    fn seeking_and_volume_clamp_to_valid_ranges() {
        let mut view = View::default();
        let state = Snapshot {
            duration_ms: 200_000,
            ..Snapshot::default()
        };
        view.dragging = 1;
        assert!(matches!(
            view.drag(999, 0, &state),
            Some(Action::Send(Command::Seek(200_000)))
        ));
        view.dragging = 2;
        assert!(matches!(
            view.drag(999, 0, &state),
            Some(Action::Send(Command::Volume(65535)))
        ));
    }
}
