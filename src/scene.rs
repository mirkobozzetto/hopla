//! The 8-bit scenes: the vault door on the lock screen and the rooms of
//! boxes behind it, painted on a `Canvas`.

use std::f32::consts::TAU;

use crate::pixel::{Canvas, Rgb, hex, mix, pal};
use crate::sprites::{self, Eyes, LAPLA_H};

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
