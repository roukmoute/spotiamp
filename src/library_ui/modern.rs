//! Modern chrome shares the library's content, geometry and framebuffer.
use super::{BG, BLACK, Canvas, DIM, GOLD, GREEN, HEIGHT, Skin, WHITE, WIDTH};

pub(super) struct Palette {
    pub background: u32,
    pub well: u32,
    pub primary: u32,
    pub bright: u32,
    pub muted: u32,
    pub accent: u32,
    pub chrome_text: u32,
    pub chrome_muted: u32,
    pub header: u32,
    pub header_alt: u32,
    pub selection: u32,
    pub title_line: u32,
    pub title_text: u32,
    pub separator_shadow: u32,
    pub separator_highlight: u32,
}

impl Palette {
    pub fn new(skin: Skin, focused: bool) -> Self {
        match skin {
            Skin::Classic => Self {
                background: BG,
                well: BLACK,
                primary: GREEN,
                bright: WHITE,
                muted: DIM,
                accent: GOLD,
                chrome_text: GOLD,
                chrome_muted: DIM,
                header: 0x454360,
                header_alt: 0x343344,
                selection: 0x252d52,
                title_line: if focused { GOLD } else { 0x6c6870 },
                title_text: if focused { WHITE } else { 0x86838e },
                separator_shadow: 0x11111b,
                separator_highlight: 0x656174,
            },
            Skin::Modern => Self {
                background: 0xc3ceda,
                well: 0x071c30,
                primary: 0xeaf8ff,
                bright: 0xffffff,
                muted: 0x91b3cb,
                accent: 0x6ce5ff,
                chrome_text: 0x1d4365,
                chrome_muted: 0x40566b,
                header: 0x25455f,
                header_alt: 0x16364f,
                selection: 0x225372,
                title_line: if focused { 0x1d4365 } else { 0x6a7b8c },
                title_text: if focused { 0x1d4365 } else { 0x6a7b8c },
                separator_shadow: 0x7f95ab,
                separator_highlight: 0xf4faff,
            },
        }
    }
}

impl Canvas<'_> {
    pub(super) fn modern_chrome(&mut self, title_line: u32) {
        crate::appearance::panel(self.pixels, WIDTH, (0, 0, WIDTH, HEIGHT), 0xc3ceda, false);
        // Fill inside the border after panel(), preserving the silver gradient.
        crate::appearance::gradient(
            self.pixels,
            WIDTH,
            (2, 2, WIDTH - 4, HEIGHT - 4),
            0xe2e8ef,
            0xa4b2c2,
        );
        crate::appearance::gradient(self.pixels, WIDTH, (5, 3, 691, 17), 0xf9fcff, 0xafbfce);
        self.rect((6, 21, WIDTH - 12, 1), 0x7f95ab);
        self.rect((6, 22, WIDTH - 12, 1), 0xf4faff);
        // Compact rails keep the original hit geometry while giving the
        // Modern header its smooth metallic finish and clear focus state.
        self.rect((16, 10, 200, 1), title_line);
        self.rect((490, 10, 196, 1), title_line);
        self.rect((16, 11, 200, 1), 0xf4faff);
        self.rect((490, 11, 196, 1), 0xf4faff);
    }
}
