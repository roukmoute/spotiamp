//! Built-in skins share the same windows, input geometry and pixel buffers.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
pub enum Skin {
    #[default]
    Classic,
    Modern,
}

impl Skin {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Classic => "classic",
            Self::Modern => "modern",
        }
    }

    pub fn parse(value: &str) -> Option<Self> {
        match value.trim() {
            "classic" => Some(Self::Classic),
            "modern" => Some(Self::Modern),
            _ => None,
        }
    }
}

fn fill(pixels: &mut [u32], stride: usize, rect: (usize, usize, usize, usize), color: u32) {
    if stride == 0 {
        return;
    }
    let (x, y, width, height) = rect;
    let left = x.min(stride);
    let right = x.saturating_add(width).min(stride);
    for row in y.min(pixels.len() / stride)..y.saturating_add(height).min(pixels.len() / stride) {
        pixels[row * stride + left..row * stride + right].fill(color);
    }
}

fn mix(top: u32, bottom: u32, at: usize, extent: usize) -> u32 {
    let step = (at * 256 / extent.max(1)) as i32;
    let mut color = 0;
    for shift in [0, 8, 16] {
        let a = ((top >> shift) & 255) as i32;
        let b = ((bottom >> shift) & 255) as i32;
        color |= ((a + ((b - a) * step / 256)) as u32) << shift;
    }
    color
}

/// Paint a vertical RGB gradient directly into an existing framebuffer.
pub fn gradient(
    pixels: &mut [u32],
    stride: usize,
    rect: (usize, usize, usize, usize),
    top: u32,
    bottom: u32,
) {
    if stride == 0 {
        return;
    }
    let (x, y, width, height) = rect;
    let left = x.min(stride);
    let right = x.saturating_add(width).min(stride);
    for row in y.min(pixels.len() / stride)..y.saturating_add(height).min(pixels.len() / stride) {
        let color = mix(top, bottom, row - y, height.saturating_sub(1));
        pixels[row * stride + left..row * stride + right].fill(color);
    }
}

/// Silver rim around either a recessed display or a raised panel.
pub fn panel(
    pixels: &mut [u32],
    stride: usize,
    rect: (usize, usize, usize, usize),
    color: u32,
    inset: bool,
) {
    let (x, y, width, height) = rect;
    fill(pixels, stride, rect, color);
    if width < 3 || height < 3 {
        return;
    }
    let (light, dark) = if inset {
        (0x56677b, 0xf7fbff)
    } else {
        (0xf7fbff, 0x536377)
    };
    fill(pixels, stride, (x, y, width, 1), light);
    fill(pixels, stride, (x, y, 1, height), light);
    fill(pixels, stride, (x, y + height - 1, width, 1), dark);
    fill(pixels, stride, (x + width - 1, y, 1, height), dark);
    let inner = if inset { 0x10283d } else { 0xc0ccda };
    fill(pixels, stride, (x + 1, y + 1, width - 2, 1), inner);
    fill(pixels, stride, (x + 1, y + 1, 1, height - 2), inner);
}

/// Rounded metal button, including disabled and depressed states, with no cache.
pub fn button(
    pixels: &mut [u32],
    stride: usize,
    rect: (usize, usize, usize, usize),
    enabled: bool,
    down: bool,
) {
    let (x, y, width, height) = rect;
    if width < 6 || height < 6 {
        panel(pixels, stride, rect, 0xb6c3d0, down);
        return;
    }
    let (top, middle, bottom) = if !enabled {
        (0xd2d9e0, 0xb0bdc9, 0x99a8b8)
    } else if down {
        (0x879bb0, 0xacbfce, 0xd4e2eb)
    } else {
        (0xffffff, 0xe1eaf1, 0x9cabbc)
    };
    for row in 0..height {
        let edge = row.min(height - 1 - row);
        let inset = if edge == 0 {
            3
        } else if edge == 1 {
            1
        } else {
            0
        };
        let color = if row == 0 || row == height - 1 {
            if down { 0x526d86 } else { 0x6c7e92 }
        } else if row <= height / 2 {
            mix(top, middle, row - 1, height / 2)
        } else {
            mix(middle, bottom, row - height / 2, height - 1 - height / 2)
        };
        fill(
            pixels,
            stride,
            (x + inset, y + row, width - 2 * inset, 1),
            color,
        );
        if row > 0 && row < height - 1 {
            fill(
                pixels,
                stride,
                (x + inset, y + row, 1, 1),
                if down { 0x60778e } else { 0xf4f9ff },
            );
            fill(
                pixels,
                stride,
                (x + width - 1 - inset, y + row, 1, 1),
                0x71869c,
            );
        }
    }
}
