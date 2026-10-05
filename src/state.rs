//! hopla's state: the unlock, the palette, its dialogs, and what runs in
//! the background. Drawing lives in view.rs.

use std::process::Command;
use std::sync::mpsc::{self, Receiver, Sender, TryRecvError};
use std::thread;
use std::time::{Duration, Instant};

use arboard::{Clipboard, SetExtApple};
use tachyonfx::{Effect, Interpolation, fx};

use crate::palette::{self, Item};
use crate::vault::{self, Account};
use crate::{auth, hop};

pub const SHOWN: Duration = Duration::from_secs(30);
pub const NAP: Duration = Duration::from_secs(20);
const IDLE: Duration = Duration::from_secs(5 * 60);
const NOTE_TTL: Duration = Duration::from_millis(3000);
// The check mark on the lock screen before the palette.
const PASSED: Duration = Duration::from_millis(400);
const ALARM_AFTER: u8 = 3;

pub enum Lock {
    Idle,
    Scan(Instant, Receiver<Result<(), String>>),
    Done(Instant),
    Open,
}

#[derive(Clone, Copy, PartialEq)]
pub enum Tone {
    Ok,
    Info,
    Warn,
    Crit,
}

pub struct Field {
    pub label: &'static str,
    pub value: String,
    pub hidden: bool,
}

pub enum FormKind {
    /// A value for a secret of this account; the name is the first field
    /// unless it is already known.
    Secret { account: String, name: Option<String> },
    Account,
}

pub struct Form {
    pub kind: FormKind,
    pub fields: Vec<Field>,
    pub at: usize,
}

/// A deletion waiting for Enter, then for Touch ID.
#[derive(Clone)]
pub enum Ask {
    Secret(String, String),
    Account(String),
}

pub enum Mode {
    Browse,
    Form(Form),
    Ask(Ask),
    Gate(Ask, Receiver<Result<(), String>>),
    Status(Vec<String>),
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

enum Job {
    Loaded(Result<Vec<Account>, String>),
    Checked(String, String, Result<bool, String>),
    Value(String, String, Result<Option<String>, String>),
    Stored(String, String, Result<(), String>),
    Deleted(String, String, Result<(), String>),
    Switched(String, Result<String, String>),
    Status(Result<Vec<String>, String>),
    Created(String, Result<String, String>),
    Removed(String, Result<(), String>),
}

pub struct App {
    pub lock: Lock,
    pub start: Instant,
    pub accounts: Vec<Account>,
    pub active: Option<String>,
    pub query: String,
    pub sel: usize,
    pub mode: Mode,
    pub note: Option<(String, Tone, Instant)>,
    pub clip: Option<Clip>,
    /// What runs in the background, for the spinner.
    pub busy: Option<String>,
    /// When the selection and the query last changed, for the transitions.
    pub picked: Instant,
    pub typed: Instant,
    pub alarm: Option<Instant>,
    /// A command that needs the whole terminal, such as `gh auth login`.
    pub suspend: Option<Command>,
    pub fx: Option<(Effect, Instant)>,
    pub quit: bool,
    pub last_key: Instant,
    fails: u8,
    tx: Sender<Job>,
    rx: Receiver<Job>,
}

impl App {
    pub fn new() -> Self {
        let (tx, rx) = mpsc::channel();
        let now = Instant::now();
        Self {
            lock: Lock::Idle,
            start: now,
            accounts: Vec::new(),
            active: vault::active(),
            query: String::new(),
            sel: 0,
            mode: Mode::Browse,
            note: None,
            clip: None,
            busy: None,
            picked: now,
            typed: now,
            alarm: None,
            suspend: None,
            fx: None,
            quit: false,
            last_key: now,
            fails: 0,
            tx,
            rx,
        }
    }

    pub fn items(&self) -> Vec<Item> {
        palette::items(&self.accounts, self.active.as_deref(), &self.query)
    }

    pub fn selected(&self) -> Option<Item> {
        self.items().into_iter().nth(self.sel)
    }

    pub fn asleep(&self) -> bool {
        matches!(self.lock, Lock::Open) && matches!(self.mode, Mode::Browse) && self.last_key.elapsed() > NAP
    }

    pub fn say(&mut self, text: impl Into<String>, tone: Tone) {
        self.note = Some((text.into(), tone, Instant::now()));
    }

    fn spawn(&mut self, what: impl Into<String>, work: impl FnOnce() -> Job + Send + 'static) {
        self.busy = Some(what.into());
        let tx = self.tx.clone();
        thread::spawn(move || tx.send(work()));
    }

    fn secret_mut(&mut self, account: &str, name: &str) -> Option<&mut vault::Secret> {
        self.accounts.iter_mut().find(|a| a.name == account)?.secrets.iter_mut().find(|s| s.name == name)
    }
}
