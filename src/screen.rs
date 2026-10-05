//! The screens: the vault door while locked, the rooms of boxes once open.
//! scene.rs paints the pixels; this module lays them out and writes text.

use std::time::{Duration, Instant};

use ratatui::Frame;
use ratatui::layout::{Constraint::*, Layout, Margin, Rect};
use ratatui::style::{Color, Style, Stylize};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, BorderType, Clear, Paragraph};

use crate::app::{App, Lock, Mode, SHOWN, Tone};
use crate::pixel::{Canvas, Rgb, color, hex};
use crate::scene::{self, FLOOR, NIGHT, Pose};
use crate::sprites::{Eyes, LAPLA_W};

const BG: Color = color(NIGHT);
const GOLD: Color = Color::Rgb(0xf8, 0xb8, 0x00);
const CREAM: Color = Color::Rgb(0xfc, 0xe0, 0xa8);
const WHITE: Color = Color::Rgb(0xfc, 0xfc, 0xfc);
const RED: Color = Color::Rgb(0xff, 0x20, 0x20);
const GREEN: Color = Color::Rgb(0x58, 0xd8, 0x54);
const SKY: Color = Color::Rgb(0x3c, 0xbc, 0xfc);
const DIM: Color = Color::Rgb(0x7c, 0x7c, 0x9c);
const FAINT: Color = Color::Rgb(0x44, 0x44, 0x6c);
const SPIN: [char; 10] = ['⠋', '⠙', '⠹', '⠸', '⠼', '⠴', '⠦', '⠧', '⠇', '⠏'];
const LOGO: [&str; 2] = ["█▄▄ ▄▀▄ █▀▄ █   ▄▀▄", "█ █ ▀▄▀ █▀  █▄▄ █▀█"];
// The same colours as pixels, for text written over the canvas.
const GOLD_PX: Rgb = hex(0xf8b800);
const CREAM_PX: Rgb = hex(0xfce0a8);
const WHITE_PX: Rgb = hex(0xfcfcfc);
const SKY_PX: Rgb = hex(0x3cbcfc);
const ALARM_FOR: Duration = Duration::from_secs(4);
const TRAVEL: f32 = 0.7;

fn spin(t: f32) -> char {
    SPIN[(t * 12.0) as usize % SPIN.len()]
}

fn since(at: Option<Instant>, within: f32) -> Option<f32> {
    at.map(|at| at.elapsed().as_secs_f32()).filter(|age| *age < within)
}

pub fn draw(f: &mut Frame, app: &mut App) {
    let t = app.start.elapsed().as_secs_f32();
    let area = f.area();
    f.render_widget(Block::new().style(Style::new().bg(BG).fg(WHITE)), area);
    if matches!(app.lock, Lock::Open) {
        room(f, app, area, t);
        if let Some((effect, last)) = &mut app.fx {
            effect.process(last.elapsed().into(), f.buffer_mut(), area);
            *last = Instant::now();
        }
        if app.fx.as_ref().is_some_and(|(effect, _)| effect.done()) {
            app.fx = None;
        }
    } else {
        lock(f, app, area, t);
    }
    note(f, app, area);
}

fn lock(f: &mut Frame, app: &App, area: Rect, t: f32) {
    let alarm = app.fun.alarm.is_some_and(|at| at.elapsed() < ALARM_FOR);
    let flash = (t * 4.0) as i32 % 2 == 0;
    let refused = app.note.as_ref().is_some_and(|n| matches!(n.1, Tone::Crit));
    let (title, sub) = match &app.lock {
        _ if alarm => (
            Line::from("ALARM !  ALARM !".fg(if flash { RED } else { WHITE }).bold()),
            Line::from(vec!["Enter".fg(GOLD), " tries again".fg(DIM)]),
        ),
        Lock::Scan(at, _) => {
            let n = (at.elapsed().as_secs_f32() / 0.45) as usize % 4;
            let dial: Vec<String> = ["3", "7", "1"].iter().enumerate().map(|(i, d)| if i < n { (*d).into() } else { "_".into() }).collect();
            (
                Line::from(vec![format!("cracking the dial {}  ", spin(t)).fg(CREAM), dial.join(" · ").fg(GOLD).bold()]),
                Line::from("Touch ID, or your session password".fg(DIM)),
            )
        }
        Lock::Done(_) => (Line::from("clac !  welcome back".fg(GOLD).bold()), Line::default()),
        _ => (
            Line::from(if refused { "ACCESS DENIED".fg(RED).bold() } else { "THE VAULT IS CLOSED".fg(GOLD).bold() }),
            Line::from(vec!["Enter".fg(GOLD), " opens with Touch ID   ".fg(DIM), "q".fg(GOLD), " quits".fg(DIM)]),
        ),
    };
    let lines = vec![title, sub, Line::default(), Line::from("values are copied, never shown".fg(FAINT))];
    let rows = area.height.saturating_sub(lines.len() as u16 + 3).min(28);
    let [pic, _, text] = Layout::vertical([Length(rows), Length(1), Length(lines.len() as u16)])
        .flex(ratatui::layout::Flex::Center)
        .areas(area);
    f.render_widget(Paragraph::new(lines).centered(), text);
    if rows < 16 {
        return;
    }
    let mut cv = Canvas::new(pic.width, pic.height);
    scene::wall(&mut cv, alarm && flash);
    let ground = cv.h - FLOOR;
    let r = ((ground - 14) / 2).clamp(8, 18);
    let (cx, cy) = (cv.w / 2 + 12, ground - r - 5);
    let (turn, open) = match &app.lock {
        Lock::Scan(at, _) => (at.elapsed().as_secs_f32() * 5.0, 0.0),
        Lock::Done(at) => {
            let age = at.elapsed().as_secs_f32();
            (2.0 + age.min(0.35) * 14.0, (age - 0.35) / 0.5)
        }
        _ => (0.3, 0.0),
    };
    scene::door(&mut cv, cx, cy, r, turn, open, alarm.then_some(flash));
    let x = cx - r - 10 - LAPLA_W;
    match &app.lock {
        // Hides behind the counter: only his hat, ears and eyes show.
        _ if alarm => scene::lapla(&mut cv, x, ground + 17, Eyes::Open, None, 0, false),
        Lock::Done(_) => scene::lapla(&mut cv, x, ground, Eyes::Happy, None, 4, false),
        Lock::Scan(..) => scene::lapla(&mut cv, x, ground, Eyes::Open, Some((t * 4.0) as i32 % 2 == 0), 0, false),
        _ if refused => scene::lapla(&mut cv, x + (t * 20.0) as i32 % 2, ground, Eyes::Shut, None, 0, false),
        _ => scene::lapla(&mut cv, x, ground, if t % 4.1 < 0.15 { Eyes::Shut } else { Eyes::Open }, None, if t % 6.0 < 0.5 { 3 } else { 0 }, false),
    }
    scene::floor(&mut cv);
    cv.draw(f.buffer_mut(), pic, NIGHT);
}

fn room(f: &mut Frame, app: &App, area: Rect, t: f32) {
    let area = area.inner(Margin::new(1, 0));
    let [head, pic, info, foot] = Layout::vertical([Length(3), Fill(1), Length(2), Length(1)]).areas(area);
    header(f, app, head, t);
    let mut cv = Canvas::new(pic.width, pic.height);
    scene::wall(&mut cv, false);
    let shown: Vec<(&str, bool)> = app.rows().iter().map(|s| (s.name.as_str(), s.set)).collect();
    let travel = app.travel.as_ref().map(|tr| (tr, tr.at.elapsed().as_secs_f32() / TRAVEL)).filter(|(_, p)| *p < 1.0);
    let flip = since(app.fun.flip, 1.0);
    let mut labels = Vec::new();
    if let Some((tr, p)) = travel {
        let old: Vec<(&str, bool)> = app.accounts[tr.from].secrets.iter().map(|s| (s.name.as_str(), s.set)).collect();
        let w = cv.w as f32;
        labels.extend(scene::shelf(&mut cv, (-tr.dir as f32 * p * w) as i32, &old, None, false, t));
        labels.extend(scene::shelf(&mut cv, (tr.dir as f32 * (1.0 - p) * w) as i32, &shown, None, false, t));
    } else {
        labels = scene::shelf(&mut cv, 0, &shown, Some(app.sel), flip.is_some(), t);
    }
    scene::floor(&mut cv);
    let rain = since(app.fun.rain, 3.2);
    let pose = if let Some((tr, _)) = travel {
        Pose::Walk { step: (t * 8.0) as i32 % 2 == 0, left: tr.dir < 0 }
    } else if app.asleep() {
        Pose::Sleep
    } else if let Some(age) = since(app.fun.woke, 0.7) {
        Pose::Startle(age / 0.7)
    } else if let Some(age) = flip {
        Pose::Flip(age)
    } else if rain.is_some() {
        Pose::Cheer(t * 8.0)
    } else {
        Pose::Stand { blink: t % 4.1 < 0.15, tip: t % 7.0 < 0.5 }
    };
    scene::host(&mut cv, &pose);
    if let Some(age) = rain {
        scene::rain(&mut cv, age);
    }
    // The slot machine of a copy, centred over the wall.
    let slot = app.spin.as_ref().map(|(_, at)| at.elapsed().as_secs_f32()).filter(|age| *age < scene::SLOT_FOR);
    let machine = Rect::new(pic.x + pic.width.saturating_sub(scene::SLOT_COLS) / 2 + 8, pic.y + pic.height.saturating_sub(scene::SLOT_ROWS) / 2, scene::SLOT_COLS, scene::SLOT_ROWS).intersection(pic);
    if let Some(age) = slot {
        let mut m = Canvas::new(machine.width, machine.height);
        scene::slot(&mut m, age, t);
        cv.blit(&m, i32::from(machine.x - pic.x), i32::from(machine.y - pic.y) * 2);
    }
    let buf = f.buffer_mut();
    cv.draw(buf, pic, NIGHT);
    for l in &labels {
        let hidden = slot.is_some() && machine.intersects(Rect::new(pic.x + l.col.max(0) as u16, pic.y + l.row.max(0) as u16, l.text.len() as u16, 1));
        if !hidden {
            cv.text(buf, pic, l.col, l.row, &l.text, l.fg, NIGHT);
        }
    }
    let ground = cv.h - FLOOR;
    let (lx, ly) = (i32::from(machine.x - pic.x), i32::from(machine.y - pic.y));
    match (&pose, slot) {
        (_, Some(age)) => {
            let (word, fg) = if age > scene::JACKPOT_AT { ("JACKPOT !", if (t * 6.0) as i32 % 2 == 0 { GOLD_PX } else { WHITE_PX }) } else { ("· hopla ·", CREAM_PX) };
            cv.text(buf, pic, lx + (i32::from(scene::SLOT_COLS) - 7 - word.chars().count() as i32) / 2, ly + 1, word, fg, NIGHT);
        }
        (Pose::Sleep, _) => {
            for k in 0..3 {
                let rise = (t * 0.6 + k as f32 / 3.0) % 1.0;
                let z = if k % 2 == 0 { "z" } else { "Z" };
                cv.text(buf, pic, 3 + LAPLA_W - 3 + (rise * 6.0) as i32, (ground - 38) / 2 - (rise * 5.0) as i32, z, SKY_PX, NIGHT);
            }
        }
        (Pose::Startle(_), _) => cv.text(buf, pic, 3 + LAPLA_W / 2, (ground - 46) / 2, "!", GOLD_PX, NIGHT),
        (Pose::Flip(_), _) => cv.text(buf, pic, 3, 1, "HOPLA !", GOLD_PX, NIGHT),
        _ => {}
    }
    details(f, app, info, t);
    keys(f, app, foot);
    dialog(f, app, pic);
}
