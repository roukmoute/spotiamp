//! Place and restore windows without assuming the primary monitor starts at zero.

use crate::window_settings::SavedLayout;

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

type Point = (i64, i64);
type Size = (i64, i64);

#[derive(Clone, Copy, PartialEq, Eq)]
struct Area {
    origin: Point,
    size: Size,
}

#[derive(Clone, Copy)]
struct Movement {
    x: (i64, i64),
    y: (i64, i64),
}

impl Movement {
    fn intersect(self, other: Self) -> Option<Self> {
        let x = (self.x.0.max(other.x.0), self.x.1.min(other.x.1));
        let y = (self.y.0.max(other.y.0), self.y.1.min(other.y.1));
        (x.0 <= x.1 && y.0 <= y.1).then_some(Self { x, y })
    }

    fn nearest(self) -> Point {
        (
            0_i64.clamp(self.x.0, self.x.1),
            0_i64.clamp(self.y.0, self.y.1),
        )
    }
}

fn native_point(point: (isize, isize)) -> Point {
    (
        (point.0 as i64).clamp(i32::MIN as i64, i32::MAX as i64),
        (point.1 as i64).clamp(i32::MIN as i64, i32::MAX as i64),
    )
}

fn native_size(size: (usize, usize)) -> Size {
    (
        size.0.clamp(1, i32::MAX as usize) as i64,
        size.1.clamp(1, i32::MAX as usize) as i64,
    )
}

fn output_point(point: Point) -> (isize, isize) {
    (
        point.0.clamp(i32::MIN as i64, i32::MAX as i64) as isize,
        point.1.clamp(i32::MIN as i64, i32::MAX as i64) as isize,
    )
}

fn add(a: Point, b: Point) -> Point {
    (a.0 + b.0, a.1 + b.1)
}

// Each bound describes translations that leave a useful piece of the title
// bar visible. Tiny work areas use their available width/height. During repair
// we prefer fitting each whole axis when possible, never changing an already
// reachable saved position merely because its body extends beyond a monitor.
fn movement(position: Point, size: Size, area: Area, scale: i64, fit: bool) -> Movement {
    let useful = (96 * scale).min(size.0).min(area.size.0);
    let title = (18 * scale).min(size.1).min(area.size.1);
    let x = if fit && size.0 <= area.size.0 {
        (
            area.origin.0 - position.0,
            area.origin.0 + area.size.0 - size.0 - position.0,
        )
    } else {
        (
            area.origin.0 + useful - size.0 - position.0,
            area.origin.0 + area.size.0 - useful - position.0,
        )
    };
    let y = (
        area.origin.1 - position.1,
        area.origin.1 + area.size.1
            - if fit && size.1 <= area.size.1 {
                size.1
            } else {
                title
            }
            - position.1,
    );
    Movement {
        x: (
            x.0.max(i32::MIN as i64 - position.0),
            x.1.min(i32::MAX as i64 - position.0),
        ),
        y: (
            y.0.max(i32::MIN as i64 - position.1),
            y.1.min(i32::MAX as i64 - position.1),
        ),
    }
}

fn reachable(position: Point, size: Size, areas: &[Area], scale: i64) -> bool {
    areas.iter().any(|&area| {
        let limits = movement(position, size, area, scale, false);
        limits.x.0 <= 0 && limits.x.1 >= 0 && limits.y.0 <= 0 && limits.y.1 >= 0
    })
}

fn nearest_translation(
    player: Point,
    library: Point,
    sizes: (Size, Size),
    areas: &[Area],
    scale: i64,
) -> Option<Point> {
    if reachable(player, sizes.0, areas, scale) && reachable(library, sizes.1, areas, scale) {
        return Some((0, 0));
    }
    for fit in [true, false] {
        let mut best: Option<Point> = None;
        for &player_area in areas {
            for &library_area in areas {
                if let Some(limits) = movement(player, sizes.0, player_area, scale, fit)
                    .intersect(movement(library, sizes.1, library_area, scale, fit))
                {
                    let delta = limits.nearest();
                    if best.is_none_or(|old| distance(delta) < distance(old)) {
                        best = Some(delta);
                    }
                }
            }
        }
        if best.is_some() {
            return best;
        }
    }
    None
}

fn distance(point: Point) -> i64 {
    point.0.abs() + point.1.abs()
}

fn native_translation(player: Point, library: Point) -> Point {
    let limits = |position: Point| Movement {
        x: (i32::MIN as i64 - position.0, i32::MAX as i64 - position.0),
        y: (i32::MIN as i64 - position.1, i32::MAX as i64 - position.1),
    };
    limits(player)
        .intersect(limits(library))
        .map_or((0, 0), Movement::nearest)
}

fn restore_one(position: Point, size: Size, areas: &[Area], scale: i64) -> Point {
    if reachable(position, size, areas, scale) {
        return position;
    }
    areas
        .iter()
        .filter_map(|&area| {
            let limits = movement(position, size, area, scale, true);
            (limits.x.0 <= limits.x.1 && limits.y.0 <= limits.y.1).then(|| limits.nearest())
        })
        .min_by_key(|&delta| distance(delta))
        .map_or(position, |delta| add(position, delta))
}

fn adjusted_offset(
    offset: Point,
    sizes: (Size, Size),
    old_scale: i64,
    scale: i64,
) -> Option<Point> {
    let old_player = (sizes.0.0 * old_scale / scale, sizes.0.1 * old_scale / scale);
    let old_library = (sizes.1.0 * old_scale / scale, sizes.1.1 * old_scale / scale);
    let scaled = (offset.0 * scale / old_scale, offset.1 * scale / old_scale);
    if offset.1 < old_player.1 && offset.1 + old_library.1 > 0 {
        if offset.0 == old_player.0 {
            return Some((sizes.0.0, scaled.1));
        }
        if offset.0 == -old_library.0 {
            return Some((-sizes.1.0, scaled.1));
        }
    }
    if offset.0 < old_player.0 && offset.0 + old_library.0 > 0 {
        if offset.1 == old_player.1 {
            return Some((scaled.0, sizes.0.1));
        }
        if offset.1 == -old_library.1 {
            return Some((scaled.0, -sizes.1.1));
        }
    }
    None
}

/// Restore exact reachable coordinates, or repair them using current work
/// areas. `area_at` returns the area containing a point or its nearest monitor.
/// The probes lie in the title bars, including near both ends, so a window
/// spanning multiple monitors can retain its original placement.
pub fn restore(
    saved: SavedLayout,
    player_size: (usize, usize),
    library_size: (usize, usize),
    scale: u8,
    mut area_at: impl FnMut((isize, isize)) -> Option<(isize, isize, usize, usize)>,
) -> SavedLayout {
    let scale = scale.clamp(1, 2);
    let sizes = (native_size(player_size), native_size(library_size));
    let player = native_point(saved.player);
    let original_library = native_point(saved.library);
    let offset = saved
        .attached
        .then(|| {
            adjusted_offset(
                (original_library.0 - player.0, original_library.1 - player.1),
                sizes,
                saved.scale.clamp(1, 2) as i64,
                scale as i64,
            )
        })
        .flatten();
    let library = offset.map_or(original_library, |offset| add(player, offset));
    // At most three areas per title bar. This restoration runs only when
    // opening windows; it needs no monitor polling or heap allocation.
    let empty = Area {
        origin: (0, 0),
        size: (1, 1),
    };
    let mut areas = [empty; 6];
    let mut count = 0;
    for (position, size) in [(player, sizes.0), (library, sizes.1)] {
        let inset = (48 * scale as i64).min((size.0 - 1) / 2);
        for x in [
            position.0 + inset,
            position.0 + size.0 / 2,
            position.0 + size.0 - 1 - inset,
        ] {
            let point = output_point((x, position.1 + (9 * scale as i64).min(size.1 / 2)));
            if let Some((x, y, width, height)) = area_at(point).filter(|a| a.2 > 0 && a.3 > 0) {
                let area = Area {
                    origin: native_point((x, y)),
                    size: native_size((width, height)),
                };
                if !areas[..count].contains(&area) {
                    areas[count] = area;
                    count += 1;
                }
            }
        }
    }
    let areas = &areas[..count];
    let mut result = SavedLayout {
        player: output_point(player),
        library: output_point(library),
        scale,
        ..saved
    };
    if !saved.attached {
        result.player = output_point(restore_one(player, sizes.0, areas, scale as i64));
        result.library = output_point(restore_one(library, sizes.1, areas, scale as i64));
        return result;
    }
    if offset.is_some() {
        if areas.is_empty() {
            let delta = native_translation(player, library);
            result.player = output_point(add(player, delta));
            result.library = output_point(add(library, delta));
            return result;
        }
        if let Some(delta) = nearest_translation(player, library, sizes, areas, scale as i64) {
            result.player = output_point(add(player, delta));
            result.library = output_point(add(library, delta));
            return result;
        }
    }
    // An invalid old attachment or an unavailable orientation gets a genuine
    // adjacent placement, never an arbitrary gap with an "attached" flag.
    let mut best: Option<(Point, Point)> = None;
    for offset in [
        (sizes.0.0, 0),
        (0, sizes.0.1),
        (-sizes.1.0, 0),
        (0, -sizes.1.1),
    ] {
        let library = add(player, offset);
        let delta = if areas.is_empty() {
            Some(native_translation(player, library))
        } else {
            nearest_translation(player, library, sizes, areas, scale as i64)
        };
        if let Some(delta) = delta {
            if best.is_none_or(|(old_delta, _)| distance(delta) < distance(old_delta)) {
                best = Some((delta, offset));
            }
        }
    }
    if let Some((delta, offset)) = best {
        result.player = output_point(add(player, delta));
        result.library = output_point(add(add(player, delta), offset));
    } else {
        // On a physically too-small screen no attached orientation can expose
        // both title bars. Keep both windows individually recoverable instead.
        result.attached = false;
        result.player = output_point(restore_one(player, sizes.0, areas, scale as i64));
        result.library = output_point(restore_one(library, sizes.1, areas, scale as i64));
    }
    result
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

#[cfg(test)]
mod restore_tests {
    use super::*;

    type WorkArea = (isize, isize, usize, usize);
    const PLAYER: (usize, usize) = (560, 470);
    const LIBRARY: (usize, usize) = (760, 470);

    fn saved(player: (isize, isize), library: (isize, isize), attached: bool) -> SavedLayout {
        SavedLayout {
            player,
            library,
            attached,
            library_open: true,
            scale: 1,
        }
    }

    fn monitors(areas: &[WorkArea]) -> impl FnMut((isize, isize)) -> Option<WorkArea> + '_ {
        |point| {
            areas.iter().copied().min_by_key(|&(x, y, width, height)| {
                let x = x as i128;
                let y = y as i128;
                let px = point.0 as i128;
                let py = point.1 as i128;
                (px - px.clamp(x, x + width as i128 - 1)).abs()
                    + (py - py.clamp(y, y + height as i128 - 1)).abs()
            })
        }
    }

    fn assert_title_accessible(
        position: (isize, isize),
        size: (usize, usize),
        scale: u8,
        areas: &[WorkArea],
    ) {
        assert!(
            areas.iter().any(|&(x, y, width, height)| {
                let horizontal = ((position.0 as i64 + size.0.min(i32::MAX as usize) as i64)
                    .min(x as i64 + width as i64)
                    - (position.0 as i64).max(x as i64))
                .max(0);
                horizontal >= (96 * i64::from(scale)).min(size.0 as i64).min(width as i64)
                    && position.1 >= y
                    && position.1 as i64 + (18 * i64::from(scale)).min(height as i64)
                        <= y as i64 + height as i64
            }),
            "unreachable title: {position:?} in {areas:?}"
        );
    }

    #[test]
    fn reachable_attachments_on_all_sides_preserve_exact_positions_and_partial_alignment() {
        let areas = [(0, 0, 3600, 2400)];
        for library in [(1760, 1030), (440, 1030), (1320, 1370), (1320, 430)] {
            let old = saved((1200, 900), library, true);
            assert_eq!(restore(old, PLAYER, LIBRARY, 1, monitors(&areas)), old);
        }
        // Full containment is deliberately not required for a reachable title.
        let old = saved((-464, 100), (96, 100), true);
        let areas = [(0, 0, 320, 240)];
        assert_eq!(restore(old, PLAYER, LIBRARY, 1, monitors(&areas)), old);
    }

    #[test]
    fn negative_monitors_separate_windows_and_a_pair_across_monitors_remain_exact() {
        let areas = [(-1920, -300, 1920, 1080), (0, 0, 1920, 1040)];
        let separate = saved((120, 100), (-1500, -200), false);
        assert_eq!(
            restore(separate, PLAYER, LIBRARY, 1, monitors(&areas)),
            separate
        );
        let across = saved((-200, 100), (360, 100), true);
        assert_eq!(
            restore(across, PLAYER, LIBRARY, 1, monitors(&areas)),
            across
        );
        // Only the rightmost 96 pixels of this oversized title reach monitor 2.
        let partial = saved((-1000, 100), (-664, 900), false);
        let right_only = [(0, 0, 1920, 1040)];
        let restored = restore(partial, PLAYER, LIBRARY, 1, monitors(&right_only));
        assert_eq!(restored.library, partial.library);
    }

    #[test]
    fn missing_monitors_repair_free_windows_independently_and_keep_closed_state() {
        let areas = [(0, 30, 1920, 1040)];
        let old = SavedLayout {
            library_open: false,
            ..saved((2400, 100), (-1900, -200), false)
        };
        let new = restore(old, PLAYER, LIBRARY, 1, monitors(&areas));
        assert_eq!(new.player, (1360, 100));
        assert_eq!(new.library, (0, 30));
        assert!(!new.attached && !new.library_open);
        assert_title_accessible(new.player, PLAYER, 1, &areas);
        assert_title_accessible(new.library, LIBRARY, 1, &areas);
    }

    #[test]
    fn missing_monitor_translates_the_group_preserving_all_four_offsets() {
        let areas = [(0, 0, 1920, 1080)];
        for offset in [(560, 70), (-760, 70), (120, 470), (120, -470)] {
            let old = saved((3000, 2000), (3000 + offset.0, 2000 + offset.1), true);
            let new = restore(old, PLAYER, LIBRARY, 1, monitors(&areas));
            assert!(new.attached);
            assert_eq!(
                (new.library.0 - new.player.0, new.library.1 - new.player.1),
                offset
            );
            assert_title_accessible(new.player, PLAYER, 1, &areas);
            assert_title_accessible(new.library, LIBRARY, 1, &areas);
        }
    }

    #[test]
    fn scale_changes_keep_player_fixed_and_scale_attached_offsets_but_not_free_positions() {
        let areas = [(-4000, -3000, 8000, 6000)];
        for offset in [(560, 70), (-760, 70), (120, 470), (120, -470)] {
            let old = saved((-1000, -800), (-1000 + offset.0, -800 + offset.1), true);
            let new = restore(old, (1120, 940), (1520, 940), 2, monitors(&areas));
            assert_eq!(new.player, old.player);
            assert_eq!(new.library, (-1000 + offset.0 * 2, -800 + offset.1 * 2));
            assert_eq!(new.scale, 2);
            assert_eq!(restore(new, PLAYER, LIBRARY, 1, monitors(&areas)), old);
        }
        let old = saved((-1000, -800), (1500, 1200), false);
        let new = restore(old, (1120, 940), (1520, 940), 2, monitors(&areas));
        assert_eq!(new, SavedLayout { scale: 2, ..old });
    }

    #[test]
    fn invalid_attached_gap_overlap_and_corner_get_a_real_adjacent_placement() {
        let areas = [(0, 0, 1920, 1080)];
        for library in [(900, 100), (661, 100), (300, 100), (660, 570)] {
            let new = restore(
                saved((100, 100), library, true),
                PLAYER,
                LIBRARY,
                1,
                monitors(&areas),
            );
            assert_eq!(new.player, (100, 100));
            assert_eq!(new.library, (660, 100));
            assert!(new.attached);
        }
    }

    #[test]
    fn unavailable_orientation_uses_an_accessible_side_without_requiring_whole_windows_to_fit() {
        let areas = [(0, 0, 512, 300)];
        let old = SavedLayout {
            scale: 2,
            ..saved((100, 100), (100, 1040), true)
        };
        let new = restore(old, (1120, 940), (1520, 940), 2, monitors(&areas));
        assert!(new.attached);
        assert_eq!(new.library.1, new.player.1);
        assert!(new.library.0 == new.player.0 + 1120 || new.library.0 + 1520 == new.player.0);
        assert_title_accessible(new.player, (1120, 940), 2, &areas);
        assert_title_accessible(new.library, (1520, 940), 2, &areas);
    }

    #[test]
    fn physically_impossible_attached_layout_detaches_and_keeps_each_title_reachable() {
        let areas = [(0, 0, 320, 240)];
        let old = SavedLayout {
            scale: 2,
            ..saved((100, 100), (1220, 100), true)
        };
        let new = restore(old, (1120, 940), (1520, 940), 2, monitors(&areas));
        assert!(!new.attached);
        assert_title_accessible(new.player, (1120, 940), 2, &areas);
        assert_title_accessible(new.library, (1520, 940), 2, &areas);
    }

    #[test]
    fn native_coordinate_extremes_and_missing_monitor_data_never_overflow() {
        let areas = [(0, 0, 1920, 1080)];
        for (a, b) in [
            (i32::MIN as isize, i32::MAX as isize),
            (i32::MAX as isize, i32::MIN as isize),
        ] {
            for attached in [true, false] {
                let old = saved((a, b), (b, a), attached);
                let new = restore(old, PLAYER, LIBRARY, 2, monitors(&areas));
                assert_title_accessible(new.player, PLAYER, 2, &areas);
                assert_title_accessible(new.library, LIBRARY, 2, &areas);
            }
        }
        let old = saved((-1000, -200), (-440, -200), true);
        assert_eq!(restore(old, PLAYER, LIBRARY, 1, |_| None), old);
        let edge = saved((i32::MAX as isize - 560, 0), (i32::MAX as isize, 0), true);
        let new = restore(edge, (1120, 940), (1520, 940), 2, |_| None);
        assert_eq!(new.library.0 - new.player.0, 1120);
        assert_eq!(new.library.0, i32::MAX as isize);
        let new = restore(
            old,
            (usize::MAX, usize::MAX),
            (usize::MAX, usize::MAX),
            2,
            monitors(&areas),
        );
        assert_title_accessible(
            new.player,
            (i32::MAX as usize, i32::MAX as usize),
            2,
            &areas,
        );
        assert_title_accessible(
            new.library,
            (i32::MAX as usize, i32::MAX as usize),
            2,
            &areas,
        );
    }
}
