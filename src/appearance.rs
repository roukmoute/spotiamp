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

// Subpixel coverage is evaluated at compile time. Only the two corner masks
// are kept; rendering needs neither a larger framebuffer nor a drawing cache.
const fn corner<const R: usize>(inset: usize) -> [[u8; R]; R] {
    let mut coverage = [[0; R]; R];
    let radius = ((R - inset) * 16) as i32;
    let mut y = inset;
    while y < R {
        let mut x = inset;
        while x < R {
            let mut sy = 0;
            while sy < 8 {
                let dy = (y * 16 + sy * 2 + 1) as i32 - (R * 16) as i32;
                let mut sx = 0;
                while sx < 8 {
                    let dx = (x * 16 + sx * 2 + 1) as i32 - (R * 16) as i32;
                    if dx * dx + dy * dy <= radius * radius {
                        coverage[y][x] += 1;
                    }
                    sx += 1;
                }
                sy += 1;
            }
            x += 1;
        }
        y += 1;
    }
    coverage
}

fn edge_pixel(
    pixels: &mut [u32],
    stride: usize,
    x: usize,
    y: usize,
    coverage: (u8, u8),
    colors: (u32, u32),
) {
    let (outer, inner) = coverage;
    if outer == 0 || x >= stride {
        return;
    }
    let Some(pixel) = pixels.get_mut(y.saturating_mul(stride).saturating_add(x)) else {
        return;
    };
    let (border, face) = colors;
    let mut color = 0;
    for shift in [0, 8, 16] {
        let channel = (((*pixel >> shift) & 255) * u32::from(64 - outer)
            + ((border >> shift) & 255) * u32::from(outer - inner)
            + ((face >> shift) & 255) * u32::from(inner)
            + 32)
            / 64;
        color |= channel << shift;
    }
    *pixel = color;
}

fn rounded_button<const R: usize>(
    pixels: &mut [u32],
    stride: usize,
    rect: (usize, usize, usize, usize),
    enabled: bool,
    down: bool,
    outer: &[[u8; R]; R],
    inner: &[[u8; R]; R],
) {
    let (x, y, width, height) = rect;
    if width < 2 * R || height < 2 * R {
        panel(pixels, stride, rect, 0xb6c3d0, down);
        return;
    }
    let (top, middle, bottom, rim_top, rim_bottom) = if !enabled {
        (0xd4dee6, 0xbdcbd6, 0xa7b9c8, 0x91a3b2, 0x7b90a2)
    } else if down {
        (0x8ba6bc, 0xb1c6d6, 0xd9e8f2, 0x496b87, 0x91aabc)
    } else {
        (0xf8fcff, 0xdbe8f1, 0xa4bdcf, 0x8ba1b3, 0x55738c)
    };
    for row in 0..height {
        let edge = row.min(height - 1 - row);
        let border = mix(rim_top, rim_bottom, row, height - 1);
        let face = if row <= height / 2 {
            mix(top, middle, row.saturating_sub(1), height / 2 - 1)
        } else {
            mix(middle, bottom, row - height / 2, height - 2 - height / 2)
        };
        fill(
            pixels,
            stride,
            (x + R, y + row, width - 2 * R, 1),
            if edge == 0 { border } else { face },
        );
        for col in 0..R {
            let coverage = if edge < R {
                (outer[edge][col], inner[edge][col])
            } else {
                (64, if col == 0 { 0 } else { 64 })
            };
            edge_pixel(pixels, stride, x + col, y + row, coverage, (border, face));
            edge_pixel(
                pixels,
                stride,
                x + width - 1 - col,
                y + row,
                coverage,
                (border, face),
            );
        }
    }
}

/// Rounded metal button with a continuous gradient and antialiased rim.
pub fn button(
    pixels: &mut [u32],
    stride: usize,
    rect: (usize, usize, usize, usize),
    enabled: bool,
    down: bool,
) {
    const OUTER: [[u8; 4]; 4] = corner(0);
    const INNER: [[u8; 4]; 4] = corner(1);
    rounded_button(pixels, stride, rect, enabled, down, &OUTER, &INNER);
}

/// The transport keeps its 38-by-22 hitbox and shares the other controls' finish.
pub fn transport_button(
    pixels: &mut [u32],
    stride: usize,
    rect: (usize, usize, usize, usize),
    enabled: bool,
    down: bool,
) {
    const OUTER: [[u8; 11]; 11] = corner(0);
    const INNER: [[u8; 11]; 11] = corner(1);
    rounded_button(pixels, stride, rect, enabled, down, &OUTER, &INNER);
}
