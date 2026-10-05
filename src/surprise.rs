//! Lapla's surprises: typing "hopla", the Konami code, his nap, the alarm.
//! Each one is the moment it started; the scenes animate from there.

use std::time::{Duration, Instant};

use ratatui::crossterm::event::KeyCode::{self, Char, Down, Left, Right, Up};

const KONAMI: [KeyCode; 10] = [Up, Up, Down, Down, Left, Right, Left, Right, Char('b'), Char('a')];
const HOPLA: [KeyCode; 5] = [Char('h'), Char('o'), Char('p'), Char('l'), Char('a')];
/// Lapla falls asleep after this long without a key.
pub const NAP: Duration = Duration::from_secs(20);
const ALARM_AFTER: u8 = 3;

#[derive(Default)]
pub struct Fun {
    keys: Vec<KeyCode>,
    fails: u8,
    pub flip: Option<Instant>,
    pub rain: Option<Instant>,
    pub woke: Option<Instant>,
    pub alarm: Option<Instant>,
}

impl Fun {
    /// Watches the keys pressed in the vault room.
    pub fn key(&mut self, code: KeyCode, asleep: bool) {
        let now = Instant::now();
        if asleep {
            self.woke = Some(now);
        }
        self.keys.push(code);
        if self.keys.len() > KONAMI.len() {
            self.keys.remove(0);
        }
        if self.keys.ends_with(&KONAMI) {
            self.rain = Some(now);
            self.keys.clear();
        } else if self.keys.ends_with(&HOPLA) {
            self.flip = Some(now);
        }
    }

    /// A refused Touch ID; the third in a row sets off the alarm.
    pub fn refused(&mut self) {
        self.fails += 1;
        if self.fails >= ALARM_AFTER {
            self.fails = 0;
            self.alarm = Some(Instant::now());
        }
    }

    pub fn passed(&mut self) {
        self.fails = 0;
        self.alarm = None;
    }
}
