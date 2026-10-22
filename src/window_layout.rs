//! Choose an adjacent placement without assuming the primary monitor starts at zero.

#[derive(Debug, PartialEq, Eq)]
pub struct Placement {
    pub player: (isize, isize),
    pub offset: (isize, isize),
}

pub fn place(
    player: (isize, isize),
    player_size: (usize, usize),
    library_size: (usize, usize),
    work_area: Option<(isize, isize, usize, usize)>,
) -> Placement {
    let horizontal = (
        player_size.0 + library_size.0,
        player_size.1.max(library_size.1),
    );
    let Some((left, top, width, height)) = work_area.filter(|area| area.2 > 0 && area.3 > 0) else {
        return Placement {
            player,
            offset: (player_size.0 as isize, 0),
        };
    };
    // On a narrow monitor stack below the player, keeping both title bars
    // reachable. An explicitly requested 2x UI may exceed the available space.
    let (size, offset) = if horizontal.0 > width && player_size.0.max(library_size.0) <= width {
        (
            (
                player_size.0.max(library_size.0),
                player_size.1 + library_size.1,
            ),
            (0, player_size.1 as isize),
        )
    } else {
        (horizontal, (player_size.0 as isize, 0))
    };
    Placement {
        player: (
            player
                .0
                .clamp(left, left + width.saturating_sub(size.0) as isize),
            player
                .1
                .clamp(top, top + height.saturating_sub(size.1) as isize),
        ),
        offset,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn right_dock_shifts_only_as_far_as_needed_to_fit_the_monitor() {
        assert_eq!(
            place((680, 305), (560, 470), (760, 470), Some((0, 0, 1920, 1040))),
            Placement {
                player: (600, 305),
                offset: (560, 0)
            }
        );
        assert_eq!(
            place((40, 80), (560, 470), (760, 470), Some((0, 0, 1920, 1040))).player,
            (40, 80)
        );
    }

    #[test]
    fn negative_monitor_origin_and_taskbar_are_preserved() {
        assert_eq!(
            place(
                (-1000, -200),
                (560, 470),
                (760, 470),
                Some((-1920, 30, 1920, 1010))
            ),
            Placement {
                player: (-1320, 30),
                offset: (560, 0)
            }
        );
    }

    #[test]
    fn narrow_monitor_stacks_the_library_flush_below_the_player() {
        assert_eq!(
            place((232, 100), (560, 470), (760, 470), Some((0, 0, 1024, 1000))),
            Placement {
                player: (232, 60),
                offset: (0, 470)
            }
        );
    }

    #[test]
    fn native_sizes_already_include_scale() {
        assert_eq!(
            place(
                (1600, 400),
                (1120, 940),
                (1520, 940),
                Some((0, 0, 3840, 2120))
            ),
            Placement {
                player: (1200, 400),
                offset: (1120, 0)
            }
        );
    }

    #[test]
    fn oversized_group_keeps_its_origin_in_the_work_area() {
        assert_eq!(
            place(
                (200, 200),
                (1120, 940),
                (1520, 940),
                Some((0, 30, 1920, 1010))
            ),
            Placement {
                player: (200, 30),
                offset: (0, 940)
            }
        );
    }

    #[test]
    fn unavailable_work_area_keeps_the_player_position() {
        assert_eq!(
            place((-200, 90), (560, 470), (760, 470), None),
            Placement {
                player: (-200, 90),
                offset: (560, 0)
            }
        );
    }
}
