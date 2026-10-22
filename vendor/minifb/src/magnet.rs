//! Geometry shared by the native magnetic-window implementations. No allocation.

#[derive(Clone, Copy, Debug)]
pub(crate) struct Rect {
    pub x: f64,
    pub y: f64,
    pub width: f64,
    pub height: f64,
}

impl Rect {
    fn valid(self) -> bool {
        [
            self.x,
            self.y,
            self.width,
            self.height,
            self.right(),
            self.bottom(),
        ]
        .iter()
        .all(|v| v.is_finite())
            && self.width > 0.0
            && self.height > 0.0
    }
    fn right(self) -> f64 {
        self.x + self.width
    }
    fn bottom(self) -> f64 {
        self.y + self.height
    }
}

fn align(value: f64, first: f64, last: f64, distance: f64) -> f64 {
    let a = (value - first).abs();
    let b = (value - last).abs();
    if a <= distance && a <= b {
        first
    } else if b <= distance {
        last
    } else {
        value
    }
}

/// Return the closest adjacent position within `distance` of a facing edge.
/// Coordinates and distance use the same native screen units; callers scale
/// the threshold. A corner alone is not enough: the facing sides must overlap.
pub(crate) fn snap(moving: Rect, stationary: Rect, distance: f64) -> Option<(f64, f64)> {
    if !moving.valid() || !stationary.valid() || !distance.is_finite() || distance < 0.0 {
        return None;
    }
    let mut best: Option<(f64, f64, f64)> = None;
    let mut consider = |x: f64, y: f64| {
        let score = (x - moving.x).abs() + (y - moving.y).abs();
        if best.is_none_or(|(_, _, previous)| score < previous) {
            best = Some((x, y, score));
        }
    };
    let y = align(
        moving.y,
        stationary.y,
        stationary.bottom() - moving.height,
        distance,
    );
    if moving.y < stationary.bottom() && moving.bottom() > stationary.y {
        for x in [stationary.right(), stationary.x - moving.width] {
            if (x - moving.x).abs() <= distance {
                consider(x, y);
            }
        }
    }
    let x = align(
        moving.x,
        stationary.x,
        stationary.right() - moving.width,
        distance,
    );
    if moving.x < stationary.right() && moving.right() > stationary.x {
        for y in [stationary.bottom(), stationary.y - moving.height] {
            if (y - moving.y).abs() <= distance {
                consider(x, y);
            }
        }
    }
    best.map(|(x, y, _)| (x, y))
}

// AppKit uses CGFloat doubles. Keep this scalar ABI independent of NSRect's
// platform ABI and share the exact same edge rules with Windows and X11.
#[cfg(target_os = "macos")]
#[unsafe(no_mangle)]
unsafe extern "C" fn mfb_snap_dock_rect(
    mx: f64,
    my: f64,
    mw: f64,
    mh: f64,
    fx: f64,
    fy: f64,
    fw: f64,
    fh: f64,
    distance: f64,
    out_x: *mut f64,
    out_y: *mut f64,
) -> bool {
    if out_x.is_null() || out_y.is_null() {
        return false;
    }
    let moving = Rect {
        x: mx,
        y: my,
        width: mw,
        height: mh,
    };
    let fixed = Rect {
        x: fx,
        y: fy,
        width: fw,
        height: fh,
    };
    if let Some((x, y)) = snap(moving, fixed, distance) {
        unsafe {
            *out_x = x;
            *out_y = y;
        }
        true
    } else {
        false
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn rect(x: f64, y: f64, width: f64, height: f64) -> Rect {
        Rect {
            x,
            y,
            width,
            height,
        }
    }

    #[test]
    fn all_four_sides_attach_and_align_nearby_top_or_left_edges() {
        let fixed = rect(100.0, 200.0, 560.0, 470.0);
        for (x, y, expected) in [
            (669.0, 205.0, (660.0, 200.0)),
            (-650.0, 193.0, (-660.0, 200.0)),
            (108.0, 678.0, (100.0, 670.0)),
            (94.0, -261.0, (100.0, -270.0)),
        ] {
            assert_eq!(snap(rect(x, y, 760.0, 470.0), fixed, 12.0), Some(expected));
        }
    }

    #[test]
    fn unequal_sizes_align_bottom_and_right_edges_or_keep_partial_overlap() {
        let fixed = rect(100.0, 200.0, 560.0, 470.0);
        assert_eq!(
            snap(rect(668.0, 478.0, 300.0, 200.0), fixed, 12.0),
            Some((660.0, 470.0))
        );
        assert_eq!(
            snap(rect(367.0, 679.0, 300.0, 200.0), fixed, 12.0),
            Some((360.0, 670.0))
        );
        assert_eq!(
            snap(rect(668.0, 330.0, 300.0, 200.0), fixed, 12.0),
            Some((660.0, 330.0))
        );
    }

    #[test]
    fn distant_overlapping_and_corner_only_windows_remain_free() {
        let fixed = rect(100.0, 100.0, 200.0, 200.0);
        for moving in [
            rect(313.0, 100.0, 100.0, 100.0),
            rect(150.0, 150.0, 100.0, 100.0),
            rect(300.0, 300.0, 100.0, 100.0),
            rect(305.0, 305.0, 100.0, 100.0),
            rect(305.0, 305.0, 1.0, 1.0),
            rect(300.0, 300.0, 8.0, 8.0),
        ] {
            assert_eq!(snap(moving, fixed, 12.0), None);
        }
        assert_eq!(
            snap(rect(312.0, 100.0, 100.0, 100.0), fixed, 12.0),
            Some((300.0, 100.0))
        );
    }

    #[test]
    fn negative_monitor_coordinates_and_scaled_thresholds_are_consistent() {
        for scale in [1.0, 2.0] {
            let fixed = rect(
                -1200.0 * scale,
                -200.0 * scale,
                560.0 * scale,
                470.0 * scale,
            );
            let moving = rect(-631.0 * scale, -195.0 * scale, 760.0 * scale, 470.0 * scale);
            assert_eq!(
                snap(moving, fixed, 12.0 * scale),
                Some((-640.0 * scale, -200.0 * scale))
            );
        }
    }

    #[test]
    fn either_window_can_be_the_one_moved_and_exact_contact_is_stable() {
        let player = rect(100.0, 100.0, 560.0, 470.0);
        let library = rect(668.0, 104.0, 760.0, 470.0);
        assert_eq!(snap(library, player, 12.0), Some((660.0, 100.0)));
        assert_eq!(snap(player, library, 12.0), Some((108.0, 104.0)));
        assert_eq!(
            snap(rect(660.0, 100.0, 760.0, 470.0), player, 0.0),
            Some((660.0, 100.0))
        );
    }

    #[test]
    fn invalid_native_geometry_never_produces_a_position() {
        let fixed = rect(100.0, 100.0, 100.0, 100.0);
        for moving in [
            rect(f64::NAN, 100.0, 100.0, 100.0),
            rect(200.0, 100.0, 0.0, 100.0),
            rect(f64::MAX, 100.0, f64::MAX, 100.0),
        ] {
            assert_eq!(snap(moving, fixed, 12.0), None);
        }
        assert_eq!(snap(fixed, fixed, f64::INFINITY), None);
        assert_eq!(snap(fixed, fixed, -1.0), None);
    }
}
