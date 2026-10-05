//! The 8-bit scenes: the vault door on the lock screen and the rooms of
//! boxes behind it, painted on a `Canvas`.

use std::f32::consts::TAU;

use crate::pixel::{Canvas, Rgb, hex, mix, pal};
use crate::sprites::{self, Eyes, LAPLA_H, LAPLA_W};

pub const NIGHT: Rgb = hex(0x0c0c1e);
const WALL: Rgb = hex(0x1c1c3c);
const MORTAR: Rgb = hex(0x26265a);
const TILE_A: Rgb = hex(0xbcbcbc);
const TILE_B: Rgb = hex(0x7c7c7c);
const SIREN: Rgb = hex(0xff2020);
pub const FLOOR: i32 = 4;

fn c(ch: char) -> Rgb {
    pal(ch).unwrap_or(NIGHT)
}

/// Brick wall, tinted red while the alarm flashes.
pub fn wall(cv: &mut Canvas, alarm: bool) {
    let (brick, line) = if alarm { (mix(WALL, SIREN, 0.35), mix(MORTAR, SIREN, 0.35)) } else { (WALL, MORTAR) };
    for y in 0..cv.h {
        for x in 0..cv.w {
            let joint = (x + if (y / 6) % 2 == 0 { 0 } else { 6 }) % 12 == 0;
            cv.set(x, y, if y % 6 == 5 || joint { line } else { brick });
        }
    }
}

/// The checkered bank floor on the last `FLOOR` pixel rows.
pub fn floor(cv: &mut Canvas) {
    for y in cv.h - FLOOR..cv.h {
        for x in 0..cv.w {
            let dark = ((x / 4) + (y / 2)) % 2 == 0;
            cv.set(x, y, if dark { TILE_B } else { TILE_A });
        }
    }
    cv.rect(0, cv.h - FLOOR, cv.w, 1, c('k'));
}

/// Lapla standing with his feet at `ground`. `hat` lifts the hat and ears.
pub fn lapla(cv: &mut Canvas, x: i32, ground: i32, eyes: Eyes, step: Option<bool>, hat: i32, flip: bool) {
    let rows = sprites::lapla(eyes, step);
    let top = ground - LAPLA_H;
    cv.sprite(x, top - hat, &rows[..12], flip, 0);
    cv.sprite(x, top + 12, &rows[12..], flip, 0);
}

/// One pixel of the round door, `dx`, `dy` from its centre, wheel at `turn`.
fn door_px(dx: f32, dy: f32, r: f32, turn: f32) -> Rgb {
    let d = dx.hypot(dy);
    if d > r - 1.0 {
        return c('k');
    }
    if d > r - 4.0 {
        // Eight rivets on the rim.
        let a = dy.atan2(dx);
        let rivet = (a * 8.0 / TAU).rem_euclid(1.0);
        return if (d - (r - 2.5)).abs() < 1.0 && (rivet < 0.08 || rivet > 0.92) { c('y') } else { c('S') };
    }
    if d < 3.0 {
        return if d > 2.0 { c('o') } else { c('y') };
    }
    let ring = r * 0.42;
    if (d - ring).abs() < 0.8 {
        let a = dy.atan2(dx) - turn * 0.5;
        return if (a * 8.0 / TAU).rem_euclid(1.0) < 0.2 { c('w') } else { c('G') };
    }
    for k in 0..3 {
        let a = turn + k as f32 * TAU / 3.0;
        let (along, across) = (dx * a.cos() + dy * a.sin(), -dx * a.sin() + dy * a.cos());
        let knob = (along - r * 0.66).hypot(across);
        if knob < 2.6 {
            return if knob > 1.8 { c('o') } else { c('y') };
        }
        if (0.0..r * 0.66).contains(&along) && across.abs() < 1.5 {
            return if across > 0.5 { c('o') } else { c('Y') };
        }
    }
    if dx + dy < -r * 0.8 { c('g') } else { c('s') }
}

/// The vault door: frame, gold behind it, the door itself swung `open`
/// (0 shut, 1 wide open) on its left hinge, wheel turned by `turn`.
pub fn door(cv: &mut Canvas, cx: i32, cy: i32, r: i32, turn: f32, open: f32, beacon: Option<bool>) {
    let side = r * 2 + 8;
    let (fx, fy) = (cx - side / 2, cy - side / 2);
    cv.rect(fx - 1, fy - 1, side + 2, side + 2, c('k'));
    cv.rect(fx, fy, side, side, c('d'));
    for (bx, by) in [(fx + 2, fy + 2), (fx + side - 3, fy + 2), (fx + 2, fy + side - 3), (fx + side - 3, fy + side - 3)] {
        cv.rect(bx, by, 1, 1, c('G'));
    }
    let rf = r as f32;
    let (cxf, cyf) = (cx as f32, cy as f32);
    cv.disc(cxf, cyf, rf, |_, _| Some(c('k')));
    // Gold bars stacked three, two, one at the bottom of the vault.
    for (row, count) in [(0, 3), (1, 2), (2, 1)] {
        for i in 0..count {
            let x = cx - count * 5 + i * 10 + 1;
            let y = cy + r / 2 - row * 4;
            cv.rect(x, y, 8, 3, c('y'));
            cv.rect(x, y, 8, 1, c('Y'));
            cv.rect(x, y + 2, 8, 1, c('o'));
        }
    }
    let squeeze = 1.0 - 0.85 * open.clamp(0.0, 1.0);
    let hinge = cxf - rf;
    for y in cy - r..=cy + r {
        for x in cx - r..=cx + r {
            let u = (x as f32 + 0.5 - hinge) / squeeze;
            let (dx, dy) = (u - rf, y as f32 + 0.5 - cyf);
            if dx.hypot(dy) <= rf {
                cv.set(x, y, mix(door_px(dx, dy, rf, turn), c('k'), open * 0.5));
            }
        }
    }
    if let Some(on) = beacon {
        let light = if on { SIREN } else { c('b') };
        cv.disc(cxf, (fy - 1) as f32, 4.0, |_, dy| (dy <= 0.0).then_some(light));
        cv.rect(cx - 5, fy - 1, 10, 1, c('k'));
        if on {
            for (dx, dy) in [(-8, -3), (-10, -5), (8, -3), (10, -5), (0, -8), (0, -10)] {
                cv.rect(cx + dx, fy + dy, 1, 1, c('Y'));
            }
        }
    }
}

pub const BOX_W: i32 = 26;
const BOX_H: i32 = 10;
const LANE: i32 = 3 + LAPLA_W + 8;

/// A box name to write over the canvas, in cells.
pub struct Label {
    pub col: i32,
    pub row: i32,
    pub text: String,
    pub fg: Rgb,
}

/// One deposit box with its top left at (x, y), y even so its two text
/// rows fall on cells: steel, a name plate and a gold handle when `full`,
/// an open dark hole when not.
fn deposit_box(cv: &mut Canvas, x: i32, y: i32, full: bool, frame: Rgb) {
    cv.rect(x, y, BOX_W, BOX_H, frame);
    let (w, h) = (BOX_W - 2, BOX_H - 2);
    if full {
        cv.rect(x + 1, y + 1, w, h, c('s'));
        cv.rect(x + 1, y + 1, w, 1, c('w'));
        cv.rect(x + 1, y + h, w, 1, c('S'));
        cv.rect(x + 2, y + 2, w - 4, 2, c('g'));
        cv.rect(x + w / 2 - 3, y + 6, 8, 1, c('y'));
        cv.rect(x + w / 2 - 3, y + 7, 8, 1, c('o'));
        cv.rect(x + w - 2, y + 5, 1, 2, c('k'));
    } else {
        cv.rect(x + 1, y + 1, w, h, c('k'));
        cv.rect(x + 4, y + 2, w - 4, h - 2, c('d'));
        cv.rect(x + 1, y + 1, 2, h, c('S'));
        cv.rect(x + 3, y + 1, 1, h, c('G'));
    }
}

/// A marble pillar with a gold capital, `w` wide, from `top` to the floor.
fn pillar(cv: &mut Canvas, x: i32, top: i32, w: i32) {
    let ground = cv.h - FLOOR;
    cv.rect(x - 1, top, w + 2, 3, c('y'));
    cv.rect(x - 1, top + 2, w + 2, 1, c('o'));
    cv.rect(x, top + 3, w, ground - top - 5, c('g'));
    cv.rect(x + 1, top + 3, 1, ground - top - 5, c('w'));
    cv.rect(x + w - 1, top + 3, 1, ground - top - 5, c('G'));
    cv.rect(x - 1, ground - 2, w + 2, 2, c('G'));
}

/// The wall of boxes of one room, shifted `dx` pixels, `sel` lit in gold.
/// Returns the names and numbers to write over it.
pub fn shelf(cv: &mut Canvas, dx: i32, boxes: &[(&str, bool)], sel: Option<usize>, lit_all: bool, t: f32) -> Vec<Label> {
    pillar(cv, LANE - 6 + dx, 0, 4);
    let cols = ((cv.w - LANE - 2) / (BOX_W + 2)).max(1);
    let rows = ((cv.h - FLOOR - 4) / (BOX_H + 2)).max(1);
    // Scroll by whole rows to keep the selected box on the wall.
    let first = sel.map_or(0, |s| (s as i32 / cols - rows + 1).max(0)) * cols;
    let shown = (boxes.len() as i32 - first).clamp(1, cols * rows);
    let used = (shown + cols - 1) / cols * (BOX_H + 2);
    // Centred on the wall, on an even row so names land on cells.
    let top = ((cv.h - FLOOR - used) / 2).max(2) & !1;
    let blink = (t * 3.0) as i32 % 2 == 0;
    let mut labels = Vec::new();
    for (i, (name, full)) in boxes.iter().enumerate().skip(first as usize).take(shown as usize) {
        let k = i as i32 - first;
        let (x, y) = (LANE + dx + (k % cols) * (BOX_W + 2), top + (k / cols) * (BOX_H + 2));
        let lit = lit_all || sel == Some(i);
        deposit_box(cv, x, y, *full, if lit { c(if blink { 'y' } else { 'Y' }) } else { c('k') });
        let text: String = name.chars().take((BOX_W - 5) as usize).collect();
        let (fg, number) = if *full { (c('k'), format!("No {:02}", i + 1)) } else { (c('g'), "empty".into()) };
        labels.push(Label { col: x + 2, row: y / 2 + 1, text, fg });
        labels.push(Label { col: x + 2, row: y / 2 + 2, text: number, fg: if *full { c('o') } else { c('G') } });
    }
    labels
}

/// The coins of the Konami rain, `age` seconds in.
pub fn rain(cv: &mut Canvas, age: f32) {
    let ground = cv.h - FLOOR;
    for i in 0..28 {
        let start = (i * 37 % 100) as f32 / 100.0 * 1.6;
        let fall = age - start;
        if fall < 0.0 {
            continue;
        }
        let x = (i * 53 % 97) as f32 / 97.0 * (cv.w - 8) as f32;
        let y = (fall * fall * 60.0 - 8.0).min((ground - 8) as f32);
        let frame = [0, 1, 2, 1][((age * 10.0) as usize + i as usize) % 4];
        cv.sprite(x as i32, y as i32, &sprites::COIN[frame], false, 0);
    }
}

/// What Lapla does in the vault room.
pub enum Pose {
    Stand { blink: bool, tip: bool },
    Walk { step: bool, left: bool },
    Sleep,
    /// Just woken up, 0 to 1.
    Startle(f32),
    /// Typing "hopla": a flip, 0 to 1.
    Flip(f32),
    /// Coins raining: hopping at this phase.
    Cheer(f32),
}

/// Lapla in his lane at the left of the room.
pub fn host(cv: &mut Canvas, pose: &Pose) {
    let ground = cv.h - FLOOR;
    let x = 3;
    match *pose {
        Pose::Stand { blink, tip } => {
            lapla(cv, x, ground, if blink { Eyes::Shut } else { Eyes::Open }, None, if tip { 3 } else { 0 }, false)
        }
        Pose::Walk { step, left } => lapla(cv, x, ground - i32::from(step), Eyes::Open, Some(step), 0, left),
        Pose::Sleep => {
            cv.sprite(x + 3, ground - sprites::NOTES.len() as i32, &sprites::NOTES, false, 0);
            lapla(cv, x, ground - 7, Eyes::Shut, None, 0, false);
        }
        Pose::Startle(q) => lapla(cv, x, ground - (8.0 * (q * TAU / 2.0).sin()) as i32, Eyes::Open, None, 4, false),
        Pose::Cheer(phase) => lapla(cv, x, ground - (5.0 * phase.sin().abs()) as i32, Eyes::Happy, None, 0, false),
        Pose::Flip(q) => {
            let rows = sprites::lapla(Eyes::Happy, None);
            let lift = (16.0 * (q * TAU / 2.0).sin()) as i32;
            let turn = (q * 4.0) as u8 % 4;
            // A quarter turn lays him on his side: centre the turned sprite.
            let (w, h) = if turn % 2 == 0 { (LAPLA_W, LAPLA_H) } else { (LAPLA_H, LAPLA_W) };
            cv.sprite(x + (LAPLA_W - w) / 2, ground - h - lift, &rows, false, turn);
        }
    }
}
