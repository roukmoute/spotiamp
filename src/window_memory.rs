//! Small in-memory layout until shutdown: no idle polling or disk writes.
use crate::{library_ui, window_layout, window_settings::SavedLayout};
use minifb::{RestorablePlacement, Window};

pub struct LayoutMemory {
    pub saved: Option<SavedLayout>,
    pub library_open: bool,
    scale: u8,
    original: Option<SavedLayout>,
}

impl LayoutMemory {
    pub fn new(original: Option<SavedLayout>, scale: u8, player: &Window) -> Self {
        Self {
            saved: original.map(|saved| repair(saved, scale, player)),
            library_open: original.is_none_or(|saved| saved.library_open),
            scale,
            original,
        }
    }

    /// F3 reopening keeps a free companion's absolute position. An attached
    /// companion follows the player's displacement while it was closed.
    pub fn reopening(&mut self, player: &Window) -> Option<SavedLayout> {
        self.capture(player.get_restorable_placement(), None);
        self.saved = self.saved.map(|saved| repair(saved, self.scale, player));
        self.saved
    }

    pub fn capture(
        &mut self,
        player: Option<RestorablePlacement>,
        library: Option<RestorablePlacement>,
    ) {
        let Some(player) = player else { return };
        let old = self.saved;
        let (position, attached) = if let Some(library) = library {
            (library.position, library.docked)
        } else if let Some(old) = old {
            let position = if old.attached {
                (
                    old.library.0 + player.position.0 - old.player.0,
                    old.library.1 + player.position.1 - old.player.1,
                )
            } else {
                old.library
            };
            (position, old.attached)
        } else {
            (
                (
                    player.position.0 + crate::ui::WIDTH as isize * self.scale as isize,
                    player.position.1,
                ),
                true,
            )
        };
        self.saved = Some(SavedLayout {
            player: player.position,
            library: position,
            attached,
            library_open: self.library_open,
            scale: self.scale,
        });
    }

    pub fn save(&self) -> Result<(), &'static str> {
        if let Some(mut saved) = self.saved {
            saved.library_open = self.library_open;
            if Some(saved) != self.original {
                crate::window_settings::save(saved)?;
            }
        }
        Ok(())
    }
}

fn repair(saved: SavedLayout, scale: u8, player: &Window) -> SavedLayout {
    window_layout::restore(
        saved,
        player.get_size(),
        (
            library_ui::WIDTH * scale as usize,
            library_ui::HEIGHT * scale as usize,
        ),
        scale,
        |(x, y)| player.get_work_area_at(x, y),
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn closing_library_and_moving_player_preserves_free_or_attached_placement() {
        for attached in [false, true] {
            let original = SavedLayout {
                player: (-560, 200),
                library: (0, 240),
                attached,
                library_open: true,
                scale: 1,
            };
            let mut memory = LayoutMemory {
                saved: Some(original),
                original: Some(original),
                scale: 1,
                library_open: false,
            };
            memory.capture(
                Some(RestorablePlacement {
                    position: (-500, 250),
                    docked: false,
                }),
                None,
            );
            let saved = memory.saved.unwrap();
            assert_eq!(saved.library, if attached { (60, 290) } else { (0, 240) });
            assert!(!saved.library_open);
            assert_eq!(saved.attached, attached);
            // An explicit library snapshot supersedes remembered geometry.
            memory.capture(
                Some(RestorablePlacement {
                    position: (-500, 250),
                    docked: true,
                }),
                Some(RestorablePlacement {
                    position: (400, 600),
                    docked: false,
                }),
            );
            assert_eq!(memory.saved.unwrap().library, (400, 600));
            assert!(!memory.saved.unwrap().attached);
        }
    }
}
