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
            cv.text(buf, pic, l.col, l.row, &l.text, l.fg);
        }
    }
    let ground = cv.h - FLOOR;
    let (lx, ly) = (i32::from(machine.x - pic.x), i32::from(machine.y - pic.y));
    match (&pose, slot) {
        (_, Some(age)) => {
            let (word, fg) = if age > scene::JACKPOT_AT { ("JACKPOT !", if (t * 6.0) as i32 % 2 == 0 { GOLD_PX } else { WHITE_PX }) } else { ("· hopla ·", CREAM_PX) };
            cv.text(buf, pic, lx + (i32::from(scene::SLOT_COLS) - 7 - word.chars().count() as i32) / 2, ly + 1, word, fg);
        }
        (Pose::Sleep, _) => {
            for k in 0..3 {
                let rise = (t * 0.6 + k as f32 / 3.0) % 1.0;
                let z = if k % 2 == 0 { "z" } else { "Z" };
                cv.text(buf, pic, 3 + LAPLA_W - 3 + (rise * 6.0) as i32, (ground - 38) / 2 - (rise * 5.0) as i32, z, SKY_PX);
            }
        }
        (Pose::Startle(_), _) => cv.text(buf, pic, 3 + LAPLA_W / 2, (ground - 46) / 2, "!", GOLD_PX),
        (Pose::Flip(_), _) => cv.text(buf, pic, 3, 1, "HOPLA !", GOLD_PX),
        _ => {}
    }
    details(f, app, info, t);
    keys(f, app, foot);
    dialog(f, app, pic);
}

fn header(f: &mut Frame, app: &App, area: Rect, t: f32) {
    let logo: Vec<Line> = LOGO
        .iter()
        .enumerate()
        .map(|(r, text)| {
            let wave = |i: usize| ((t * 3.0 - i as f32 * 0.4 + r as f32).sin() + 1.0) / 2.0;
            text.chars().map(|ch| ch.to_string()).enumerate().map(|(i, ch)| ch.fg(if wave(i) > 0.5 { GOLD } else { RED })).collect()
        })
        .collect();
    f.render_widget(Paragraph::new(logo), area);
    let mut rooms = vec!["◀ ".fg(DIM)];
    for (i, a) in app.accounts.iter().enumerate() {
        rooms.push(if i == app.account { format!(" ◆ {} ◆ ", a.name.to_uppercase()).fg(GOLD).bold() } else { format!("  {}  ", a.name).fg(DIM) });
    }
    rooms.push(" ▶".fg(DIM));
    let mut chips: Vec<Span> = Vec::new();
    if let Some(clip) = &app.clip {
        let left = SHOWN.saturating_sub(clip.at.elapsed()).as_secs() + 1;
        chips.push(format!("⎘ {} · {left} s   ", clip.name).fg(SKY));
    }
    if matches!(app.mode, Mode::Filter) || !app.filter.is_empty() {
        let caret = if matches!(app.mode, Mode::Filter) && (t * 4.0) as i32 % 2 == 0 { "▌" } else { " " };
        chips.extend(["/ ".fg(GOLD), app.filter.clone().fg(WHITE), caret.fg(GOLD), format!("  {} found   ", app.rows().len()).fg(DIM)]);
    }
    chips.push("unlocked".fg(GREEN));
    f.render_widget(Paragraph::new(vec![Line::from(rooms).right_aligned(), Line::from(chips).right_aligned()]), area);
    // A gold light running along the rule under the header.
    let rule = Rect { y: area.y + 2, height: 1, ..area };
    let pos = (t * 30.0) as u16 % rule.width.max(1);
    let line: Line = (0..rule.width).map(|x| "▀".fg(if x.abs_diff(pos) < 4 { GOLD } else { FAINT })).collect();
    f.render_widget(Paragraph::new(line), rule);
}

fn details(f: &mut Frame, app: &App, area: Rect, t: f32) {
    let room = app.account().map_or("", |a| a.name.as_str());
    let lines = match app.selected() {
        _ if app.loading => vec![Line::from(format!("reading the Keychain {}", spin(t)).fg(DIM))],
        _ if app.asleep() => vec![Line::from("Lapla is asleep on the cash · any key wakes him".fg(DIM))],
        None if !app.filter.is_empty() => vec![Line::from("nothing matches · Esc clears the search".fg(DIM))],
        None => vec![Line::from(format!("no box in room {room} · n adds one").fg(DIM))],
        Some(s) => {
            let (mark, state) = if s.set { ("▣ ".fg(GOLD), "  in the vault · y copies it, never shows it") } else { ("✕ ".fg(RED), "  empty box · Enter puts a value in it") };
            vec![
                Line::from(vec![mark, s.name.clone().fg(WHITE).bold(), state.fg(DIM)]),
                Line::from(format!("room {room} · {}", crate::vault::service(room, &s.name)).fg(FAINT)),
            ]
        }
    };
    f.render_widget(Paragraph::new(lines), area);
}

fn keys(f: &mut Frame, app: &App, area: Rect) {
    let keys = [("← →", "room"), ("↑ ↓", "box"), ("y", "copy"), ("⏎", "deposit"), ("n", "new"), ("d", "empty"), ("/", "find"), ("L", "lock"), ("q", "quit")];
    let hints: Line = keys.iter().flat_map(|(k, what)| [k.fg(GOLD), format!(" {what}   ").fg(DIM)]).collect();
    f.render_widget(Paragraph::new(hints), area);
    let total: usize = app.accounts.iter().map(|a| a.secrets.len()).sum();
    let count = format!("{total} boxes · {} rooms", app.accounts.len());
    f.render_widget(Paragraph::new(count.fg(FAINT)).right_aligned(), area);
}

/// The deposit and empty dialogs, an 8-bit text box over the room.
fn dialog(f: &mut Frame, app: &App, area: Rect) {
    let room = app.account().map_or("", |a| a.name.as_str());
    let caret = "▌".fg(GOLD);
    let (title, lines, hint) = match &app.mode {
        Mode::Deposit { name, naming: true, .. } => (
            format!(" NEW BOX · room {room} "),
            vec![Line::from(vec!["name   ".fg(DIM), name.clone().fg(WHITE).bold(), caret])],
            "Enter next · Esc cancels",
        ),
        Mode::Deposit { name, value, .. } => (
            format!(" DEPOSIT · {name} "),
            vec![
                Line::from(vec!["value  ".fg(DIM), "*".repeat(value.chars().count()).fg(GOLD), caret]),
                Line::from("typed blind, never shown".fg(FAINT)),
            ],
            "Enter stores · Esc cancels",
        ),
        Mode::Confirm => (
            format!(" EMPTY {} ? ", app.selected().map_or("", |s| s.name.as_str())),
            vec![Line::from(format!("its value leaves the Keychain of room {room}").fg(DIM))],
            "y empties · any other key keeps it",
        ),
        _ => return,
    };
    let mut body = vec![Line::default()];
    body.extend(lines);
    body.extend([Line::default(), Line::from(hint.fg(DIM))]);
    let (w, h) = (52.min(area.width), (body.len() as u16 + 2).min(area.height));
    let r = Rect::new(area.x + (area.width - w) / 2, area.y + (area.height - h) / 2, w, h);
    let frame = Block::bordered().border_type(BorderType::Double).border_style(WHITE).title(title.fg(GOLD).bold()).style(Style::new().bg(Color::Black));
    f.render_widget(Clear, r);
    f.render_widget(Paragraph::new(body).block(frame.padding(ratatui::widgets::Padding::horizontal(2))), r);
}

fn note(f: &mut Frame, app: &App, area: Rect) {
    let Some((text, tone, born)) = &app.note else { return };
    let (mark, tint) = match tone {
        Tone::Ok => ("✓", GREEN),
        Tone::Info => ("i", SKY),
        Tone::Warn => ("!", GOLD),
        Tone::Crit => ("✗", RED),
    };
    let width = (text.chars().count() as u16 + 6).min(area.width.saturating_sub(4));
    // Slides in from the right edge for 0.3 s.
    let slide = ((1.0 - born.elapsed().as_secs_f32() / 0.3).max(0.0) * f32::from(width)) as u16;
    let x = area.right().saturating_sub(width + 2) + slide;
    let r = Rect::new(x, area.bottom().saturating_sub(6), width - slide, 3).intersection(area);
    let frame = Block::bordered().border_type(BorderType::Double).border_style(tint).style(Style::new().bg(Color::Black));
    f.render_widget(Clear, r);
    f.render_widget(Paragraph::new(Line::from(vec![format!(" {mark} ").fg(tint), text.clone().fg(WHITE)])).block(frame), r);
}
