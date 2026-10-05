//! hopla's state: the unlock, the palette, its dialogs, and what runs in
//! the background. Drawing lives in view.rs.

use std::process::Command;
use std::sync::mpsc::{self, Receiver, Sender, TryRecvError};
use std::thread;
use std::time::{Duration, Instant};

use arboard::{Clipboard, SetExtApple};
use ratatui::crossterm::event::{KeyCode, KeyEvent, KeyModifiers};
use tachyonfx::{Effect, Interpolation, fx};

use crate::palette::{self, Action, Item};
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

impl App {
    pub fn tick(&mut self) {
        if let Lock::Scan(_, rx) = &self.lock {
            match rx.try_recv() {
                Ok(Ok(())) => {
                    self.lock = Lock::Done(Instant::now());
                    (self.fails, self.alarm) = (0, None);
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
        let gate = if let Mode::Gate(ask, rx) = &self.mode { Some((ask.clone(), rx.try_recv())) } else { None };
        match gate {
            Some((ask, Ok(Ok(())))) => {
                self.mode = Mode::Browse;
                self.delete(ask);
            }
            Some((_, Ok(Err(e)))) => {
                self.mode = Mode::Browse;
                self.say(format!("kept: {e}"), Tone::Info);
            }
            Some((_, Err(TryRecvError::Disconnected))) => self.mode = Mode::Browse,
            _ => {}
        }
        if matches!(self.lock, Lock::Open) && self.last_key.elapsed() > IDLE {
            self.lock();
            self.say("locked after 5 min without a key", Tone::Info);
        }
        while let Ok(job) = self.rx.try_recv() {
            self.done(job);
        }
        if let Some(clip) = self.clip.take_if(|c| c.at.elapsed() > SHOWN) {
            self.say(format!("{} left the clipboard", clip.name), Tone::Info);
        }
        if self.note.as_ref().is_some_and(|n| n.2.elapsed() > NOTE_TTL) {
            self.note = None;
        }
    }

    /// A refused Touch ID; the third in a row sets off the alarm.
    fn refuse(&mut self, reason: String) {
        self.lock = Lock::Idle;
        self.fails += 1;
        if self.fails >= ALARM_AFTER {
            (self.fails, self.alarm) = (0, Some(Instant::now()));
        }
        self.say(format!("still locked: {reason}"), Tone::Crit);
    }

    fn open(&mut self) {
        self.lock = Lock::Open;
        self.last_key = Instant::now();
        self.fx = Some((fx::coalesce((180, Interpolation::QuadOut)), Instant::now()));
        if self.accounts.is_empty() {
            self.spawn("reading hop", || Job::Loaded(vault::load()));
        }
    }

    fn lock(&mut self) {
        self.lock = Lock::Idle;
        // Drops a value being typed: it never outlives the unlocked screen.
        self.mode = Mode::Browse;
        self.fx = None;
    }

    fn done(&mut self, job: Job) {
        if !matches!(job, Job::Checked(..)) {
            self.busy = None;
        }
        match job {
            Job::Loaded(Ok(accounts)) => {
                self.accounts = accounts;
                let names: Vec<(String, String)> =
                    self.accounts.iter().flat_map(|a| a.secrets.iter().map(|s| (a.name.clone(), s.name.clone()))).collect();
                let tx = self.tx.clone();
                thread::spawn(move || {
                    for (account, name) in names {
                        let set = vault::exists(&account, &name);
                        if tx.send(Job::Checked(account, name, set)).is_err() {
                            break;
                        }
                    }
                });
            }
            Job::Checked(account, name, Ok(set)) => {
                if let Some(s) = self.secret_mut(&account, &name) {
                    s.set = Some(set);
                }
            }
            Job::Value(_, name, Ok(Some(value))) => self.copy(name, value),
            Job::Value(account, name, Ok(None)) => self.say(format!("{name} is empty in {account}: Enter fills it"), Tone::Warn),
            Job::Stored(account, name, Ok(())) => {
                match self.secret_mut(&account, &name) {
                    Some(s) => s.set = Some(true),
                    None => {
                        if let Some(a) = self.accounts.iter_mut().find(|a| a.name == account) {
                            a.secrets.push(vault::Secret { name: name.clone(), set: Some(true) });
                        }
                    }
                }
                self.say(format!("{name} stored in {account}"), Tone::Ok);
            }
            Job::Deleted(account, name, Ok(())) => {
                if let Some(a) = self.accounts.iter_mut().find(|a| a.name == account) {
                    a.secrets.retain(|s| s.name != name);
                }
                self.sel = self.sel.min(self.items().len().saturating_sub(1));
                self.say(format!("{name} deleted from {account}"), Tone::Info);
            }
            Job::Switched(account, Ok(verdict)) => {
                let tone = if verdict.starts_with("OK") { Tone::Ok } else { Tone::Warn };
                self.active = Some(account);
                self.say(verdict, tone);
            }
            Job::Status(Ok(lines)) => self.mode = Mode::Status(lines),
            Job::Created(name, Ok(public_key)) => {
                let _ = Clipboard::new().and_then(|mut c| c.set_text(public_key));
                hop::open_keys_page();
                self.say(format!("{name} created · public key copied, paste it on GitHub"), Tone::Ok);
                let mut login = Command::new("gh");
                login.args(["auth", "login", "--hostname", "github.com", "--git-protocol", "ssh", "--web"]);
                self.suspend = Some(login);
                self.spawn("reading hop", || Job::Loaded(vault::load()));
            }
            Job::Removed(name, Ok(())) => {
                self.accounts.retain(|a| a.name != name);
                self.sel = 0;
                self.say(format!("{name} removed · its secrets stay in the Keychain"), Tone::Info);
            }
            Job::Loaded(Err(e))
            | Job::Checked(_, _, Err(e))
            | Job::Value(_, _, Err(e))
            | Job::Stored(_, _, Err(e))
            | Job::Deleted(_, _, Err(e))
            | Job::Switched(_, Err(e))
            | Job::Status(Err(e))
            | Job::Created(_, Err(e))
            | Job::Removed(_, Err(e)) => self.say(e, Tone::Crit),
        }
    }

    fn copy(&mut self, name: String, value: String) {
        // Let go of the previous copy first: it would empty the clipboard
        // again if both copies hold the same value.
        self.clip = None;
        match Clipboard::new().and_then(|mut c| c.set().exclude_from_history().text(value.clone())) {
            Ok(()) => {
                self.say(format!("copied {name} · gone in 30 s"), Tone::Ok);
                self.clip = Some(Clip { name, value, at: Instant::now() });
            }
            Err(e) => self.say(format!("clipboard: {e}"), Tone::Crit),
        }
    }
}

impl App {
    pub fn key(&mut self, key: KeyEvent) {
        let now = Instant::now();
        self.last_key = now;
        let ctrl = key.modifiers.contains(KeyModifiers::CONTROL);
        if ctrl && key.code == KeyCode::Char('c') {
            self.quit = true;
            return;
        }
        if !matches!(self.lock, Lock::Open) {
            match key.code {
                KeyCode::Enter | KeyCode::Char(' ') if matches!(self.lock, Lock::Idle) => self.lock = Lock::Scan(now, auth::unlock()),
                KeyCode::Esc | KeyCode::Char('q') => self.quit = true,
                _ => {}
            }
            return;
        }
        match std::mem::replace(&mut self.mode, Mode::Browse) {
            Mode::Browse => {}
            Mode::Form(form) => {
                self.mode = Mode::Form(form);
                return self.form_key(key);
            }
            Mode::Ask(ask) => {
                if key.code == KeyCode::Enter {
                    self.mode = Mode::Gate(ask, auth::unlock());
                }
                return;
            }
            gate @ Mode::Gate(..) => {
                self.mode = gate;
                return;
            }
            Mode::Status(_) => return,
        }
        match (ctrl, key.code) {
            (true, KeyCode::Char('l')) => self.lock(),
            (true, KeyCode::Char('b')) => self.switch_selected(),
            (true, KeyCode::Char('n')) => self.new_secret(),
            (true, KeyCode::Char('a')) => self.new_account(),
            (true, KeyCode::Char('e')) => self.refill(),
            (true, KeyCode::Char('d')) => self.ask_delete(),
            (true, KeyCode::Char('s')) => self.status(),
            (true, KeyCode::Char('u')) => self.retype(String::new()),
            (_, KeyCode::Up | KeyCode::BackTab) | (true, KeyCode::Char('p' | 'k')) => self.step(-1),
            (_, KeyCode::Down | KeyCode::Tab) | (true, KeyCode::Char('j')) => self.step(1),
            (_, KeyCode::Enter) => self.enter(),
            (_, KeyCode::Esc) if self.query.is_empty() => self.quit = true,
            (_, KeyCode::Esc) => self.retype(String::new()),
            (_, KeyCode::Backspace) => {
                let mut q = self.query.clone();
                q.pop();
                self.retype(q);
            }
            (false, KeyCode::Char(c)) => self.retype(format!("{}{c}", self.query)),
            _ => {}
        }
    }

    fn retype(&mut self, query: String) {
        self.query = query;
        self.sel = 0;
        (self.typed, self.picked) = (Instant::now(), Instant::now());
    }

    fn step(&mut self, d: isize) {
        let n = self.items().len() as isize;
        if n > 0 {
            self.sel = (self.sel as isize + d).rem_euclid(n) as usize;
            self.picked = Instant::now();
        }
    }

    /// Enter does the obvious thing for the row: switch to an account, copy
    /// a secret, fill an empty one, run an action.
    fn enter(&mut self) {
        match self.selected() {
            Some(Item::Account { name, active: false, .. }) => self.switch(name),
            Some(Item::Account { .. }) => self.status(),
            Some(Item::Secret { account, name, set: Some(true) }) => {
                self.spawn(format!("copying {name}"), move || {
                    let value = vault::read(&account, &name);
                    Job::Value(account, name, value)
                });
            }
            Some(Item::Secret { account, name, set: Some(false) }) => self.form_secret(account, Some(name)),
            Some(Item::Secret { .. }) => self.say("still asking the Keychain", Tone::Info),
            Some(Item::Action(Action::NewSecret(account))) => self.form_secret(account, None),
            Some(Item::Action(Action::NewAccount)) => self.new_account(),
            Some(Item::Action(Action::Status)) => self.status(),
            Some(Item::Action(Action::Remove(account))) => self.mode = Mode::Ask(Ask::Account(account)),
            None => {}
        }
    }

    fn switch_selected(&mut self) {
        if let Some(account) = self.selected().and_then(|i| i.account().map(str::to_owned)) {
            self.switch(account);
        }
    }

    fn switch(&mut self, account: String) {
        if self.active.as_deref() == Some(account.as_str()) {
            return self.say(format!("{account} is already active"), Tone::Info);
        }
        self.spawn(format!("switching to {account}"), move || {
            let verdict = hop::switch(&account);
            Job::Switched(account, verdict)
        });
    }

    fn status(&mut self) {
        self.spawn("asking GitHub", || Job::Status(hop::status()));
    }

    fn new_secret(&mut self) {
        let account = self.selected().and_then(|i| i.account().map(str::to_owned)).or_else(|| self.active.clone());
        if let Some(account) = account {
            self.form_secret(account, None);
        }
    }

    fn refill(&mut self) {
        if let Some(Item::Secret { account, name, .. }) = self.selected() {
            self.form_secret(account, Some(name));
        }
    }

    fn ask_delete(&mut self) {
        match self.selected() {
            Some(Item::Secret { account, name, .. }) => self.mode = Mode::Ask(Ask::Secret(account, name)),
            Some(Item::Account { name, active: false, .. }) => self.mode = Mode::Ask(Ask::Account(name)),
            Some(Item::Account { .. }) => self.say("switch to another account before removing this one", Tone::Warn),
            _ => {}
        }
    }

    /// After Touch ID.
    fn delete(&mut self, ask: Ask) {
        match ask {
            Ask::Secret(account, name) => self.spawn(format!("deleting {name}"), move || {
                let done = vault::remove(&account, &name);
                Job::Deleted(account, name, done)
            }),
            Ask::Account(name) => {
                let active = self.active.clone();
                self.spawn(format!("removing {name}"), move || {
                    let done = hop::remove(&name, active.as_deref());
                    Job::Removed(name, done)
                });
            }
        }
    }
}

impl App {
    fn form_secret(&mut self, account: String, name: Option<String>) {
        let mut fields = Vec::new();
        if name.is_none() {
            fields.push(Field { label: "name", value: String::new(), hidden: false });
        }
        fields.push(Field { label: "value", value: String::new(), hidden: true });
        self.mode = Mode::Form(Form { kind: FormKind::Secret { account, name }, fields, at: 0 });
    }

    fn new_account(&mut self) {
        let field = |label| Field { label, value: String::new(), hidden: false };
        let fields = vec![field("name"), field("email"), field("SSH key"), field("GitHub login")];
        self.mode = Mode::Form(Form { kind: FormKind::Account, fields, at: 0 });
    }

    fn form_key(&mut self, key: KeyEvent) {
        let Mode::Form(form) = &mut self.mode else { return };
        let last = form.at + 1 == form.fields.len();
        match key.code {
            KeyCode::Esc => self.mode = Mode::Browse,
            KeyCode::BackTab | KeyCode::Up => form.at = form.at.saturating_sub(1),
            KeyCode::Enter if last => self.submit(),
            KeyCode::Enter | KeyCode::Tab | KeyCode::Down if !last => {
                // One SSH key per account, named after it, unless typed.
                if matches!(form.kind, FormKind::Account) && form.at == 0 && form.fields[2].value.is_empty() {
                    form.fields[2].value = format!("~/.ssh/id_ed25519_{}", form.fields[0].value);
                }
                form.at += 1;
            }
            KeyCode::Backspace => _ = form.fields[form.at].value.pop(),
            KeyCode::Char(c) if !key.modifiers.contains(KeyModifiers::CONTROL) => {
                let field = &mut form.fields[form.at];
                let c = match (&form.kind, field.label) {
                    (FormKind::Secret { .. }, "name") => c.to_ascii_uppercase(),
                    (FormKind::Account, "name") => c.to_ascii_lowercase(),
                    _ => c,
                };
                field.value.push(c);
            }
            _ => {}
        }
    }

    /// Checks the form; stays open on a mistake, else runs it in the
    /// background.
    fn submit(&mut self) {
        let Mode::Form(form) = &self.mode else { return };
        let v = |i: usize| form.fields[i].value.clone();
        match &form.kind {
            FormKind::Secret { account, name } => {
                let (name, value) = match name {
                    Some(name) => (name.clone(), v(0)),
                    None => (v(0), v(1)),
                };
                if !vault::valid_name(&name) {
                    return self.say("a name is UPPERCASE_AND_DIGITS", Tone::Warn);
                }
                if !vault::valid_value(&value) {
                    return self.say("empty, or a quote, backslash or space: not stored", Tone::Warn);
                }
                let account = account.clone();
                self.mode = Mode::Browse;
                self.spawn(format!("storing {name}"), move || {
                    let done = vault::store(&account, &name, &value);
                    Job::Stored(account, name, done)
                });
            }
            FormKind::Account => {
                let new = hop::NewAccount { name: v(0), email: v(1), key: v(2), login: v(3) };
                if let Err(e) = hop::valid(&new) {
                    return self.say(e, Tone::Warn);
                }
                self.mode = Mode::Browse;
                self.spawn(format!("creating {}", new.name), move || {
                    let done = hop::create(&new);
                    Job::Created(new.name, done)
                });
            }
        }
    }
}
