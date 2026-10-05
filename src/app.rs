//! What hopla holds and how keys change it. Drawing lives in ui.rs.

use std::sync::mpsc::{self, Receiver, Sender, TryRecvError};
use std::thread;
use std::time::{Duration, Instant};

use arboard::{Clipboard, SetExtApple};
use ratatui::crossterm::event::{KeyCode, KeyEvent, KeyModifiers};
use tachyonfx::{Effect, Interpolation, fx};

use crate::auth;
use crate::vault::{self, Account, Secret};

const NOTE_TTL: Duration = Duration::from_millis(3500);
pub const SHOWN: Duration = Duration::from_secs(30);
const IDLE: Duration = Duration::from_secs(5 * 60);
// Time the lock screen shows its check mark before the vault.
const PASSED: Duration = Duration::from_millis(550);

#[derive(PartialEq)]
pub enum Focus {
    Accounts,
    Secrets,
}

pub enum Lock {
    Idle,
    Scan(Instant, Receiver<Result<(), String>>),
    Done(Instant),
    Open,
}

#[derive(Clone, Copy)]
pub enum Tone {
    Ok,
    Info,
    Warn,
    Crit,
}

#[derive(Clone, Copy)]
enum Use {
    Reveal,
    Copy,
}

enum Job {
    Loaded(Result<Vec<Account>, String>),
    Value(Use, String, String, Result<Option<String>, String>),
}

pub struct Reveal {
    pub account: String,
    pub name: String,
    pub value: String,
    pub at: Instant,
}

/// A copied value. Dropping it, on time out or on exit, empties the
/// clipboard only if it still holds that value.
pub struct Clip {
    pub name: String,
    value: String,
    pub at: Instant,
}

impl Drop for Clip {
    fn drop(&mut self) {
        if let Ok(mut clipboard) = Clipboard::new()
            && clipboard.get_text().is_ok_and(|text| text == self.value)
        {
            let _ = clipboard.clear();
        }
    }
}

pub struct App {
    pub lock: Lock,
    pub start: Instant,
    pub accounts: Vec<Account>,
    pub active: Option<String>,
    pub loading: bool,
    pub account: usize,
    pub sel: usize,
    pub focus: Focus,
    pub filter: String,
    pub filtering: bool,
    pub note: Option<(String, Tone, Instant)>,
    pub reveal: Option<Reveal>,
    pub clip: Option<Clip>,
    pub reading: Option<(String, String)>,
    pub quit: bool,
    /// The unlock effect and when it last ran.
    pub fx: Option<(Effect, Instant)>,
    last_key: Instant,
    tx: Sender<Job>,
    rx: Receiver<Job>,
}

impl App {
    pub fn new() -> Self {
        let (tx, rx) = mpsc::channel();
        Self {
            lock: Lock::Idle,
            start: Instant::now(),
            accounts: Vec::new(),
            active: vault::active(),
            loading: false,
            account: 0,
            sel: 0,
            focus: Focus::Secrets,
            filter: String::new(),
            filtering: false,
            note: None,
            reveal: None,
            clip: None,
            reading: None,
            quit: false,
            fx: None,
            last_key: Instant::now(),
            tx,
            rx,
        }
    }

    pub fn account(&self) -> Option<&Account> {
        self.accounts.get(self.account)
    }

    /// The current account's secrets that match the filter, in hop's order.
    pub fn rows(&self) -> Vec<&Secret> {
        let q = self.filter.to_lowercase();
        self.account()
            .map(|a| a.secrets.iter().filter(|s| s.name.to_lowercase().contains(&q)).collect())
            .unwrap_or_default()
    }

    pub fn selected(&self) -> Option<&Secret> {
        self.rows().get(self.sel).copied()
    }

    fn say(&mut self, text: impl Into<String>, tone: Tone) {
        self.note = Some((text.into(), tone, Instant::now()));
    }

    pub fn tick(&mut self) {
        if let Lock::Scan(_, rx) = &self.lock {
            match rx.try_recv() {
                Ok(Ok(())) => self.lock = Lock::Done(Instant::now()),
                Ok(Err(e)) => self.refuse(e),
                Err(TryRecvError::Disconnected) => self.refuse("no answer".into()),
                Err(TryRecvError::Empty) => {}
            }
        }
        if let Lock::Done(at) = self.lock
            && at.elapsed() > PASSED
        {
            self.open();
        }
        if matches!(self.lock, Lock::Open) && self.last_key.elapsed() > IDLE {
            self.lock();
            self.say("locked after 5 min without a key", Tone::Info);
        }
        while let Ok(job) = self.rx.try_recv() {
            match job {
                Job::Loaded(Ok(accounts)) => {
                    self.account = accounts.iter().position(|a| Some(&a.name) == self.active.as_ref()).unwrap_or(0);
                    self.accounts = accounts;
                    self.loading = false;
                }
                Job::Loaded(Err(e)) => {
                    self.loading = false;
                    self.say(e, Tone::Crit);
                }
                Job::Value(how, account, name, value) => {
                    self.reading = None;
                    match value {
                        Ok(Some(value)) if matches!(how, Use::Reveal) => {
                            self.reveal = Some(Reveal { account, name, value, at: Instant::now() });
                        }
                        Ok(Some(value)) => self.copy(name, value),
                        Ok(None) => self.say(format!("missing · hop secret set {name} {account}"), Tone::Warn),
                        Err(e) => self.say(e, Tone::Crit),
                    }
                }
            }
        }
        if self.reveal.as_ref().is_some_and(|r| r.at.elapsed() > SHOWN) {
            self.reveal = None;
        }
        if let Some(clip) = self.clip.take_if(|c| c.at.elapsed() > SHOWN) {
            self.say(format!("{} left the clipboard", clip.name), Tone::Info);
        }
        if self.note.as_ref().is_some_and(|n| n.2.elapsed() > NOTE_TTL) {
            self.note = None;
        }
    }

    fn refuse(&mut self, reason: String) {
        self.lock = Lock::Idle;
        self.say(format!("still locked: {reason}"), Tone::Crit);
    }

    fn lock(&mut self) {
        self.lock = Lock::Idle;
        self.reveal = None;
        self.fx = None;
    }

    fn open(&mut self) {
        self.lock = Lock::Open;
        self.last_key = Instant::now();
        self.fx = Some((fx::coalesce((600, Interpolation::QuadOut)), Instant::now()));
        self.say("unlocked · L locks again", Tone::Ok);
        if self.accounts.is_empty() && !self.loading {
            self.loading = true;
            let tx = self.tx.clone();
            thread::spawn(move || tx.send(Job::Loaded(vault::load())));
        }
    }

    pub fn key(&mut self, key: KeyEvent) {
        self.last_key = Instant::now();
        if key.modifiers.contains(KeyModifiers::CONTROL) && key.code == KeyCode::Char('c') {
            self.quit = true;
            return;
        }
        if !matches!(self.lock, Lock::Open) {
            match key.code {
                KeyCode::Enter | KeyCode::Char(' ') if matches!(self.lock, Lock::Idle) => {
                    self.lock = Lock::Scan(Instant::now(), auth::unlock());
                }
                KeyCode::Char('q') => self.quit = true,
                _ => {}
            }
            return;
        }
        if self.filtering {
            match key.code {
                KeyCode::Esc => {
                    self.filter.clear();
                    self.filtering = false;
                }
                KeyCode::Enter => self.filtering = false,
                KeyCode::Backspace => _ = self.filter.pop(),
                KeyCode::Down => self.step(1),
                KeyCode::Up => self.step(-1),
                KeyCode::Char(c) => {
                    self.filter.push(c);
                    self.sel = 0;
                }
                _ => {}
            }
            return;
        }
        match key.code {
            KeyCode::Char('q') => self.quit = true,
            KeyCode::Tab | KeyCode::BackTab => {
                self.focus = if self.focus == Focus::Accounts { Focus::Secrets } else { Focus::Accounts };
            }
            KeyCode::Char('j') | KeyCode::Down => self.step(1),
            KeyCode::Char('k') | KeyCode::Up => self.step(-1),
            KeyCode::Char('/') => {
                self.filtering = true;
                self.filter.clear();
                self.focus = Focus::Secrets;
                self.sel = 0;
            }
            KeyCode::Esc => self.filter.clear(),
            KeyCode::Enter if self.focus == Focus::Accounts => self.focus = Focus::Secrets,
            KeyCode::Enter | KeyCode::Char('r') => self.fetch(Use::Reveal),
            KeyCode::Char('y') => self.fetch(Use::Copy),
            KeyCode::Char('L') => self.lock(),
            _ => {}
        }
    }

    fn step(&mut self, d: isize) {
        let move_by = |i: usize, len: usize| i.saturating_add_signed(d).min(len.saturating_sub(1));
        if self.focus == Focus::Accounts {
            self.account = move_by(self.account, self.accounts.len());
            self.sel = 0;
        } else {
            self.sel = move_by(self.sel, self.rows().len());
        }
    }

    /// `r` and `y` on the selected secret. A second `r` hides it again.
    fn fetch(&mut self, how: Use) {
        let (Some(account), Some(secret)) = (self.account(), self.selected()) else { return };
        let (account, name, set) = (account.name.clone(), secret.name.clone(), secret.set);
        if !set {
            return self.say(format!("missing · hop secret set {name} {account}"), Tone::Warn);
        }
        let shown = self.reveal.as_ref().is_some_and(|r| r.account == account && r.name == name);
        if matches!(how, Use::Reveal) && shown {
            self.reveal = None;
            return;
        }
        self.reading = Some((account.clone(), name.clone()));
        let tx = self.tx.clone();
        thread::spawn(move || {
            let value = vault::read(&account, &name);
            tx.send(Job::Value(how, account, name, value))
        });
    }

    fn copy(&mut self, name: String, value: String) {
        // Let go of the previous copy first: it would empty the clipboard
        // again if both copies hold the same value.
        self.clip = None;
        match Clipboard::new().and_then(|mut c| c.set().exclude_from_history().text(value.clone())) {
            Ok(()) => {
                self.say(format!("copied {name} · cleared in 30 s"), Tone::Ok);
                self.clip = Some(Clip { name, value, at: Instant::now() });
            }
            Err(e) => self.say(format!("clipboard: {e}"), Tone::Crit),
        }
    }
}
