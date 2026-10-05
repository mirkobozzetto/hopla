//! A pixel canvas drawn with half blocks: each cell shows two stacked
//! pixels, the top one as the foreground of `▀`, the bottom one as its
//! background. Sprites are written one char per pixel in `pal` colours.

use ratatui::buffer::Buffer;
use ratatui::layout::Rect;
use ratatui::style::Color;

pub type Rgb = [u8; 3];

pub const fn hex(c: u32) -> Rgb {
    [(c >> 16) as u8, (c >> 8) as u8, c as u8]
}

pub const fn color([r, g, b]: Rgb) -> Color {
    Color::Rgb(r, g, b)
}

pub fn mix(a: Rgb, b: Rgb, p: f32) -> Rgb {
    let p = p.clamp(0.0, 1.0);
    let m = |x: u8, y: u8| (f32::from(x) + (f32::from(y) - f32::from(x)) * p).round() as u8;
    [m(a[0], b[0]), m(a[1], b[1]), m(a[2], b[2])]
}

/// The NES-like palette of the sprites. `.` and anything unknown is clear.
pub fn pal(c: char) -> Option<Rgb> {
    Some(hex(match c {
        'k' => 0x000000,
        'w' => 0xfcfcfc,
        'g' => 0xbcbcbc,
        'G' => 0x7c7c7c,
        'd' => 0x404040,
        'p' => 0xf8a4c0,
        'P' => 0xe45c8c,
        'r' => 0xd82800,
        'b' => 0x881400,
        'y' => 0xf8b800,
        'Y' => 0xfce0a8,
        'o' => 0xac7c00,
        'n' => 0x0000bc,
        'e' => 0x00a800,
        'E' => 0x58d854,
        's' => 0xa8a8a8,
        'S' => 0x6c6c6c,
        _ => return None,
    }))
}

pub struct Canvas {
    pub w: i32,
    pub h: i32,
    px: Vec<Option<Rgb>>,
}

impl Canvas {
    /// A canvas covering `cols` x `rows` cells, so twice as many pixel rows.
    pub fn new(cols: u16, rows: u16) -> Self {
        let (w, h) = (i32::from(cols), i32::from(rows) * 2);
        Self { w, h, px: vec![None; (w * h) as usize] }
    }

    pub fn get(&self, x: i32, y: i32) -> Option<Rgb> {
        let inside = (0..self.w).contains(&x) && (0..self.h).contains(&y);
        if inside { self.px[(y * self.w + x) as usize] } else { None }
    }

    pub fn set(&mut self, x: i32, y: i32, c: Rgb) {
        if (0..self.w).contains(&x) && (0..self.h).contains(&y) {
            self.px[(y * self.w + x) as usize] = Some(c);
        }
    }

    pub fn rect(&mut self, x: i32, y: i32, w: i32, h: i32, c: Rgb) {
        for yy in y..y + h {
            for xx in x..x + w {
                self.set(xx, yy, c);
            }
        }
    }

    /// Paints every pixel of the disc of radius `r` with `paint(dx, dy)`,
    /// where (dx, dy) is the offset from the centre; `None` leaves it.
    pub fn disc(&mut self, cx: f32, cy: f32, r: f32, paint: impl Fn(f32, f32) -> Option<Rgb>) {
        let (x0, x1) = ((cx - r).floor() as i32, (cx + r).ceil() as i32);
        let (y0, y1) = ((cy - r).floor() as i32, (cy + r).ceil() as i32);
        for y in y0..=y1 {
            for x in x0..=x1 {
                let (dx, dy) = (x as f32 + 0.5 - cx, y as f32 + 0.5 - cy);
                if dx.hypot(dy) <= r
                    && let Some(c) = paint(dx, dy)
                {
                    self.set(x, y, c);
                }
            }
        }
    }

    /// Draws a sprite with its top left at (x, y), mirrored when `flip`,
    /// turned by `turn` quarter turns clockwise.
    pub fn sprite(&mut self, x: i32, y: i32, rows: &[&str], flip: bool, turn: u8) {
        let h = rows.len() as i32;
        for (j, row) in rows.iter().enumerate() {
            let w = row.chars().count() as i32;
            for (i, ch) in row.chars().enumerate() {
                let Some(c) = pal(ch) else { continue };
                let i = if flip { w - 1 - i as i32 } else { i as i32 };
                let j = j as i32;
                let (dx, dy) = match turn % 4 {
                    0 => (i, j),
                    1 => (h - 1 - j, i),
                    2 => (w - 1 - i, h - 1 - j),
                    _ => (j, w - 1 - i),
                };
                self.set(x + dx, y + dy, c);
            }
        }
    }

    /// Copies the drawn pixels of `other` with its top left at (x, y).
    pub fn blit(&mut self, other: &Canvas, x: i32, y: i32) {
        for j in 0..other.h {
            for i in 0..other.w {
                if let Some(c) = other.get(i, j) {
                    self.set(x + i, y + j, c);
                }
            }
        }
    }

    /// Puts the canvas on screen at `area`, `bg` where nothing was drawn.
    pub fn draw(&self, buf: &mut Buffer, area: Rect, bg: Rgb) {
        for row in 0..area.height.min((self.h / 2) as u16) {
            for col in 0..area.width.min(self.w as u16) {
                let (x, y) = (i32::from(col), i32::from(row) * 2);
                let top = self.get(x, y).unwrap_or(bg);
                let bottom = self.get(x, y + 1).unwrap_or(bg);
                buf[(area.x + col, area.y + row)].set_symbol("▀").set_fg(color(top)).set_bg(color(bottom));
            }
        }
    }

    /// Writes text over the drawn canvas, on the colour of the cells it
    /// covers. `col` and `row` count cells from the top left of `area`.
    pub fn text(&self, buf: &mut Buffer, area: Rect, col: i32, row: i32, text: &str, fg: Rgb) {
        for (i, ch) in text.chars().enumerate() {
            let (c, r) = (col + i as i32, row);
            if !(0..i32::from(area.width)).contains(&c) || !(0..i32::from(area.height)).contains(&r) {
                continue;
            }
            let under = self.get(c, r * 2).unwrap_or([0; 3]);
            let cell = &mut buf[(area.x + c as u16, area.y + r as u16)];
            cell.set_char(ch).set_fg(color(fg)).set_bg(color(under));
        }
    }
}
