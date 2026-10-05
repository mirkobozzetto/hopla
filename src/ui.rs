//! Drawing, in the forest palette of draft/hop-ui.html.

use std::time::Instant;

use ratatui::Frame;
use ratatui::layout::{Constraint::*, Flex, Layout, Margin, Rect};
use ratatui::style::{Color, Style, Stylize};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, BorderType, Clear, Padding, Paragraph};

use crate::app::{App, Focus, Lock, SHOWN, Tone};
use crate::vault::{self, Secret};

const BG: Color = Color::Rgb(0x0b, 0x0f, 0x0d);
const PANEL: Color = Color::Rgb(0x0f, 0x15, 0x12);
const BORDER: Color = Color::Rgb(0x22, 0x30, 0x2a);
const TEXT: Color = Color::Rgb(0xd6, 0xe4, 0xdc);
const DIM: Color = Color::Rgb(0x71, 0x85, 0x7b);
const FAINT: Color = Color::Rgb(0x34, 0x44, 0x3c);
const ACCENT: Color = Color::Rgb(0x3d, 0xdc, 0x97);
const ACCENT2: Color = Color::Rgb(0xa7, 0x8b, 0xfa);
const WARN: Color = Color::Rgb(0xff, 0xb4, 0x54);
const CRIT: Color = Color::Rgb(0xff, 0x5c, 0x7a);
const INFO: Color = Color::Rgb(0x5c, 0xcf, 0xe6);
const SEL: Color = Color::Rgb(0x15, 0x22, 0x1c);
const SPIN: [char; 10] = ['⠋', '⠙', '⠹', '⠸', '⠼', '⠴', '⠦', '⠧', '⠇', '⠏'];
const LOGO: [&str; 2] = ["█▄▄ ▄▀▄ █▀▄ █   ▄▀▄", "█ █ ▀▄▀ █▀  █▄▄ █▀█"];
const FUR: Color = Color::Rgb(0xf4, 0xef, 0xe4);
const PINK: Color = Color::Rgb(0xff, 0x9f, 0xb8);
const FINGERPRINT: [&str; 9] = [
    "    ▄▄▀▀▀▀▀▀▄▄    ",
    "  ▄▀  ▄▀▀▀▀▄  ▀▄  ",
    " █  ▄▀ ▄▀▀▄ ▀▄  █ ",
    "█  █  █ ▄▄ █  █  █",
    "█  █  █ █  █  █  █",
    "█  ▀▄ ▀▄▀  █  █  █",
    " █  ▀▄    ▄▀ ▄▀  █ ",
    "  ▀▄  ▀▀▀▀ ▄▀  ▄▀ ",
    "    ▀▀▄▄▄▄▀▀▀▀    ",
];
const MASK: &str = "••••••••••••";
const GLYPHS: [char; 18] = ['▓', '▒', '░', '#', '%', '&', '@', '$', '*', '+', '=', '<', '>', '?', '/', '|', '§', '¤'];

fn pulse(t: f32, speed: f32) -> f32 {
    ((t * speed).sin() + 1.0) / 2.0
}

fn mix(a: Color, b: Color, p: f32) -> Color {
    let (Color::Rgb(r1, g1, b1), Color::Rgb(r2, g2, b2)) = (a, b) else { return a };
    let m = |x: u8, y: u8| (f32::from(x) + (f32::from(y) - f32::from(x)) * p.clamp(0.0, 1.0)).round() as u8;
    Color::Rgb(m(r1, r2), m(g1, g2), m(b1, b2))
}

fn spin(t: f32) -> char {
    SPIN[(t * 12.0) as usize % SPIN.len()]
}

fn scramble(i: usize, frame: usize) -> char {
    let h = (i as u32).wrapping_mul(0x9e37_79b9) ^ (frame as u32).wrapping_mul(0x85eb_ca6b);
    GLYPHS[((h ^ (h >> 13)).wrapping_mul(0x27d4_eb2f) >> 16) as usize % GLYPHS.len()]
}

/// The value cell: masked, being read, or decoding glyph by glyph.
fn value(app: &App, account: &str, s: &Secret, t: f32) -> Vec<Span<'static>> {
    let this = |a: &str, n: &str| a == account && n == s.name;
    if let Some(r) = app.reveal.as_ref().filter(|r| this(&r.account, &r.name)) {
        let age = r.at.elapsed().as_secs_f32();
        return r
            .value
            .chars()
            .enumerate()
            .map(|(i, c)| {
                let at = 0.25 + i as f32 * 0.02;
                if age < at {
                    Span::from(scramble(i, (t * 26.0) as usize).to_string()).fg(ACCENT2)
                } else if age < at + 0.15 {
                    Span::from(c.to_string()).fg(ACCENT).bold()
                } else {
                    Span::from(c.to_string()).fg(TEXT)
                }
            })
            .collect();
    }
    if app.reading.as_ref().is_some_and(|(a, n)| this(a, n)) {
        return vec![format!("reading the Keychain {}", spin(t)).fg(DIM)];
    }
    vec![if s.set { MASK.fg(DIM) } else { "-".fg(FAINT) }]
}

fn panel<'a>(title: Span<'a>, border: Color) -> Block<'a> {
    Block::bordered()
        .border_type(BorderType::Rounded)
        .border_style(border)
        .title(Line::from(vec![" ".into(), title, " ".into()]))
        .style(Style::new().bg(PANEL))
        .padding(Padding::horizontal(1))
}

/// A row whose selection background spans the whole width.
fn row(mut spans: Vec<Span<'static>>, selected: bool, width: u16) -> Line<'static> {
    if !selected {
        return Line::from(spans);
    }
    spans.push(" ".repeat(width.into()).into());
    Line::from(spans).bg(SEL)
}

pub fn draw(f: &mut Frame, app: &mut App) {
    let t = app.start.elapsed().as_secs_f32();
    let area = f.area();
    f.render_widget(Block::new().style(Style::new().bg(BG).fg(TEXT)), area);
    if matches!(app.lock, Lock::Open) {
        vault(f, app, area.inner(Margin::new(2, 1)), t);
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

/// Lapla, the hop bunny: hops, blinks, watches the scan, cheers.
fn lapla(app: &App, t: f32) -> Vec<Line<'static>> {
    let refused = app.note.as_ref().is_some_and(|n| matches!(n.1, Tone::Crit));
    let (eyes, says) = match app.lock {
        Lock::Done(_) => ("^   ^", "hopla!"),
        Lock::Scan(..) => ("o   o", "sniff sniff..."),
        _ if refused => (";   ;", "no keys, then"),
        _ if t % 4.1 < 0.15 => ("-   -", ""),
        _ => ("•   •", ""),
    };
    // Every art line is 13 cells, framed by two 16-cell margins so the
    // bunny stays centred whatever the bubble says.
    let side = " ".repeat(16);
    let art = |spans: Vec<Span<'static>>, bubble: &str| {
        let mut line = vec![side.clone().into()];
        line.extend(spans);
        line.push(format!("{bubble:<16}").fg(ACCENT2));
        Line::from(line)
    };
    let mut lines = vec![
        art(vec!["  (\\     /)  ".fg(FUR)], ""),
        art(vec!["   \\\\   //   ".fg(FUR)], ""),
        art(vec!["  ( ".fg(FUR), eyes.fg(ACCENT), " )  ".fg(FUR)], says),
        art(vec![" =(   ".fg(FUR), "w".fg(PINK), "   )= ".fg(FUR)], ""),
        art(vec!["   (\")_(\")   ".fg(FUR)], ""),
    ];
    let hop = matches!(app.lock, Lock::Done(_)) || t % 2.6 < 0.3;
    lines.insert(if hop { 5 } else { 0 }, Line::default());
    lines.push(Line::from("─────────────".fg(FAINT)));
    lines
}

fn lock(f: &mut Frame, app: &App, area: Rect, t: f32) {
    let done = matches!(app.lock, Lock::Done(_));
    let scan = match &app.lock {
        Lock::Scan(at, _) => Some(at.elapsed().as_secs_f32() / 1.2 % 1.0),
        _ => None,
    };
    let n = (FINGERPRINT.len() - 1) as f32;
    let print = FINGERPRINT.iter().enumerate().map(|(r, row)| {
        let level = (n - r as f32) / n;
        let color = match scan {
            _ if done => ACCENT,
            Some(p) if (level - p).abs() < 0.1 => Color::White,
            Some(p) if level <= p => ACCENT,
            _ => mix(FAINT, DIM, pulse(t - r as f32 * 0.12, 2.2)),
        };
        Line::from(row.fg(color))
    });
    let title = if done {
        "✓ unlocked".fg(ACCENT)
    } else if scan.is_some() {
        format!("reading fingerprint {}", spin(t)).into()
    } else {
        "hop is locked".into()
    };
    let hint = if scan.is_none() && !done {
        Line::from(vec!["Touch ID · ".fg(DIM), "Enter".fg(ACCENT), "   q quits".fg(FAINT)])
    } else {
        Line::default()
    };
    let mut lines = if area.height >= 24 { lapla(app, t) } else { Vec::new() };
    lines.push(Line::default());
    lines.extend(print);
    lines.extend([Line::default(), Line::from(title.bold()), hint]);
    let [mid] = Layout::vertical([Length(lines.len() as u16)]).flex(Flex::Center).areas(area);
    f.render_widget(Paragraph::new(lines).centered(), mid);
}

fn vault(f: &mut Frame, app: &App, area: Rect, t: f32) {
    let [head, scan, main, foot] = Layout::vertical([Length(2), Length(2), Fill(1), Length(1)]).areas(area);
    let logo: Vec<Line> = LOGO
        .iter()
        .enumerate()
        .map(|(r, text)| {
            let wave = |i: usize| pulse(t * 1.4 - i as f32 * 0.5 + r as f32 * 0.9, 1.0);
            text.chars().enumerate().map(|(i, c)| Span::from(c.to_string()).fg(mix(ACCENT, ACCENT2, wave(i)))).collect()
        })
        .collect();
    f.render_widget(Paragraph::new(logo), head);
    let clip = app.clip.as_ref().map(|c| {
        let left = SHOWN.saturating_sub(c.at.elapsed()).as_secs() + 1;
        format!("⎘ {} · {left} s   ", c.name).fg(INFO)
    });
    let chips = Line::from(vec![
        clip.unwrap_or_default(),
        "● ".fg(ACCENT),
        app.account().map_or("", |a| a.name.as_str()).into(),
        "  unlocked".fg(DIM),
    ]);
    f.render_widget(Paragraph::new(vec![Line::default(), chips.right_aligned()]), head);

    let pos = (t * 22.0) % (f32::from(scan.width) + 24.0) - 12.0;
    let sweep: Line = (0..scan.width)
        .map(|x| Span::from("─").fg(mix(BORDER, ACCENT, 1.0 - (f32::from(x) - pos).abs() / 12.0)))
        .collect();
    f.render_widget(Paragraph::new(sweep), scan);

    let [left, mid, right] = Layout::horizontal([Length(26), Fill(3), Fill(2)]).spacing(2).areas(main);
    accounts(f, app, left);
    secrets(f, app, mid, t);
    detail(f, app, right, t);

    let keys = [("j/k", "move"), ("Tab", "panel"), ("/", "filter"), ("r", "reveal"), ("y", "copy"), ("L", "lock"), ("q", "quit")];
    let hints: Line = keys.iter().flat_map(|(k, what)| [k.fg(ACCENT), format!(" {what}   ").fg(DIM)]).collect();
    f.render_widget(Paragraph::new(hints), foot);
    let total: usize = app.accounts.iter().map(|a| a.secrets.len()).sum();
    let count = format!("{total} secrets · {} accounts", app.accounts.len());
    f.render_widget(Paragraph::new(count.fg(DIM)).right_aligned(), foot);
}

fn accounts(f: &mut Frame, app: &App, area: Rect) {
    let focus = app.focus == Focus::Accounts;
    let block = panel("accounts".fg(if focus { ACCENT } else { DIM }), if focus { ACCENT } else { BORDER });
    let inner = block.inner(area);
    let width = usize::from(inner.width).saturating_sub(7);
    let mut lines: Vec<Line> = app
        .accounts
        .iter()
        .enumerate()
        .map(|(i, a)| {
            let selected = i == app.account;
            let active = app.active.as_ref() == Some(&a.name);
            let spans = vec![
                (if selected { "▌" } else { " " }).fg(ACCENT),
                (if active { "● " } else { "○ " }).fg(if active { ACCENT } else { DIM }),
                format!("{:<width$.width$}", a.name).into(),
                format!("{:>3}", a.secrets.len()).fg(DIM),
            ];
            row(spans, selected, inner.width)
        })
        .collect();
    lines.extend([Line::default(), Line::from("● active account".fg(FAINT))]);
    f.render_widget(Paragraph::new(lines).block(block), area);
}

/// The name with the filter match lit, padded to `width`.
fn lit(name: &str, filter: &str, width: usize) -> Vec<Span<'static>> {
    let pad = " ".repeat(width.saturating_sub(name.len()));
    // Names are ASCII, so lowercasing keeps byte offsets.
    let Some(at) = name.to_lowercase().find(&filter.to_lowercase()).filter(|_| !filter.is_empty()) else {
        return vec![name.to_owned().bold(), pad.into()];
    };
    let end = at + filter.len();
    vec![name[..at].to_owned().bold(), name[at..end].to_owned().fg(BG).bg(ACCENT), name[end..].to_owned().bold(), pad.into()]
}

fn secrets(f: &mut Frame, app: &App, area: Rect, t: f32) {
    let focus = app.focus == Focus::Secrets;
    let name = app.account().map_or("", |a| a.name.as_str());
    let block = panel(format!("secrets · {name}").fg(if focus { ACCENT } else { DIM }), if focus { ACCENT } else { BORDER });
    let inner = block.inner(area);
    f.render_widget(block, area);
    let rows = app.rows();
    let mut lines = Vec::new();
    if app.filtering || !app.filter.is_empty() {
        let caret = if app.filtering && pulse(t, 8.0) > 0.5 { "▌" } else { " " };
        let found = format!("  {} match", rows.len());
        lines.push(Line::from(vec!["/ ".fg(ACCENT), app.filter.clone().into(), caret.fg(ACCENT), found.fg(DIM)]));
    }
    let width = rows.iter().map(|s| s.name.len()).max().unwrap_or(4).clamp(4, 32) + 2;
    lines.push(Line::from(format!("    {:<width$}{:<9}VALUE", "NAME", "STATE").fg(FAINT)));
    let room = usize::from(inner.height).saturating_sub(lines.len()).max(1);
    let first = app.sel.saturating_sub(room - 1);
    for (i, s) in rows.iter().enumerate().skip(first).take(room) {
        let selected = i == app.sel;
        let mut spans = vec![(if selected { "▌ " } else { "  " }).fg(ACCENT), "● ".fg(if s.set { ACCENT } else { WARN })];
        spans.extend(lit(&s.name, &app.filter, width));
        spans.push(format!("{:<9}", if s.set { "set" } else { "missing" }).fg(if s.set { DIM } else { WARN }));
        spans.extend(value(app, name, s, t));
        lines.push(row(spans, selected, inner.width));
    }
    let empty = if app.loading {
        format!("reading the Keychain {}", spin(t))
    } else if !app.filter.is_empty() {
        "nothing matches · Esc clears".into()
    } else {
        format!("no secrets · hop secret set NAME {name}")
    };
    if rows.is_empty() {
        lines.push(Line::from(empty.fg(DIM)));
    }
    f.render_widget(Paragraph::new(lines), inner);
}

fn detail(f: &mut Frame, app: &App, area: Rect, t: f32) {
    let (Some(account), Some(s)) = (app.account(), app.selected()) else {
        return f.render_widget(panel("detail".fg(DIM), BORDER), area);
    };
    let block = panel(s.name.clone().fg(ACCENT2), BORDER);
    let inner = block.inner(area);
    f.render_widget(block, area);
    let [info, label, vbox, hint] = Layout::vertical([Length(4), Length(1), Length(3), Length(1)]).areas(inner);
    let state = if s.set { "● set".fg(ACCENT) } else { "● missing".fg(WARN) };
    f.render_widget(
        Paragraph::new(vec![
            Line::from(vec!["account  ".fg(DIM), account.name.clone().into()]),
            Line::from(vec!["service  ".fg(DIM), vault::service(&account.name, &s.name).into()]),
            Line::from(vec!["state    ".fg(DIM), state]),
        ]),
        info,
    );
    f.render_widget(Paragraph::new("V A L U E".fg(DIM)), label);
    let shown = if s.set {
        Line::from(value(app, &account.name, s, t))
    } else {
        Line::from(format!("hop secret set {} {}", s.name, account.name).fg(ACCENT))
    };
    let frame = Block::bordered().border_type(BorderType::Rounded).border_style(BORDER).padding(Padding::horizontal(1));
    f.render_widget(Paragraph::new(shown).block(frame), vbox);
    let revealed = app.reveal.as_ref().filter(|r| r.account == account.name && r.name == s.name);
    let keys: Line = if let Some(r) = revealed {
        let left = SHOWN.saturating_sub(r.at.elapsed());
        let full = (left.as_secs_f32() / SHOWN.as_secs_f32() * 24.0).round() as usize;
        let hides = format!("hides in {} s ", left.as_secs() + 1);
        Line::from(vec![hides.fg(DIM), "█".repeat(full).fg(ACCENT2), "─".repeat(24 - full).fg(FAINT)])
    } else if s.set {
        Line::from(vec!["r".fg(ACCENT), " reveal   ".fg(DIM), "y".fg(ACCENT), " copy".fg(DIM)])
    } else {
        Line::from("not stored yet: run the command above".fg(DIM))
    };
    f.render_widget(Paragraph::new(keys), hint);
}

fn note(f: &mut Frame, app: &App, area: Rect) {
    let Some((text, tone, born)) = &app.note else { return };
    let (mark, color) = match tone {
        Tone::Ok => ("✓", ACCENT),
        Tone::Info => ("i", INFO),
        Tone::Warn => ("!", WARN),
        Tone::Crit => ("✗", CRIT),
    };
    let width = (text.chars().count() as u16 + 6).min(area.width.saturating_sub(4));
    // Slides in from the right edge for 0.3 s.
    let slide = ((1.0 - born.elapsed().as_secs_f32() / 0.3).max(0.0) * f32::from(width)) as u16;
    let x = area.right().saturating_sub(width + 2) + slide;
    let r = Rect::new(x, area.bottom().saturating_sub(5), width - slide, 3).intersection(area);
    f.render_widget(Clear, r);
    let frame = Block::bordered().border_type(BorderType::Rounded).border_style(color).style(Style::new().bg(PANEL));
    f.render_widget(Paragraph::new(Line::from(vec![format!(" {mark} ").fg(color), text.clone().into()])).block(frame), r);
}
