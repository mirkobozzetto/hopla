//! Lapla, the bunny of draft/hopla-lapla.html, painted in half-block pixels:
//! each cell is `▀`, its foreground the top pixel, its background the bottom.

use std::f32::consts::{PI, TAU};

use ratatui::Frame;
use ratatui::layout::Rect;
use ratatui::style::{Color, Stylize};
use ratatui::text::{Line, Span};
use ratatui::widgets::Paragraph;

type Rgb = [f32; 3];

const fn hex(c: u32) -> Rgb {
    [((c >> 16) & 255) as f32, ((c >> 8) & 255) as f32, (c & 255) as f32]
}

const BG: Rgb = hex(0x0b0f0d);
const FUR: Rgb = hex(0xf4efe4);
const FUR_DARK: Rgb = hex(0x7d7466);
const TAIL_DARK: Rgb = hex(0xb9b2a4);
const WHITE: Rgb = hex(0xffffff);
const PINK: Rgb = hex(0xff9fb8);
const CHEEK: Rgb = hex(0xff8caa);
const PUPIL: Rgb = hex(0x15110e);
const LID: Rgb = hex(0x2a2622);
const WHISKER: Rgb = hex(0xe6e1d7);
const GRASS: Rgb = hex(0x3f7a52);
const SOIL_TOP: Rgb = hex(0x2c4a36);
const SOIL: Rgb = hex(0x0a120d);
const SPARKS: [Rgb; 5] = [hex(0xffd166), hex(0x3ddc97), hex(0xa78bfa), hex(0xf472b6), hex(0x5ccfe6)];
const BUBBLE: Color = Color::Rgb(0xd6, 0xe4, 0xdc);
const SAYS: Color = Color::Rgb(0x3d, 0xdc, 0x97);

pub enum Mood {
    Idle,
    Sniff,
    Cheer(f32),
    Sad,
}

struct Pose {
    y: f32,
    squash: f32,
    spin: f32,
    look: f32,
    perk: f32,
    twitch: f32,
    blink: bool,
    nose: f32,
}

fn mix(a: Rgb, b: Rgb, p: f32) -> Rgb {
    let p = p.clamp(0.0, 1.0);
    [a[0] + (b[0] - a[0]) * p, a[1] + (b[1] - a[1]) * p, a[2] + (b[2] - a[2]) * p]
}

fn pose(mood: &Mood, t: f32) -> Pose {
    // A small hop every 2.6 s, then a spring squash on landing.
    let phase = t % 2.6;
    let (y, squash) = if phase < 0.5 {
        (6.0 * (PI * phase / 0.5).sin(), 1.12)
    } else {
        let land = phase - 0.5;
        (0.0, 1.0 - 0.3 * (-land * 9.0).exp() * (land * 22.0).cos())
    };
    let tw = t % 3.3;
    let mut p = Pose {
        y,
        squash,
        spin: 0.0,
        look: 0.0,
        perk: -0.12,
        twitch: 0.35 * (-tw * 6.0).exp() * (tw * 30.0).sin(),
        blink: t % 4.1 < 0.15,
        nose: (t * 14.0).sin() * 0.35,
    };
    match *mood {
        Mood::Idle => {}
        Mood::Sniff => {
            (p.y, p.squash, p.look, p.perk) = (0.0, 1.0 + 0.04 * (t * 10.0).sin(), 1.0, -0.3);
            p.nose = (t * 30.0).sin() * 0.5;
        }
        Mood::Cheer(age) => {
            // A back flip over the time the lock screen shows its check mark.
            let q = (age / 0.9).min(1.0);
            (p.y, p.squash, p.spin, p.blink) = (20.0 * (PI * q).sin(), 1.0, -TAU * q, false);
        }
        Mood::Sad => (p.y, p.squash, p.perk, p.look, p.blink) = (0.0, 1.0, -1.0, 0.6, true),
    }
    p
}

fn inside(x: f32, y: f32, [cx, cy, rx, ry]: [f32; 4]) -> bool {
    ((x - cx) / rx).powi(2) + ((y - cy) / ry).powi(2) <= 1.0
}

/// Paints an ellipse lit from its top left, like the draft's radial gradient.
fn blob(c: &mut Option<Rgb>, x: f32, y: f32, e: [f32; 4], light: Rgb, dark: Rgb) {
    if inside(x, y, e) {
        let r = e[2].max(e[3]);
        let d = (x - (e[0] - 0.45 * r)).hypot(y - (e[1] - 0.55 * r));
        *c = Some(mix(light, dark, (d - 0.1 * r) / (1.4 * r)));
    }
}

fn tint(c: &mut Option<Rgb>, hit: bool, color: Rgb, alpha: f32) {
    if hit {
        *c = Some(mix(c.unwrap_or(BG), color, alpha));
    }
}

/// The bunny's colour at a point of its own frame, in draft units.
fn bunny(x: f32, y: f32, p: &Pose, px: f32) -> Option<Rgb> {
    let mut c = None;
    blob(&mut c, x, y, [-11.0, -9.0, 3.6, 3.4], WHITE, TAIL_DARK);
    blob(&mut c, x, y, [-5.0, -1.8, 6.5, 2.3], FUR, FUR_DARK);
    blob(&mut c, x, y, [0.0, -9.5, 11.5, 9.0], FUR, FUR_DARK);
    blob(&mut c, x, y, [6.5, -1.4, 2.6, 1.8], FUR, FUR_DARK);
    blob(&mut c, x, y, [9.5, -1.2, 2.4, 1.7], FUR, FUR_DARK);
    for (back, ex, base) in [(true, 3.0, -0.35), (false, 7.5, 0.05)] {
        let a = base + p.perk * if back { 1.3 } else { 1.0 } + p.twitch * if back { 1.0 } else { -1.0 };
        let (dx, dy) = (x - ex, y + 25.0);
        let (ux, uy) = (dx * a.cos() + dy * a.sin(), -dx * a.sin() + dy * a.cos());
        blob(&mut c, ux, uy, [0.0, -8.0, 2.8, 9.5], FUR, FUR_DARK);
        blob(&mut c, ux, uy, [0.2, -7.5, 1.3, 7.0], PINK, mix(PINK, FUR_DARK, 0.5));
    }
    blob(&mut c, x, y, [7.0, -20.0, 8.5, 7.2], FUR, FUR_DARK);
    tint(&mut c, inside(x, y, [11.0, -17.5, 2.4, 1.5]), CHEEK, 0.55);
    for (ex, ey, r) in [(4.8, -21.5, 2.3), (10.4, -21.2, 2.1)] {
        if p.blink {
            tint(&mut c, inside(x, y, [ex, ey + 0.4, r, 0.55]), LID, 1.0);
            continue;
        }
        let (lx, ly) = (ex + 0.7, ey + p.look * 0.7);
        tint(&mut c, inside(x, y, [ex, ey, r, r * 1.15]), WHITE, 0.97);
        tint(&mut c, inside(x, y, [lx, ly, r * 0.62, r * 0.75]), PUPIL, 1.0);
        tint(&mut c, inside(x, y, [lx - 0.5, ly - 0.6, 0.6, 0.6]), WHITE, 1.0);
    }
    blob(&mut c, x, y, [14.8, -19.0 + p.nose, 1.2, 0.9], PINK, PINK);
    for d in [-1.0, 0.0, 1.0] {
        // Whiskers: within half a pixel of the segment from the nose.
        let (ax, ay, bx, by) = (14.5, -18.6, 20.0, -18.6 + d * 1.8 + p.nose * 0.5);
        let k = (((x - ax) * (bx - ax) + (y - ay) * (by - ay)) / ((bx - ax).powi(2) + (by - ay).powi(2))).clamp(0.0, 1.0);
        let hit = (x - ax - k * (bx - ax)).hypot(y - ay - k * (by - ay)) < 0.45 * px;
        tint(&mut c, hit && k > 0.15, WHISKER, 0.8);
    }
    c
}

/// Lapla on a patch of grass, sized to `area`, with a word bubble.
pub fn render(f: &mut Frame, area: Rect, mood: Mood, t: f32) {
    let p = pose(&mood, t);
    let (w, h) = ((area.height * 2).min(area.width), area.height * 2);
    let s = f32::from(h - 6) / 50.0;
    let (gx, gy) = (f32::from(w) / 2.0 - 2.5 * s, f32::from(h) - 3.0);
    let rise = (1.0 - p.y / 40.0).max(0.25);
    let pixel = |col: u16, row: u16| -> Rgb {
        let (cx, cy) = (f32::from(col) + 0.5, f32::from(row) + 0.5);
        let edge = ((cx - f32::from(w) / 2.0).abs() / (f32::from(w) / 2.0)).powi(3);
        let blade = (2.0 + f32::from(col * 7 % 3)) * (0.8 + 0.2 * (t * 2.0 + f32::from(col)).sin());
        let mut c = if cy >= gy {
            Some(mix(mix(SOIL_TOP, SOIL, (cy - gy) / 3.0), BG, edge))
        } else if col.is_multiple_of(3) && cy >= gy - blade {
            Some(mix(GRASS, BG, edge))
        } else {
            None
        };
        let (wx, wy) = ((cx - gx) / s, (cy - gy) / s);
        tint(&mut c, inside(wx, wy, [0.0, 0.6, 14.0 * rise, 2.4 * rise]), [0.0; 3], 0.55 * rise);
        let (lx, ly) = (wx * p.squash.sqrt(), (wy + p.y) / p.squash + 12.0);
        let (lx, ly) = (lx * p.spin.cos() + ly * p.spin.sin(), -lx * p.spin.sin() + ly * p.spin.cos() - 12.0);
        if let Some(fur) = bunny(lx, ly, &p, 1.0 / s) {
            return fur;
        }
        if let Mood::Cheer(age) = mood {
            // Confetti bursting from the head, falling as it fades.
            for i in 0..18 {
                let a = i as f32 * TAU / 18.0 + 0.3;
                let v = 16.0 + (i * 7 % 5) as f32 * 5.0;
                let (sx, sy) = (7.0 + a.cos() * v * age, -22.0 - p.y + a.sin() * v * age + 30.0 * age * age);
                if (wx - sx).hypot(wy - sy) < 0.7 / s {
                    return mix(SPARKS[i % SPARKS.len()], BG, (age - 0.4) / 0.5);
                }
            }
        }
        c.unwrap_or(BG)
    };
    let rgb = |c: Rgb| Color::Rgb(c[0] as u8, c[1] as u8, c[2] as u8);
    let lines: Vec<Line> = (0..area.height)
        .map(|r| (0..w).map(|col| Span::from("▀").fg(rgb(pixel(col, r * 2))).bg(rgb(pixel(col, r * 2 + 1)))).collect())
        .collect();
    let sprite = Rect { x: area.x + (area.width - w) / 2, width: w, ..area };
    f.render_widget(Paragraph::new(lines), sprite);

    let says = match &mood {
        Mood::Idle => return,
        Mood::Sniff => "sniff sniff...",
        Mood::Cheer(_) => "hopla !",
        Mood::Sad => "no keys, then",
    };
    let bar = "─".repeat(says.chars().count() + 2);
    let bubble = vec![
        Line::from(format!("╭{bar}╮").fg(BUBBLE)),
        Line::from(vec!["│ ".fg(BUBBLE), says.fg(SAYS), " │".fg(BUBBLE)]),
        Line::from(format!("╰─┬{}╯", &bar[3..]).fg(BUBBLE)),
        Line::from("  ╵".fg(BUBBLE)),
    ];
    // The tail points at the top of the head.
    let col = sprite.x + (gx + 13.0 * s) as u16;
    let row = area.y + ((gy - (29.0 + p.y) * s) / 2.0).max(4.0) as u16;
    let at = Rect::new(col.saturating_sub(2), row - 4, bar.chars().count() as u16 + 2, 4);
    f.render_widget(Paragraph::new(bubble), at.intersection(f.area()));
}
