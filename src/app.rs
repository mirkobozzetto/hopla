//! What hopla holds and how keys change it. Drawing lives in ui.rs.

use std::sync::mpsc::{self, Receiver, Sender, TryRecvError};
use std::thread;
use std::time::{Duration, Instant};

use ratatui::crossterm::event::{KeyCode, KeyEvent, KeyModifiers};

use crate::auth;
use crate::vault::{self, Account, Secret};

const NOTE_TTL: Duration = Duration::from_millis(3500);

#[derive(PartialEq)]
pub enum Focus {
    Accounts,
    Secrets,
}

pub enum Lock {
    Idle,
    Scan(Instant, Receiver<Result<(), String>>),
    Open,
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
    pub quit: bool,
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
            quit: false,
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
                Ok(Ok(())) => self.open(),
                Ok(Err(e)) => self.refuse(e),
                Err(TryRecvError::Disconnected) => self.refuse("no answer".into()),
                Err(TryRecvError::Empty) => {}
            }
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
            }
        }
        if self.note.as_ref().is_some_and(|n| n.2.elapsed() > NOTE_TTL) {
            self.note = None;
        }
    }

    fn refuse(&mut self, reason: String) {
        self.lock = Lock::Idle;
        self.say(format!("still locked: {reason}"), Tone::Crit);
    }

    fn open(&mut self) {
        self.lock = Lock::Open;
        self.say("unlocked · L locks again", Tone::Ok);
        if self.accounts.is_empty() && !self.loading {
            self.loading = true;
            let tx = self.tx.clone();
            thread::spawn(move || tx.send(Job::Loaded(vault::load())));
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

    /// `r` and `y` on the selected secret.
    fn fetch(&mut self) {
        let (Some(account), Some(secret)) = (self.account(), self.selected()) else { return };
        if !secret.set {
            let hint = format!("missing · hop secret set {} {}", secret.name, account.name);
            self.say(hint, Tone::Warn);
        }
    }
}
