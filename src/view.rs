//! The screens: a compact palette that fits 80 x 24, and the lock.

use std::time::Instant;

use ratatui::Frame;
use ratatui::layout::{Constraint::*, Flex, Layout, Margin, Rect};
use ratatui::style::{Color, Style, Stylize};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, Paragraph};

use crate::palette::Item;
use crate::pixel::{Canvas, Rgb, color, hex, mix};
use crate::sprites::{self, Eyes, LAPLA_H, LAPLA_W, MINI_H, MINI_W};
use crate::state::{App, Ask, Form, FormKind, Lock, Mode, SHOWN, Tone};

const NIGHT: Rgb = hex(0x0c0c1e);
const BG: Color = color(NIGHT);
const SEL: Rgb = hex(0x1e1e44);
const FLASH: Rgb = hex(0x5a4a10);
const GOLD: Color = Color::Rgb(0xf8, 0xb8, 0x00);
const CREAM: Color = Color::Rgb(0xfc, 0xe0, 0xa8);
const WHITE: Color = Color::Rgb(0xfc, 0xfc, 0xfc);
const RED: Color = Color::Rgb(0xff, 0x50, 0x50);
const GREEN: Color = Color::Rgb(0x58, 0xd8, 0x54);
const SKY: Color = Color::Rgb(0x3c, 0xbc, 0xfc);
const DIM: Color = Color::Rgb(0x8c, 0x8c, 0xac);
const FAINT: Color = Color::Rgb(0x44, 0x44, 0x6c);
const SPIN: [char; 10] = ['⠋', '⠙', '⠹', '⠸', '⠼', '⠴', '⠦', '⠧', '⠇', '⠏'];
/// Lapla's corner, when the terminal is wide enough for it.
const GUTTER: u16 = 18;

fn spin(t: f32) -> char {
    SPIN[(t * 12.0) as usize % SPIN.len()]
}

fn age(at: Instant) -> f32 {
    at.elapsed().as_secs_f32()
}

/// Fades a colour in from `FAINT` over 0.12 s, `delay` seconds late.
fn fade(c: Color, since: f32, delay: f32) -> Color {
    let (Color::Rgb(r, g, b), Color::Rgb(fr, fg, fb)) = (c, FAINT) else { return c };
    let [r, g, b] = mix([fr, fg, fb], [r, g, b], (since - delay) / 0.12);
    Color::Rgb(r, g, b)
}

fn tone_color(tone: Tone) -> Color {
    match tone {
        Tone::Ok => GREEN,
        Tone::Info => SKY,
        Tone::Warn => GOLD,
        Tone::Crit => RED,
    }
}

pub fn draw(f: &mut Frame, app: &mut App) {
    let t = age(app.start);
    let area = f.area();
    f.render_widget(Block::new().style(Style::new().bg(BG).fg(WHITE)), area);
    if matches!(app.lock, Lock::Open) {
        palette(f, app, area.inner(Margin::new(1, 0)), t);
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
}

fn lock(f: &mut Frame, app: &App, area: Rect, t: f32) {
    let alarm = app.alarm.is_some_and(|at| age(at) < 4.0);
    let flash = (t * 4.0) as i32 % 2 == 0;
    let refused = app.note.as_ref().is_some_and(|n| n.1 == Tone::Crit);
    let (title, eyes) = match &app.lock {
        _ if alarm => ("ALARM !  ALARM !".fg(if flash { RED } else { WHITE }).bold(), Eyes::Open),
        Lock::Scan(..) => (format!("waiting for Touch ID {}", spin(t)).fg(CREAM), Eyes::Open),
        Lock::Done(_) => ("✓ unlocked".fg(GREEN).bold(), Eyes::Happy),
        _ if refused => ("still locked".fg(RED).bold(), Eyes::Shut),
        _ => ("hopla is locked".fg(GOLD).bold(), if t % 4.1 < 0.15 { Eyes::Shut } else { Eyes::Open }),
    };
    let mut lines = vec![Line::from(title)];
    if let Some((text, tone, _)) = &app.note {
        lines.push(Line::from(text.clone().fg(tone_color(*tone))));
    }
    lines.push(Line::from(vec!["↵".fg(GOLD), " unlock with Touch ID   ".fg(DIM), "esc".fg(GOLD), " quit".fg(DIM)]));
    let big = area.height >= (LAPLA_H / 2) as u16 + 8;
    let (w, h) = if big { (LAPLA_W, LAPLA_H) } else { (MINI_W, MINI_H) };
    let rows = (h / 2) as u16;
    let [pic, _, text] = Layout::vertical([Length(rows), Length(1), Length(lines.len() as u16)]).flex(Flex::Center).areas(area);
    let mut cv = Canvas::new(w as u16, rows);
    let sprite = if big { sprites::lapla(eyes) } else { sprites::mini(eyes) };
    // Hides when the alarm rings: only his ears and hat peek out.
    let sink = if alarm { h / 2 } else { 0 };
    let shake = i32::from(refused) * ((t * 20.0) as i32 % 2);
    cv.sprite(shake, sink, &sprite, false, 0);
    let x = pic.x + pic.width.saturating_sub(w as u16) / 2;
    cv.draw(f.buffer_mut(), Rect { x, width: w as u16, ..pic }, NIGHT);
    f.render_widget(Paragraph::new(lines).centered(), text);
}

fn palette(f: &mut Frame, app: &App, area: Rect, t: f32) {
    let [head, query, rule, body, note, keys] =
        Layout::vertical([Length(1), Length(1), Length(1), Fill(1), Length(1), Length(1)]).areas(area);
    header(f, app, head, t);
    let items = app.items();
    let caret = if (t * 2.5) as i32 % 2 == 0 { "▌" } else { " " };
    f.render_widget(Paragraph::new(Line::from(vec!["❯ ".fg(GOLD).bold(), app.query.clone().fg(WHITE), caret.fg(GOLD)])), query);
    f.render_widget(Paragraph::new(format!("{} rows", items.len()).fg(FAINT)).right_aligned(), query);
    f.render_widget(Paragraph::new("─".repeat(rule.width.into()).fg(FAINT)), rule);
    let wide = body.width >= 72 && body.height >= (MINI_H / 2) as u16 + 3;
    let [list, corner] = Layout::horizontal([Fill(1), Length(if wide { GUTTER } else { 0 })]).areas(body);
    match &app.mode {
        Mode::Form(form) => form_box(f, form, list, t),
        Mode::Status(lines) => status_box(f, lines, list),
        _ => rows(f, app, &items, list),
    }
    if wide {
        lapla(f, app, corner, t);
    }
    footer(f, app, items.get(app.sel), note, keys, t);
}

fn header(f: &mut Frame, app: &App, area: Rect, t: f32) {
    let mut left = vec![" hopla ".fg(Color::Black).bg(GOLD).bold(), "  ".into()];
    if let Some(active) = &app.active {
        left.extend(["● ".fg(GREEN), active.clone().fg(WHITE).bold(), " active".fg(DIM)]);
    }
    f.render_widget(Paragraph::new(Line::from(left)), area);
    let mut right: Vec<Span> = Vec::new();
    if let Some(busy) = &app.busy {
        right.push(format!("{} {busy}   ", spin(t)).fg(CREAM));
    }
    if let Some(clip) = &app.clip {
        let left = SHOWN.saturating_sub(clip.at.elapsed()).as_secs() + 1;
        right.push(format!("⎘ {} · {left} s", clip.name).fg(SKY));
    }
    f.render_widget(Paragraph::new(Line::from(right)).right_aligned(), area);
}

fn rows(f: &mut Frame, app: &App, items: &[Item], area: Rect) {
    let height = usize::from(area.height);
    let first = app.sel.saturating_sub(height.saturating_sub(1));
    let since = age(app.typed);
    let grouped = app.query.is_empty();
    let mut lines = Vec::new();
    for (i, item) in items.iter().enumerate().skip(first).take(height) {
        // Rows fade in one after the other when the query changes.
        let delay = (i - first) as f32 * 0.012;
        let (left, right): (Vec<Span>, &str) = match item {
            Item::Account { name, active, secrets } => {
                let (mark, tint) = if *active { ("● ".fg(GREEN), GOLD) } else { ("○ ".fg(DIM), CREAM) };
                let hint = if *active { "active" } else { "↵ switch" };
                let count = format!("  {secrets} secrets").fg(FAINT);
                (vec![mark, name.clone().fg(fade(tint, since, delay)).bold(), count], hint)
            }
            Item::Secret { account, name, set } => {
                let (mark, tint, hint) = match set {
                    Some(true) => ("▣ ", WHITE, "↵ copy"),
                    Some(false) => ("✕ ", RED, "empty · ↵ fill"),
                    None => ("· ", DIM, "checking"),
                };
                let mut left = vec![(if grouped { "   " } else { "" }).into(), mark.fg(fade(tint, since, delay)), name.clone().fg(fade(WHITE, since, delay))];
                if !grouped {
                    left.push(format!("  {account}").fg(DIM));
                }
                (left, hint)
            }
            Item::Action(a) => (vec!["› ".fg(SKY), a.label().fg(fade(SKY, since, delay))], "↵ run"),
        };
        let selected = i == app.sel;
        let used = 2 + left.iter().map(|s| s.content.chars().count()).sum::<usize>() + right.chars().count();
        let mut spans = vec![(if selected { "▌ " } else { "  " }).fg(GOLD)];
        spans.extend(left);
        spans.push(" ".repeat(usize::from(area.width).saturating_sub(used)).into());
        spans.push(right.fg(if selected { CREAM } else { FAINT }));
        let mut line = Line::from(spans);
        if selected {
            // A short gold flash where the selection lands.
            line = line.bg(color(mix(FLASH, SEL, age(app.picked) / 0.15)));
        }
        lines.push(line);
    }
    if items.is_empty() {
        let empty = if app.busy.is_some() { "reading hop" } else { "nothing matches · esc clears" };
        lines.push(Line::from(empty.fg(DIM)));
    }
    f.render_widget(Paragraph::new(lines), area);
}

fn form_box(f: &mut Frame, form: &Form, area: Rect, t: f32) {
    let title = match &form.kind {
        FormKind::Secret { account, name: Some(name) } => format!("new value for {name} in {account}"),
        FormKind::Secret { account, name: None } => format!("new secret in {account}"),
        FormKind::Account => "new account".into(),
    };
    let caret = if (t * 2.5) as i32 % 2 == 0 { "▌" } else { " " };
    let mut lines = vec![Line::from(title.fg(GOLD).bold()), Line::default()];
    for (i, field) in form.fields.iter().enumerate() {
        let here = i == form.at;
        let shown = if field.hidden { "*".repeat(field.value.chars().count()) } else { field.value.clone() };
        lines.push(Line::from(vec![
            (if here { "▌ " } else { "  " }).fg(GOLD),
            format!("{:<14}", field.label).fg(if here { CREAM } else { DIM }),
            shown.fg(if field.hidden { GOLD } else { WHITE }),
            (if here { caret } else { "" }).fg(GOLD),
        ]));
    }
    lines.push(Line::default());
    let note: &[&str] = match form.kind {
        FormKind::Account => &[
            "saves it in hop's .env, makes the SSH key if missing, runs install.sh,",
            "copies the public key for GitHub, then runs gh auth login",
        ],
        FormKind::Secret { .. } => &["typed blind, stored in the Keychain like hop secret set"],
    };
    lines.extend(note.iter().map(|l| Line::from(l.fg(FAINT))));
    f.render_widget(Paragraph::new(lines), area);
}

fn status_box(f: &mut Frame, status: &[String], area: Rect) {
    let mut lines = vec![Line::from("hop s".fg(GOLD).bold()), Line::default()];
    for l in status {
        let tint = if l.starts_with("OK") { GREEN } else if l.starts_with("WARNING") { GOLD } else { WHITE };
        lines.push(Line::from(l.clone().fg(tint)));
    }
    f.render_widget(Paragraph::new(lines), area);
}

/// Lapla in his corner, reacting to what happens.
fn lapla(f: &mut Frame, app: &App, area: Rect, t: f32) {
    let note = app.note.as_ref().filter(|n| age(n.2) < 1.5);
    let typing_secret = matches!(&app.mode, Mode::Form(form) if form.fields[form.at].hidden);
    let (eyes, says, hop, flip) = if app.query.eq_ignore_ascii_case("hopla") {
        (Eyes::Happy, "hopla !", ((t * 8.0).sin().abs() * 3.0) as i32, (t * 4.0) as i32 % 2 == 0)
    } else if typing_secret {
        (Eyes::Shut, "not looking", 0, false)
    } else if app.asleep() {
        (Eyes::Shut, "z z z", 0, false)
    } else if matches!(app.mode, Mode::Ask(_) | Mode::Gate(..)) {
        (Eyes::Open, "sure ?", 0, false)
    } else if app.busy.is_some() {
        (Eyes::Open, "on it", 0, false)
    } else if let Some((_, tone, at)) = note {
        let bounce = ((1.0 - age(*at) / 0.4).max(0.0) * 3.0) as i32;
        match tone {
            Tone::Ok => (Eyes::Happy, "done !", bounce, false),
            Tone::Crit => (Eyes::Shut, "oops", 0, false),
            Tone::Warn => (Eyes::Open, "hmm", 0, false),
            Tone::Info => (Eyes::Open, "noted", 0, false),
        }
    } else {
        (if t % 4.1 < 0.15 { Eyes::Shut } else { Eyes::Open }, "", 0, false)
    };
    let rows = (MINI_H / 2) as u16 + 2;
    let pic = Rect { y: area.bottom().saturating_sub(rows), height: rows.min(area.height), ..area };
    let mut cv = Canvas::new(pic.width, pic.height);
    cv.sprite((i32::from(pic.width) - MINI_W) / 2, 4 - hop, &sprites::mini(eyes), flip, 0);
    cv.draw(f.buffer_mut(), pic, NIGHT);
    if !says.is_empty() {
        f.render_widget(Paragraph::new(says.fg(CREAM)).centered(), Rect { height: 1, ..pic });
    }
}

fn footer(f: &mut Frame, app: &App, selected: Option<&Item>, note: Rect, keys: Rect, t: f32) {
    let line = match &app.mode {
        Mode::Ask(ask) => {
            let what = match ask {
                Ask::Secret(account, name) => format!("delete {name} from {account} ?"),
                Ask::Account(account) => format!("remove account {account} ? its secrets stay in the Keychain"),
            };
            Line::from(vec![what.fg(RED).bold(), "  ↵ yes, with Touch ID · any key no".fg(DIM)])
        }
        Mode::Gate(..) => Line::from(format!("waiting for Touch ID {}", spin(t)).fg(CREAM)),
        _ => app.note.as_ref().map(|(text, tone, _)| Line::from(text.clone().fg(tone_color(*tone)))).unwrap_or_default(),
    };
    f.render_widget(Paragraph::new(line), note);
    let pairs: &[(&str, &str)] = match (&app.mode, selected) {
        (Mode::Form(_), _) => &[("tab", "next"), ("↵", "save"), ("esc", "cancel")],
        (Mode::Status(_), _) => &[("any key", "closes")],
        (_, Some(Item::Secret { set: Some(true), .. })) => &[("↵", "copy"), ("^e", "replace"), ("^d", "delete"), ("^b", "switch here"), ("^n", "new"), ("^l", "lock"), ("esc", "quit")],
        (_, Some(Item::Secret { .. })) => &[("↵", "fill"), ("^d", "delete"), ("^n", "new"), ("^l", "lock"), ("esc", "quit")],
        (_, Some(Item::Account { active: false, .. })) => &[("↵", "switch"), ("^d", "remove"), ("^n", "new secret"), ("^a", "new account"), ("^l", "lock"), ("esc", "quit")],
        (_, Some(Item::Account { .. })) => &[("↵", "status"), ("^n", "new secret"), ("^a", "new account"), ("^l", "lock"), ("esc", "quit")],
        _ => &[("↵", "run"), ("^a", "new account"), ("^l", "lock"), ("esc", "quit")],
    };
    let hints: Line = pairs.iter().flat_map(|(k, what)| [k.fg(GOLD), format!(" {what}  ").fg(DIM)]).collect();
    f.render_widget(Paragraph::new(hints), keys);
}
