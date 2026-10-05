//! What hopla holds and how keys change it. Drawing lives in ui.rs.

use std::sync::mpsc::{self, Receiver, Sender, TryRecvError};
use std::thread;
use std::time::{Duration, Instant};

use arboard::{Clipboard, SetExtApple};
use ratatui::crossterm::event::{KeyCode, KeyEvent, KeyModifiers};
use tachyonfx::{Effect, Interpolation, fx};

use crate::auth;
use crate::surprise::{Fun, NAP};
use crate::vault::{self, Account, Secret};

const NOTE_TTL: Duration = Duration::from_millis(3500);
pub const SHOWN: Duration = Duration::from_secs(30);
const IDLE: Duration = Duration::from_secs(5 * 60);
// Time the lock screen shows its check mark, and Lapla flips, before the vault.
const PASSED: Duration = Duration::from_millis(900);

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

/// What the keys do in the vault room.
pub enum Mode {
    Browse,
    Filter,
    /// Typing a value for `name`; `naming` while a new box gets its name.
    Deposit { name: String, value: String, naming: bool },
    /// Waiting for `y` before emptying the selected box.
    Confirm,
}

/// Lapla walking to the next room: from which account, which way.
pub struct Travel {
    pub from: usize,
    pub dir: i32,
    pub at: Instant,
}

#[derive(Clone, Copy)]
pub enum Tone {
    Ok,
    Info,
    Warn,
    Crit,
}

enum Job {
    Loaded(Result<Vec<Account>, String>),
    Value(String, String, Result<Option<String>, String>),
    Stored(String, String, Result<(), String>),
    Removed(String, String, Result<(), String>),
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
    pub clip: Option<Clip>,
    pub reading: Option<(String, String)>,
    pub mode: Mode,
    pub travel: Option<Travel>,
    /// The slot machine of the last copy: which secret, since when.
    pub spin: Option<(String, Instant)>,
    pub fun: Fun,
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
            clip: None,
            reading: None,
            mode: Mode::Browse,
            travel: None,
            spin: None,
            fun: Fun::default(),
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

    /// Lapla naps when the vault room has had no key for a while.
    pub fn asleep(&self) -> bool {
        matches!(self.lock, Lock::Open) && matches!(self.mode, Mode::Browse) && self.last_key.elapsed() > NAP
    }

    fn say(&mut self, text: impl Into<String>, tone: Tone) {
        self.note = Some((text.into(), tone, Instant::now()));
    }

    pub fn tick(&mut self) {
        if let Lock::Scan(_, rx) = &self.lock {
            match rx.try_recv() {
                Ok(Ok(())) => {
                    self.lock = Lock::Done(Instant::now());
                    self.fun.passed();
                }
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
                Job::Value(account, name, value) => {
                    self.reading = None;
                    match value {
                        Ok(Some(value)) => self.copy(name, value),
                        Ok(None) => self.say(format!("missing · hop secret set {name} {account}"), Tone::Warn),
                        Err(e) => self.say(e, Tone::Crit),
                    }
                }
                Job::Stored(account, name, done) => match done {
                    Ok(()) => {
                        if let Some(a) = self.accounts.iter_mut().find(|a| a.name == account) {
                            match a.secrets.iter_mut().find(|x| x.name == name) {
                                Some(x) => x.set = true,
                                None => a.secrets.push(Secret { name: name.clone(), set: true }),
                            }
                        }
                        self.say(format!("{name} is in the vault"), Tone::Ok);
                    }
                    Err(e) => self.say(e, Tone::Crit),
                },
                Job::Removed(account, name, done) => match done {
                    Ok(()) => {
                        if let Some(a) = self.accounts.iter_mut().find(|a| a.name == account) {
                            a.secrets.retain(|x| x.name != name);
                        }
                        self.sel = self.sel.min(self.rows().len().saturating_sub(1));
                        self.say(format!("box {name} emptied"), Tone::Info);
                    }
                    Err(e) => self.say(e, Tone::Crit),
                },
            }
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
        self.fun.refused();
        self.say(format!("still locked: {reason}"), Tone::Crit);
    }

    fn lock(&mut self) {
        self.lock = Lock::Idle;
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
            KeyCode::Char('y') => self.copy_selected(),
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

    /// `y`: reads the selected secret off screen and copies it. A value is
    /// never drawn, only copied.
    fn copy_selected(&mut self) {
        let (Some(account), Some(secret)) = (self.account(), self.selected()) else { return };
        let (account, name, set) = (account.name.clone(), secret.name.clone(), secret.set);
        if !set {
            return self.say(format!("missing · hop secret set {name} {account}"), Tone::Warn);
        }
        self.reading = Some((account.clone(), name.clone()));
        let tx = self.tx.clone();
        thread::spawn(move || {
            let value = vault::read(&account, &name);
            tx.send(Job::Value(account, name, value))
        });
    }

    fn copy(&mut self, name: String, value: String) {
        // Let go of the previous copy first: it would empty the clipboard
        // again if both copies hold the same value.
        self.clip = None;
        match Clipboard::new().and_then(|mut c| c.set().exclude_from_history().text(value.clone())) {
            Ok(()) => {
                self.say(format!("copied {name} · cleared in 30 s"), Tone::Ok);
                self.spin = Some((name.clone(), Instant::now()));
                self.clip = Some(Clip { name, value, at: Instant::now() });
            }
            Err(e) => self.say(format!("clipboard: {e}"), Tone::Crit),
        }
    }

    /// Keys of the deposit and confirm dialogs.
    fn dialog(&mut self, code: KeyCode) {
        if let Mode::Confirm = self.mode {
            self.mode = Mode::Browse;
            if code == KeyCode::Char('y') {
                self.write(None);
            }
            return;
        }
        let Mode::Deposit { name, value, naming } = &mut self.mode else { return };
        let field = if *naming { &mut *name } else { &mut *value };
        match code {
            KeyCode::Esc => self.mode = Mode::Browse,
            KeyCode::Backspace => _ = field.pop(),
            KeyCode::Char(c) if *naming && (c.is_ascii_alphanumeric() || c == '_') => field.push(c.to_ascii_uppercase()),
            KeyCode::Char(c) if !*naming => field.push(c),
            KeyCode::Enter if *naming && vault::valid_name(name) => *naming = false,
            KeyCode::Enter if *naming => self.say("a name is UPPERCASE_AND_DIGITS", Tone::Warn),
            KeyCode::Enter if vault::valid_value(value) => {
                let value = std::mem::take(value);
                let name = std::mem::take(name);
                self.mode = Mode::Browse;
                self.write(Some((name, value)));
            }
            KeyCode::Enter => self.say("empty, or a quote, backslash or space: not stored", Tone::Warn),
            _ => {}
        }
    }

    /// Stores `(name, value)` in the current account, or empties the
    /// selected box when `None`.
    fn write(&mut self, deposit: Option<(String, String)>) {
        let Some(account) = self.account().map(|a| a.name.clone()) else { return };
        let tx = self.tx.clone();
        match deposit {
            Some((name, value)) => {
                thread::spawn(move || {
                    let done = vault::store(&account, &name, &value);
                    tx.send(Job::Stored(account, name, done))
                });
            }
            None => {
                let Some(name) = self.selected().map(|s| s.name.clone()) else { return };
                thread::spawn(move || {
                    let done = vault::remove(&account, &name);
                    tx.send(Job::Removed(account, name, done))
                });
            }
        }
    }
}
